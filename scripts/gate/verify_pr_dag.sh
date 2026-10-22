#!/usr/bin/env bash
# =============================================================================
# check_pr_dag.sh / verify_pr_dag.sh — PR-DAG Consistency Gate (issue #5457)
# =============================================================================
# Purpose: Verify that the PR-DAG dependency graph declared in the release
#          plan agrees with what the repository's history actually contains.
# Coverage: GATE_CONDITIONS.md BE5 / B-F5 ("PR-DAG 图表存在且与实际提交一致")
# Principle: P3 (证据优先 — 判据必须可复现) / ADR-001 G-01 (Claim ≠ Evidence)
# Verify:   the DAG's per-PR status column matches the merge commits in git.
#
# Why this gate exists (issue #5457):
#   `docs/governance/GATE_CONDITIONS.md` cites `scripts/gate/verify_pr_dag.sh`
#   as the criterion for BE5 (line 103) and B-F5 (line 129) — but that script
#   did not exist:
#
#     $ ls -l scripts/gate/verify_pr_dag.sh
#     ls: scripts/gate/verify_pr_dag.sh: No such file or directory
#
#   Consequences, none of them self-healing:
#     1. The criterion was NOT EXECUTABLE — no PASS, no FAIL, ever.
#     2. v3.11.0 and v4.0.0 BETA gate reports had already recorded it as
#        "PASS" / "All V400-* PRs merged in documented order", with NO
#        reproducible way to re-verify those claims:
#          docs/releases/v4.0.0/FEATURE_CHECKLIST.md:17
#          docs/releases/v4.0.0/BETA_GATE_REPORT.md:19
#          docs/releases/v3.11.0/BETA_GATE_REPORT.md:38,58
#     3. The criterion table was accepted without an existence check.
#
#   A second name for the same missing gate also appears in the repo:
#     docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md:92
#     docs/releases/v3.8.0/alpha/ALPHA_STAGE_REVIEW.md:81
#   Both names are provided by this PR (see the alias at the bottom).
#
# The document under test:
#   docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md
#     §2   "PR DAG（依赖执行图）" — an ASCII dependency chain of PR-8xx nodes
#     §2.1 "PR DAG 实际执行状态" — a table:
#          | PR | 规划阶段 | 实际状态 | 说明 |
#          | PR-800 (COM_QUERY AST Routing) | Phase 0 | ⚠️ 部分落地 | ... |
#          | PR-810 (ExecutionEngine → Router) | Phase 0 | ❌ 未合并 | 无 Issue 追踪 |
#          | PR-830 (WAL + WriteBuffer) | Phase 1 | ✅ 已合并 | PR-830A~E + PR-830F |
#
# What is checked, and what is deliberately NOT checked
# -----------------------------------------------------
# CHECKED:
#   (a) FALSE PASS — the document claims "✅ 已合并" but git has no merge
#       commit for that PR. This is the dangerous direction: the gate would
#       certify a PR as landed that never landed. => FAIL
#   (b) DEPENDENCY ORDER — the §2 chain declares "（依赖NNN）" edges. If A
#       depends on B but B appears AFTER A in the chain, the declared
#       topological order is violated. => FAIL
#   (c) COMPLETENESS — the §2 chain and the §2.1 table must agree on which
#       PRs exist. A PR in one and not the other is a broken document. => FAIL
#
# NOT CHECKED (and why):
#   * Merge TIME order vs row order. The §2 chain is a DEPENDENCY order
#     ("按依赖顺序排列"), not a chronological one. Comparing it to commit
#     timestamps would be a category error, and enforcing it would produce
#     a gate that can never pass. Only the dependency edges are checked.
#   * Whether "❌ 未合并" is still accurate. That is DOCUMENT DRIFT (the
#     table went stale), not a gate failure: the criterion asks whether
#     what is claimed as merged really is merged. Stale rows are reported
#     as WARN with the contradicting evidence, so the drift is visible
#     without failing a criterion that is not about staleness.
#
# PR identifier resolution
# -------------------------
# The DAG uses PLANNING numbers (PR-800, PR-830, ...), which are NOT Gitea
# PR numbers: GET /pulls/830 returns HTTP 404. The repository's two prose
# mappings disagree with each other AND with git:
#
#   docs/releases/v3.8.0/beta/BETA_GATE_REPORT.md:120-124
#       PR-830C -> #2669, PR-830D -> #2670, PR-830E -> #2675
#   git merge commit titles (measured 2026-10-22)
#       PR-830C -> #2670, PR-830D -> #2674, PR-830E -> #2691
#
# So prose maps are not trusted. The AUTHORITATIVE source is the git merge
# commit title itself, which carries both identifiers:
#
#   Merge pull request 'fix(storage): PR-830F WAL lifecycle — ...' (#2697) ...
#
# One grep yields both the planning number and the Gitea number, so no
# mapping table has to be maintained (and none can rot).
#
# Exit codes:
#   0 = PASS — no false pass, dependency order intact, chain and table agree
#   1 = FAIL — a false pass, a dependency inversion, a chain/table mismatch,
#       or nothing parseable
#   2 = DRIFT / ENV_ERROR — the plan document or the base ref cannot be
#       resolved, or §2 / §2.1 is missing
#
# DRIFT is NOT PASS (ADR-006 P14 / scripts/gate/check_drift_not_pass.sh).
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# --- ADR-014 5 evidence fields -------------------------------------------------
# The defaults identify WHO is running the gate and WHICH run it is; both can
# be overridden from the environment. `source_run` is bound to the commit
# under test, so an artifact can be tied back to the exact tree it judged.
SOURCE_AGENT="${SOURCE_AGENT:-openclaw-mcode}"
SOURCE_RUN="${SOURCE_RUN:-openclaw-mcode-pr-dag-$(git rev-parse --short HEAD 2>/dev/null || echo nohead)}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# --- Configuration -------------------------------------------------------------
PLAN_FILE=""
# The release line the gate judges against. The DEFAULT is a *specification*
# ("develop/v4.1.0"), NOT a literal ref: it is resolved against the refs the
# current checkout actually has (see resolve_base_ref). An explicitly
# requested --base is used verbatim and is NEVER fallen back on.
BASE_REF_SPEC="develop/v4.1.0"
BASE_REF=""
BASE_REF_EXPLICIT=0
# Bypasses the shallow-history refusal. The gate cannot judge on a
# truncated history, so it reports DRIFT (exit 2) — which is correct for
# a CI runner, but would prevent the test suite from exercising the
# detection logic there. Only the tests set this.
ALLOW_SHALLOW=0
# Scratch directory of the self-test. It is a GLOBAL on purpose: the EXIT
# trap that removes it runs after the function has returned.
PR_DAG_SELFTEST_TMP=""

