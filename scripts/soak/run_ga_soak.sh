#!/usr/bin/env bash
# =============================================================================
# run_ga_soak.sh — run the GA-2 mixed-workload SOAK and validate its artifact
# =============================================================================
# #5491: the GA-2 harness (`tests/soak/v312_mixed_soak.rs`) advertises
# itself as "nightly-only via scripts/gate/run_ga_soak.sh" — in its own
# `#[ignore]` reason string — and **that script did not exist**. The 1h demo
# therefore had no caller, and the summary it writes had no consumer.
#
# This script is that caller. It:
#
#   1. runs the harness (the 60s smoke, or the 1h demo);
#   2. hands the produced summary to the ONE implementation of the GA-2
#      artifact rule — scripts/gate/check_ga2_soak_artifact.sh;
#   3. exits 0 only if the run succeeded AND the artifact validates.
#
# It does NOT decide whether a SOAK "counts" for a release — that is the
# gate's job, and the gate's job is the artifact validator
# (scripts/gate/check_ga2_soak_artifact.sh). This script only makes the
# artifact exist and proves it is good.
#
# WHY THIS LIVES IN scripts/soak/ AND NOT IN scripts/gate/
# ------------------------------------------------------
# ADR-008 §Policy 2 ("No-Ignore Gate Tests") forbids a **gate script** from
# invoking a test target that carries an `#[ignore]`, because such an
# invocation runs no test by default and the gate would then claim a result
# it never earned. That rule is enforced over `scripts/gate/*.sh` and the CI
# workflows — the places where a gate lives.
#
# This file is NOT a gate: it **produces** the SOAK artifact, and the gate
# that judges that artifact is scripts/gate/check_ga2_soak_artifact.sh,
# which contains no `cargo test --test` reference at all. Every other SOAK
# runner in this repository lives next to it (run_168h_ci.sh,
# run_soak_loop.sh, v400_1h_soak.sh, …), so this is where the file belongs
# on its own terms — and after the move, no gate script in the tree
# references an ignored test target.
#
# The artifact is written OUTSIDE the repository tree by default. A run that
# dropped a generated file into `docs/` would leave the working tree dirty
# and would overwrite whatever was committed there — the shape of #5577.
#
# Usage:
#   run_ga_soak.sh [--smoke | --demo] [--output <file>] [--keep]
#   run_ga_soak.sh --validate-only <file> [--max-age-days N]
#
#   --smoke          run the 60s smoke (default; the harness's non-ignored test)
#   --demo           run the 1h demo (the harness's --ignored test)
#   --output <file>  where to write the artifact (default: a temp dir)
#   --keep           keep the temporary output directory
#   --validate-only  validate an existing artifact and exit
#
# Exit codes:
#   0 = the run succeeded and the artifact validates
#   1 = the run failed, or the artifact does not validate
#   2 = usage error
# =============================================================================
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

VALIDATOR="$SCRIPT_DIR/../gate/check_ga2_soak_artifact.sh"

MODE="smoke"
OUTPUT=""
KEEP=0
VALIDATE_ONLY=""
MAX_AGE_DAYS=30

usage() {
    sed -n '2,42p' "$0" | sed 's/^# \{0,1\}//'
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --smoke) MODE="smoke"; shift ;;
        --demo) MODE="demo"; shift ;;
        --output) OUTPUT="${2:-}"; shift 2 ;;
        --keep) KEEP=1; shift ;;
        --validate-only) VALIDATE_ONLY="${2:-}"; shift 2 ;;
        --max-age-days) MAX_AGE_DAYS="${2:-}"; shift 2 ;;
        -h|--help) usage; exit 2 ;;
        *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
    esac
done

if [ ! -f "$VALIDATOR" ]; then
    echo "ERROR: the GA-2 artifact validator is missing: $VALIDATOR" >&2
    exit 2
fi

# --- validate-only mode -----------------------------------------------------
if [ -n "$VALIDATE_ONLY" ]; then
    out=$(bash "$VALIDATOR" --artifact "$VALIDATE_ONLY" --max-age-days "$MAX_AGE_DAYS")
    rc=$?
    echo "$out"
    exit $rc
fi

# --- run mode ---------------------------------------------------------------
TMPDIR_OUT=""
if [ -z "$OUTPUT" ]; then
    TMPDIR_OUT="$(mktemp -d "${TMPDIR:-/tmp}/sqlrustgo-ga-soak.XXXXXX")" || {
        echo "ERROR: cannot create a temporary output directory" >&2
        exit 2
    }
    OUTPUT="$TMPDIR_OUT/v312_mixed_soak_summary.txt"
fi

cleanup() {
    if [ -n "$TMPDIR_OUT" ] && [ "$KEEP" -eq 0 ]; then
        rm -rf "$TMPDIR_OUT"
    elif [ -n "$TMPDIR_OUT" ]; then
        echo "artifact kept at: $OUTPUT"
    fi
}
trap cleanup EXIT

echo "GA-2 mixed-workload SOAK — mode=$MODE"
echo "  artifact: $OUTPUT"

export SQLRUSTGO_SOAK_SUMMARY_PATH="$OUTPUT"

if [ "$MODE" = "demo" ]; then
    echo "  running the 1h demo (this takes an hour) ..."
    cargo test --test v312_mixed_soak -- --ignored --nocapture v312_ga_mixed_workload_demo
else
    echo "  running the 60s smoke ..."
    cargo test --test v312_mixed_soak -- --nocapture v312_ga_mixed_workload_smoke_60s
fi
RUN_RC=$?

if [ "$RUN_RC" -ne 0 ]; then
    echo "ERROR: the SOAK harness itself failed (exit $RUN_RC)" >&2
    exit 1
fi

if [ ! -s "$OUTPUT" ]; then
    echo "ERROR: the harness produced no artifact at $OUTPUT" >&2
    exit 1
fi

echo "--- artifact ---"
cat "$OUTPUT"
echo "--- validation ---"
out=$(bash "$VALIDATOR" --artifact "$OUTPUT" --max-age-days "$MAX_AGE_DAYS")
rc=$?
echo "$out"
exit $rc
