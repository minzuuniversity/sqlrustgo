#!/usr/bin/env bash
# =============================================================================
# check_test_inventory.sh — v3.8.0 D6-Test-Integration Gate (STATIC-ONLY)
# =============================================================================
# Implements 5-Principle P4 enforcement:
#   "测试必须经过审核和验证，必须集成到门禁测试"
#
# D6B_MODE: STATIC_ONLY_NO_CARGO
#
# ------------------------------------------------------------------
# WHY THIS GATE NO LONGER EXECUTES THE TEST SUITE   (issue #3847)
# ------------------------------------------------------------------
# This gate used to re-execute the *entire* test suite in the `postcheck`
# job: for every file under `tests/` it ran
#
#     timeout 180 cargo test --test "$name" -- --test-threads=1
#
# Measured on CI run 6408 (job 13864, `postcheck`):
#
#   * the D6b step alone took 1h44m48s,
#   * compiling every test target grew the act_runner per-job volume to
#     ~130GB, which pushed the 250 host's root partition to 97% (35G left)
#     and had to be reclaimed by hand,
#   * Gitea's job log endpoint truncated the job log at 104887 bytes, so
#     everything after the D6b start was unreadable.
#
# The re-execution bought nothing that the `test` job does not already
# prove, at a cost that broke the runner:
#
#   * "was every declared target actually executed?" is already a hard
#     gate inside the `test` job -- `check_test_target_execution.sh`
#     (run 6335 failed there with "112 of 307 declared test targets never
#     ran"), and
#   * "is every test file declared, and does the declaration count match
#     what cargo actually builds?" is already a hard gate inside the same
#     job -- `check_test_manifest_honesty.sh` (C1..C6).
#
# A failing test target fails the `test` job, so a second execution in
# `postcheck` cannot discover anything new. The only thing it removed was
# a redundant second opinion, at the price of the runner.
#
# Therefore D6b is now a **static consistency check**:
#
#   * every `tests/**/*.rs` file must have a `[[test]]` declaration,
#   * the declared target count must equal the number of test targets
#     cargo itself reports (`cargo metadata --no-deps`, read-only),
#   * there must be no orphan test file and no duplicate target name.
#
# The authoritative pass/fail counts of the suite are produced by the
# `test` job (its log and `check_test_target_execution.sh`); D6b does not
# re-derive them.
#
# Exit codes:
#   0 = PASS (inventory is consistent)
#   1 = FAIL (a declaration is missing, duplicated, or disagrees with cargo)
#   2 = DEGRADED (cargo metadata unavailable; the manifest cross-check
#       could not be performed -- NOT a silent PASS)
#
# Bash compatibility: works on bash 3.2.57 (macOS default) AND bash 4+.
# The original implementation used `declare -A` which is bash 4+ only.
# We use temp files and grep-based lookup, which works in all bash
# versions.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== D6: Test Inventory Gate (static-only, issue #3847) ==="
echo

WORK_DIR=$(mktemp -d -t d6_inventory.XXXXXX)
PATH_TO_NAME_FILE="$WORK_DIR/path_to_name"
METADATA_JSON="$WORK_DIR/metadata.json"
HONESTY_LOG="$WORK_DIR/manifest_honesty.log"
trap 'rm -rf "$WORK_DIR"' EXIT

# Find all test files. `tests/disabled/` holds test files that are
# deliberately not built; they are excluded by the repository's own
# rule (tests/disabled/IGNORE_REASON.md).
TEST_FILES=$(find tests -name "*.rs" -type f ! -path "*/common/*" ! -path "*/disabled/*" ! -name "mod.rs" 2>/dev/null | sort)
TOTAL_FILES=$(echo "$TEST_FILES" | wc -l | tr -d ' ')
echo "Test files discovered: $TOTAL_FILES"

# Build a temp file `path<TAB>name` per line for [[test]] entries.
# bash 3.2.57 does NOT support `declare -A`; we use grep over a temp file.
awk '
    /^\[\[test\]\]/{ in_t = 1; name = ""; path = ""; next }
    in_t && /^name = /{ gsub(/name = "|"/, "", $0); name = $0 }
    in_t && /^path = /{ gsub(/path = "|"/, "", $0); path = $0; print path "\t" name; in_t = 0 }
' Cargo.toml > "$PATH_TO_NAME_FILE"

DECLARED_TARGETS=$(wc -l < "$PATH_TO_NAME_FILE" | tr -d ' ')

# Bash 3.2 / 4 portable lookup function: PATH_TO_NAME <path>
# Echoes the cargo target name for a given path (empty if no explicit entry).
PATH_TO_NAME() {
    local key="$1"
    # The temp file keys include "tests/..." since Cargo paths are
    # relative to the workspace root and [[test]] paths look like
    # "tests/foo.rs". So we lookup the full path.
    grep -F -e "$(printf '%s\t' "$key")" "$PATH_TO_NAME_FILE" 2>/dev/null \
        | head -1 | cut -f2-
}

# Count the test files that have no `[[test]]` entry of their own.
# A file that is legitimately reachable from a declared target (a shared
# module) is not an orphan -- the manifest honesty gate (C3) decides that.
UNDECLARED_COUNT=0
for f in $TEST_FILES; do
    if [ -z "$(PATH_TO_NAME "$f")" ]; then
        UNDECLARED_COUNT=$((UNDECLARED_COUNT + 1))
    fi
done

echo "Declared [[test]] targets in Cargo.toml: $DECLARED_TARGETS"
echo

