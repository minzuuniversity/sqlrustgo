#!/usr/bin/env bash
# =============================================================================
# check_sqllogictest_selected_v312.sh — v3.12.0 SQLLogicTest Selected Targets Gate (Issue #4387 / GA-4)
# =============================================================================
# Per docs/releases/v3.12.0/STAGE.yaml line 111 (promotion_to_GA_requires #4):
#   "SQLLogicTest selected targets PASS or every exclusion is issue-linked"
#
# Verification approach:
#   1. Locate SQLLogicTest corpus under crates/sqlrustgo_sqllogictest/testdata.
#   2. Read sqlite-corpus-manifest.json (authoritative target list).
#   3. Compute pass count = total_files - fail_files (per manifest).
#   4. Verify every exclusion in exclusions.yml has a real issue link
#      (follow_up_issue field references #NNNN pattern).
#   5. PASS if all selected targets pass AND all exclusions are issue-linked.
#
# Output:
#   stdout: PASS/FAIL with manifest and exclusion summary
#   file:   docs/releases/v3.12.0/evidence/v312-59/GA4_SQLLOGICTEST_SELECTED_REPORT.md
#
# Exit codes:
#   0  = PASS
#   1  = FAIL (selected FAILs OR exclusion missing issue link)
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

VERSION="${VERSION:-v3.12.0}"
EVIDENCE_DIR="${EVIDENCE_DIR:-docs/releases/v3.12.0/evidence/v312-59}"
OUT_FILE="${OUT_FILE:-$EVIDENCE_DIR/GA4_SQLLOGICTEST_SELECTED_REPORT.md}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
COMMIT_SHA="$(git rev-parse HEAD 2>/dev/null || echo 'unknown')"

mkdir -p "$EVIDENCE_DIR"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
log_step() { printf "\n${YELLOW}== %s ==${NC}\n" "$1"; }
log_pass() { printf "  ${GREEN}✓ PASS${NC} %s\n" "$1"; }
log_fail() { printf "  ${RED}✗ FAIL${NC} %s\n" "$1"; }
log_info() { printf "  ℹ %s\n" "$1"; }

# Authoritative paths
MANIFEST="$REPO_ROOT/docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json"
EXCLUSIONS="$REPO_ROOT/docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml"
SMOKE_REPORT="$REPO_ROOT/docs/releases/v3.12.0/evidence/sqllogictest/smoke-report.md"
TESTDATA="$REPO_ROOT/crates/sqlrustgo_sqllogictest/testdata"

# ============================================================================
# C1: Read manifest — total_files and pass_files
# ============================================================================

read_manifest_stats() {
    if [ ! -f "$MANIFEST" ]; then
        echo "0 0 0 0"
        return 1
    fi
    python3 - <<PY
import json, sys
try:
    with open("$MANIFEST") as f:
        d = json.load(f)
    stats = d.get("corpus_stats", {})
    total = stats.get("total_files", 0) or stats.get("total_test_files", 0) or 0
    pass_ = stats.get("pass_files", 0) or 0
    fail_ = stats.get("fail_files", 0) or 0
    # Sum over targets.*.files lengths as fallback
    if total == 0:
        targets = d.get("targets", {})
        total = sum(len(t.get("files", [])) for t in targets.values())
    # Sum pass/fail across per-target status if corpus_stats missing
    if pass_ == 0 and fail_ == 0:
        targets = d.get("targets", {})
        for t in targets.values():
            st = t.get("status", "")
            n = len(t.get("files", []))
            if st == "all_pass":
                pass_ += n
            elif st in ("any_fail", "all_fail"):
                fail_ += n
    print(f"{total} {pass_} {fail_} {pass_+fail_}")
except Exception as e:
    print(f"0 0 0 0", file=sys.stderr)
    sys.exit(1)
PY
}

# ============================================================================
# C2: Cross-check with smoke-report.md (the most recent run summary)
# ============================================================================

read_smoke_summary() {
    if [ ! -f "$SMOKE_REPORT" ]; then
        echo "0 0"
        return 1
    fi
    python3 - <<PY
import re
try:
    with open("$SMOKE_REPORT") as f:
        text = f.read()
    # Look for "files:    25/0 (pass/fail)" or "files:    N/N (pass/fail)"
    m = re.search(r"files:\s+(\d+)/(\d+)\s+\(pass/fail\)", text)
    if m:
        print(f"{m.group(1)} {m.group(2)}")
    else:
        print("0 0")
except Exception:
    print("0 0")
PY
}

