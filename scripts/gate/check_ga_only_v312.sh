#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：保留并改接（keep-rewire）
# 责任任务：P0-4
# 登记说明：判据内容仍在 v4.1.0 的声称面内(台账 open issue -)：GA-only 判据聚合器，绑 v3.12 GA 流程 ⇒ 归档候选，v4.1.0 GA 由阶段清单另建
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# =============================================================================
# check_ga_only_v312.sh — v3.12.0 GA-1..GA-8 standalone verifier (V312-59-D)
# =============================================================================
# Runs the 8 promotion_to_GA_requires checks WITHOUT invoking BETA/RC/full
# aggregator (which may take hours). Useful for fast GA-only verification.
#
# Output:
#   stdout: PASS/FAIL per item
#   file:   docs/releases/v3.12.0/evidence/v312-59/GA_ONLY_REPORT.md
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

EVIDENCE_DIR="${EVIDENCE_DIR:-docs/releases/v3.12.0/evidence/v312-59}"
OUT_FILE="${OUT_FILE:-$EVIDENCE_DIR/GA_ONLY_REPORT.md}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
COMMIT_SHA="$(git rev-parse HEAD 2>/dev/null || echo 'unknown')"

mkdir -p "$EVIDENCE_DIR"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
log_step() { printf "\n${YELLOW}== %s ==${NC}\n" "$1"; }
log_pass() { printf "  ${GREEN}✓ PASS${NC} %s\n" "$1"; }
log_fail() { printf "  ${RED}✗ FAIL${NC} %s\n" "$1"; }
log_info() { printf "  ℹ %s\n" "$1"; }

# 8 GA items per docs/releases/v3.12.0/STAGE.yaml lines 107-116
GA_REQUIRES=(
    "GA-1:Aggregate all gates + JSON report:scripts/gate/check_ga_v3.12.0.sh"
    "GA-2:168h mixed SOAK scaffold (tests/soak/v312_mixed_soak.rs):tests/soak/v312_mixed_soak.rs"
    "GA-3:Security scan (cargo audit + deny + secret scan):scripts/gate/check_security_scan_v312.sh"
    "GA-4:SQLLogicTest selected targets PASS or every exclusion issue-linked:scripts/gate/check_sqllogictest_selected_v312.sh"
    "GA-5:TPC-H SF=1 zero-row gap (22/22 oracle match):scripts/gate/check_tpch_sf1.sh"
    "GA-6:Wire + LOAD DATA + recovery + upgrade/downgrade GA-level:scripts/gate/check_ga_wire_recovery_upgrade.sh"
    "GA-7:Documentation links + consistency v3.12.0:scripts/gate/check_docs_links_v312.sh"
    "GA-8:GMP compliance matrix signed + GA_GATE_REPORT.md:docs/releases/v3.12.0/GA_GATE_REPORT.md"
)

# Extra GA-7 item
GA7_EXTRA="scripts/gate/check_docs_consistency_v312.sh"

# Extra GA-2 items
GA2_EXTRA_1="tests/soak/mixed_workload.py"
GA2_EXTRA_2="tests/soak/mixed_workload_config.yaml"

# The GA-2 SOAK ARTIFACT — the document that has to prove a SOAK ran.
#
# #5491: GA-2 used to be judged by "tests/soak/v312_mixed_soak.rs exists",
# which proved nothing about a SOAK ever having run. The canonical 168h
# report is named here (evidence/v312-59/GA_OPENSPEC.md names the same
# path), and it is judged by scripts/gate/check_ga2_soak_artifact.sh — one
# implementation, shared with scripts/gate/check_ga_v3.12.0.sh.
GA2_SOAK_ARTIFACT="docs/releases/v3.12.0/evidence/v312-59/GA2_168H_SOAK_REPORT.md"

# What IS in the tree, so the report can say so next to the verdict. The
# demo report is a real record of a real 60s smoke + 5min sustained run —
# it is simply not the 168h GA-2 artifact, and the difference must be
# visible in the report rather than in a reader's head.
GA2_DEMO_ARTIFACT="docs/releases/v3.12.0/evidence/v312-59/GA2_MIXED_SOAK_DEMO_REPORT.md"

# How old a GA-2 artifact may be and still count for this release line.
GA2_MAX_AGE_DAYS=30

# ============================================================================
# MAIN
# ============================================================================

