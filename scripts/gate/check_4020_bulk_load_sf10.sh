#!/usr/bin/env bash
# check_4020_bulk_load_sf10.sh — V312-26 Issue #4020 Bulk-load SF=10 Gate
#
# Verifies the TPC-H SF=10 bulk-load runner infrastructure for #4020:
# 1. scripts/tpch/bulk_load_sf10.sh exists with valid bash syntax
# 2. The script has all 8 TPC-H schemas defined (region, nation, supplier,
#    customer, part, partsupp, orders, lineitem)
# 3. The script uses LOAD DATA LOCAL INFILE pattern (V312-13 wire load data)
# 4. The script has per-table elapsed_sec + rows/sec + row_count_parity
# 5. The script uses per-step timeout to bound wire-protocol hangs
# 6. Evidence directory exists with at least one captured run + metadata.json
#
# Exit code: 0 = PASS, 1 = FAIL
#
# IMPORTANT (STRICT PROOF MODE): this gate does NOT claim SF=10 fully loaded.
# It only verifies the runner infrastructure and recorded result codes. The
# 4020_evidence.md is the authoritative record of what was actually captured.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== V312-26 #4020 Bulk-load SF=10 Gate ==="
echo ""

PASS=0
FAIL=0

check() {
    local name="$1"
    local cmd="$2"
    if eval "$cmd" >/dev/null 2>&1; then
        echo "  [PASS] $name"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] $name"
        FAIL=$((FAIL+1))
    fi
}

# 1. Runner script exists
echo "--- Runner Script ---"
if [ -f "scripts/tpch/bulk_load_sf10.sh" ]; then
    echo "  [PASS] scripts/tpch/bulk_load_sf10.sh exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] scripts/tpch/bulk_load_sf10.sh missing"
    FAIL=$((FAIL+1))
fi

check "bulk_load_sf10.sh syntax valid" "bash -n scripts/tpch/bulk_load_sf10.sh"
check "bulk_load_sf10.sh has main()" "grep -q 'main()' scripts/tpch/bulk_load_sf10.sh"
check "bulk_load_sf10.sh has preflight()" "grep -q 'preflight()' scripts/tpch/bulk_load_sf10.sh"
check "bulk_load_sf10.sh has start_server()" "grep -q 'start_server()' scripts/tpch/bulk_load_sf10.sh"
check "bulk_load_sf10.sh has create_schemas()" "grep -q 'create_schemas()' scripts/tpch/bulk_load_sf10.sh"
check "bulk_load_sf10.sh has load_one_table()" "grep -q 'load_one_table()' scripts/tpch/bulk_load_sf10.sh"
check "bulk_load_sf10.sh has write_summary_json()" "grep -q 'write_summary_json()' scripts/tpch/bulk_load_sf10.sh"
check "bulk_load_sf10.sh has write_metadata()" "grep -q 'write_metadata()' scripts/tpch/bulk_load_sf10.sh"

# 2. All 8 TPC-H schemas defined
echo ""
echo "--- TPC-H Schema Coverage ---"
for tbl in region nation supplier customer part partsupp orders lineitem; do
    if grep -q "SCHEMA_${tbl^^}=" "scripts/tpch/bulk_load_sf10.sh"; then
        echo "  [PASS] SCHEMA_${tbl^^} defined"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] SCHEMA_${tbl^^} missing"
        FAIL=$((FAIL+1))
    fi
done

# 3. TABLES array has all 8 entries (small-to-large ordering)
echo ""
echo "--- TABLES Array ---"
if grep -q 'TABLES=(region nation supplier customer part partsupp orders lineitem)' \
        "scripts/tpch/bulk_load_sf10.sh"; then
    echo "  [PASS] TABLES array has all 8 tables in small-to-large order"
    PASS=$((PASS+1))
else
    echo "  [FAIL] TABLES array missing or wrong order"
    FAIL=$((FAIL+1))
fi

# 4. LOAD DATA LOCAL INFILE pattern present
echo ""
echo "--- MySQL Wire Pattern ---"
if grep -q "LOAD DATA LOCAL INFILE" "scripts/tpch/bulk_load_sf10.sh"; then
    echo "  [PASS] LOAD DATA LOCAL INFILE pattern present"
    PASS=$((PASS+1))
else
    echo "  [FAIL] LOAD DATA LOCAL INFILE missing"
    FAIL=$((FAIL+1))
fi
if grep -q "FIELDS TERMINATED BY '|' LINES TERMINATED BY '\\\\n'" "scripts/tpch/bulk_load_sf10.sh"; then
    echo "  [PASS] dbgen .tbl field/line terminators present"
    PASS=$((PASS+1))
