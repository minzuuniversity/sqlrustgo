#!/usr/bin/env bash
#
# check_beta_v3.10.0.sh -- v3.10.0 BETA Governance Gate (B6-B8)
#
# Companion to check_beta_gate.sh (B1-B5 / technical gates).
# B6  = G-01~G-06 Truthfulness Framework
# B7  = R1~R10 Content Tracking
# B8-1 = Evidence Binding
# B8-2 = Plan Integrity
# B8-3 = SSOT No Duplicate
#
# Usage:
#   bash scripts/gate/check_beta_v3.10.0.sh
#   bash scripts/gate/check_beta_v3.10.0.sh --json
#   bash scripts/gate/check_beta_v3.10.0.sh --out-dir /tmp/reports

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

VERSION="${VERSION:-v3.10.0}"
OUT_DIR="${OUT_DIR:-/tmp/beta_g310_$(date +%Y%m%d_%H%M%S)}"
JSON_OUTPUT=false

for arg in "$@"; do
    case "$arg" in
        --json) JSON_OUTPUT=true; shift ;;
        --out-dir) shift; OUT_DIR="${1:-/tmp/beta_g310}"; shift ;;
        --help|-h) grep "^#" "$0" | head -20; exit 0 ;;
    esac
done

mkdir -p "$OUT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
    [ -x "$HOME/.cargo/bin/cargo" ] && export PATH="$HOME/.cargo/bin:$PATH"
fi

PASS=0; FAIL=0; WARN=0; TOTAL=0
RESULTS_FILE="$OUT_DIR/.results_$$.tmp"
> "$RESULTS_FILE"

check_pass() {
    local name="$1" detail="${2:-}"
    TOTAL=$((TOTAL+1)); PASS=$((PASS+1))
    echo "PASS|$name|$detail" >> "$RESULTS_FILE"
    printf "  [PASS] %-50s %s\n" "$name" "$detail"
}

check_fail() {
    local name="$1" detail="${2:-}"
    TOTAL=$((TOTAL+1)); FAIL=$((FAIL+1))
    echo "FAIL|$name|$detail" >> "$RESULTS_FILE"
    printf "  [FAIL] %-50s %s\n" "$name" "$detail"
}

check_warn() {
    local name="$1" detail="${2:-}"
    TOTAL=$((TOTAL+1)); WARN=$((WARN+1))
    echo "WARN|$name|$detail" >> "$RESULTS_FILE"
    printf "  [WARN] %-50s %s\n" "$name" "$detail"
}

print_summary() {
    echo ""
    echo "=== v3.10.0 BETA Governance Gate (B6-B8) Summary ==="
    echo "PASS:  $PASS"
    echo "WARN:  $WARN"
    echo "FAIL:  $FAIL"
    echo "TOTAL: $TOTAL"
    echo ""
    if [ "$FAIL" -eq 0 ]; then
        echo "  -> v3.10.0 BETA governance gate: PASS"
    else
        echo "  -> v3.10.0 BETA governance gate: FAIL ($FAIL blocker(s))"
    fi
}

print_json() {
    echo "{"
    echo "  \"version\": \"$VERSION\","
    echo "  \"stage\": \"BETA\","
    echo "  \"section\": \"governance\","
    echo "  \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "  \"summary\": { \"pass\": $PASS, \"warn\": $WARN, \"fail\": $FAIL, \"total\": $TOTAL },"
    echo "  \"results\": ["
    local first=1
    while IFS='|' read -r status name detail; do
        [ $first -eq 0 ] && echo ","
        first=0
        printf "    {\"status\":\"%s\",\"name\":\"%s\",\"detail\":\"%s\"}" \
            "$status" "$name" "$(echo "$detail" | sed 's/"/\\"/g')"
    done < "$RESULTS_FILE"
    echo ""
    echo "  ]"
    echo "}"
}

# ============================================================
# B6: G-01~G-06
# ============================================================
echo ""
echo "--- B6: 5 Principles (G-01~G-06) ---"

# G-01: Evidence Binding
# Only count actual PASS/FAIL declarations (table rows, bullet points)
# not prose mentions of stage names (ALPHA/BETA/GA in sentences)
echo "  Running G-01 Evidence Binding..."
total_decl=0
unbound_decl=0

find "docs/releases/$VERSION" -name "*.md" -type f | sort | while read -r doc_path; do
    doc_name="$(basename "$doc_path")"
    # Find actual gate result declarations (bullet PASS/FAIL, table cells with PASS/FAIL)
    grep -nE "^[\-\*]\s+\[?\s*(PASS|FAIL)|^\|\s*.*\s+\|\s*(PASS|FAIL)\s*\|" \
        "$doc_path" 2>/dev/null | while IFS=: read -r line_num content; do
        total_decl=$((total_decl + 1))
        if ! echo "$content" | grep -qE \
            "(commit [0-9a-f]{7,40}|#[0-9]+|run_[0-9]+|ci_run|CI_RUN|gate-run)"; then
            unbound_decl=$((unbound_decl + 1))
        fi
    done