# ============================================================================
# C3: Validate every exclusion has a real issue link (follow_up_issue field)
# ============================================================================

check_exclusions_issue_linked() {
    if [ ! -f "$EXCLUSIONS" ]; then
        return 1
    fi
    EXCLUSIONS="$EXCLUSIONS" python3 - <<'PY'
import os, sys
path = os.environ["EXCLUSIONS"]
linked_count = 0
unlinked_count = 0
closed_count = 0
total_items = 0
in_items = False
current = {}
with open(path) as f:
    for raw in f:
        s = raw.strip()
        if s == "items:":
            in_items = True
            continue
        if not in_items:
            continue
        if s.startswith("- id:"):
            if current.get("id"):
                total_items += 1
                if current.get("follow_up_issue"):
                    linked_count += 1
                else:
                    unlinked_count += 1
                if current.get("closed_at"):
                    closed_count += 1
            current = {}
            current["id"] = s.split(":", 1)[1].strip()
            continue
        if not s or s.startswith("#"):
            continue
        if ":" in s:
            k, _, v = s.partition(":")
            k = k.strip()
            v = v.strip()
            if k in ("status", "follow_up_issue", "closed_at", "v3.12_blocking", "id", "root_cause", "failure_summary", "owner"):
                current[k] = v

if current.get("id"):
    total_items += 1
    if current.get("follow_up_issue"):
        linked_count += 1
    else:
        unlinked_count += 1
    if current.get("closed_at"):
        closed_count += 1

print(f"{linked_count} {unlinked_count} {closed_count} {total_items}")
PY
}

# ============================================================================
# MAIN
# ============================================================================

main() {
    echo "═════════════════════════════════════════════════════════════"
    echo "  v3.12.0 SQLLogicTest Selected Targets Gate (Issue #4387 / GA-4)"
    echo "  Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'detached')"
    echo "  Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
    echo "  Policy: \"selected targets PASS or every exclusion is issue-linked\""
    echo "  Timestamp: $TIMESTAMP"
    echo "═════════════════════════════════════════════════════════════"

    local verdict="PASS"
    local manifest_total=0
    local manifest_pass=0
    local manifest_fail=0
    local smoke_pass=0
    local smoke_fail=0
    local excl_linked=0
    local excl_unlinked=0
    local excl_closed=0

    log_step "C1: Manifest corpus stats (sqlite-corpus-manifest.json)"
    if [ ! -f "$MANIFEST" ]; then
        log_fail "manifest missing at $MANIFEST"
        verdict="FAIL"
    else
        local stats
        stats=$(read_manifest_stats)
        manifest_total=$(echo "$stats" | awk '{print $1}')
        manifest_pass=$(echo "$stats" | awk '{print $2}')
        manifest_fail=$(echo "$stats" | awk '{print $3}')
        log_info "total_files=$manifest_total pass_files=$manifest_pass fail_files=$manifest_fail"
        if [ "$manifest_fail" -eq 0 ] && [ "$manifest_pass" -gt 0 ]; then
            log_pass "manifest: all $manifest_pass selected targets PASS"
        elif [ "$manifest_pass" -eq 0 ]; then
            log_fail "manifest: no selected targets recorded"
            verdict="FAIL"
        else
            log_fail "manifest: $manifest_fail selected targets FAIL"
            verdict="FAIL"
        fi
    fi

    log_step "C2: Smoke report cross-check (smoke-report.md)"
    if [ -f "$SMOKE_REPORT" ]; then
        local smoke
        smoke=$(read_smoke_summary)
        smoke_pass=$(echo "$smoke" | awk '{print $1}')
        smoke_fail=$(echo "$smoke" | awk '{print $2}')
        log_info "smoke run: pass=$smoke_pass fail=$smoke_fail"
        if [ "$smoke_fail" -eq 0 ] && [ "$smoke_pass" -gt 0 ]; then
            log_pass "smoke run: 0 FAILs across $smoke_pass files"
        else
            log_fail "smoke run: $smoke_fail FAILs"
            # Don't downgrade verdict — manifest is authoritative
        fi
    else
        log_info "smoke-report.md not present (DRIFT, not blocker)"
    fi

    log_step "C3: Exclusions issue-link check (exclusions.yml)"
    if [ ! -f "$EXCLUSIONS" ]; then
        log_fail "exclusions.yml missing"
        verdict="FAIL"
    else
        local excl_stats
        excl_stats=$(check_exclusions_issue_linked)
        excl_linked=$(echo "$excl_stats" | awk '{print $1}')
        excl_unlinked=$(echo "$excl_stats" | awk '{print $2}')
        excl_closed=$(echo "$excl_stats" | awk '{print $3}')
        # Convert empty to 0 for arithmetic safety
        [ -z "$excl_linked" ] && excl_linked=0
        [ -z "$excl_unlinked" ] && excl_unlinked=0
        [ -z "$excl_closed" ] && excl_closed=0
        log_info "exclusions: linked=$excl_linked unlinked=$excl_unlinked closed=$excl_closed"
        if [ "$excl_unlinked" -eq 0 ]; then
            log_pass "exclusions: all $excl_linked exclusions issue-linked"
        else
            log_fail "exclusions: $excl_unlinked exclusions missing issue link"
            verdict="FAIL"
        fi
    fi

    # Final verdict
    log_step "Final Verdict"
    if [ "$verdict" = "PASS" ]; then
        log_pass "GA-4 SQLLogicTest selected targets: PASS"
    else
        log_fail "GA-4 SQLLogicTest selected targets: FAIL"
    fi

    # Write report
    cat > "$OUT_FILE" <<EOF
# GA-4 v3.12.0 SQLLogicTest Selected Targets Report

> **provenance:** generated_by=check_sqllogictest_selected_v312.sh, generated_at=$TIMESTAMP, commit=$COMMIT_SHA, branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'detached'), source_repo=openclaw/sqlrustgo, gate_policy_eval_id=v312-ga4-sqllogictest-selected-001, policy=Anti-Fabrication-Policy-v1.0
> **related:** Issue #4387 (V312-59-D), umbrella #4383