usage() {
    cat <<'EOF'
Usage: bash scripts/gate/verify_pr_dag.sh [options]

Options:
  --plan <path>   Path to the release plan holding the "PR DAG" section
                  (default: docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md)
  --base <ref>    Base ref to verify against. Used VERBATIM: if it cannot be
                  resolved the gate reports DRIFT (exit 2) and never falls
                  back to another ref.
                  (default: the release line "develop/v4.1.0", resolved
                  against the refs this checkout has — see below)
  --allow-shallow Proceed even on a truncated history, instead of
                  reporting DRIFT. The gate CANNOT judge on a truncated
                  history, so the default is to refuse. This flag exists
                  for the test suite, which needs to exercise the
                  detection logic in a shallow checkout.
  --selftest      Run the gate's own self-test (base-ref resolution over
                  throwaway git repositories) and exit. No plan is read.
  -h, --help      Show this help

Default base resolution (deterministic, and the gate PRINTS the ref it used):
  1. refs/remotes/<remote>/develop/v4.1.0, remotes tried in the fixed order
     origin -> gitea252 -> gitea250 -> github -> gitcode -> gitee
  2. the same line on any other remote (sorted => deterministic)
  3. refs/heads/develop/v4.1.0
  4. HEAD — the only ref a SHA-only CI checkout has
  If none of them resolves, the gate reports DRIFT (exit 2).

Environment:
  SOURCE_AGENT / SOURCE_RUN
EOF
}