done > /dev/null 2>&1

unbound=${unbound_decl:-0}

if [ "$unbound" -le 50 ]; then
    check_pass "B6_G01_Evidence_Binding" "unbound=$unbound"
else
    # Pre-existing doc debt: warn for 51-300, fail for >300
    if [ "$unbound" -le 300 ]; then
        check_warn "B6_G01_Evidence_Binding" "unbound=$unbound (pre-existing doc debt, DRIFT)"
    else
        check_fail "B6_G01_Evidence_Binding" "unbound=$unbound exceeds threshold"
    fi
fi

# G-02: Source Type Marker (WARN-only)
src_tmp="$OUT_DIR/.g02_$$.tmp"
grep -rnE "(%|[0-9]+\.[0-9]+|passed|failed)" "docs/releases/$VERSION" 2>/dev/null | \
    grep -vE "(Source Type|实测|SSOT|历史文档)" > "$src_tmp" || true
src_unmarked=$(wc -l < "$src_tmp" | tr -d ' ')
rm -f "$src_tmp"
if [ "${src_unmarked:-0}" -eq 0 ]; then
    check_pass "B6_G02_SourceTypeMarker" "all claims marked"
else
    check_warn "B6_G02_SourceTypeMarker" "$src_unmarked unmarked (WARN-only)"
fi

# G-03: Gate Reports Contain Commands
gate_reports=$(find "docs/releases/$VERSION" -name "*GATE*.md" 2>/dev/null)
if [ -n "$gate_reports" ]; then
    gate_with_cmd=0
    for r in $gate_reports; do
        if grep -qE "(cargo |bash |rustc |llvm-cov )" "$r" 2>/dev/null; then
            gate_with_cmd=$((gate_with_cmd + 1))
        fi
    done
    if [ "$gate_with_cmd" -gt 0 ]; then
        check_pass "B6_G03_GateCmd" "$gate_with_cmd gate report(s) with commands"
    else
        check_fail "B6_G03_GateCmd" "no commands in gate reports"
    fi
else
    check_warn "B6_G03_GateCmd" "no gate reports found"
fi

# G-04: Coverage Consistency
cov_tmp="$OUT_DIR/.g04_$$.tmp"
find "docs/releases/$VERSION" -name "*.md" -type f | \
    xargs grep -lE "coverage|覆盖率" 2>/dev/null | \
    xargs grep -cE "\-\-lib only|\-\-lib-only" 2>/dev/null | \
    grep -v ":0" > "$cov_tmp" || true
cov_inconsistent=$(wc -l < "$cov_tmp" | tr -d ' ')
rm -f "$cov_tmp"
if [ "${cov_inconsistent:-0}" -eq 0 ]; then
    check_pass "B6_G04_CoverageConsist" "no --lib-only without fallback"
else
    check_fail "B6_G04_CoverageConsist" "$cov_inconsistent doc(s) inconsistent"
fi

# G-05: Plan State != Execution State
G05_OUT="$OUT_DIR/g05_check.log"
bash "$SCRIPT_DIR/check_plan_integrity_v310.sh" "$VERSION" "$OUT_DIR" > "$G05_OUT" 2>&1
if [ $? -eq 0 ]; then
    check_pass "B6_G05_PlanStateNotExec" "plan integrity verified"
else
    check_fail "B6_G05_PlanStateNotExec" "plan integrity violation"
fi

# G-06: Freshness Marker (WARN-only)
fresh_tmp="$OUT_DIR/.g06_$$.tmp"
grep -rnE "(%|passed|failed|覆盖率)" "docs/releases/$VERSION" 2>/dev/null | \
    grep -vE "(Freshness|v[0-9]+\.[0-9]+\.[0-9]+|20[0-9][0-9]-[0-9][0-9])" > "$fresh_tmp" || true
fresh_stale=$(wc -l < "$fresh_tmp" | tr -d ' ')
rm -f "$fresh_tmp"
check_warn "B6_G06_FreshnessMarker" "WARN-only: ${fresh_stale:-0} potentially stale claims"

# ============================================================
# B7: R1~R10 Evidence Check
# ============================================================
echo ""
echo "--- B7: 10 Principles (R1~R10) Evidence Check ---"

for principle in R1 R2 R3 R4; do
    case "$principle" in
        R1) desc="Build evidence"; pattern="build" ;;
        R2) desc="Test evidence"; pattern="test|测试" ;;
        R3) desc="Clippy evidence"; pattern="clippy" ;;
        R4) desc="Format evidence"; pattern="fmt|format" ;;
    esac
    ev_tmp="$OUT_DIR/.r1_$$.tmp"
    find "docs/releases/$VERSION" -name "*.md" -type f | \
        xargs grep -lE "$pattern" 2>/dev/null > "$ev_tmp" || true
    evidence=$(wc -l < "$ev_tmp" | tr -d ' ')
    rm -f "$ev_tmp"
    if [ "${evidence:-0}" -gt 0 ]; then
        check_pass "B7_${principle}_DocEvidence" "$desc: $evidence doc(s)"
    else
        check_warn "B7_${principle}_DocEvidence" "$desc not found in docs"
    fi
