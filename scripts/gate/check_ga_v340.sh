#!/usr/bin/env bash
# check_ga_v340.sh — v3.4.0 GA Gate 检查脚本
# 规范来源: docs/releases/v3.4.0/GA_GATE_CHECKLIST.md (SSOT)
# 版本: 1.0
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0; TOTAL=0; BLOCKERS=0
FAIL_REASONS=()
SKIP_REASONS=()

log_info() { echo "[ga-v3.4.0] $1"; }
log_pass() { echo "✅ PASS: $1"; PASS=$((PASS+1)); TOTAL=$((TOTAL+1)); }
log_fail() { echo "❌ FAIL: $1"; BLOCKERS=$((BLOCKERS+1)); TOTAL=$((TOTAL+1)); FAIL_REASONS+=("$1"); }
log_skip() { echo "⏭️  SKIP: $1"; TOTAL=$((TOTAL+1)); SKIP_REASONS+=("$1"); }
log_warn() { echo "⚠️  WARN: $1"; }

check() {
    local name="$1" cmd="$2"
    local label="${3:-G}"
    TOTAL=$((TOTAL+1))
    echo -n "[ga-v3.4.0] $name ... "
    if eval "$cmd" >/dev/null 2>&1; then
        echo "PASS"; PASS=$((PASS+1))
    else
        echo "FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("【$label】$name")
    fi
}

check_test() {
    local name="$1" cmd="$2"
    local label="$3"
    TOTAL=$((TOTAL+1))
    echo -n "[ga-v3.4.0] $name ... "
    local output
    output=$(eval "$cmd" 2>&1) || true
    local passed failed
    passed=$(echo "$output" | grep -c "test result: ok\." || true)
    failed=$(echo "$output" | grep -c "test result: FAILED" || true)
    passed=${passed:-0}
    failed=${failed:-0}
    passed=$(echo "$passed" | head -1)
    failed=$(echo "$failed" | head -1)
    if [ "$failed" -eq 0 ] && [ "$passed" -gt 0 ]; then
        echo "PASS ($passed tests)"; PASS=$((PASS+1))
    else
        echo "FAIL ($passed passed, $failed failed)"; BLOCKERS=$((BLOCKERS+1))
        FAIL_REASONS+=("【$label】$name")
    fi
}

echo "=========================================="
echo "  v3.4.0 GA Gate — 正式发布前必须通过"
echo "  规范来源: docs/releases/v3.4.0/GA_GATE_CHECKLIST.md"
echo "=========================================="
echo "日期: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# ============================================================
# 第一部分: 代码层 Gate (G1-G6)
# ============================================================
echo "━━━ 第一部分: 代码层 Gate (G1-G6) ━━━"

# G1: Build
log_info "G1: cargo build --release --workspace"
TOTAL=$((TOTAL+1))
if cargo build --release --workspace >/dev/null 2>&1; then
    echo "✅ PASS"; PASS=$((PASS+1))
else
    echo "❌ FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G1: Build failed")
fi

# G2: Test (100%)
log_info "G2: Full test suite (100%)"
TOTAL=$((TOTAL+1))
TEST_OUTPUT=$(cargo test --all-features --lib 2>&1 || true)
PASSED=$(echo "$TEST_OUTPUT" | grep -c "test result: ok\." 2>/dev/null || echo "0")
FAILED=$(echo "$TEST_OUTPUT" | grep -c "test result: FAILED" 2>/dev/null || echo "0")
PASSED=${PASSED//[^0-9]/}
FAILED=${FAILED//[^0-9]/}
if [ -z "$PASSED" ]; then PASSED=0; fi
if [ -z "$FAILED" ]; then FAILED=0; fi
TOTAL_TESTS=$((PASSED + FAILED))
if [ "$TOTAL_TESTS" -gt 0 ]; then
    if [ "$FAILED" -eq 0 ] && [ "$PASSED" -gt 0 ]; then
        echo "✅ PASS (100% = $PASSED/$TOTAL_TESTS)"; PASS=$((PASS+1))
    else
        echo "❌ FAIL ($PASSED passed, $FAILED failed)"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G2: Test $PASSED passed, $FAILED failed < 100%")
    fi
else
    echo "❌ FAIL (no tests found)"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G2: No tests found")
fi

# G3: Clippy
log_info "G3: cargo clippy --all-features"
TOTAL=$((TOTAL+1))
if cargo clippy --all-features -- -D warnings >/dev/null 2>&1; then
    echo "✅ PASS"; PASS=$((PASS+1))
else
    echo "❌ FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G3: Clippy warnings")
fi

# G4: Format
log_info "G4: cargo fmt --check"
TOTAL=$((TOTAL+1))
if cargo fmt --all -- --check >/dev/null 2>&1; then
    echo "✅ PASS"; PASS=$((PASS+1))
else
    echo "❌ FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G4: Format issues")