else
    echo "  [FAIL] dbgen terminators ('|'/'\n') missing"
    FAIL=$((FAIL+1))
fi
if grep -q "local-infile=1" "scripts/tpch/bulk_load_sf10.sh"; then
    echo "  [PASS] --local-infile=1 enabled on mysql client"
    PASS=$((PASS+1))
else
    echo "  [FAIL] --local-infile=1 missing"
    FAIL=$((FAIL+1))
fi

# 5. Per-step timeout for wire-protocol hang protection
echo ""
echo "--- Wire-protocol Hardening ---"
if grep -q "MYSQL_TIMEOUT_SEC" "scripts/tpch/bulk_load_sf10.sh"; then
    echo "  [PASS] MYSQL_TIMEOUT_SEC knob present"
    PASS=$((PASS+1))
else
    echo "  [FAIL] MYSQL_TIMEOUT_SEC knob missing"
    FAIL=$((FAIL+1))
fi
if grep -q "timeout.*MYSQL_TIMEOUT_SEC" "scripts/tpch/bulk_load_sf10.sh"; then
    echo "  [PASS] mysql_q() wrapped in timeout"
    PASS=$((PASS+1))
else
    echo "  [FAIL] mysql_q() not timeout-bounded"
    FAIL=$((FAIL+1))
fi
if grep -q "timeout.*LOAD_TIMEOUT_SEC" "scripts/tpch/bulk_load_sf10.sh"; then
    echo "  [PASS] load_one_table() LOAD_TIMEOUT_SEC present"
    PASS=$((PASS+1))
else
    echo "  [FAIL] load_one_table() not timeout-bounded"
    FAIL=$((FAIL+1))
fi

# 6. Per-table metrics: elapsed_sec, rows_per_sec, row_count_parity
echo ""
echo "--- Per-Table Metrics ---"
for metric in elapsed_sec rows_per_sec row_count_parity parity bulk_load_summary.json; do
    if grep -q "${metric}" "scripts/tpch/bulk_load_sf10.sh"; then
        echo "  [PASS] metric: ${metric}"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] metric: ${metric} missing"
        FAIL=$((FAIL+1))
    fi
done

# 7. Evidence directory
echo ""
echo "--- Evidence Directory ---"
EVIDENCE_DIR="docs/releases/v3.12.0/evidence/issue-4020"
if [ -d "$EVIDENCE_DIR" ]; then
    echo "  [PASS] $EVIDENCE_DIR exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] $EVIDENCE_DIR missing"
    FAIL=$((FAIL+1))
fi

# 8. At least one captured run with metadata.json
LATEST_RUN=$(ls -1dt "$EVIDENCE_DIR"/*/ 2>/dev/null | head -1 || true)
if [ -n "$LATEST_RUN" ]; then
    echo "  [PASS] latest run dir: $LATEST_RUN"
    PASS=$((PASS+1))
    if [ -f "$LATEST_RUN/metadata.json" ]; then
        echo "  [PASS] metadata.json present"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] metadata.json missing in $LATEST_RUN"
        FAIL=$((FAIL+1))
    fi
    if [ -f "$LATEST_RUN/server.log" ]; then
        echo "  [PASS] server.log present (server-side trace of what happened)"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] server.log missing in $LATEST_RUN"
        FAIL=$((FAIL+1))
    fi
    # Either bulk_load_summary.jsonl exists (success path) or schema_create.log
    # shows the honest failure record
    if [ -f "$LATEST_RUN/bulk_load_summary.jsonl" ] || [ -f "$LATEST_RUN/schema_create.log" ]; then
        echo "  [PASS] either summary.jsonl (loaded) or schema_create.log (attempted) present"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] neither summary nor schema_create log present"
        FAIL=$((FAIL+1))
    fi
else
    echo "  [FAIL] no captured run in $EVIDENCE_DIR"
    FAIL=$((FAIL+1))
fi

# 9. Evidence doc (the authoritative record)
echo ""
echo "--- Evidence Document ---"
EVIDENCE_DOC="docs/releases/v3.12.0/evidence/issue-4020/4020_evidence.md"
if [ -f "$EVIDENCE_DOC" ]; then
    echo "  [PASS] $EVIDENCE_DOC exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] $EVIDENCE_DOC missing — STRICT PROOF MODE requires the doc"
    FAIL=$((FAIL+1))
fi

echo ""
echo "=== #4020 Bulk-load SF=10 Gate Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "✅ #4020 gate PASSED (infrastructure verified — see 4020_evidence.md for actual results)"
    exit 0
else
    echo "❌ #4020 gate FAILED ($FAIL blocker(s))"
    exit 1
fi
