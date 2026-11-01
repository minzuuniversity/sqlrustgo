#!/usr/bin/env bash
# =============================================================================
# check_d6_smoke.sh — v3.8.0 D6 Smoke Gate (minimal REAL execution)
# =============================================================================
# Issue #3847 — companion to scripts/gate/check_test_inventory.sh
#
# Why this gate exists
# --------------------
# #3847 turned the D6b gate into a **static** consistency check: it no longer
# executes the test suite (that cost 1h44m and ~130GB of runner disk).
#
# The two accounting gates that remain in the `test` job
# (`check_test_target_execution.sh` and `check_test_manifest_honesty.sh`)
# prove that every declared test target *was executed*. They do NOT prove
# that the execution *passed* — if the `test` job ever ran the suite but
# failed to surface a failure, no static gate could notice.
#
# This gate closes that gap with the smallest possible real execution:
# a fixed set of smoke targets covering the core storage read/write paths
# (SQL aggregate, WAL, batch load, wire protocol, TPCH smoke). It runs in
# the `postcheck` job, so it is an *independent* execution of the engine
# and does not depend on the `test` job's bookkeeping.
#
# It is intentionally small: a handful of targets, a hard timeout, and an
# evidence file. If this gate ever grows into "run everything", the
# #3847 cost problem returns.
#
# D6_SMOKE_MODE: MINIMAL_REAL_EXECUTION
#
# Exit codes:
#   0 = PASS  (every smoke target ran and passed)
#   1 = FAIL  (a smoke target failed, timed out, or does not exist)
#   2 = DEGRADED (cargo is unavailable — NOT a silent PASS)
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Keep the smoke set small on purpose. Every entry must be a declared
# [[test]] target in Cargo.toml and must exercise a different core path.
SMOKE_TARGETS=(
    aggregate_smoke_test
    wal_integration_test
    batch_insert_test
    wire_protocol_smoke
    tpch_wire_smoke
)

# Per-target wall-clock budget. A target that cannot finish in this budget
# is a failure, not a hang.
PER_TARGET_TIMEOUT=900
SMOKE_TOTAL_TIMEOUT=3600

echo "=== D6 Smoke Gate (minimal real execution, issue #3847) ==="
echo "Smoke targets: ${SMOKE_TARGETS[*]}"
echo

if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "⚠️ D6 Smoke Gate: DEGRADED (cargo is unavailable; no smoke target was executed)"
    exit 2
fi

WORK_DIR=$(mktemp -d -t d6_smoke.XXXXXX)
trap 'rm -rf "$WORK_DIR"' EXIT

EVIDENCE_DIR="artifacts/gate/v3.8.0"
EVIDENCE_FILE="$EVIDENCE_DIR/d6_smoke.json"
mkdir -p "$EVIDENCE_DIR"

RESULTS_LINES=""
PASSED=0
FAILED=0
FAILED_TARGETS=""

overall_start=$(date +%s)

for target in "${SMOKE_TARGETS[@]}"; do
    log="$WORK_DIR/$target.log"
    start=$(date +%s)
    if timeout -k 30 "$PER_TARGET_TIMEOUT" cargo test --all-features --test "$target" \
            -- --test-threads=1 > "$log" 2>&1; then
        status="PASS"
        PASSED=$((PASSED + 1))
        echo "  ✅ $target: PASS ($(($(date +%s) - start))s)"
    else
        rc=$?
        status="FAIL"
        FAILED=$((FAILED + 1))
        FAILED_TARGETS="$FAILED_TARGETS $target"
        echo "  ❌ $target: FAIL (exit $rc, $(($(date +%s) - start))s)"
        tail -20 "$log" | sed 's/^/      /'
    fi
    RESULTS_LINES="$RESULTS_LINES{\"target\": \"$target\", \"status\": \"$status\"},"
done

total_seconds=$(($(date +%s) - overall_start))

# Strip the trailing comma of the last entry.
RESULTS_LINES="${RESULTS_LINES%,}"

if [ "$FAILED" -gt 0 ]; then
    RESULT="FAIL"
    EXIT_CODE=1
else
    RESULT="PASS"
    EXIT_CODE=0
fi

cat > "$EVIDENCE_FILE" <<EOF
{
    "gate": "D6-smoke",
    "name": "D6 Smoke (minimal real execution)",
    "mode": "MINIMAL_REAL_EXECUTION",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "targets": [$(printf '"%s", ' "${SMOKE_TARGETS[@]}" | sed 's/, $//')],
    "per_target_timeout_seconds": $PER_TARGET_TIMEOUT,
    "total_seconds": $total_seconds,
    "passed": $PASSED,
    "failed": $FAILED,
    "results": [$RESULTS_LINES],
    "result": "$RESULT",
    "note": "Independent minimal engine execution in the postcheck job; it complements the static D6b inventory check and the test job's accounting gates."
}
