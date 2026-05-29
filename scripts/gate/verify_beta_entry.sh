#!/usr/bin/env bash
# Beta Entry Verification Script
# 逐项验证 Beta Gate B1-B8，必须全部 PASS 才能进入 Beta
# 必须实际运行命令，不能只检查文档存在

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$REPO_DIR"

PASS_COUNT=0
FAIL_COUNT=0
TOTAL_COUNT=8
COMMIT=$(git rev-parse HEAD)
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# 创建日志目录
VERSION_DIR=$(git rev-parse --abbrev-ref HEAD | grep -o 'v[0-9]\+\.[0-9]\+\.[0-9]\+' | head -1)
if [ -z "$VERSION_DIR" ]; then
    VERSION_DIR="v3.6.0"
fi
LOG_DIR="docs/releases/${VERSION_DIR}/logs"
mkdir -p "$LOG_DIR"

# 日志文件
LOG_FILE="${LOG_DIR}/beta_entry_${COMMIT}_${TIMESTAMP}.log"

log() {
    echo "$@" | tee -a "$LOG_FILE"
}

log "=== Beta Entry Verification ==="
log "Date: $(date)"
log "Commit: $COMMIT"
log "Timestamp: $TIMESTAMP"
log ""

# B1: Build (release)
log "--- B1: Build (release) ---"
BUILD_START=$(date +%s)
if cargo build --release --workspace > "$LOG_DIR/b1_build.log" 2>&1; then
    BUILD_END=$(date +%s)
    BUILD_DURATION=$((BUILD_END - BUILD_START))
    log "B1: PASS (${BUILD_DURATION}s)"
    ((PASS_COUNT++))
else
    log "B1: FAIL - see $LOG_DIR/b1_build.log"
    ((FAIL_COUNT++))
fi
log ""

# B2: Workspace test
log "--- B2: Workspace test ---"
TEST_START=$(date +%s)
TEST_OUTPUT=$(cargo test --workspace 2>&1 || true)
TEST_END=$(date +%s)
TEST_DURATION=$((TEST_END - TEST_START))

echo "$TEST_OUTPUT" > "$LOG_DIR/b2_test.log"

# 分析测试通过率
TOTAL_TESTS=$(echo "$TEST_OUTPUT" | grep -oE '[0-9]+ tests|tests run' | head -1 || echo "unknown")
FAILED_TESTS=$(echo "$TEST_OUTPUT" | grep -oE '[0-9]+ failed|failed' | head -1 || echo "0 failed")
PASSED_TESTS=$(echo "$TEST_OUTPUT" | grep -oE '[0-9]+ passed|passed' | tail -1 || echo "unknown")

log "B2: Test output: $TOTAL_TESTS, $FAILED_TESTS, $PASSED_TESTS (${TEST_DURATION}s)"

if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    log "B2: PASS"
    ((PASS_COUNT++))
else
    log "B2: FAIL - see $LOG_DIR/b2_test.log"
    ((FAIL_COUNT++))
fi
log ""

# B3: Clippy zero
log "--- B3: Clippy zero ---"
CLIPPY_START=$(date +%s)
if cargo clippy --all-features -- -D warnings > "$LOG_DIR/b3_clippy.log" 2>&1; then
    CLIPPY_END=$(date +%s)
    CLIPPY_DURATION=$((CLIPPY_END - CLIPPY_START))
    log "B3: PASS (${CLIPPY_DURATION}s)"
    ((PASS_COUNT++))
else
    log "B3: FAIL - see $LOG_DIR/b3_clippy.log"
    ((FAIL_COUNT++))
fi
log ""

# B4: Format
log "--- B4: Format ---"
if cargo fmt --all -- --check > "$LOG_DIR/b4_fmt.log" 2>&1; then
    log "B4: PASS"
    ((PASS_COUNT++))
else
    log "B4: FAIL - see $LOG_DIR/b4_fmt.log"
    ((FAIL_COUNT++))
fi
log ""