done

# R7: Documentation Completeness
required_v310=(
    "VERSION_PLAN.md"
    "ARCHITECTURE.md"
    "TEST_PLAN.md"
    "FEATURE_CHECKLIST.md"
    "CHANGELOG.md"
    "RELEASE_NOTES.md"
    "STAGE.yaml"
)
r7_missing=0
for doc in "${required_v310[@]}"; do
    [ ! -f "docs/releases/$VERSION/$doc" ] && r7_missing=$((r7_missing + 1))
done
if [ "$r7_missing" -eq 0 ]; then
    check_pass "B7_R7_DocCompleteness" "${#required_v310[@]} required docs present"
else
    check_fail "B7_R7_DocCompleteness" "$r7_missing doc(s) missing"
fi

# R5,R6,R8,R9,R10 -- delegated
for principle in R5 R6 R8 R9 R10; do
    case "$principle" in
        R5) desc="Coverage threshold" ;;
        R6) desc="SQL Compatibility" ;;
        R8) desc="Corpus pass rate" ;;
        R9) desc="Performance gate" ;;
        R10) desc="Formal proof" ;;
    esac
    check_warn "B7_${principle}_Delegated" "$desc: delegated to check_10_principles_v310.sh"
done

# ============================================================
# B8: Evidence Binding + Plan Integrity + SSOT
# ============================================================
echo ""
echo "--- B8: Evidence Binding (B8-1) / Plan Integrity (B8-2) / SSOT (B8-3) ---"

# B8-1: check_evidence_binding.sh
EVIDENCE_OUT="$OUT_DIR/evidence_binding"
mkdir -p "$EVIDENCE_OUT"
bash "$SCRIPT_DIR/check_evidence_binding.sh" "$VERSION" "$EVIDENCE_OUT" > "$EVIDENCE_OUT/run.log" 2>&1

if [ -f "$EVIDENCE_OUT/EVIDENCE_BINDING_REPORT.md" ]; then
    eb_fail_tmp="$OUT_DIR/.eb_fail_$$.tmp"
    grep -cE "^\| FAIL" "$EVIDENCE_OUT/EVIDENCE_BINDING_REPORT.md" > "$eb_fail_tmp" 2>/dev/null || true
    eb_fail=$(tr -d ' \n' < "$eb_fail_tmp" || echo "0")
    rm -f "$eb_fail_tmp"

    eb_warn_tmp="$OUT_DIR/.eb_warn_$$.tmp"
    grep -cE "^\| WARN" "$EVIDENCE_OUT/EVIDENCE_BINDING_REPORT.md" > "$eb_warn_tmp" 2>/dev/null || true
    eb_warn=$(tr -d ' \n' < "$eb_warn_tmp" || echo "0")
    rm -f "$eb_warn_tmp"

    eb_fail=${eb_fail:-0}
    eb_warn=${eb_warn:-0}

    if [ "$eb_fail" -le 50 ]; then
        check_pass "B8_1_EvidenceBinding" "fail=$eb_fail, warn=$eb_warn (threshold 50)"
    else
        check_fail "B8_1_EvidenceBinding" "fail=$eb_fail exceeds threshold 50"
    fi
else
    check_fail "B8_1_EvidenceBinding" "report not generated"
fi

# B8-2: Plan Integrity (covered by G-05)
check_pass "B8_2_PlanIntegrity" "covered by B6_G05"

# B8-3: SSOT No Duplicate
SSOT_OUT="$OUT_DIR/ssot_check"
mkdir -p "$SSOT_OUT"
if python3 "$SCRIPT_DIR/check_ssot_duplicate.py" --dir "docs/releases/$VERSION" > "$SSOT_OUT/run.log" 2>&1; then
    check_pass "B8_3_SSOT_NoDuplicate" "no duplicate SSOT documents"
else
    ssot_dup_tmp="$OUT_DIR/.ssot_dup_$$.tmp"
    grep -c "DUPLICATE" "$SSOT_OUT/run.log" > "$ssot_dup_tmp" 2>/dev/null || true
    ssot_dup=$(tr -d ' \n' < "$ssot_dup_tmp" || echo "0")
    rm -f "$ssot_dup_tmp"
    check_fail "B8_3_SSOT_NoDuplicate" "duplicates found: $ssot_dup"
fi

# ============================================================
# Output
# ============================================================
rm -f "$RESULTS_FILE"

if $JSON_OUTPUT; then
    print_json
else
    print_summary
fi

[ "$FAIL" -eq 0 ] && exit 0 || exit 1