# ---------------------------------------------------------------------------
# Cross-check against cargo itself. `cargo metadata --no-deps` is a
# read-only query: it resolves the manifest and prints the target list
# without building or running anything.
# ---------------------------------------------------------------------------
CARGO_TARGETS=""
if command -v cargo >/dev/null 2>&1; then
    if cargo metadata --no-deps --format-version 1 --manifest-path Cargo.toml \
            > "$METADATA_JSON" 2>/dev/null; then
        CARGO_TARGETS=$(python3 - "$METADATA_JSON" "$REPO_ROOT/Cargo.toml" <<'PY' 2>/dev/null || true
import json, os, sys
try:
    with open(sys.argv[1]) as fh:
        meta = json.load(fh)
except Exception:
    sys.exit(0)
# Only the root package matters here: the workspace members under crates/
# declare their own test targets and are not part of the root manifest's
# [[test]] inventory.
root = os.path.realpath(sys.argv[2])
count = 0
for pkg in meta.get("packages", []):
    if os.path.realpath(pkg.get("manifest_path", "")) != root:
        continue
    for target in pkg.get("targets", []):
        if "test" in target.get("kind", []):
            count += 1
print(count)
PY
)
    fi
fi

if [ -n "$CARGO_TARGETS" ]; then
    echo "cargo metadata reports test targets: $CARGO_TARGETS"
else
    echo "cargo metadata unavailable -- the declared-versus-actual cross-check is DEGRADED"
fi
echo

# ---------------------------------------------------------------------------
# The authoritative static consistency verdict comes from the manifest
# honesty gate, which already implements C1..C6 (declared target exists on
# disk, no duplicate names, no orphan test file, every top-level file has
# its own declaration, declared count == cargo's test target count).
# Re-using it keeps one single source of truth for those rules instead of
# a second, drifting copy inside this script.
# ---------------------------------------------------------------------------
HONESTY_RC=0
bash "$SCRIPT_DIR/check_test_manifest_honesty.sh" > "$HONESTY_LOG" 2>&1 || HONESTY_RC=$?
cat "$HONESTY_LOG"

# Pick up the numbers the honesty gate already computed, so the evidence
# file and this script's summary agree with the gate that produced them.
HONESTY_CARGO_TARGETS=$(grep -oE 'reports [0-9]+ test targets' "$HONESTY_LOG" | grep -oE '[0-9]+' | head -1)
HONESTY_DECLARED=$(grep -oE 'manifest declares [0-9]+' "$HONESTY_LOG" | grep -oE '[0-9]+' | head -1)
if [ -n "$HONESTY_CARGO_TARGETS" ]; then
    CARGO_TARGETS="$HONESTY_CARGO_TARGETS"
fi
if [ -n "$HONESTY_DECLARED" ]; then
    DECLARED_TARGETS="$HONESTY_DECLARED"
fi

# The declared-vs-cargo cross-check counts as performed when either this
# script's own `cargo metadata` call or the honesty gate's C5 step
# produced a number. "Could not measure" is never reported as "measured
# and fine".
CROSSCHECK_OK=0
if [ -n "$CARGO_TARGETS" ]; then
    CROSSCHECK_OK=1
fi

echo
echo "=== D6 Test Inventory Summary (static-only) ==="
echo "Test files:              $TOTAL_FILES"
echo "Declared test targets:   $DECLARED_TARGETS"
echo "test targets reported by cargo: ${CARGO_TARGETS:-unknown}"
echo "Files without own [[test]] entry (shared modules): $UNDECLARED_COUNT"
echo "Manifest honesty gate:   exit $HONESTY_RC"
echo "Note: per-target pass/fail counts are produced by the 'test' job"
echo "      (check_test_target_execution.sh + its job log), not here."
echo

RESULT="PASS"
EXIT_CODE=0
if [ "$HONESTY_RC" -ne 0 ]; then
    RESULT="FAIL"
    EXIT_CODE=1
elif [ "$CROSSCHECK_OK" -ne 1 ]; then
    # The manifest itself is consistent, but cargo could not be queried,
    # so the "declared == cargo" half of the cross-check did not happen.
    # Report DEGRADED rather than pretending the check was complete.
    RESULT="DEGRADED"
    EXIT_CODE=2
fi

# Generate evidence
EVIDENCE_FILE="artifacts/gate/v3.8.0/d6_test_inventory.json"
mkdir -p "$(dirname "$EVIDENCE_FILE")"
cat > "$EVIDENCE_FILE" <<EOF
{
    "gate": "D6",
    "name": "Test Inventory",
    "principle": "P4",
    "mode": "STATIC_ONLY_NO_CARGO",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "total_files": $TOTAL_FILES,
    "declared_targets": $DECLARED_TARGETS,
    "cargo_test_targets": ${CARGO_TARGETS:-0},
    "files_without_own_declaration": $UNDECLARED_COUNT,
    "files_without_own_declaration_note": "shared modules reachable from a declared target; manifest honesty gate C3 verifies reachability",
    "manifest_honesty_gate_exit": $HONESTY_RC,
    "result": "$RESULT",
    "note": "Per-target pass/fail counts are produced by the 'test' job (check_test_target_execution.sh); D6b verifies declaration consistency only and never runs a test binary."
}
EOF
echo "Evidence written to $EVIDENCE_FILE"

if [ "$EXIT_CODE" -eq 1 ]; then
    echo "❌ D6 Test Inventory: FAILED (inventory is inconsistent)"
    exit 1
fi
if [ "$EXIT_CODE" -eq 2 ]; then
    echo "⚠️ D6 Test Inventory: DEGRADED (cargo metadata unavailable; no test binary was executed)"
    exit 2
fi
echo "✅ D6 Test Inventory: PASS ($TOTAL_FILES test files, $DECLARED_TARGETS declared targets, 0 inconsistencies)"
exit 0