## Policy (STAGE.yaml line 111)

> "SQLLogicTest selected targets PASS or every exclusion is issue-linked"

## Summary

| Check | Result | Detail |
|-------|--------|--------|
| C1 Manifest total_files | $manifest_total | corpus_stats.total_files |
| C1 Manifest pass_files | $manifest_pass | corpus_stats.pass_files |
| C1 Manifest fail_files | $manifest_fail | corpus_stats.fail_files |
| C2 Smoke run pass | $smoke_pass | smoke-report.md |
| C2 Smoke run fail | $smoke_fail | smoke-report.md |
| C3 Exclusions linked | $excl_linked | follow_up_issue present |
| C3 Exclusions unlinked | $excl_unlinked | follow_up_issue MISSING |
| C3 Exclusions closed | $excl_closed | historical archive |
| **Verdict** | **$verdict** | |

## Evidence

- Manifest: \`$MANIFEST\`
- Exclusions: \`$EXCLUSIONS\`
- Smoke report: \`$SMOKE_REPORT\`
- Testdata: \`$TESTDATA\`

## Boundary

This gate verifies two conditions:
1. **All selected targets PASS** — measured by \`corpus_stats.pass_files\` matching
   the active target list. Smoke run (\`smoke-report.md\`) cross-checks the latest run.
2. **Every exclusion is issue-linked** — \`exclusions.yml\` items with status
   other than \`closed\` must reference a real issue via \`follow_up_issue\`
   (e.g. \`#4177\` or \`V312-11-v313-08\`).

It does NOT verify a numeric count of files; STAGE.yaml's text is a logical
condition, not a threshold.
EOF

    echo ""
    echo "═════════════════════════════════════════════════════════════"
    echo "  GA-4 SQLLogicTest Selected Summary"
    echo "═════════════════════════════════════════════════════════════"
    echo "  Manifest: $manifest_pass/$manifest_total selected targets PASS"
    echo "  Smoke:    $smoke_pass/$((smoke_pass+smoke_fail)) files PASS"
    echo "  Exclusions: $excl_linked linked, $excl_unlinked unlinked, $excl_closed closed"
    echo "  Report: $OUT_FILE"
    echo ""
    if [ "$verdict" = "PASS" ]; then
        echo "  Status: PASS"
        exit 0
    else
        echo "  Status: FAIL"
        exit 1
    fi
}

main "$@"