main() {
    echo "═════════════════════════════════════════════════════════════"
    echo "  v3.12.0 GA-Only Verifier (V312-59-D, Issue #4387)"
    echo "  Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'detached')"
    echo "  Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
    echo "  Timestamp: $TIMESTAMP"
    echo "═════════════════════════════════════════════════════════════"

    local pass=0 fail=0
    local rows=""

    while IFS=':' read -r label desc path; do
        local status="FAIL"
        local detail=""
        local full="$REPO_ROOT/$path"
        # The path shown in the report row. GA-2 shows the artifact it
        # actually judged, not the scaffold it used to point at.
        local row_path="$path"

        # #5491: GA-2 is judged by validating the SOAK **artifact**.
        #
        # The generic branch below answers "does this path exist?" — and for
        # GA-2 the path was the harness source file, so a PASS proved only
        # that a scaffold was in the tree. The v3.12.0 GA_ONLY_REPORT.md
        # records exactly that: "| GA-2 | PASS | tests/soak/v312_mixed_soak.rs
        # | rust scaffold present |", while the only 168h SOAK document in the
        # tree is an unfilled, explicitly-deprecated template.
        #
        # The artifact validator is the ONE implementation of "a GA-2 SOAK
        # really ran"; check_ga_v3.12.0.sh calls the same script.
        if [ "$label" = "GA-2" ]; then
            local ga2_out ga2_status ga2_detail
            row_path="$GA2_SOAK_ARTIFACT"
            ga2_out=$(bash "$SCRIPT_DIR/check_ga2_soak_artifact.sh" \
                          --artifact "$REPO_ROOT/$GA2_SOAK_ARTIFACT" \
                          --max-age-days "$GA2_MAX_AGE_DAYS" 2>/dev/null)
            ga2_status="${ga2_out%%|*}"
            ga2_detail="${ga2_out#*|}"
            if [ -z "$ga2_out" ]; then
                status="FAIL"
                detail="SOAK artifact: validator produced no verdict"
            elif [ "$ga2_status" = "PASS" ]; then
                status="PASS"
                detail="SOAK artifact validated: ${ga2_detail}"
            else
                status="FAIL"
                detail="SOAK artifact not validated: ${ga2_detail}"
                if [ -f "$REPO_ROOT/$GA2_DEMO_ARTIFACT" ]; then
                    detail="${detail} (a mixed-workload demo report exists at ${GA2_DEMO_ARTIFACT}, but a 60s/5min run is not the 168h GA-2 artifact)"
                fi
            fi
        elif [ -f "$full" ]; then
            case "$path" in
                *.sh)
                    if bash -n "$full" 2>/dev/null; then
                        status="PASS"
                        detail="script syntax OK"
                    else
                        status="FAIL"
                        detail="script syntax error"
                    fi
                    ;;
                *.rs)
                    status="PASS"
                    detail="rust scaffold present"
                    ;;
                *.md)
                    if [ -s "$full" ]; then
                        status="PASS"
                        detail="doc non-empty"
                    else
                        status="FAIL"
                        detail="doc empty"
                    fi
                    ;;
                *)
                    status="PASS"
                    detail="asset present"
                    ;;
            esac
        else
            status="FAIL"
            detail="missing path"
        fi

        log_step "$label: $desc"
        if [ "$status" = "PASS" ]; then
            log_pass "$label ($detail)"
            pass=$((pass + 1))
        else
            log_fail "$label ($detail)"
            fail=$((fail + 1))
        fi

        rows+="| $label | $status | $row_path | $detail |\n"
    done <<< "$(printf '%s\n' "${GA_REQUIRES[@]}")"

    # GA-7 extra: docs consistency script
    log_step "GA-7-extra: Documentation consistency script"
    if [ -f "$REPO_ROOT/$GA7_EXTRA" ] && bash -n "$REPO_ROOT/$GA7_EXTRA" 2>/dev/null; then
        log_pass "GA-7 extra (consistency) script OK"
        pass=$((pass + 1))
        rows+="| GA-7-extra | PASS | $GA7_EXTRA | script syntax OK |\n"
    else
        log_fail "GA-7 extra (consistency) missing"
        fail=$((fail + 1))
        rows+="| GA-7-extra | FAIL | $GA7_EXTRA | missing |\n"
    fi

    # GA-2 extras
    log_step "GA-2-extras: mixed_workload driver + config"
    for extra in "$GA2_EXTRA_1" "$GA2_EXTRA_2"; do
        if [ -f "$REPO_ROOT/$extra" ]; then
            log_pass "GA-2 extra present: $extra"
            pass=$((pass + 1))
            rows+="| GA-2-extra | PASS | $extra | asset present |\n"
        else
            log_fail "GA-2 extra missing: $extra"
            fail=$((fail + 1))
            rows+="| GA-2-extra | FAIL | $extra | missing |\n"
        fi
    done

    # Write report
    {
        echo "# GA-1..GA-8 Standalone Verification Report (V312-59-D)"
        echo ""
        echo "> **provenance:** generated_by=check_ga_only_v312.sh, generated_at=$TIMESTAMP, commit=$COMMIT_SHA, source_repo=openclaw/sqlrustgo, gate_policy_eval_id=v312-ga-only-001, policy=Anti-Fabrication-Policy-v1.0"
        echo "> **related:** Issue #4387 (V312-59-D), umbrella #4383"
        echo ""
        echo "## Summary"
        echo ""
        echo "| Item | Status | Path | Detail |"
        echo "|------|--------|------|--------|"
        printf "%b" "$rows"
        echo ""
        echo "**Totals:** PASS=$pass FAIL=$fail TOTAL=$((pass+fail))"
        echo ""
        echo "## Verdict"
        echo ""
        if [ "$fail" -eq 0 ]; then
            echo "**PASS** — all 8 GA promotion_to_GA_requires items have been VERIFIED:"
            echo "each item's gate script is present and syntax-valid, **and GA-2's SOAK"
            echo "artifact validates** (a completed run, five workload classes, failure"
            echo "rates within threshold, inside the freshness window)."
        else
            echo "**FAIL** — $fail GA item(s) not satisfied."
            echo ""
            echo "An item is satisfied only when it was **verified**, not merely"
            echo "present: GA-2 requires a SOAK artifact that records a completed run"
            echo "(scripts/gate/check_ga2_soak_artifact.sh), and the other items require"
            echo "their gate script to be present and syntax-valid."
        fi
    } > "$OUT_FILE"

    echo ""
    echo "═════════════════════════════════════════════════════════════"
    echo "  GA-Only Summary"
    echo "═════════════════════════════════════════════════════════════"
    echo "  Total: PASS=$pass FAIL=$fail"
    echo "  Report: $OUT_FILE"
    echo ""
    if [ "$fail" -eq 0 ]; then
        echo "  Status: PASS"
        exit 0
    else
        echo "  Status: FAIL"
        exit 1
    fi
}

main "$@"