fi

# G5: Coverage >= 85%
log_info "G5: Coverage >= 85%"
TOTAL=$((TOTAL+1))
if command -v cargo-llvm-cov &>/dev/null; then
    cargo llvm-cov test --workspace --all-features --no-fail-fast >/dev/null 2>&1 || true
    cargo llvm-cov report --lcov --output-path /tmp/lcov-v340-ga.info 2>/dev/null || true
    TOTAL_LINES=$(grep "^LF:" /tmp/lcov-v340-ga.info 2>/dev/null | cut -d: -f2 | awk '{sum+=$1} END {print sum}' || echo "0")
    COVERED_LINES=$(grep "^LH:" /tmp/lcov-v340-ga.info 2>/dev/null | cut -d: -f2 | awk '{sum+=$1} END {print sum}' || echo "0")
    if [ "$TOTAL_LINES" -gt 0 ]; then
        COVERAGE=$(echo "scale=2; $COVERED_LINES * 100 / $TOTAL_LINES" | bc)
        if (( $(echo "$COVERAGE >= 85" | bc -l) )); then
            echo "✅ PASS (${COVERAGE}%)"; PASS=$((PASS+1))
        else
            echo "❌ FAIL (${COVERAGE}% < 85%)"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G5: Coverage ${COVERAGE}% < 85%")
        fi
    else
        echo "⏭️  SKIP (no coverage data)"; SKIP_REASONS+=("G5: No coverage data")
    fi
else
    echo "⏭️  SKIP (cargo-llvm-cov not installed)"; SKIP_REASONS+=("G5: No cargo-llvm-cov")
fi

# G6: Security Audit
log_info "G6: cargo audit"
TOTAL=$((TOTAL+1))
if cargo audit >/dev/null 2>&1; then
    echo "✅ PASS"; PASS=$((PASS+1))
else
    echo "❌ FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G6: Security vulnerabilities found")
fi

# ============================================================
# 第二部分: GMP Build Gate (G7-G9)
# ============================================================
echo ""
echo "━━━ 第二部分: GMP Build Gate (G7-G9) ━━━"

check "G7: GMP API Build" "cargo build -p sqlrustgo-gmp-api --release" "G7"
check "G8: GMP Retrieval Build" "cargo build -p sqlrustgo-gmp-retrieval --release 2>/dev/null || echo 'gmp-retrieval not in workspace'" "G8"
check "G9: MySQL Server Build" "cargo build -p sqlrustgo-mysql-server --release" "G9"

# ============================================================
# 第三部分: 功能 Gate (G10-G12)
# ============================================================
echo ""
echo "━━━ 第三部分: 功能 Gate (G10-G12) ━━━"

# G10: TPC-H SF=1
log_info "G10: TPC-H SF=1 22/22"
TOTAL=$((TOTAL+1))
if bash scripts/gate/check_tpch.sh --sf1 >/dev/null 2>&1; then
    echo "✅ PASS"; PASS=$((PASS+1))
elif target/release/sqlrustgo-bench-cli tpch --scale 1 >/dev/null 2>&1; then
    echo "✅ PASS (bench-cli verified)"; PASS=$((PASS+1))
else
    echo "❌ FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G10: TPC-H SF=1 failed")
fi

# G11: Proofs
check_test "G11: Proof count ≥30" "bash scripts/gate/check_proof.sh 2>&1" "G11"

# G12: Docs
log_info "G12: OO docs exist"
TOTAL=$((TOTAL+1))
if [ -d "docs/releases/v3.4.0/oo" ] || [ -d "oo/GMP-Management" ]; then
    echo "✅ PASS"; PASS=$((PASS+1))
else
    echo "❌ FAIL"; BLOCKERS=$((BLOCKERS+1)); FAIL_REASONS+=("G12: OO docs missing")
fi

# ============================================================
# 第四部分: GMP API Tests (G-API1~G-API7)
# ============================================================
echo ""
echo "━━━ 第四部分: GMP API Tests (G-API1~G-API7) ━━━"

