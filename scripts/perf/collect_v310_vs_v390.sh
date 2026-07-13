#!/usr/bin/env bash
# scripts/perf/collect_v310_vs_v390.sh
#
# Collect v3.9.0 vs v3.10.0 performance baseline for RC gate R8.
# Output: docs/releases/v3.10.0/perf/
#   - V310_BASELINE.md       (extracted from existing SF1_BASELINE_REPORT.md)
#   - COMPARISON.md          (v3.9.0 vs v3.10.0 regression matrix, populated if v3.9.0 data exists)
#   - sysbench_v310.txt      (sysbench OLTP QPS — populated if sysbench installed)
#   - gap_lock_v310.txt      (gap-locking P99 latency — populated if measurable)
#
# Required tools (checked at runtime):
#   - cargo (build sqlrustgo server)
#   - sysbench (optional; fallback to in-process OLTP loop if missing)
#   - dbgen (optional; only needed to regenerate SF1 fixture; uses existing tests/data/tpch-sf01/ if absent)
#
# Usage:
#   bash scripts/perf/collect_v310_vs_v390.sh
#   V390_BIN=/path/to/sqlrustgo-v3.9.0 bash scripts/perf/collect_v310_vs_v390.sh
#
# Exits 0 if all three comparisons (TPC-H, sysbench, gap-locking) are populated.
# Exits 1 if any component is missing data; the script writes a placeholder
# COMPARISON.md that documents which components are missing and why.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

PERF_DIR="$REPO_ROOT/docs/releases/v3.10.0/perf"
mkdir -p "$PERF_DIR"

# Ensure cargo on PATH
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo "❌ cargo not on PATH"
        exit 2
    fi
fi

SRV_BIN_V310="${SRV_BIN_V310:-$REPO_ROOT/target/release/sqlrustgo}"
V390_BIN="${V390_BIN:-}"
SERVER_PORT="${SERVER_PORT:-3399}"
TEST_DB="perf_$$"

PASS=0
FAIL=0

echo "=== v3.10.0 vs v3.9.0 performance baseline collection ==="
echo "Output dir: $PERF_DIR"
echo "v3.10.0 binary: $SRV_BIN_V310"
echo "v3.9.0 binary:  ${V390_BIN:-(not provided)}"
echo ""

# ----------------------------------------------------------------------
# 1. TPC-H SF1: extract v3.10.0 data from SF1_BASELINE_REPORT.md
# ----------------------------------------------------------------------
echo "[1/3] TPC-H SF=1"
if [ -f "$PERF_DIR/SF1_BASELINE_REPORT.md" ]; then
    # v3.10.0 data already in repo (from PR #3404 / commit bf046134)
    cp "$PERF_DIR/SF1_BASELINE_REPORT.md" "$PERF_DIR/V310_BASELINE.md"
    echo "  ✅ v3.10.0 baseline copied to V310_BASELINE.md (22 queries, ~7.8 min total)"
    PASS=$((PASS + 1))
else
    echo "  ❌ SF1_BASELINE_REPORT.md missing — v3.10.0 baseline not yet run"
    FAIL=$((FAIL + 1))
fi
echo ""

# ----------------------------------------------------------------------
# 2. Sysbench OLTP QPS
# ----------------------------------------------------------------------
echo "[2/3] Sysbench OLTP QPS"
if command -v sysbench >/dev/null 2>&1 && [ -x "$SRV_BIN_V310" ]; then
    # Start server (assumes target/release/sqlrustgo built; user can pre-build)
    "$SRV_BIN_V310" --port "$SERVER_PORT" --data-dir "/tmp/perf_sysbench_$$" \
        >/tmp/perf_srv_$$.log 2>&1 &
    SRV_PID=$!
    sleep 3
    mysql -h 127.0.0.1 -P "$SERVER_PORT" -u root -e "CREATE DATABASE IF NOT EXISTS sbtest;" 2>/dev/null
    sysbench --db-driver=mysql --mysql-host=127.0.0.1 --mysql-port="$SERVER_PORT" \
        --mysql-user=root --mysql-db=sbtest --table-size=10000 --tables=4 \
        --threads=4 --time=30 oltp_read_only run \
        > "$PERF_DIR/sysbench_v310.txt" 2>&1 || true
    kill "$SRV_PID" 2>/dev/null || true
    if grep -q "transactions:" "$PERF_DIR/sysbench_v310.txt" 2>/dev/null; then
        echo "  ✅ sysbench v3.10.0 OLTP captured"
        PASS=$((PASS + 1))
    else
        echo "  ⚠️  sysbench run produced no transactions (server may not be ready)"
        FAIL=$((FAIL + 1))
    fi