# --- base-ref resolution (issue #5542) ---------------------------------------
#
# The gate judges ONE question: "is there a merge commit for PR-X behind the
# release line?". The question does not change between environments; only
# the place the history is READ from does:
#
#   * a local worktree of this project has NO `origin` remote at all — its
#     remotes are `gitea252` / `gitee` / `github` / `gitcode` / `gitea250`;
#   * the CI runner checks out a single SHA (`git init` + `git remote add
#     origin` + `git fetch origin +<SHA>`), which creates NO remote-tracking
#     branch — only `HEAD`.
#
# Pinning the default to one literal ref (`origin/develop/v4.1.0`) therefore
# made the gate answer DRIFT(exit 2) EVERYWHERE: the BE5 / B-F5 criterion was
# never executable, and the five regression cases in
# tests/integration/gate/gate_criterion_existence_test.rs failed for exactly
# that reason (measured on the batch-24 tree `cd4a02c6b5`).
#
# The order below is deterministic, and the gate prints the ref it actually
# used — a fallback must never be silent.
resolve_base_ref() {
    local repo="${1:-$REPO_ROOT}"
    local spec="$2"
    local remote ref

    # 1. the release line as a remote-tracking branch, in a fixed priority
    for remote in origin gitea252 gitea250 github gitcode gitee; do
        ref="refs/remotes/$remote/$spec"
        if git -C "$repo" rev-parse --verify --quiet "$ref" >/dev/null 2>&1; then
            printf '%s\n' "$ref"
            return 0
        fi
    done

    # 2. the same line on any other remote (sorted => deterministic)
    for ref in $(git -C "$repo" for-each-ref --format='%(refname)' "refs/remotes/*/$spec" 2>/dev/null | sort); do
        printf '%s\n' "$ref"
        return 0
    done

    # 3. a local branch of the same name
    ref="refs/heads/$spec"
    if git -C "$repo" rev-parse --verify --quiet "$ref" >/dev/null 2>&1; then
        printf '%s\n' "$ref"
        return 0
    fi

    # 4. HEAD — the only ref a SHA-only CI checkout has
    if git -C "$repo" rev-parse --verify --quiet HEAD >/dev/null 2>&1; then
        printf '%s\n' "HEAD"
        return 0
    fi

    return 1
}

# --- selftest (issue #5542) ----------------------------------------------------
#
# The base-ref resolution is a RULE, so it is testable on its own: the
# self-test builds throwaway git repositories shaped like the three checkouts
# the gate meets in the wild (a full worktree, a SHA-only CI checkout, an
# explicitly broken --base).
run_selftest() {
    local pass=0 fail=0 tmp repo_a repo_b got rc out
    local spec="develop/v4.1.0"

    # The scratch directory is a GLOBAL: the EXIT trap below runs after this
    # function has returned, when its `local` variables are already gone.
    PR_DAG_SELFTEST_TMP="$(mktemp -d "${TMPDIR:-/tmp}/sqlrustgo-prdag-selftest.XXXXXX" 2>/dev/null)" || PR_DAG_SELFTEST_TMP=""
    tmp="$PR_DAG_SELFTEST_TMP"
    if [ -z "$tmp" ] || [ ! -d "$tmp" ]; then
        echo "[FAIL] selftest: cannot create a temporary directory — refusing to report a verdict"
        return 1
    fi
    trap 'rm -rf -- "$PR_DAG_SELFTEST_TMP"' EXIT

    # fixture A: a checkout that HAS the release line as a remote-tracking branch
    repo_a="$tmp/with-origin"
    mkdir -p "$repo_a"
    git -C "$repo_a" init -q >/dev/null 2>&1
    git -C "$repo_a" -c user.name=selftest -c user.email=selftest@example.invalid \
        commit -q --allow-empty -m "selftest" >/dev/null 2>&1
    git -C "$repo_a" update-ref refs/remotes/origin/develop/v4.1.0 HEAD >/dev/null 2>&1

    # fixture B: a SHA-only CI checkout — no remote-tracking branch at all
    repo_b="$tmp/head-only"
    mkdir -p "$repo_b"
    git -C "$repo_b" init -q >/dev/null 2>&1
    git -C "$repo_b" -c user.name=selftest -c user.email=selftest@example.invalid \
        commit -q --allow-empty -m "selftest" >/dev/null 2>&1

    # case 1: the release line as a remote-tracking branch is preferred
    got="$(resolve_base_ref "$repo_a" "$spec")"
    if [ "$got" = "refs/remotes/origin/develop/v4.1.0" ]; then
        echo "[PASS] selftest: a checkout carrying the release line resolves to the remote-tracking branch"
        pass=$((pass + 1))
    else
        echo "[FAIL] selftest: expected refs/remotes/origin/develop/v4.1.0, got '$got'"
        fail=$((fail + 1))
    fi

    # case 2: a SHA-only checkout falls back to HEAD
    got="$(resolve_base_ref "$repo_b" "$spec")"
    if [ "$got" = "HEAD" ]; then
        echo "[PASS] selftest: a SHA-only checkout resolves to HEAD"
        pass=$((pass + 1))
    else
        echo "[FAIL] selftest: expected HEAD, got '$got'"
        fail=$((fail + 1))
    fi

    # case 3: an explicitly requested base ref is NEVER fallen back on
    out="$(bash "$SCRIPT_DIR/verify_pr_dag.sh" --base refs/heads/definitely_nonexistent_selftest_zzz 2>&1)"
    rc=$?
    if [ "$rc" -eq 2 ] && printf '%s\n' "$out" | grep -q "base ref cannot be resolved"; then
        echo "[PASS] selftest: an unresolvable explicit --base stays DRIFT (exit 2) — no fallback"
        pass=$((pass + 1))
    else
        echo "[FAIL] selftest: an unresolvable explicit --base must be DRIFT (exit 2), got exit $rc"
        fail=$((fail + 1))
    fi

    # case 4: the gate must not print integer-expression noise on stderr
    out="$(bash "$SCRIPT_DIR/verify_pr_dag.sh" 2>&1 >/dev/null)"
    if printf '%s\n' "$out" | grep -q "integer expression expected"; then
        echo "[FAIL] selftest: the gate still emits 'integer expression expected' on stderr"
        fail=$((fail + 1))
    else
        echo "[PASS] selftest: the gate emits no integer-expression noise on stderr"
        pass=$((pass + 1))
    fi

    # case 5: the resolved ref must be visible in the gate's own output
    out="$(bash "$SCRIPT_DIR/verify_pr_dag.sh" 2>/dev/null)"
    if printf '%s\n' "$out" | grep -q "base ref resolved:"; then
        echo "[PASS] selftest: the gate prints the base ref it actually used"
        pass=$((pass + 1))
    else
        echo "[FAIL] selftest: the gate must print the base ref it actually used"
        fail=$((fail + 1))
    fi

    echo
    echo "--- selftest summary ---"
    echo "  passed: $pass"
    echo "  failed: $fail"
    if [ "$fail" -gt 0 ]; then
        echo
        echo "❌ selftest FAILED"
        return 1
    fi
    echo
    echo "✅ selftest PASS"
    return 0
}