# B5: Coverage L1 >= 85%
log "--- B5: Coverage L1 >= 85% ---"
COVERAGE_START=$(date +%s)
if command -v cargo-tarpaulin >/dev/null 2>&1; then
    COVERAGE_OUTPUT=$(cargo tarpaulin --ignore-tests --out Json 2>/dev/null || echo '{"metrics":{"LineCoverage":0}}")
    COVERAGE=$(echo "$COVERAGE_OUTPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('metrics',{}).get('LineCoverage',0))" 2>/dev/null || echo "0")
    # 转换为百分比
    COVERAGE_PCT=$(echo "scale=2; $COVERAGE * 100" | bc 2>/dev/null || echo "0")
    COVERAGE_END=$(date +%s)
    COVERAGE_DURATION=$((COVERAGE_END - COVERAGE_START))
    log "B5: Coverage = ${COVERAGE_PCT}% (${COVERAGE_DURATION}s)"
    
    MIN_COVERAGE=85.0
    if (( $(echo "$COVERAGE_PCT >= $MIN_COVERAGE" | bc -l 2>/dev/null || echo "0 >= 85" | bc -l) )); then
        log "B5: PASS"
        ((PASS_COUNT++))
    else
        log "B5: FAIL (${COVERAGE_PCT}% < ${MIN_COVERAGE}%)"
        ((FAIL_COUNT++))
    fi
else
    log "B5: SKIP (cargo-tarpaulin not installed)"
    log "B5: Manual coverage check required"
    ((FAIL_COUNT++))
fi
log ""

# B6: TPC-H SF=0.1
log "--- B6: TPC-H SF=0.1 ---"
TPCH_START=$(date +%s)
if [ -f "$REPO_DIR/scripts/tpch/run_tpch.sh" ]; then
    if bash "$REPO_DIR/scripts/tpch/run_tpch.sh" --sf 0.1 > "$LOG_DIR/b6_tpch.log" 2>&1; then
        TPCH_END=$(date +%s)
        TPCH_DURATION=$((TPCH_END - TPCH_START))
        log "B6: PASS (${TPCH_DURATION}s)"
        ((PASS_COUNT++))
    else
        log "B6: FAIL - see $LOG_DIR/b6_tpch.log"
        ((FAIL_COUNT++))
    fi
else
    log "B6: SKIP (TPC-H script not found)"
    log "B6: TPC-H test required but not available"
    ((FAIL_COUNT++))
fi
log ""

# B7: Security
log "--- B7: Security (cargo audit) ---"
SECURITY_START=$(date +%s)
if command -v cargo-audit >/dev/null 2>&1; then
    if cargo audit > "$LOG_DIR/b7_audit.log" 2>&1; then
        SECURITY_END=$(date +%s)
        SECURITY_DURATION=$((SECURITY_END - SECURITY_START))
        log "B7: PASS (${SECURITY_DURATION}s)"
        ((PASS_COUNT++))
    else
        log "B7: FAIL - see $LOG_DIR/b7_audit.log"
        ((FAIL_COUNT++))
    fi
else
    log "B7: SKIP (cargo-audit not installed)"
    log "B7: Manual security audit required"
    ((FAIL_COUNT++))
fi
log ""

# B8: SQL compat
log "--- B8: SQL compat ---"
SQL_START=$(date +%s)
if [ -f "$REPO_DIR/scripts/test/run_sql_corpus.sh" ]; then
    SQL_OUTPUT=$(bash "$REPO_DIR/scripts/test/run_sql_corpus.sh" --json 2>&1 || echo '{"pass_rate":0}')
    SQL_RATE=$(echo "$SQL_OUTPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('pass_rate',0))" 2>/dev/null || echo "0")
    SQL_END=$(date +%s)
    SQL_DURATION=$((SQL_END - SQL_START))
    log "B8: SQL pass rate = ${SQL_RATE}% (${SQL_DURATION}s)"
    
    MIN_SQL_RATE=85.0
    if (( $(echo "$SQL_RATE >= $MIN_SQL_RATE" | bc -l 2>/dev/null || echo "0 >= 85" | bc -l) )); then
        log "B8: PASS"
        ((PASS_COUNT++))
    else
        log "B8: FAIL (${SQL_RATE}% < ${MIN_SQL_RATE}%)"
        ((FAIL_COUNT++))
    fi
else
    log "B8: SKIP (SQL corpus script not found)"
    log "B8: Manual SQL compatibility check required"
    ((FAIL_COUNT++))
fi
log ""

# 汇总
log "=== Summary ==="
log "PASS: $PASS_COUNT/$TOTAL_COUNT"
log "FAIL: $FAIL_COUNT/$TOTAL_COUNT"
log "Log file: $LOG_FILE"
log ""

if [ "$PASS_COUNT" -eq "$TOTAL_COUNT" ]; then
    log "Beta Entry: APPROVED"
    log "Can proceed to Beta Gate"
    exit 0
else
    log "Beta Entry: REJECTED"
    log "Must fix $FAIL_COUNT failures before Beta entry"
    exit 1
fi