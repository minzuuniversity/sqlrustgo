#!/usr/bin/env bash
#
# Executor Operator-level Coverage Check Script
# Issue #1468: Executor Operator-level Coverage SSOT
#
# This script measures per-operator coverage using cargo-llvm-cov
# and compares against targets defined in COVERAGE_TARGETS.md
#
# Usage:
#   bash scripts/coverage/check_operator_coverage.sh [--verbose] [--json OUTPUT]
#

set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXECUTOR_CRATE="sqlrustgo-executor"
OUTPUT_DIR="${REPO_ROOT}/target/operator_coverage"
JSON_OUTPUT=""
VERBOSE=false

# ============================================================================
# Operator Definitions (Python-based for portability)
# ============================================================================

# This script uses Python for heavy lifting to ensure macOS/Linux compatibility

# ============================================================================
# Utility Functions
# ============================================================================

log_info() { echo "[INFO]  $*"; }
log_pass() { echo "[PASS]  $*"; }
log_fail() { echo "[FAIL]  $*"; }
log_warn() { echo "[WARN]  $*"; }

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --verbose|-v)
                VERBOSE=true
                shift
                ;;
            --json)
                JSON_OUTPUT="$2"
                shift 2
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                log_fail "Unknown argument: $1"
                exit 1
                ;;
        esac
    done
}

show_help() {
    cat << 'EOF'
Usage: bash scripts/coverage/check_operator_coverage.sh [OPTIONS]

Options:
    --verbose, -v    Show detailed output
    --json FILE      Write JSON report to FILE
    --help, -h       Show this help

Description:
    Measures per-operator coverage for the executor crate using cargo-llvm-cov.
    Compares against targets and reports PASS/FAIL/WARNING status.

Exit codes:
    0 - All operators at or above target
    1 - Any operator below (target - 5%)
    2 - Error running coverage
EOF
}

# ============================================================================
# Coverage Measurement
# ============================================================================

# Run llvm-cov and generate coverage data
run_coverage() {
    log_info "Running cargo-llvm-cov for executor crate..."
    
    mkdir -p "$OUTPUT_DIR"
    
    # Check if cargo-llvm-cov is available
    if ! command -v cargo-llvm-cov &> /dev/null; then
        log_info "Installing cargo-llvm-cov..."
        cargo install cargo-llvm-cov --locked 2>/dev/null || {
            log_fail "Failed to install cargo-llvm-cov"
            return 2
        }
    fi
    
    # Generate JSON coverage report (stderr mixed with stdout, capture separately)
    # Use temporary file to capture clean JSON
    local tmp_json=$(mktemp)
    cargo llvm-cov -p "$EXECUTOR_CRATE" --json 2>"$OUTPUT_DIR/llvm-cov.log" > "$tmp_json"
    # The JSON output starts with coverage info line which we extract
    if grep -q "^{" "$tmp_json"; then
        cp "$tmp_json" "$OUTPUT_DIR/cov.json"
        rm -f "$tmp_json"
        log_pass "Coverage generation complete"
        return 0
    else
        # cargo-llvm-cov adds info lines, extract JSON starting from first {
        grep "^{" "$tmp_json" > "$OUTPUT_DIR/cov.json" || true
        if [[ -s "$OUTPUT_DIR/cov.json" ]]; then
            rm -f "$tmp_json"
            log_pass "Coverage generation complete"
            return 0
        fi
        rm -f "$tmp_json"
        log_fail "Coverage generation failed"
        cat "$OUTPUT_DIR/llvm-cov.log"
        return 2
    fi
}

# ============================================================================
# Python-based Report Generation (portable)
# ============================================================================

