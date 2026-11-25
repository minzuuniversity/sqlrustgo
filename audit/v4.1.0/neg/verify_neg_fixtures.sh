#!/usr/bin/env bash
# verify_neg_fixtures.sh — the two-layer RED proof for the seven ALPHA stage
# gates of v4.1.0 (Task 8 of the readiness plan).
#
# WHAT IT PROVES, AND HOW
# ------------------------
# For every ALPHA gate the plan lists (docs/governance/STAGE_CONFIG.yaml ->
# stages.ALPHA.required_gates -> the seven check_*.sh entries), this script
# runs the gate's negative fixture and reads TWO verdicts off ONE execution:
#
#   gate=<RED|INCONCLUSIVE>   the gate layer — the target gate's own row in
#                             the results.jsonl of one `run_gates.py execute`
#                             run says state=FAIL with a non-zero exit code
#   stage=<RED|INCONCLUSIVE>  the stage layer — `gate_report.py report`
#                             re-parses the SAME results.jsonl independently
#                             and concludes FAIL (not merely "not PASS")
#
# A timeout is NOT a red (R50, issue #5598): exit 124 and signals are
# INCOMPLETE and are reported as INCONCLUSIVE.  This script never rounds a
# partial result up to 7/7 — see the exit codes below.
#
# WARMUP (R50)
# -----------
# Before the lane runs, the scratch tree's target is warmed
# (`cargo build --all-features && cargo test --all-features --lib --no-run`).
# The warmup is UNCONDITIONAL: there is no environment switch to skip it,
# because a switch to skip the warmup is a switch to skip the lane, and a
# skipped lane must never be indistinguishable from a lane that ran.  Its
# output goes to its own log (warmup.log in the log directory) and a failure
# aborts the lane with exit 4 (setup failure) — never a silent skip and
# never a green.
#
# TREE IDENTITY
# -------------
# The lane prints its own identity on the line
#     verify_neg_fixtures: tree_sha=<40 hex> dirty_files=<n>
# `dirty_files` is part of the identity and not a nicety: when the scratch
# tree is NOT the repository, the dirty check is skipped, so a modified tree
# shares its commit's SHA with a pristine one.  Only the pair identifies the
# measured tree.
#
# EXIT CODES
# ----------
#   0  all seven gates are RED on BOTH layers  (7/7)
#   3  at least one gate is not RED on both layers — the summary line names
#      how many, and every gate's line says whether it was RED or
#      INCONCLUSIVE.  A non-zero exit here is the point: it is the machine
#      readable form of "this did not reach 7/7".
#   4  the run could not be set up (dirty tree, missing fixture, failed
#      warmup, ...)
#
# USAGE
# -----
#   bash audit/v4.1.0/neg/verify_neg_fixtures.sh [scratch-tree]
#
# With no argument the repository itself is used as the scratch tree — the
# fixtures mutate it and restore it after every run.  Because the restore
# step is `git checkout -- . && git clean -qfd`, an UNCOMMITTED working tree
# would lose work; the script therefore refuses to start on a dirty tree
# unless it is given an explicit scratch tree.

set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$REPO" || exit 4

TREE="${1:-$REPO}"
if [ "$TREE" = "$REPO" ]; then
    if [ -n "$(git status --porcelain)" ]; then
        echo "FATAL: the working tree has uncommitted changes." >&2
        echo "       The fixtures restore with 'git checkout -- . && git clean -qfd'," >&2
        echo "       which would discard them.  Commit first, or pass a scratch tree:" >&2
        echo "         bash audit/v4.1.0/neg/verify_neg_fixtures.sh <scratch-tree>" >&2
        exit 4
    fi
fi
if [ ! -d "$TREE/.git" ] && [ ! -f "$TREE/.git" ]; then
    echo "FATAL: $TREE is not a git checkout" >&2
    exit 4
fi

