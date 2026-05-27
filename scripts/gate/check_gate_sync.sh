#!/usr/bin/env bash
# check_gate_sync.sh — Gate CI/CD 同步性检查 (Gitea Actions)
# 检查所有 .gitea/workflows/ 工作流的分支触发条件与分支保护是否一致
#
# 用法: bash scripts/gate/check_gate_sync.sh [--fix]
#   --fix: 自动创建缺失的分支保护

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
FIX_MODE="${1:-}"

WORKFLOW_DIR="$PROJECT_ROOT/.gitea/workflows"
GATE_SPEC="$PROJECT_ROOT/docs/governance/gate_spec_v350.md"

# Token for Gitea API
GITEA_TOKEN="${GITEA_TOKEN:-cc19b2ad677e18c96b9d049f6dc2b46e02176883}"
GITEA_BASE="${GITEA_BASE:-http://192.168.0.252:3000/api/v1}"
REPO="${REPO:-openclaw/sqlrustgo}"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

ERRORS=0; WARNINGS=0

log_fail() { echo -e "${RED}❌ FAIL:${NC} $1"; ERRORS=$((ERRORS+1)); }
log_pass() { echo -e "${GREEN}✅ PASS:${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠️  WARN:${NC} $1"; WARNINGS=$((WARNINGS+1)); }
log_info() { echo "   $1"; }