generate_reports() {
    local json_file="$OUTPUT_DIR/cov.json"
    local output_file="${JSON_OUTPUT:-$OUTPUT_DIR/operator_coverage.json}"
    local verbose="$VERBOSE"
    
    python3 - "$json_file" "$output_file" "$verbose" << 'PYEOF'
import json
import sys
import os
from datetime import datetime

json_file = sys.argv[1]
output_file = sys.argv[2]
verbose = sys.argv[3].lower() == "true"

# Operator definitions
OPERATOR_FILES = {
    "SeqScan": "scan.rs",
    "IndexScan": "index_scan.rs",
    "IndexOnlyScan": "index_scan.rs",
    "Selection": "filter.rs",
    "Filter": "filter.rs",
    "HashJoin": "parallel_executor.rs",
    "NestedLoop": "parallel_executor.rs",
    "MergeJoin": "merge.rs",
    "HashAgg": "parallel_executor.rs",
    "SortAgg": "merge.rs",
    "SimpleAgg": "parallel_executor.rs",
    "Sort": "merge.rs",
    "Limit": "local_executor.rs",
    "TopN": "local_executor.rs",
    "Insert": "local_executor.rs",
    "Update": "local_executor.rs",
    "Delete": "local_executor.rs",
}

CATEGORY_TARGETS = {
    "Scan": 90,
    "Filter": 85,
    "Join": 80,
    "Aggregation": 85,
    "Sort/Limit": 85,
    "Write": 80,
}

OPERATOR_CATEGORIES = {
    "SeqScan": "Scan",
    "IndexScan": "Scan",
    "IndexOnlyScan": "Scan",
    "Selection": "Filter",
    "Filter": "Filter",
    "HashJoin": "Join",
    "NestedLoop": "Join",
    "MergeJoin": "Join",
    "HashAgg": "Aggregation",
    "SortAgg": "Aggregation",
    "SimpleAgg": "Aggregation",
    "Sort": "Sort/Limit",
    "Limit": "Sort/Limit",
    "TopN": "Sort/Limit",
    "Insert": "Write",
    "Update": "Write",
    "Delete": "Write",
}

try:
    with open(json_file) as f:
        data = json.load(f)
except FileNotFoundError:
    print(f"[FAIL] Coverage file not found: {json_file}")
    sys.exit(2)
except json.JSONDecodeError:
    print(f"[FAIL] Invalid JSON in coverage file: {json_file}")
    sys.exit(2)

# Extract file coverage
operators = []
file_coverage = {}

for entry in data.get("data", []):
    for file in entry.get("files", []):
        filename = file.get("filename", "")
        summary = file.get("summary", {})
        totals = summary.get("totals", {})
        line_pct = totals.get("line", {}).get("percent", 0)
        branch_pct = totals.get("branch", {}).get("percent", 0)
        
        # Store file coverage
        for op, file_pattern in OPERATOR_FILES.items():
            if file_pattern in filename:
                file_coverage[file_pattern] = {
                    "line": line_pct,
                    "branch": branch_pct
                }
        
        # Check for overall totals
        if "totals" in file and file.get("totals"):
            pass

# Calculate overall
overall = 0
for entry in data.get("data", []):
    totals = entry.get("totals", {})
    overall = totals.get("line", {}).get("percent", 0)
    break

# Build operator data
for op, file_pattern in OPERATOR_FILES.items():
    cov = file_coverage.get(file_pattern, {"line": 0, "branch": 0})
    category = OPERATOR_CATEGORIES[op]
    target = CATEGORY_TARGETS[category]
    line_pct = cov["line"]
    branch_pct = cov["branch"]
    
    if line_pct >= target:
        status = "pass"
    elif line_pct >= target - 5:
        status = "warning"
    else:
        status = "fail"
    
    operators.append({
        "name": op,
        "category": category,
        "file": file_pattern,
        "line_coverage": round(line_pct, 2),
        "branch_coverage": round(branch_pct, 2),
        "target": target,
        "status": status
    })

# Write JSON report
report = {
    "timestamp": datetime.now().isoformat(),
    "version": "3.6.0",
    "overall_coverage": round(overall, 2),
    "operators": operators
}

with open(output_file, "w") as f:
    json.dump(report, f, indent=2)

print("")
print("=" * 80)
print("           EXECUTOR OPERATOR COVERAGE REPORT")
print("=" * 80)
print(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
print(f"Overall Coverage: {overall:.2f}%")
print("")

# Count statuses
pass_count = sum(1 for op in operators if op["status"] == "pass")
warn_count = sum(1 for op in operators if op["status"] == "warning")
fail_count = sum(1 for op in operators if op["status"] == "fail")

# Group by category
categories = ["Scan", "Filter", "Join", "Aggregation", "Sort/Limit", "Write"]
for category in categories:
    cat_ops = [op for op in operators if op["category"] == category]
    if not cat_ops:
        continue
    
    target = CATEGORY_TARGETS[category]
    print("-" * 80)
    print(f"{category} Operators (Target: {target}%)")
    print("-" * 80)
    
    for op in cat_ops:
        line_pct = op["line_coverage"]
        status_symbol = {"pass": "✓", "warning": "⚠", "fail": "✗"}[op["status"]]
        print(f"  {op['name']:<15} {line_pct:>6.2f}%  [{op['status']:^7}]  ({op['file']})")
        if verbose:
            print(f"                    Target: {op['target']}%, Branch: {op['branch_coverage']:.2f}%")
    print("")

print("=" * 80)
print("SUMMARY")
print("=" * 80)
print(f"Total Operators: {len(operators)}")
print(f"  PASS:     {pass_count}")
print(f"  WARNING:  {warn_count}")
print(f"  FAIL:     {fail_count}")
print("")

# Overall result
if fail_count > 0:
    print(f"Result: FAIL - {fail_count} operator(s) below warning threshold")
    sys.exit(1)
elif warn_count > 0:
    print(f"Result: WARNING - {warn_count} operator(s) below target but above threshold")
    sys.exit(0)
else:
    print("Result: PASS - All operators at or above target")
    sys.exit(0)

PYEOF
}

# ============================================================================
# Main
# ============================================================================

main() {
    parse_args "$@"
    
    export REPO_ROOT JSON_OUTPUT VERBOSE
    
    cd "$REPO_ROOT"
    
    log_info "Executor Operator Coverage Check"
    log_info "Issue #1468: Executor Operator-level Coverage SSOT"
    echo ""
    
    # Run coverage
    if ! run_coverage; then
        log_fail "Coverage measurement failed"
        exit 2
    fi
    
    # Generate reports
    generate_reports
    local exit_code=$?
    
    echo ""
    log_info "JSON report: $OUTPUT_DIR/operator_coverage.json"
    
    exit $exit_code
}

main "$@"
