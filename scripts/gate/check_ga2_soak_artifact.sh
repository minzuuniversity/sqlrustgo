#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：孤儿门禁（orphan_referenced）—— 只在其它脚本正文里被引用，没有任何执行点
# 处置：需外部主机（needs-external-host）
# 责任任务：P0-2
# 登记说明：校验 GA-2 混负载 SOAK 工件 ⇒ 与看板 ING-1(168h SOAK 无 FINAL_REPORT)同一缺口，缺的是工件与主机，不是脚本
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# =============================================================================
# check_ga2_soak_artifact.sh — validate a GA-2 mixed-workload SOAK artifact
# =============================================================================
# #5491: the GA-2 criterion of the v3.12.0 GA gates used to be
#
#     "GA-2:168h mixed SOAK scaffold (tests/soak/v312_mixed_soak.rs):tests/soak/v312_mixed_soak.rs"
#
# evaluated by a generic "does this path exist?" loop. A PASS therefore
# proved only that a **scaffold file** was in the tree — not that a SOAK ever
# ran, not that it ran to completion, not that its numbers were within
# bounds. The v3.12.0 GA_ONLY_REPORT.md records exactly that:
#
#     | GA-2 | PASS | tests/soak/v312_mixed_soak.rs | rust scaffold present |
#
# This script is the ONE implementation of the rule "a GA-2 SOAK artifact is
# valid". Both GA gates call it; neither keeps its own copy.
#
# A SOAK artifact is valid when ALL of the following hold:
#
#   1. it exists and is a regular, non-empty file;
#   2. it is not a TEMPLATE — no `<<...>>` placeholders, not a pre-filled
#      template, not a document that says it must not be published as a
#      completion report;
#   3. it states a COMPLETED run — it carries a PASS marker and none of the
#      markers that mean "still running", "cancelled", "never ran to
#      completion" or "deprecated";
#   4. it covers all FIVE workload classes (W1..W5) — a SOAK that exercised
#      only some of them did not exercise the mixed workload the criterion
#      is about;
#   5. every failure rate it reports is within the threshold (default 1%);
#   6. it is FRESH — its own timestamp is within --max-age-days of now.
#      A stale artifact is a record of a past run, not evidence for this
#      release.
#
# Usage:
#   check_ga2_soak_artifact.sh --artifact <file> [--max-age-days N]
#                              [--max-failure-rate R]
#   check_ga2_soak_artifact.sh --selftest
#
# Output (stdout): one line, "<STATUS>|<detail>".
#   STATUS = PASS | FAIL
#
# Exit codes:
#   0 = the artifact validates
#   1 = the artifact does not validate (the reason is the detail)
#   2 = usage error
# =============================================================================
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# The five workload classes the mixed-workload SOAK is defined over
# (tests/soak/v312_mixed_soak.rs: W1-OLTP W2-ReadHeavy W3-Aggregate
# W4-DDL W5-Report). A report that does not cover all five did not run the
# mixed workload.
GA2_WORKLOAD_CLASSES=(W1 W2 W3 W4 W5)

DEFAULT_MAX_AGE_DAYS=30
DEFAULT_MAX_FAILURE_RATE=0.01

usage() {
    cat >&2 <<'EOF'
Usage: check_ga2_soak_artifact.sh --artifact <file> [options]
       check_ga2_soak_artifact.sh --selftest

Options:
  --artifact <file>       the SOAK artifact to validate
  --max-age-days <N>      how old the artifact's timestamp may be (default 30)
  --max-failure-rate <R>  the permitted failure rate (default 0.01)
EOF
}

# ---------------------------------------------------------------------------
# display_path <path>
# ---------------------------------------------------------------------------
# Prints the path as it should appear in a report: relative to the repository
# root when it is inside the repository.
#
# An absolute path from whichever worktree the gate happened to run in is
# machine-specific noise in an evidence document — it names a directory that
# will not exist on the next machine that reads the report.
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