while [ $# -gt 0 ]; do
    case "$1" in
        --plan) PLAN_FILE="$2"; shift 2 ;;
        --base) BASE_REF="$2"; BASE_REF_EXPLICIT=1; shift 2 ;;
        --allow-shallow) ALLOW_SHALLOW=1; shift ;;
        --selftest) SELFTEST=1; shift ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown option: $1"; usage; exit 2 ;;
    esac
done

if [ "${SELFTEST:-0}" -eq 1 ]; then
    run_selftest
    exit $?
fi

# --- house-style counters (see check_anti_fabrication.sh) ----------------------
ERRORS=0
WARNINGS=0
PASSED=0

log_info()  { echo "[INFO] $*"; }
log_pass()  { PASSED=$((PASSED + 1)); echo "[PASS] $*"; }
log_warn()  { WARNINGS=$((WARNINGS + 1)); echo "[WARN] $*"; }
log_error() { ERRORS=$((ERRORS + 1)); echo "[ERROR] $*"; }

echo "=== PR-DAG Consistency Gate (issue #5457) ==="
echo "  source_agent: $SOURCE_AGENT"
echo "  source_run:   $SOURCE_RUN"
echo "  timestamp:    $TIMESTAMP"
if [ "$BASE_REF_EXPLICIT" -eq 1 ]; then
    echo "  base_ref:     $BASE_REF (explicitly requested — used verbatim)"
else
    echo "  base_ref:     $BASE_REF_SPEC (release line — resolved below)"
fi
echo

# --- Phase 1: locate the plan document ----------------------------------------
if [ -z "$PLAN_FILE" ]; then
    # Prefer the known location; fall back to any release plan that has a
    # "PR DAG" section.
    if [ -f "docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md" ]; then
        PLAN_FILE="docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md"
    else
        while IFS= read -r candidate; do
            [ -z "$candidate" ] && continue
            if grep -q "PR DAG" "$candidate" 2>/dev/null; then
                PLAN_FILE="$candidate"
                break
            fi
        done < <(find docs/releases -type f -path '*/plans/DEVELOPMENT_PLAN.md' 2>/dev/null | sort)
    fi