# gate | fixture
#
# ORDERING RULE — "凡消费 cargo 产物的夹具，必须排在任何改 Rust 源码的夹具之前"
# -------------------------------------------------------------------------------
# The fixtures restore the scratch tree with `git checkout -- .`, which changes
# the MTIME of every restored file.  cargo keys its fingerprints on mtime, so
# after ANY fixture that touched Rust source, the next cargo-consuming fixture
# sees a stale fingerprint and rebuilds the root package AND its eleven
# dependents.  Measured on this machine: >90 minutes for that rebuild — far
# past any per-fixture budget, and the reason the lane used to produce
# timeouts instead of verdicts.
#
# So the lane is split in two:
#
#   * FIRST: every fixture that CONSUMES cargo artefacts (it needs a warm,
#     already-built target) — fx02, fx20, fx19, fx15, fx18;
#   * THEN: the fixtures whose gate is static (they read source or a ledger,
#     they compile nothing) — fx16, fx17.  They run in seconds and they are
#     last so that a lane cut short still yields the expensive evidence.
#
# Within the cargo-consuming group the order is by cost: fx02 (static, cheap),
# fx20 (runtime, needs the TPC-H dataset), fx19 (cargo), fx15 (composite),
# fx18 (cargo llvm-cov over eight crates — the most expensive by far).
GATES=(
    "scripts/gate/check_anti_ignore_gate.sh|audit/v4.1.0/neg/fx02-anti-ignore-registry-drift.sh"
    "scripts/gate/check_g1_tpch_baseline.sh|audit/v4.1.0/neg/fx20-g1-tpch-baseline-hash-tampered.sh"
    "scripts/gate/check_sql_corpus_gate.sh|audit/v4.1.0/neg/fx19-sql-corpus-gate-corpus-truncated.sh"
    "scripts/gate/check_alpha_v410.sh|audit/v4.1.0/neg/fx15-alpha-v410-composite-entry-doc-missing.sh"
    "scripts/gate/check_coverage_v312.sh|audit/v4.1.0/neg/fx18-coverage-v312-crate-coverage-below-threshold.sh"
    "scripts/gate/check_arch_invariants.sh|audit/v4.1.0/neg/fx16-arch-invariants-invariant-impl-removed.sh"
    "scripts/gate/check_arch3_no_bypass.sh|audit/v4.1.0/neg/fx17-arch3-no-bypass-dml-guard-dropped.sh"
)

LOG_DIR="${FX_LOG_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/verify-neg-fixtures.XXXXXX")}"
mkdir -p "$LOG_DIR"
echo "verify_neg_fixtures: scratch tree = $TREE"
echo "verify_neg_fixtures: logs          = $LOG_DIR"

# --- the tree identity, self-reported ---------------------------------------
# A lane that does not say WHICH tree it measured cannot be compared with any
# other tree.  The SHA alone is not enough: when TREE != REPO the dirty check
# is skipped, so a modified scratch tree shares the SHA of its commit and only
# `dirty_files` distinguishes it.  Both are printed on ONE line so a reader
# (or a script) can lift the whole identity out of the log.
TREE_SHA="$(git -C "$TREE" rev-parse HEAD 2>/dev/null || echo unknown)"
DIRTY_FILES="$(git -C "$TREE" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
echo "verify_neg_fixtures: tree_sha=$TREE_SHA dirty_files=$DIRTY_FILES"

# --- warm the target BEFORE the lane -----------------------------------------
# R50 requires the lane to warm the target.  Without a warm target the first
# cargo-consuming fixture pays the full build, and every fixture that follows
# a source-touching fixture pays a REBUILD (see the ordering rule above).
#
# There is deliberately NO environment switch to skip the warmup: a switch
# would be a switch to skip the lane, and a skipped lane must not be
# indistinguishable from a lane that ran.  The warmup runs unconditionally,
# writes its own log, and a failure aborts the whole lane with exit 4 (setup
# failure) — it is never silently skipped and never reported as a pass.
WARMUP_LOG="$LOG_DIR/warmup.log"
echo "verify_neg_fixtures: warming the target (this is unconditional; log: $WARMUP_LOG)"
(
    cd "$TREE" &&
    cargo build --all-features &&
    cargo test --all-features --lib --no-run
) >"$WARMUP_LOG" 2>&1
WARMUP_RC=$?
if [ "$WARMUP_RC" -ne 0 ]; then
    echo "FATAL: the target warmup FAILED (rc=$WARMUP_RC) -> the lane cannot run" >&2
    echo "       (a cold or broken target would make every cargo-consuming fixture" >&2
    echo "        time out, and a timeout is INCONCLUSIVE, never a red)" >&2
    tail -20 "$WARMUP_LOG" >&2
    exit 4