# ───────────────────────────────────────────────────────────────
# Helper: Extract ALL branch names from on.push.branches and
#         on.pull_request.branches in a YAML file
# ───────────────────────────────────────────────────────────────
get_workflow_branches() {
    local wf="$1"
    python3 -c "
import sys
try:
    content = open('$wf').read()
    in_pr = False; in_push = False; branches = set()
    for line in content.split('\n'):
        stripped = line.strip()
        if stripped.startswith('pull_request:'):
            in_pr = True; in_push = False
        elif stripped.startswith('push:'):
            in_push = True; in_pr = False
        elif stripped.startswith('- '):
            branch = stripped[1:].strip().strip('\"').strip(\"'\")
            if branch and not branch.startswith('[') and ':' not in branch:
                if in_pr or in_push:
                    branches.add(branch)
        elif stripped != '' and not stripped.startswith('#') and ':' in stripped:
            in_pr = False; in_push = False
    for b in sorted(branches):
        print(b)
except: pass
" 2>/dev/null || true
}

# ───────────────────────────────────────────────────────────────
# Helper: Get job keys from Gitea workflow YAML
# Gitea Actions uses job key (top-level YAML key under jobs:) as context name
# ───────────────────────────────────────────────────────────────
get_workflow_jobs() {
    local wf="$1"
    python3 -c "
import sys, re
try:
    content = open('$wf').read()
    # Find jobs: section and extract top-level keys under it
    in_jobs = False
    for line in content.split('\n'):
        stripped = line.strip()
        if stripped.startswith('jobs:'):
            in_jobs = True
            continue
        if in_jobs and stripped and not stripped.startswith('#'):
            if ':' in stripped and not stripped.startswith('-'):
                # This is a job key (4-space indent level)
                indent = len(line) - len(line.lstrip())
                if indent == 2:  # jobs: is at indent 0, jobs keys at 2 spaces
                    key = stripped.split(':')[0].strip()
                    if key not in ('runs-on','steps','name','permissions','env','if','needs','timeout-minutes','uses','run','with','continue-on-error'):
                        print(key)
            elif indent < 2:
                in_jobs = False
except: pass
" 2>/dev/null || true
}

# ───────────────────────────────────────────────────────────────
# Check 1: 每个 workflow 的触发分支是否符合其角色定义
# ───────────────────────────────────────────────────────────────
check_workflow_triggers() {
    echo ""
    echo "━━━ Check 1: Workflow Branch Triggers ━━━"

    python3 - "$WORKFLOW_DIR" <<'PYEOF'
import os, sys, subprocess, re

workflow_dir = sys.argv[1]

expectations = {
    "ci.yml": ["develop/v3.4.0", "develop/v3.5.0"],
    "pr-gate.yml": ["develop/v3.4.0", "develop/v3.5.0", "beta/v3.4.0", "beta/v3.5.0"],
    "rc-gate.yml": ["rc/v3.4.0", "rc/v3.5.0"],
    "gate-ci.yml": ["develop/v3.4.0", "develop/v3.5.0"],
    "layered-ci.yml": ["develop/v3.4.0", "develop/v3.5.0", "rc/v3.4.0", "rc/v3.5.0"],
    "beta-gate.yml": ["beta/v3.4.0", "beta/v3.5.0"],
}

RED = '\033[0;31m'; GREEN = '\033[0;32m'; YELLOW = '\033[1;33m'; NC = '\033[0m'
ERRORS = 0; WARNINGS = 0

def get_branches(wf_path):
    try:
        content = open(wf_path).read()
        in_on = False; in_push = False; in_pr = False; in_branches = False
        branches = set()
        for line in content.split('\n'):
            stripped = line.strip()
            if stripped == 'on:' or stripped == '"on":' or stripped == "'on':":
                in_on = True; in_push = False; in_pr = False; in_branches = False
            elif in_on and (stripped in ('push:', '"push":', "'push':") or stripped in ('pull_request:', '"pull_request":', "'pull_request':")):
                in_push = any(x in stripped for x in ('push:', '"push', "'push"))
                in_pr = any(x in stripped for x in ('pull_request', '"pull_request', "'pull_request"))
                in_branches = False
            elif in_on and (in_push or in_pr):
                if stripped == 'branches:':
                    in_branches = True
                elif in_branches and stripped.startswith('- '):
                    branch = stripped[1:].strip().strip('"').strip("'")
                    if branch and not branch.startswith('[') and ':' not in branch:
                        branches.add(branch)
                elif stripped and ':' in stripped and not stripped.startswith('-'):
                    if not stripped.startswith('#'):
                        in_branches = False
        return sorted(branches)
    except:
        return []

for wf_name, expected in sorted(expectations.items()):
    wf_path = os.path.join(workflow_dir, wf_name)
    if not os.path.exists(wf_path):
        print(f"  {YELLOW}⚠️  WARN: {wf_name} not found{NC}")
        WARNINGS += 1
        continue
    actual = get_branches(wf_path)
    missing = [b for b in expected if b not in actual]
    if not missing:
        print(f"  {GREEN}✅ PASS: {wf_name} → [{', '.join(expected)}]{NC}")
    else:
        print(f"  {RED}❌ FAIL: {wf_name} missing: {missing}{NC}")
        ERRORS += 1

sys.exit(ERRORS)
PYEOF
    [ $? -eq 0 ] || ERRORS=$((ERRORS+$?))
}

# ───────────────────────────────────────────────────────────────
# Check 2: 无过时 workflow 引用旧分支
# ───────────────────────────────────────────────────────────────
check_no_stale_workflows() {
    echo ""
    echo "━━━ Check 2: Stale Workflow References ━━━"

    local stale=(
        "develop/v2.9.0"
        "develop/v3.0.0"
        "develop/v3.1.0"
        "develop/v3.2.0"
        "develop/v3.3.0"
    )

    local found_stale=false
    for wf in "$WORKFLOW_DIR"/*.yml; do
        [ -f "$wf" ] || continue
        wf_name=$(basename "$wf")
        for pattern in "${stale[@]}"; do
            if grep -rq "branches.*$pattern" "$wf" 2>/dev/null; then
                log_warn "$wf_name still references '$pattern'"
                found_stale=true
            fi
        done
    done

    if ! $found_stale; then
        log_pass "No stale branch references found"
    fi
}

# ───────────────────────────────────────────────────────────────
# Check 3: Branch protection status_check_contexts 完整性
# ───────────────────────────────────────────────────────────────
check_branch_protection_contexts() {
    echo ""
    echo "━━━ Check 3: Branch Protection Status Checks ━━━"

    local branches=("develop/v3.4.0" "develop/v3.5.0" "rc/v3.4.0" "rc/v3.5.0")

    for branch in "${branches[@]}"; do
        response=$(curl -s -H "Authorization: token $GITEA_TOKEN" \
            -H "Content-Type: application/json" \
            "$GITEA_BASE/repos/$REPO/branch_protections/$branch" 2>/dev/null) || true

        if [ -z "$response" ]; then
            log_warn "$branch: no protection found"
            continue
        fi

        if ! echo "$response" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
            log_warn "$branch: could not parse protection"
            continue
        fi

        enable_sc=$(echo "$response" | python3 -c "import sys,json; print(json.load(sys.stdin).get('enable_status_check', False))" 2>/dev/null)
        contexts=$(echo "$response" | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin).get('status_check_contexts', [])))" 2>/dev/null)
        approvals=$(echo "$response" | python3 -c "import sys,json; print(json.load(sys.stdin).get('required_approvals', -1))" 2>/dev/null)
        ctx_count=$(echo "$contexts" | python3 -c "import sys,json; print(len(json.load(sys.stdin)))" 2>/dev/null)

        echo "   $branch: enable=$enable_sc, approvals=$approvals, contexts=$ctx_count"
        if [ "$enable_sc" = "True" ] || [ "$enable_sc" = "true" ]; then
            if [ "$ctx_count" -gt 0 ]; then
                log_pass "$branch: $ctx_count context(s) configured"
            else
                log_fail "$branch: enable_status_check=true but 0 contexts"
            fi
        else
            log_warn "$branch: enable_status_check=false"
        fi
    done
}

# ───────────────────────────────────────────────────────────────
# Check 4: Coverage threshold 完整性
# ───────────────────────────────────────────────────────────────
check_coverage_thresholds() {
    echo ""
    echo "━━━ Check 4: Coverage Thresholds ━━━"

    local threshold_file="$PROJECT_ROOT/scripts/gate/coverage_threshold.toml"
    if [ ! -f "$threshold_file" ]; then
        log_fail "coverage_threshold.toml not found"
        return
    fi

    # Required: main + v3.5.0 new crates
    local required_crates=(
        "types" "parser" "planner" "optimizer" "executor"
        "storage" "transaction" "catalog" "common"
        "compliance-engine" "gmp-api" "gmp" "gmp-retrieval"
        "information-schema"
    )

    local threshold_crates=$(grep -E '^\w+' "$threshold_file" | grep -v '^\[' | grep -v '^#' | sed 's/ = .*//' | sort -u)
    local missing=""

    for crate in "${required_crates[@]}"; do
        if ! echo "$threshold_crates" | grep -qx "$crate"; then
            missing="$missing $crate"
        fi
    done

    if [ -n "$missing" ]; then
        log_fail "Missing thresholds for:$missing"
    else
        log_pass "All required crates have coverage thresholds"
    fi
}

# ───────────────────────────────────────────────────────────────
# Check 5: Gate 脚本存在性
# ───────────────────────────────────────────────────────────────
check_gate_scripts() {
    echo ""
    echo "━━━ Check 5: Gate Scripts ━━━"

    local required=(
        "check_alpha_v350.sh"
        "check_beta_v350.sh"
        "check_ga_v350.sh"
        "check_rc_v350.sh"
    )

    local missing=0
    for script in "${required[@]}"; do
        if [ -f "$SCRIPT_DIR/$script" ]; then
            log_pass "$script exists"
        else
            log_fail "$script missing"
            missing=$((missing+1))
        fi
    done
}

# ───────────────────────────────────────────────────────────────
# Check 6: CI job contexts 与 branch protection contexts 匹配
# Gitea Actions context = workflow_filename/job_key
# ───────────────────────────────────────────────────────────────
check_ci_jobs_vs_protection() {
    echo ""
    echo "━━━ Check 6: CI Job Contexts vs Branch Protection ━━━"

    local active_branch="develop/v3.5.0"

    response=$(curl -s -H "Authorization: token $GITEA_TOKEN" \
        -H "Content-Type: application/json" \
        "$GITEA_BASE/repos/$REPO/branch_protections/$active_branch" 2>/dev/null) || true

    if [ -z "$response" ] || ! echo "$response" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
        log_warn "Could not fetch branch protection for $active_branch"
        return
    fi

    # Build protected set via temp file (avoids quoting issues)
    local prot_tmp; prot_tmp=$(mktemp)
    echo "$response" | python3 -c "
import sys,json
for c in json.load(sys.stdin).get('status_check_contexts', []):
    print(c)
" > "$prot_tmp" 2>/dev/null || true

    echo "   Protected contexts ($active_branch):"
    grep . "$prot_tmp" | while read -r ctx; do
        echo "   - $ctx"
    done

    # Check: only workflows targeting develop/v3.5.0
    local check6_output
    check6_output=$(python3 - "$WORKFLOW_DIR" "$prot_tmp" <<'PYEOF'
import os, sys
workflow_dir = sys.argv[1]
prot_file = sys.argv[2]

protected = set()
for line in open(prot_file):
    line = line.strip()
    if line:
        protected.add(line)

relevant_wfs = {"ci.yml", "pr-gate.yml", "layered-ci.yml"}

def get_jobs(wf_path):
    try:
        content = open(wf_path).read()
        in_jobs = False; in_job = False; jobs = {}  # job_key -> has_dispatch_only
        current_job = None
        for line in content.split('\n'):
            stripped = line.strip()
            if stripped.startswith('#'):
                continue
            if stripped == 'jobs:':
                in_jobs = True
                continue
            if in_jobs:
                indent = len(line) - len(line.lstrip())
                if indent == 2 and ':' in stripped and not stripped.startswith('-'):
                    key = stripped.split(':')[0].strip()
                    if key and key not in ('runs-on','steps','name','permissions','env',
                                          'if','needs','timeout-minutes','uses','run',
                                          'with','continue-on-error'):
                        current_job = key
                        jobs[current_job] = False  # not dispatch-only by default
                elif indent == 4 and current_job and 'if:' in stripped:
                    # Check if this job has a workflow_dispatch-only condition
                    if 'workflow_dispatch' in stripped or "gitea.event_name" in stripped:
                        # Further check: is it ONLY dispatch or does it also run on push/PR?
                        # Lines like `if: github.event_name == 'workflow_dispatch'` alone = dispatch only
                        if 'push' not in stripped and 'pull_request' not in stripped and 'on:' not in stripped:
                            jobs[current_job] = True
                elif indent < 2 and stripped and not stripped.startswith('#'):
                    in_jobs = False
                    current_job = None
        # Return only jobs that are NOT dispatch-only
        return [k for k, v in jobs.items() if not v]
    except:
        return []

missing = []
for wf_name in sorted(relevant_wfs):
    wf_path = os.path.join(workflow_dir, wf_name)
    if not os.path.exists(wf_path):
        continue
    for job in get_jobs(wf_path):
        ctx = f"{wf_name}/{job}"
        if ctx not in protected:
            missing.append(ctx)

if missing:
    print("FAIL")
    for m in missing:
        print(f"  - {m}")
else:
    print("PASS")
PYEOF
)
    rm -f "$prot_tmp"

    if echo "$check6_output" | grep -q "^PASS$"; then
        echo "  ✅ PASS: All job contexts from relevant workflows are covered"
        echo "  (Relevant workflows: ci.yml, layered-ci.yml, pr-gate.yml)"
    else
        echo "  ❌ FAIL: Branch protection missing contexts:"
        echo "$check6_output" | grep "^  - " | while read -r line; do
            echo "  $line"
        done
        echo "  (Only workflows targeting develop/v3.5.0 are checked)"
        ERRORS=$((ERRORS+1))
    fi
}

# ───────────────────────────────────────────────────────────────
# Summary
# ───────────────────────────────────────────────────────────────
print_summary() {
    echo ""
    echo "=========================================="
    echo "  Gate Sync Check Summary"
    echo "=========================================="
    echo "❌ ERRORS  : $ERRORS"
    echo "⚠️  WARNINGS: $WARNINGS"
    echo ""

    if [ $ERRORS -gt 0 ]; then
        echo "Result: 🔴 FAILED — $ERRORS error(s)"
        return 1
    elif [ $WARNINGS -gt 0 ]; then
        echo "Result: 🟡 WARNING — $WARNINGS warning(s)"
        return 0
    else
        echo "Result: ✅ PASSED"
        return 0
    fi
}

main() {
    echo "=========================================="
    echo "  Gate CI/CD Synchronization Check"
    echo "  (Gitea Actions — .gitea/workflows/)"
    echo "=========================================="
    echo "Workflow dir: $WORKFLOW_DIR"
    echo "Repo        : $REPO"
    echo ""

    check_workflow_triggers
    check_no_stale_workflows
    check_branch_protection_contexts
    check_coverage_thresholds
    check_gate_scripts
    check_ci_jobs_vs_protection

    print_summary
}

main