else
    echo "  ⚠️  sysbench or v3.10.0 binary missing — deferring to dedicated CI env"
    cat > "$PERF_DIR/sysbench_v310.txt" <<EOF
# sysbench v3.10.0 OLTP QPS — NOT YET CAPTURED

Required:
- sysbench installed
- target/release/sqlrustgo built (cargo build --release -p sqlrustgo)
- v3.9.0 binary for comparison
EOF
    FAIL=$((FAIL + 1))
fi
echo ""

# ----------------------------------------------------------------------
# 3. Gap-locking P99 latency
# ----------------------------------------------------------------------
echo "[3/3] Gap-locking P99 latency"
# No canonical gap-locking benchmark exists in this repo yet.
# Placeholder: would need a workload that exercises REPEATABLE READ +
# range scans (e.g. TPC-C new-order transaction with stock-level).
cat > "$PERF_DIR/gap_lock_v310.txt" <<EOF
# Gap-locking P99 latency — NOT YET CAPTURED

Workload: TPC-C new-order with stock-level transaction
Target:    P99 latency on the SELECT ... FOR UPDATE range scan
Status:    No automated workload script in repo. Tracked in
           follow-up change (out of scope for #3401).
EOF
echo "  ⚠️  gap-locking benchmark not yet automated (placeholder written)"
FAIL=$((FAIL + 1))
echo ""

# ----------------------------------------------------------------------
# 4. COMPARISON.md
# ----------------------------------------------------------------------
echo "[summary] Writing COMPARISON.md"
cat > "$PERF_DIR/COMPARISON.md" <<EOF
# v3.10.0 vs v3.9.0 Performance Comparison

**Date**: 2026-07-13
**Status**: ⚠️ **PARTIAL — TPC-H SF=1 captured for v3.10.0, v3.9.0 + sysbench + gap-locking pending**

## TPC-H SF=1 (22 queries, 600K lineitem)

See \`V310_BASELINE.md\` for v3.10.0 data (extracted from \`SF1_BASELINE_REPORT.md\`).

| Query | v3.10.0 (s) | v3.9.0 (s) | Regression |
|-------|-------------:|-----------:|-----------:|
| Q1    | 4.07         | (pending)  | (pending)  |
| Q2    | 1.69         | (pending)  | (pending)  |
| ...   | ...          | (pending)  | (pending)  |

Geometric mean across 22 queries: see \`V310_BASELINE.md\` (~21s total).

## Sysbench OLTP QPS

| Version | QPS | P99 (ms) | Threads |
|---------|----:|---------:|--------:|
| v3.10.0 | (see sysbench_v310.txt) | (pending) | 4 |
| v3.9.0  | (pending — v3.9.0 binary needed) | (pending) | 4 |

## Gap-locking P99 latency

| Workload | v3.10.0 P99 (ms) | v3.9.0 P99 (ms) | Regression |
|----------|-----------------:|----------------:|-----------:|
| TPC-C new-order stock-level | (pending — workload script needed) | (pending) | (pending) |

## Acceptance

- **Target**: regression ≤ 5% on each of (TPC-H geometric mean, sysbench QPS, gap-locking P99).
- **Current state**: TPC-H v3.10.0 captured; v3.9.0 + sysbench + gap-locking pending.
- **Required to pass R8**: produce all three comparisons with ≤ 5% regression.

## How to run

\`\`\`bash
# Build v3.10.0 server
cargo build --release -p sqlrustgo

# Get v3.9.0 binary (from CI artifacts or release tag)
export V390_BIN=/path/to/sqlrustgo-v3.9.0

# Run baseline collection
bash scripts/perf/collect_v310_vs_v390.sh
\`\`\`

## Dependencies

- **dbgen** (TPC-H data generation, SF=1 = ~75GB disk; or reuse existing \`tests/data/tpch-sf01/\`)
- **sysbench** (apt: \`apt-get install sysbench\`)
- **v3.9.0 binary** (git tag v3.9.0 or release artifact)
- Dedicated test machine (no noisy neighbors for stable P99 measurements)
EOF
echo "  ✅ COMPARISON.md written"
echo ""

# ----------------------------------------------------------------------
# Summary
# ----------------------------------------------------------------------
echo "=== Summary ==="
echo "PASS: $PASS / 3"
echo "FAIL: $FAIL / 3"
echo ""
echo "Output:"
ls -la "$PERF_DIR" | sed 's/^/  /'

if [ "$FAIL" -gt 0 ]; then
    echo ""
    echo "⚠️  Some components are missing data; COMPARISON.md is partial."
    echo "   Re-run with proper tools (dbgen, sysbench, v3.9.0 binary) in dedicated env."
fi

[ "$FAIL" -eq 0 ] && exit 0 || exit 1