fi

if [ -z "$PLAN_FILE" ] || [ ! -f "$PLAN_FILE" ]; then
    echo "❌ Release plan with a PR DAG section not found."
    echo "   Cannot judge — DRIFT, not PASS."
    exit 2
fi
log_info "plan document: $PLAN_FILE"

if ! grep -q "PR DAG" "$PLAN_FILE"; then
    echo "❌ $PLAN_FILE has no 'PR DAG' section."
    echo "   Cannot judge — DRIFT, not PASS."
    exit 2
fi

# --- Phase 2: resolve the base ref --------------------------------------------
# An explicitly requested --base is used VERBATIM: if it cannot be resolved
# the gate cannot judge and says so (DRIFT). It is never replaced by another
# ref — a silent fallback would let an operator believe the gate judged the
# range they asked for.
if [ "$BASE_REF_EXPLICIT" -eq 1 ]; then
    if ! git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
        echo "❌ base ref cannot be resolved: $BASE_REF (explicitly requested — no fallback is attempted)"
        echo "   Cannot judge — DRIFT, not PASS."
        exit 2
    fi
else
    # The default is the release LINE, not one literal ref: where that line
    # lives differs per checkout (issue #5542). The resolved ref is printed
    # below, so a fallback is never silent.
    resolved_base="$(resolve_base_ref "$REPO_ROOT" "$BASE_REF_SPEC")" || resolved_base=""
    if [ -z "$resolved_base" ]; then
        echo "❌ no ref could be resolved for the release line: $BASE_REF_SPEC"
        echo "   Cannot judge — DRIFT, not PASS."
        exit 2
    fi
    BASE_REF="$resolved_base"
fi
log_info "base ref resolved: $BASE_REF ($(git rev-parse --short "$BASE_REF"))"

SHALLOW_FILE="$(git rev-parse --git-dir 2>/dev/null)/shallow"
if [ -f "$SHALLOW_FILE" ] && [ "$ALLOW_SHALLOW" -eq 0 ]; then
    shallow_depth=$(git rev-list --count "$BASE_REF" 2>/dev/null || echo 0)
    echo
    echo "⚠️  DRIFT: this is a SHALLOW checkout (.git/shallow present)."
    echo "   history depth reachable from $BASE_REF: $shallow_depth commit(s)"
    echo
    echo "   The gate cannot answer 'does PR-X have a merge commit?' on a"
    echo "   truncated history — every answer would be 'no', which would"
    echo "   turn a legitimately merged PR into a FALSE PASS."
    echo
    echo "   Remedy (operator's choice):"
    echo "     - fetch enough history:  git fetch --unshallow <remote>"
    echo "     - or run the gate against a full mirror of the repository"
    echo "     - or point --base at a ref whose history IS complete"
    echo
    echo "   Cannot judge — DRIFT, not PASS."
    exit 2
fi
echo