display_path() {
    local p="$1"
    case "$p" in
        "$REPO_ROOT"/*) printf '%s' "${p#"$REPO_ROOT"/}" ;;
        *) printf '%s' "$p" ;;
    esac
}

# ---------------------------------------------------------------------------
# validate_artifact <file> <max_age_days> <max_failure_rate>
# ---------------------------------------------------------------------------
# Prints "<STATUS>|<detail>" and returns 0 on PASS, 1 on FAIL.
#
# The checks are ordered by how fundamental they are: an artifact that is
# missing is not "stale", it is absent, and saying "stale" would send the
# reader looking in the wrong place.
validate_artifact() {
    local file="$1" max_age_days="$2" max_failure_rate="$3"
    local reason

    # 1. exists
    if [ ! -f "$file" ]; then
        printf 'FAIL|artifact missing: %s\n' "$(display_path "$file")"
        return 1
    fi

    # 2. non-empty
    if [ ! -s "$file" ]; then
        printf 'FAIL|artifact is empty: %s\n' "$(display_path "$file")"
        return 1
    fi

    # 3. not a template
    #
    # A template is a document whose fields have not been filled in. Treating
    # it as a completed run is the exact shape of the defect this script
    # exists to remove, so it is checked explicitly and before anything else
    # that could be satisfied by a template's boilerplate.
    if grep -qE '<<[A-Z_]+>>' "$file"; then
        printf 'FAIL|artifact is an unfilled template (<<...>> placeholders present): %s\n' "$(display_path "$file")"
        return 1
    fi
    if grep -qiE 'PRE-FILLED TEMPLATE|COMPLETION TEMPLATE' "$file"; then
        printf 'FAIL|artifact is a template, not a run record: %s\n' "$(display_path "$file")"
        return 1
    fi
    if grep -qiE 'Do not publish as a PASS-mode' "$file"; then
        printf 'FAIL|artifact says it must not be published as a completion report: %s\n' "$(display_path "$file")"
        return 1
    fi

    # 3b. not deprecated
    if grep -qiE 'DEPRECATED for Run' "$file"; then
        printf 'FAIL|artifact is marked DEPRECATED (superseded by a later attempt): %s\n' "$(display_path "$file")"
        return 1
    fi

    # 4. states a completed run
    #
    # A record that says the run is still live, was cancelled, or never
    # finished is not a completed run, however many PASS labels it carries
    # for its earlier sub-runs.
    local unfinished
    unfinished=$(grep -nioE 'IN PROGRESS|never ran to completion|Final status: *(cancelled|failure)|— *LIVE|LIVE \(background\)|process has since terminated' "$file" | head -3)
    if [ -n "$unfinished" ]; then
        printf 'FAIL|artifact does not record a completed run (%s): %s\n' \
            "$(echo "$unfinished" | tr '\n' ' ' | sed 's/  */ /g')" "$(display_path "$file")"
        return 1
    fi
    if ! grep -qE '\bPASS\b' "$file"; then
        printf 'FAIL|artifact carries no PASS marker for any run: %s\n' "$(display_path "$file")"
        return 1
    fi

    # 5. covers all five workload classes
    local cls missing=""
    for cls in "${GA2_WORKLOAD_CLASSES[@]}"; do
        if ! grep -q "$cls" "$file"; then
            missing="$missing $cls"
        fi
    done
    if [ -n "$missing" ]; then
        printf 'FAIL|artifact does not cover workload class(es):%s\n' "$missing"
        return 1
    fi

    # 6. failure rates within the threshold
    #
    # Every failure rate the artifact states must be within the threshold.
    # An artifact that states none is not a SOAK result — it is prose.
    local rates bad_rates=""
    rates=$(grep -oiE 'failure[ _]rate[^0-9]{0,12}[0-9]+\.[0-9]+' "$file" \
        | grep -oE '[0-9]+\.[0-9]+' || true)
    if [ -z "$rates" ]; then
        printf 'FAIL|artifact states no failure rate, so it is not a SOAK result: %s\n' "$(display_path "$file")"
        return 1
    fi
    while read -r r; do
        [ -n "$r" ] || continue
        if (( $(echo "$r > $max_failure_rate" | bc -l) )); then
            bad_rates="$bad_rates $r"
        fi
    done <<< "$rates"
    if [ -n "$bad_rates" ]; then
        printf 'FAIL|failure rate above the threshold (%s > %s): %s\n' \
            "$(echo "$bad_rates" | tr ' ' '\n' | head -1)" "$max_failure_rate" "$(display_path "$file")"
        return 1
    fi

    # 7. fresh
    #
    # The artifact's own timestamp is what is compared, not the file's mtime:
    # the mtime changes every time the file is copied or checked out, while
    # the timestamp inside the document is the one the run wrote down.
    local ts age
    ts=$(grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z?' "$file" | head -1)
    if [ -z "$ts" ]; then
        printf 'FAIL|artifact carries no timestamp, so its freshness cannot be judged: %s\n' "$(display_path "$file")"
        return 1
    fi
    age=$(python3 - "$ts" "$max_age_days" <<'PY'
import datetime
import sys

raw = sys.argv[1].rstrip("Z")
max_days = int(sys.argv[2])
try:
    stamp = datetime.datetime.strptime(raw, "%Y-%m-%dT%H:%M:%S")
except ValueError:
    print("unparsable")
    sys.exit(0)
now = datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None)
print((now - stamp).days)
PY
)
    if [ "$age" = "unparsable" ]; then
        printf 'FAIL|artifact timestamp is not parsable (%s): %s\n' "$ts" "$(display_path "$file")"
        return 1
    fi
    if [ "$age" -gt "$max_age_days" ]; then
        printf 'FAIL|artifact is stale (%s days old, limit %s): %s\n' \
            "$age" "$max_age_days" "$(display_path "$file")"
        return 1
    fi

    printf 'PASS|artifact validates (%s, %s days old, 5/5 workload classes, failure rates within %s)\n' \
        "$(display_path "$file")" "$age" "$max_failure_rate"
    return 0
}