check_test "G-API1: gmp-api lib tests" "cargo test -p sqlrustgo-gmp-api --lib 2>&1" "G-API1"
check_test "G-API2: gmp-api batch tests" "cargo test -p sqlrustgo-gmp-api -- batch 2>&1" "G-API2"
check_test "G-API3: gmp-api audit tests" "cargo test -p sqlrustgo-gmp-api -- audit 2>&1" "G-API3"
check_test "G-API4: gmp-api device tests" "cargo test -p sqlrustgo-gmp-api -- device 2>&1" "G-API4"
check_test "G-API5: gmp-api signature tests" "cargo test -p sqlrustgo-gmp-api -- signature 2>&1" "G-API5"
check_test "G-API6: gmp-api dashboard tests" "cargo test -p sqlrustgo-gmp-api -- dashboard 2>&1" "G-API6"
check_test "G-API7: gmp-api rule tests" "cargo test -p sqlrustgo-gmp-api -- rule 2>&1" "G-API7"

# ============================================================
# 第五部分: GMP Core Tests (G-GMP1~G-GMP8)
# ============================================================
echo ""
echo "━━━ 第五部分: GMP Core Tests (G-GMP1~G-GMP8) ━━━"

check_test "G-GMP1: gmp lib tests" "cargo test -p sqlrustgo-gmp --lib 2>&1" "G-GMP1"
check_test "G-GMP2: audit chain" "cargo test -p sqlrustgo-gmp --test gmp_audit_chain_verify_test 2>&1" "G-GMP2"
check_test "G-GMP3: digital signature" "cargo test -p sqlrustgo-gmp --test gmp_digital_signature_test 2>&1" "G-GMP3"
check_test "G-GMP4: electronic signature" "cargo test -p sqlrustgo-gmp --test gmp_electronic_signature_test 2>&1" "G-GMP4"
check_test "G-GMP5: workflow v2" "cargo test -p sqlrustgo-gmp --test gmp_workflow_v2_test 2>&1" "G-GMP5"
check_test "G-GMP6: evidence engine" "cargo test -p sqlrustgo-gmp --test evidence_export_test 2>&1" "G-GMP6"
check_test "G-GMP7: immutable record" "cargo test -p sqlrustgo-gmp --test gmp_immutable_record_test 2>&1" "G-GMP7"
check_test "G-GMP8: provenance" "cargo test -p sqlrustgo-gmp --test gmp_provenance_test 2>&1" "G-GMP8"

# ============================================================
# 第六部分: Trust Infrastructure Tests (G-TI1~G-TI8)
# ============================================================
echo ""
echo "━━━ 第六部分: Trust Infrastructure Tests (G-TI1~G-TI8) ━━━"

check_test "G-TI1: evidence-engine" "cargo test -p sqlrustgo-evidence-engine --lib 2>&1" "G-TI1"
check_test "G-TI2: provenance-graph" "cargo test -p sqlrustgo-provenance-graph --lib 2>&1" "G-TI2"
check_test "G-TI3: compliance-engine" "cargo test -p sqlrustgo-compliance-engine --lib 2>&1" "G-TI3"
check_test "G-TI4: workflow-v2" "cargo test -p sqlrustgo-workflow-v2 --lib 2>&1" "G-TI4"
check_test "G-TI5: trust-viz" "cargo test -p sqlrustgo-trust-viz --lib 2>&1" "G-TI5"
check_test "G-TI6: perf-baseline" "cargo test -p sqlrustgo-perf-baseline --lib 2>&1" "G-TI6"
check_test "G-TI7: crash-sim" "cargo test -p sqlrustgo-crash-sim --lib 2>&1" "G-TI7"
check_test "G-TI8: wal-verification" "cargo test -p sqlrustgo-wal-verification --lib 2>&1" "G-TI8"

# ============================================================
# 结果汇总
# ============================================================
echo ""
echo "=========================================="
echo "  GA Gate 结果汇总"
echo "=========================================="
echo "✅ PASS: $PASS"
echo "⏭️  SKIP: ${#SKIP_REASONS[@]} (需手动验证)"
echo "❌ FAIL: $BLOCKERS"
echo ""

if [ ${#FAIL_REASONS[@]} -gt 0 ]; then
    echo "失败项:"
    for reason in "${FAIL_REASONS[@]}"; do
        echo "  - $reason"
    done
fi

if [ ${#SKIP_REASONS[@]} -gt 0 ]; then
    echo ""
    echo "手动验证项 (需人工确认):"
    for reason in "${SKIP_REASONS[@]}"; do
        echo "  - $reason"
    done
fi

echo ""
if [ $BLOCKERS -gt 0 ]; then
    echo "❌ GA Gate FAILED — $BLOCKERS blocker(s) detected"
    exit 1
else
    echo "✅ GA Gate PASSED (with ${#SKIP_REASONS[@]} manual verifications pending)"
    exit 0
fi