fi
echo "verify_neg_fixtures: warmup OK (rc=0)"

RED_N=0
INC_N=0
ROWS=()

for entry in "${GATES[@]}"; do
    gate="${entry%%|*}"
    fixture="${entry##*|}"
    name="$(basename "$fixture" .sh)"
    log="$LOG_DIR/$name.log"

    echo ""
    echo "===== $gate  <-  $fixture ====="
    if [ ! -f "$REPO/$fixture" ]; then
        echo "MISSING FIXTURE: $fixture -> INCONCLUSIVE"
        ROWS+=("$gate|INCONCLUSIVE|INCONCLUSIVE|fixture file missing")
        INC_N=$((INC_N + 1))
        continue
    fi

    bash "$REPO/$fixture" "$TREE" >"$log" 2>&1
    fx_rc=$?

    g_state="$(grep -m1 -E '^FX-GATE-LAYER:' "$log" | sed -E 's/.*state=([^ ]+).*/\1/' || true)"
    g_rc="$(grep -m1 -E '^FX-GATE-LAYER:' "$log" | sed -E 's/.*exit_code=([^ ]*).*/\1/' || true)"
    s_verdict="$(grep -m1 -E '^FX-STAGE-LAYER:' "$log" | sed -E 's/.*verdict=([^ ]+).*/\1/' || true)"

    GATE_RED=0
    if [ "$g_state" = "FAIL" ] && [ -n "$g_rc" ] && [ "$g_rc" != "124" ] && [ "$g_rc" != "0" ]; then
        case "$g_rc" in
            ''|*[!0-9]*) GATE_RED=0 ;;
            *) GATE_RED=1 ;;
        esac
    fi
    STAGE_RED=0
    [ "$s_verdict" = "FAIL" ] && STAGE_RED=1

    if [ "$fx_rc" -ne 0 ]; then
        # The fixture itself did not certify its own evidence.
        GATE_RED=0
        STAGE_RED=0
    fi

    if [ "$GATE_RED" -eq 1 ]; then G="RED"; else G="INCONCLUSIVE"; fi
    if [ "$STAGE_RED" -eq 1 ]; then S="RED"; else S="INCONCLUSIVE"; fi

    if [ "$G" = "RED" ] && [ "$S" = "RED" ]; then
        RED_N=$((RED_N + 1))
    else
        INC_N=$((INC_N + 1))
    fi

    echo "  fixture_rc=$fx_rc gate_state=${g_state:-<none>} gate_rc=${g_rc:-<none>} stage_verdict=${s_verdict:-<none>}"
    echo "  gate=$G stage=$S"
    ROWS+=("$gate|$G|$S|fixture_rc=$fx_rc gate_rc=${g_rc:-?} stage_verdict=${s_verdict:-?}")
done

echo ""
echo "===== verify_neg_fixtures summary ====="
for r in "${ROWS[@]}"; do
    IFS='|' read -r g gs ss extra <<<"$r"
    printf '  %-46s gate=%-12s stage=%-12s %s\n' "$g" "$gs" "$ss" "$extra"
done
echo ""
echo "verify_neg_fixtures: RED=${RED_N}/7  INCONCLUSIVE=${INC_N}/7"
if [ "$RED_N" -eq 7 ]; then
    echo "verify_neg_fixtures: 7/7 — every ALPHA gate is RED on both layers"
    exit 0
fi
echo "verify_neg_fixtures: NOT 7/7 — ${INC_N} gate(s) did not reach a two-layer RED. This is reported as INCONCLUSIVE, not as a pass (R50)."
exit 3
