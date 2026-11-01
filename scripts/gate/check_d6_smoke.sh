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
# (SQL aggregate, WAL, batch load, wire protocol). It runs in the `postcheck`
# job, so it is an *independent* execution of the engine and does not depend
# on the `test` job's bookkeeping.
#
# It is intentionally small: a handful of targets, a hard timeout, and an
# evidence file. If this gate ever grows into "run everything", the
# #3847 cost problem returns.
#
# D6_SMOKE_MODE: MINIMAL_REAL_EXECUTION
#
# Smoke set (B1 — DeepSeek ruling DSQ31)
# --------------------------------------
# HARD targets (must run, must pass):
#   aggregate_smoke_test, wal_integration_test, batch_insert_test,
#   wire_protocol_smoke
#
# CONDITIONAL target (needs an external binary):
#   tpch_wire_smoke — requires the canonical `sqlrustgo-mysql-server`
#   binary. If the binary cannot be located, the target is recorded as
#   DEGRADED (NOT a silent PASS, and NOT a FAIL — the missing binary is a
#   capability gap, not a product defect). If the binary IS present and the
#   target FAILS, that is a FAIL.
#
# Deliberately NOT in the smoke set:
#   - tpch_gate_test — with missing fixtures it prints
#     `TPCH-NOT-RUN: reason=fixture_missing` and still reports ok, i.e. a
#     false green.
#   - v312_13_load_data_sf1_test — currently red on the target branch
#     (issue #3848); a red target must not be a merge-blocking smoke
#     target until it is fixed.
#
# Exit codes:
#   0 = PASS      (every applicable smoke target ran and passed)
#   1 = FAIL      (a smoke target failed or timed out)
#   2 = DEGRADED  (cargo or the conditional target's binary is unavailable)
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
)

# Conditional targets: they need an external binary. When the binary is
# missing the target is DEGRADED, never a silent PASS and never a FAIL.
CONDITIONAL_SMOKE_TARGETS=(
    tpch_wire_smoke
)
MYSQL_SERVER_BIN_ENV="SQLRUSTGO_MYSQL_SERVER_BIN"

# Per-target wall-clock budget. A target that cannot finish in this budget
# is a failure, not a hang.
PER_TARGET_TIMEOUT=900
SMOKE_TOTAL_TIMEOUT=3600

echo "=== D6 Smoke Gate (minimal real execution, issue #3847) ==="
echo "Smoke targets: ${SMOKE_TARGETS[*]}"
echo "Conditional targets: ${CONDITIONAL_SMOKE_TARGETS[*]}"
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
DEGRADED=0
FAILED_TARGETS=""
DEGRADED_TARGETS=""

overall_start=$(date +%s)

# Locate the canonical `sqlrustgo-mysql-server` binary for the conditional
# targets. An explicit env var wins; otherwise look in the standard target
# dirs. Prints the resolved path, or nothing when it cannot be found.
resolve_mysql_server_binary() {
    if [ -n "${MYSQL_SERVER_BIN_ENV:-}" ] && [ -x "${MYSQL_SERVER_BIN_ENV}" ]; then
        printf '%s' "${MYSQL_SERVER_BIN_ENV}"
        return 0
    fi
    local candidate
    for candidate in \
        "${CARGO_TARGET_DIR:-}/debug/sqlrustgo-mysql-server" \
        "${CARGO_TARGET_DIR:-}/release/sqlrustgo-mysql-server" \
        "target/debug/sqlrustgo-mysql-server" \
        "target/release/sqlrustgo-mysql-server"; do
        if [ -n "$candidate" ] && [ -x "$candidate" ]; then
            printf '%s' "$candidate"
            return 0
        fi
    done
    return 1
}

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

# Conditional targets: run only when the required binary is available.
for target in "${CONDITIONAL_SMOKE_TARGETS[@]}"; do
    if ! binary_path=$(resolve_mysql_server_binary); then
        DEGRADED=$((DEGRADED + 1))
        DEGRADED_TARGETS="$DEGRADED_TARGETS $target"
        echo "  ⚠️ $target: DEGRADED (canonical sqlrustgo-mysql-server binary not found;"
        echo "      set $MYSQL_SERVER_BIN_ENV or run 'cargo build -p sqlrustgo-mysql-server')"
        RESULTS_LINES="$RESULTS_LINES{\"target\": \"$target\", \"status\": \"DEGRADED\", \"reason\": \"binary_not_found\"},"
        continue
    fi
    log="$WORK_DIR/$target.log"
    start=$(date +%s)
    if timeout -k 30 "$PER_TARGET_TIMEOUT" env "$MYSQL_SERVER_BIN_ENV=$binary_path" \
            cargo test --all-features --test "$target" -- --test-threads=1 > "$log" 2>&1; then
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
    RESULTS_LINES="$RESULTS_LINES{\"target\": \"$target\", \"status\": \"$status\", \"binary\": \"$binary_path\"},"
done

total_seconds=$(($(date +%s) - overall_start))

# Strip the trailing comma of the last entry.
RESULTS_LINES="${RESULTS_LINES%,}"

if [ "$FAILED" -gt 0 ]; then
    RESULT="FAIL"
    EXIT_CODE=1
elif [ "$DEGRADED" -gt 0 ]; then
    RESULT="DEGRADED"
    EXIT_CODE=2
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
    "conditional_targets": [$(printf '"%s", ' "${CONDITIONAL_SMOKE_TARGETS[@]}" | sed 's/, $//')],
    "per_target_timeout_seconds": $PER_TARGET_TIMEOUT,
    "total_seconds": $total_seconds,
    "passed": $PASSED,
    "failed": $FAILED,
    "degraded": $DEGRADED,
    "results": [$RESULTS_LINES],
    "result": "$RESULT",
    "note": "Independent minimal engine execution in the postcheck job; it complements the static D6b inventory check and the test job's accounting gates. DEGRADED means a conditional target's binary was unavailable — not a pass."
}
EOF

if [ "$EXIT_CODE" -eq 0 ]; then
    echo "✅ D6 Smoke Gate: PASS ($PASSED target(s) passed)"
elif [ "$EXIT_CODE" -eq 2 ]; then
    echo "⚠️ D6 Smoke Gate: DEGRADED ($PASSED passed, $DEGRADED degraded:$DEGRADED_TARGETS)"
    echo "   DEGRADED is not PASS: the conditional smoke targets did not run."
else
    echo "❌ D6 Smoke Gate: FAIL ($PASSED passed, $FAILED failed:$FAILED_TARGETS)"
fi

exit "$EXIT_CODE"