# ---------------------------------------------------------------------------
# selftest
# ---------------------------------------------------------------------------
#
# The rule this script implements is "did a SOAK really run?" — and a rule
# that can silently accept a template, an unfinished run or a stale document
# is the defect it was written to remove. So the rule needs its own
# regression test, and it needs to run without cargo.
selftest() {
    # The scratch directory is a GLOBAL, not a local: a RETURN trap fires
    # after the function's locals are gone, and under `set -u` a trap body
    # that reads an unbound local aborts the script with
    # "dir: unbound variable" — after the fixtures have already been judged.
    GA2_SELFTEST_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ga2-selftest.XXXXXX")" || return 2
    local dir="$GA2_SELFTEST_DIR"
    local rc=0

    local now
    now=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    # A good artifact: five classes, a recent timestamp, a failure rate
    # inside the threshold, and a completed run.
    cat > "$dir/good.md" <<EOF
# Mixed-Workload SOAK Report
> generated_at=$now
## Smoke Run — PASS
| Failure rate | 0.0000 (≤ 0.01 threshold) |
| W1 OLTP | 179 | 179 | 0 |
| W2 read-heavy | 120 | 120 | 0 |
| W3 aggregation | 60 | 60 | 0 |
| W4 DDL | 60 | 60 | 0 |
| W5 reports | 120 | 120 | 0 |
EOF

    # The same report, but the run never finished.
    sed 's/— PASS/— LIVE (background)/' "$dir/good.md" > "$dir/unfinished.md"
    # The same report, but it is a template.
    sed "s/$now/<<TIMESTAMP>>/" "$dir/good.md" > "$dir/template.md"
    # The same report, but it is from last year.
    sed "s/$now/2020-01-01T00:00:00Z/" "$dir/good.md" > "$dir/stale.md"
    # The same report, but one workload class is missing.
    grep -v 'W5 reports' "$dir/good.md" > "$dir/missing_class.md"
    # The same report, but the failure rate blew the threshold.
    sed 's/0.0000 (≤ 0.01 threshold)/0.4200 (≤ 0.01 threshold)/' "$dir/good.md" > "$dir/bad_rate.md"
    # The same report, but it says it must not be published.
    { cat "$dir/good.md"; echo "Do not publish as a PASS-mode completion report."; } > "$dir/do_not_publish.md"
    # Not a SOAK result at all.
    echo "# Just prose" > "$dir/prose.md"

    local -a cases=(
        "good.md|PASS"
        "unfinished.md|FAIL"
        "template.md|FAIL"
        "stale.md|FAIL"
        "missing_class.md|FAIL"
        "bad_rate.md|FAIL"
        "do_not_publish.md|FAIL"
        "prose.md|FAIL"
    )

    local entry file want got
    for entry in "${cases[@]}"; do
        file="${entry%%|*}"
        want="${entry##*|}"
        got=$(validate_artifact "$dir/$file" 30 0.01 | cut -d'|' -f1)
        if [ "$got" = "$want" ]; then
            printf 'ok    %-20s => %s\n' "$file" "$got"
        else
            printf 'FAIL  %-20s => %s (want %s)\n' "$file" "$got" "$want"
            rc=1
        fi
    done

    # A missing artifact is a FAIL, and says so.
    got=$(validate_artifact "$dir/does-not-exist.md" 30 0.01 | cut -d'|' -f1)
    if [ "$got" = "FAIL" ]; then
        printf 'ok    %-20s => %s\n' "missing.md" "$got"
    else
        printf 'FAIL  %-20s => %s (want FAIL)\n' "missing.md" "$got"
        rc=1
    fi

    if [ "$rc" -ne 0 ]; then
        printf 'check_ga2_soak_artifact.sh --selftest: FAILED\n' >&2
        rm -rf "$dir"
        return 1
    fi
    printf 'check_ga2_soak_artifact.sh --selftest: all cases passed\n'
    rm -rf "$dir"
    return 0
}

# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------
main() {
    if [ "${1:-}" = "--selftest" ]; then
        selftest
        return $?
    fi

    local artifact="" max_age_days="$DEFAULT_MAX_AGE_DAYS"
    local max_failure_rate="$DEFAULT_MAX_FAILURE_RATE"

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --artifact) artifact="${2:-}"; shift 2 ;;
            --max-age-days) max_age_days="${2:-}"; shift 2 ;;
            --max-failure-rate) max_failure_rate="${2:-}"; shift 2 ;;
            -h|--help) usage; return 2 ;;
            *) printf 'unknown argument: %s\n' "$1" >&2; usage; return 2 ;;
        esac
    done

    if [ -z "$artifact" ]; then
        printf 'FAIL|--artifact is required\n'
        return 2
    fi

    # The verdict is ONE line, "<STATUS>|<detail>", so the caller can put it
    # in a report row without re-deriving anything.
    validate_artifact "$artifact" "$max_age_days" "$max_failure_rate"
    return $?
}

main "$@"