# --- Phase 3: parse §2 (dependency chain) and §2.1 (status table) -------------
# §2 chain lines look like:
#   PR-800  COM_QUERY AST Routing (L0)           ← 入口重构（先做）
#   PR-810  ExecutionEngine → Router            ← 解耦层（依赖800）
#
# §2.1 table lines look like:
#   | PR-800 (COM_QUERY AST Routing) | Phase 0 | ⚠️ 部分落地 | TransactionalFacade DEFERRED (#2603) |
#
# Extract:
#   chain: "PR-800 <dep-list>"   (dep list = the （依赖NNN） numbers)
#   table: "PR-800 <status>"
CHAIN=$(awk '
    /PR DAG/ { in_dag = 1; next }
    in_dag && /^##[^#]/ { in_dag = 0 }
    in_dag && /^PR-[0-9]+/ {
        line = $0
        match(line, /PR-[0-9]+/)
        pr = substr(line, RSTART, RLENGTH)
        deps = ""
        rest = line
        while (match(rest, /依赖[ ]*[0-9]+/)) {
            d = substr(rest, RSTART, RLENGTH)
            gsub(/[^0-9]/, "", d)
            deps = deps (deps == "" ? "" : ",") d
            rest = substr(rest, RSTART + RLENGTH)
        }
        mentioned = (line ~ /依赖/) ? 1 : 0
        state = (mentioned && deps == "") ? "UNPARSED" : "OK"
        print pr "|" deps "|" state
    }
' "$PLAN_FILE")

TABLE=$(awk -F'|' '
    /^\|[[:space:]]*PR-[0-9]+/ {
        pr = $2; st = $4;
        gsub(/^[ \t]+|[ \t]+$/, "", pr);
        gsub(/^[ \t]+|[ \t]+$/, "", st);
        match(pr, /PR-[0-9]+/)
        pr = substr(pr, RSTART, RLENGTH)
        if (pr != "") print pr "|" st
    }
' "$PLAN_FILE")

if [ -z "$CHAIN" ]; then
    log_error "no PR nodes parsed from the §2 dependency chain of $PLAN_FILE"
    echo "   An unparseable DAG must not be reported as PASS."
    exit 1
fi
if [ -z "$TABLE" ]; then
    echo "❌ No §2.1 status table found in $PLAN_FILE."
    echo "   Cannot judge — DRIFT, not PASS."
    exit 2
fi

CHAIN_COUNT=$(printf '%s\n' "$CHAIN" | grep -c .)
TABLE_COUNT=$(printf '%s\n' "$TABLE" | grep -c .)
log_info "§2 chain nodes parsed: $CHAIN_COUNT"
log_info "§2.1 status rows parsed: $TABLE_COUNT"
echo

# --- Phase 4: resolve each PR against the git history -------------------------
# One pass over the merge commits: for every "Merge pull request" commit we
# extract the planning number and the Gitea number that appear in its title.
# This is the authoritative planning-number -> Gitea-number mapping.
GIT_LOG_TMP="$(mktemp)"
trap 'rm -f "$GIT_LOG_TMP"' EXIT
git log --format='%s' "$BASE_REF" 2>/dev/null > "$GIT_LOG_TMP" || true

GITEA_MAPPING=$(awk '
    /^Merge pull request / {
        if (!match($0, /\(#[0-9]+\)/)) next
        gitea = substr($0, RSTART + 2, RLENGTH - 3)
        if (!match($0, /\047[^\047]*\047/)) next
        title = substr($0, RSTART + 1, RLENGTH - 2)
        if (!match(title, /PR-[0-9]+[A-Z]?/)) next
        plan = substr(title, RSTART, RLENGTH)
        print plan "|" gitea "|" $0
    }
' "$GIT_LOG_TMP" 2>/dev/null)

# Resolve the Gitea number for a planning id, including the letter-suffixed
# children (PR-830 -> PR-830A .. PR-830F). Returns "<gitea>|<sha>".
resolve_gitea_ref() {
    local ident="$1"
    printf '%s\n' "$GITEA_MAPPING" \
        | awk -F'|' -v p="$ident" '
            $1 == p { print $2 "|" $3; exit }
            $1 ~ ("^" p "[A-Z]$") { if (!found) { print $2 "|" $3; found=1 } }
        ' | head -1
}

build_merge_counts() {
    awk '
        /^Merge pull request .(feat|fix|refactor|perf)/ {
            if (match($0, /\047[^\047]*\047/)) {
                title = substr($0, RSTART + 1, RLENGTH - 2)
            } else {
                next
            }
            if ($0 !~ /\(#[0-9]+\)/) next
            rest = title
            while (match(rest, /PR-[0-9]+[A-Z]?/)) {
                id = substr(rest, RSTART, RLENGTH)
                rest = substr(rest, RSTART + RLENGTH)
                after = substr(rest, 1, 1)
                if (after ~ /[0-9A-Za-z]/) continue
                counts[id]++
                if (id ~ /[A-Z]$/) {
                    parent = substr(id, 1, length(id) - 1)
                    counts[parent]++
                }
            }
            next
        }
        /^(feat|fix|refactor|perf)/ {
            rest = $0
            while (match(rest, /PR-[0-9]+[A-Z]?/)) {
                id = substr(rest, RSTART, RLENGTH)
                rest = substr(rest, RSTART + RLENGTH)
                after = substr(rest, 1, 1)
                if (after ~ /[0-9A-Za-z]/) continue
                counts[id]++
                if (id ~ /[A-Z]$/) {
                    parent = substr(id, 1, length(id) - 1)
                    counts[parent]++
                }
            }
        }
        END {
            for (id in counts) print id "|" counts[id]
        }
    ' "$GIT_LOG_TMP" 2>/dev/null
}

count_merge_commits_with_children() {
    local ident="$1"
    printf '%s\n' "$MERGE_COUNTS" | awk -F'|' -v p="$ident" '$1 == p { print $2; exit }'
}

MERGE_COUNTS=$(build_merge_counts)

DECLARED_MERGED=0
DECLARED_NOT_MERGED=0
DECLARED_PARTIAL=0
ACTUALLY_MERGED=0
FALSE_PASS=0
STALE_ROW=0
CHAIN_ONLY=""
TABLE_ONLY=""

while IFS='|' read -r pr deps dep_state; do
    [ -z "$pr" ] && continue

    # --- declared status from the §2.1 table --------------------------------
    # A PR MUST appear exactly once in the status table. Two rows for the
    # same PR is a self-contradictory document: taking the first row would
    # silently ignore the second, which is how a false green hides.
    # (Measured: a table with "❌ 未合并" then "✅ 已合并" for PR-800 used
    # to be judged by the first row only.)
    row_count=$(printf '%s\n' "$TABLE" | awk -F'|' -v p="$pr" '$1 == p { c++ } END { print c+0 }')
    if [ "$row_count" -gt 1 ]; then
        log_error "$pr — appears $row_count times in the §2.1 status table. The rows must be unique."
        FALSE_PASS=$((FALSE_PASS + 1))
        continue
    fi
    status=$(printf '%s\n' "$TABLE" | awk -F'|' -v p="$pr" '$1 == p { print $2; exit }')
    if [ -z "$status" ]; then
        log_error "$pr — present in the §2 chain but missing from the §2.1 status table"
        FALSE_PASS=$((FALSE_PASS + 1))
        CHAIN_ONLY="$CHAIN_ONLY $pr"
        continue
    fi
    if ! printf '%s\n' "$CHAIN" | awk -F'|' -v p="$pr" '$1 == p { found=1 } END { exit !found }'; then
        log_error "$pr — present in the §2.1 status table but missing from the §2 chain"
        CHAIN_ONLY="$CHAIN_ONLY $pr"
        continue
    fi

    case "$status" in
        *"已合并"*) DECLARED_MERGED=$((DECLARED_MERGED + 1)) ;;
        *"未合并"*) DECLARED_NOT_MERGED=$((DECLARED_NOT_MERGED + 1)) ;;
        *"部分落地"*) DECLARED_PARTIAL=$((DECLARED_PARTIAL + 1)) ;;
        *) log_warn "$pr — unrecognised status '$status' in the §2.1 table" ;;
    esac

    # --- actual state in git -----------------------------------------------
    n_commits=$(count_merge_commits_with_children "$pr")
    # The helper prints NOTHING when the PR has no counted merge commit, and
    # `[ "" -gt 0 ]` is an undefined comparison — bash wrote
    # "[: : integer expression expected" to stderr for every such PR
    # (issue #5542), burying the gate's real diagnostics. Normalise here:
    # empty or non-numeric means "no counted merge commit".
    case "$n_commits" in
        ''|*[!0-9]*) n_commits=0 ;;
    esac
    if [ "$n_commits" -gt 0 ]; then
        ACTUALLY_MERGED=$((ACTUALLY_MERGED + 1))
        gitea_ref=$(resolve_gitea_ref "$pr")
        gitea="${gitea_ref%%|*}"
        subj="${gitea_ref#*|}"
        gitea_label="${gitea:-?}"
        subj_label="${subj:0:72}"
        [ "$subj_label" = "" ] && subj_label="(no merge commit with this id; the evidence is a squash/direct implementation commit)"

        if printf '%s' "$status" | grep -q "已合并"; then
            log_pass "$pr — declared 已合并, $n_commits implementation commit line(s) in git (Gitea #$gitea_label, $subj_label)"
        else
            # Declared not-merged (or partially) but git says otherwise.
            # That is DOCUMENT DRIFT, not a gate failure — see the header.
            STALE_ROW=$((STALE_ROW + 1))
            log_warn "$pr — declared '$status' but $n_commits implementation commit line(s) exist in git (Gitea #$gitea_label, $subj_label). Status row is stale."
        fi
    else
        if printf '%s' "$status" | grep -q "已合并"; then
            # FALSE PASS — the dangerous direction.
            FALSE_PASS=$((FALSE_PASS + 1))
            log_error "$pr — declared 已合并 but NO merge commit exists in $BASE_REF. This is a false pass."
        else
            log_pass "$pr — declared '$status', no merge commit in git (consistent)"
        fi
    fi

    # --- dependency edge check ---------------------------------------------
    if [ "$dep_state" = "UNPARSED" ]; then
        log_error "$pr — the row mentions a dependency but no dependency number could be parsed from it. The dependency check did NOT run for this row."
        FALSE_PASS=$((FALSE_PASS + 1))
    fi
    if [ -n "$deps" ]; then
        for dep in $(printf '%s' "$deps" | tr ',' ' '); do
            dep_pr="PR-$dep"
            # Position of the dependency vs this node in the chain.
            dep_pos=$(printf '%s\n' "$CHAIN" | awk -F'|' -v p="$dep_pr" '$1 == p { print NR; exit }')
            node_pos=$(printf '%s\n' "$CHAIN" | awk -F'|' -v p="$pr" '$1 == p { print NR; exit }')
            if [ -z "$dep_pos" ]; then
                log_error "$pr — declares dependency $dep_pr, which is not a node in the §2 chain"
                FALSE_PASS=$((FALSE_PASS + 1))
            elif [ -n "$node_pos" ] && [ "$dep_pos" -gt "$node_pos" ]; then
                log_error "$pr — depends on $dep_pr but appears BEFORE it in the chain (position $node_pos vs $dep_pos). Dependency order inverted."
                FALSE_PASS=$((FALSE_PASS + 1))
            fi
        done
    fi
done <<< "$CHAIN"

# --- Phase 4b: status-table rows that are not in the chain ---------------------
# The loop above walks the §2 CHAIN. A §2.1 row whose PR is NOT a chain node
# would therefore never be checked at all — measured 2026-10-22: a table
# with an extra "PR-999 | ✅ 已合并" row was judged PASS, because nothing
# ever looked at it. A claimed-merged PR nobody verifies is exactly the
# false green this gate exists to remove.
#
# Every §2.1 row must correspond to a §2 chain node.
while IFS='|' read -r pr status; do
    [ -z "$pr" ] && continue
    if ! printf '%s\n' "$CHAIN" | awk -F'|' -v p="$pr" '$1 == p { found=1 } END { exit !found }'; then
        log_error "$pr — present in the §2.1 status table (status: $status) but NOT a node in the §2 chain. It would otherwise never be verified."
        FALSE_PASS=$((FALSE_PASS + 1))
        TABLE_ONLY="$TABLE_ONLY $pr"
    fi
done <<< "$TABLE"

# --- Phase 5: verdict ------------------------------------------------------------
echo
echo "--- Summary ---"
echo "  §2 chain nodes:        $CHAIN_COUNT"
echo "  §2.1 status rows:      $TABLE_COUNT"
echo "  declared 已合并:       $DECLARED_MERGED"
echo "  declared 未合并:       $DECLARED_NOT_MERGED"
echo "  declared 部分落地:     $DECLARED_PARTIAL"
echo "  actually merged:       $ACTUALLY_MERGED"
echo "  false passes:          $FALSE_PASS"
echo "  stale status rows:     $STALE_ROW"
echo "  errors:                $ERRORS"
echo "  warnings:              $WARNINGS"

if [ "$STALE_ROW" -gt 0 ]; then
    echo
    echo "⚠️  $STALE_ROW status row(s) in the §2.1 table are STALE (they say 未合并 but git has merge commits)."
    echo "   This is document drift, not a gate failure — the criterion is about whether"
    echo "   what is claimed as merged really is merged. Updating the table is a"
    echo "   documentation change and is out of scope for this gate."
fi

if [ "$FALSE_PASS" -gt 0 ]; then
    echo
    echo "❌ FAIL: $FALSE_PASS false pass(es) or structural error(s) detected."
    echo "   A PR declared as merged with no merge commit behind it means the gate"
    echo "   would certify work that never landed."
    exit 1
fi

if [ "$ERRORS" -gt 0 ]; then
    echo
    echo "❌ FAIL: $ERRORS error(s) detected."
    exit 1
fi

if [ "$CHAIN_COUNT" -eq 0 ] || [ "$TABLE_COUNT" -eq 0 ]; then
    echo
    echo "❌ FAIL: nothing was actually checked."
    exit 1
fi

echo
echo "✅ PASS: no false pass; dependency order intact; §2 chain and §2.1 table agree."
exit 0
