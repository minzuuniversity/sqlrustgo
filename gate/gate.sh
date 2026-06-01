#!/usr/bin/env bash
# ============================================================
# gate/gate.sh — SQLRustGo unified gate entry point
# Replaces gate/hermes_gate.sh + scripts/gate/gate.sh confusion
# ============================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERSION="${1:-v3.7.0}"
EXIT_CODE=0
REPORT_DIR="${REPORT_DIR:-.}"

cd "$PROJECT_ROOT"

# Ensure helper scripts are executable
chmod +x "$PROJECT_ROOT/scripts/gate"/check_*.sh 2>/dev/null || true
chmod +x "$PROJECT_ROOT/scripts/gate"/audit_*.sh 2>/dev/null || true

echo "=== SQLRustGo Gate Check ==="
echo "Version: $VERSION"
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'detached')"
echo "Commit: $(git rev-parse HEAD 2>/dev/null || echo 'unknown')"
echo "Date: $(date -Iseconds)"
echo ""

# ── L1: Code Quality ──────────────────────────────────────
echo "[L1] === Code Quality ==="

echo "[L1] cargo build..."
if cargo build --all-features > /dev/null 2>&1; then
    echo "[PASS] build"
else
    echo "[FAIL] build"
    EXIT_CODE=1
fi

echo "[L1] cargo test (lib, exclude mysql-server)..."
if cargo test --lib --quiet 2>&1 | grep -q "test result: ok"; then
    echo "[PASS] test lib"
else
    # fallback: check exit code
    cargo test --lib --quiet > /dev/null 2>&1 || { echo "[FAIL] test lib"; EXIT_CODE=1; }
fi

echo "[L1] clippy..."
if cargo clippy --all-features -- -D warnings > /dev/null 2>&1; then
    echo "[PASS] clippy"
else
    echo "[FAIL] clippy"
    EXIT_CODE=1
fi

echo "[L1] cargo fmt..."
if cargo fmt --check --all > /dev/null 2>&1; then
    echo "[PASS] fmt"
else
    echo "[FAIL] fmt"
    EXIT_CODE=1
fi

# ── L1: Documentation ─────────────────────────────────────
echo ""
echo "[L1] === Documentation ==="

DOCS_DIR="$PROJECT_ROOT/docs/releases/$VERSION"
if [ -d "$DOCS_DIR" ]; then
    for doc in "$DOCS_DIR/RELEASE_GATE_CHECKLIST.md" "$DOCS_DIR/RELEASE_NOTES.md" "$DOCS_DIR/CHANGELOG.md"; do
        if [ -f "$doc" ]; then
            echo "[PASS] $(basename $doc)"
        else
            echo "[WARN] missing $doc"
        fi
    done
else
    echo "[WARN] docs/releases/$VERSION not found"
fi

# ── L2: Coverage (if available) ──────────────────────────
echo ""
echo "[L2] === Coverage ==="

COVERAGE_OUT="$REPORT_DIR/coverage_gate_out"
mkdir -p "$COVERAGE_OUT"
if command -v cargo-llvm-cov &>/dev/null; then
    echo "[L2] Running cargo llvm-cov..."
    if cargo llvm-cov --all-features --quiet --output-dir "$COVERAGE_OUT" > "$COVERAGE_OUT/cov.log" 2>&1; then
        TOTAL=$(grep "TOTAL" "$COVERAGE_OUT/cov.txt" 2>/dev/null | awk '{print $2}' | tr -d '%' || echo "0")
        echo "[L2] Coverage: ${TOTAL}% (threshold: 50%)"
        if [ "${TOTAL:-0}" -lt 50 ]; then
            echo "[WARN] coverage below 50%, see $COVERAGE_OUT/cov.log"
        fi
    else
        echo "[WARN] coverage measurement failed, skipping"
    fi
else
    echo "[SKIP] cargo-llvm-cov not available"
fi

# ── L2: Performance baseline (if established) ────────────
echo ""
echo "[L2] === Performance Baseline ==="

if [ -f "$PROJECT_ROOT/benchmark_baseline.json" ]; then
    echo "[L2] Benchmark baseline exists — OK"
else
    echo "[SKIP] No benchmark baseline (run cargo bench first)"
fi

# ── Governance: Evidence Binding ─────────────────────────
echo ""
echo "[Governance] === Evidence Binding ==="

if [ -f "$PROJECT_ROOT/scripts/gate/check_evidence_binding.sh" ]; then
    mkdir -p "$REPORT_DIR/evidence_binding_out"
    VERSION_DIR="$PROJECT_ROOT/docs/releases/$VERSION"
    if bash "$PROJECT_ROOT/scripts/gate/check_evidence_binding.sh" "$VERSION" "$REPORT_DIR/evidence_binding_out/" > "$REPORT_DIR/evidence_binding.log" 2>&1; then
        PASS_COUNT=$(grep -c "PASS" "$REPORT_DIR/evidence_binding.log" 2>/dev/null || echo "0")
        FAIL_COUNT=$(grep -c "FAIL" "$REPORT_DIR/evidence_binding.log" 2>/dev/null || echo "0")
        echo "[Governance] evidence binding: $PASS_COUNT passed, $FAIL_COUNT failed"
    else
        echo "[WARN] evidence binding check had issues — review $REPORT_DIR/evidence_binding.log"
    fi
else
    echo "[SKIP] check_evidence_binding.sh not found"
fi

# ── Final Report ──────────────────────────────────────────
echo ""
echo "=== Gate Result ==="
if [ $EXIT_CODE -eq 0 ]; then
    echo "PASSED"
else
    echo "FAILED (code=$EXIT_CODE)"
fi

# Write JSON report
mkdir -p "$(dirname "$REPORT_DIR/gate_report.json")"
cat > "$REPORT_DIR/gate_report.json" <<EOF
{
  "gate": "gate.sh",
  "version": "$VERSION",
  "passed": $([ $EXIT_CODE -eq 0 ] && echo "true" || echo "false"),
  "commit": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "timestamp": "$(date -Iseconds)"
}
EOF

exit $EXIT_CODE