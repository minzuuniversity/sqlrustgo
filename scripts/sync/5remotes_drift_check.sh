#!/usr/bin/env bash
# scripts/sync/5remotes_drift_check.sh — read-only checker for 5-remote drift.
#
# Compares the same ref across the 5 remotes and reports ahead/behind counts.
# Designed to run as a cron / on-demand health check. NEVER pushes.
#
# Usage:
#   scripts/sync/5remotes_drift_check.sh [--branches b1,b2,...] [--alert-threshold N]
#   scripts/sync/5remotes_drift_check.sh [--strict-main|--no-strict-main]
#
# Exit codes:
#   0 — all pairs within threshold AND main identical across remotes
#   1 — drift exceeds threshold, OR main diverged across remotes,
#       OR a remote does not carry a branch the others have
#   2 — at least one remote could not be measured (UNREACHABLE / STALE-DATA).
#       "cannot tell" is not "in sync": the run refuses to conclude.
#
# Output format (tab-separated for log scraping):
#   branch  pair_a  pair_b  ahead  behind
#
# The ahead/behind columns carry a **word** instead of a number whenever the
# pair could not be measured:
#
#   develop/v4.1.0  gitcode  gitee  UNREACHABLE  UNREACHABLE
#   develop/v4.1.0  gitcode  gitee  STALE-DATA   STALE-DATA
#   develop/v4.1.0  gitcode  gitee  NO-BRANCH    NO-BRANCH
#
# A number in those columns therefore always means "measured against the
# server". It never means "read off a local ref that might be old".
#
# WHY THE SERVER IS THE AUTHORITY
# -------------------------------
# This script used to read `git rev-parse $remote/$branch` — the **local
# remote-tracking ref** — and diff that against the other remotes' local
# refs. Those refs are only as fresh as the last successful `git fetch`, and
# the old fetch was `git fetch --all ... | tail -3`: the pipeline's exit code
# is **tail's**, so a fetch that failed for one remote was invisible, and the
# script went on to print that remote's numbers from a stale ref.
#
# Measured consequence (2026-11-13): gitcode had been pushed to the 252 tip
# (proved by `git ls-remote`), yet the report said
# `develop/v4.1.0  gitcode  gitea252  0  357` — the opposite of the truth.
#
# The same lesson was already learned in the sibling script
# (5remotes_sync.sh, Step 4): "问服务器" — ask the server, never the cache.
# This script now does the same:
#
#   tip(remote, branch) := git ls-remote <remote> refs/heads/<branch>
#
# and the diff is computed between **server tips**. The local tracking ref
# is only ever used as a cross-check; it is never the basis of a number.
#
# Per-remote, per-branch state (three states, never silently merged):
#
#   OK           — the server answered, the objects were fetched, and the
#                  tip is known ⇒ numbers may be printed.
#   UNREACHABLE  — the server did not answer (ls-remote failed) ⇒ the tip is
#                  unknown ⇒ numbers withheld.
#   STALE-DATA   — the server answered but the object view could not be
#                  confirmed (fetch failed) ⇒ the tip is known but the
#                  history is not ⇒ numbers withheld.
#
# 2026-09-30: added --strict-main (default ON).
#   Rationale: `main` is the GA release pointer (STAGE_CONFIG branches.main:
#   { stage: GA_only, mergeable: false, protected: true }). It must be
#   byte-identical across all remotes — unlike develop branches it carries no
#   in-flight work, so ANY divergence there is a defect, not normal drift.
#   On 2026-09-30 `main` was found 16657 commits behind and diverged from
#   develop/v4.1.0 (see docs/releases/v4.1.0/MAIN_DIVERGENCE_RESOLUTION_2026-09-30.md).
#   The old uniform threshold let that accumulate unnoticed.
#
# 2026-11-16: tri-state remote measurement (see "WHY THE SERVER IS THE
#   AUTHORITY" above). Numbers are now computed between server tips, and a
#   remote that cannot be measured yields a word, never a stale number.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT" || exit 2

DEFAULT_BRANCHES="develop/v4.1.0,main,release/v4.0.0,develop/v4.0.0"
DEFAULT_THRESHOLD=2
STRICT_MAIN=1

branches_csv="$DEFAULT_BRANCHES"
threshold="$DEFAULT_THRESHOLD"

while [ "${1:-}" != "" ]; do
    case "$1" in
        --branches)
            branches_csv="$2"
            shift 2
            ;;
        --alert-threshold)
            threshold="$2"
            shift 2
            ;;
        --strict-main)
            STRICT_MAIN=1
            shift
            ;;
        --no-strict-main)
            STRICT_MAIN=0
            shift
            ;;
        -h|--help)
            sed -n '2,40p' "$0"
            exit 0
            ;;
        *)
            echo "ERROR: unknown option $1" >&2
            exit 4
            ;;
    esac
done

IFS=',' read -ra branches <<< "$branches_csv"

REMOTES=(gitcode gitea250 gitea252 gitee github)

# ── SSH 通道（与 5remotes_sync.sh 同一教训，#5380）──────────────────────
#
# 调用方没有设置时由本脚本设置，否则 git 会继承调用方那个失效的 agent 而挂起
# （IdentityAgent=none 是唯一不可协商的那一项）。
if [ -z "${GIT_SSH_COMMAND:-}" ]; then
    export GIT_SSH_COMMAND="ssh -o BatchMode=yes -o ConnectTimeout=10 -o IdentitiesOnly=yes -o IdentityAgent=none -o StrictHostKeyChecking=accept-new -o ServerAliveInterval=15 -o ServerAliveCountMax=3"
fi
export GIT_TERMINAL_PROMPT=0

STATE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/drift_check.XXXXXX")" || exit 2
cleanup() { [ -n "${STATE_DIR:-}" ] && [ -d "$STATE_DIR" ] && rm -rf "$STATE_DIR"; }
trap cleanup EXIT INT TERM

echo "=== 5-remote drift check ==="
echo "branches: ${branches[*]}"
echo "threshold: $threshold commits"
echo ""

# ── Phase 1: 每个远端问一次服务器 + 把对象取回来 ──────────────────────
#
# 「远端现在是什么」必须问服务器（ls-remote 不看本地缓存）。
# 问不到 ⇒ UNREACHABLE；问到了但对象取不回来 ⇒ STALE-DATA。
echo "measuring remotes (ls-remote + fetch)..."
for r in "${REMOTES[@]}"; do
    shas_file="${STATE_DIR}/${r}.shas"
    : > "$shas_file"

    refspecs=""
    for b in "${branches[@]}"; do
        refspecs="${refspecs} refs/heads/${b}"
    done

    ls_out=$(git ls-remote "$r" $refspecs 2>/dev/null)
    ls_rc=$?

    if [ "$ls_rc" -ne 0 ]; then
        printf 'UNREACHABLE\n' > "${STATE_DIR}/${r}.state"
        printf 'ls-remote rc=%s\n' "$ls_rc" > "${STATE_DIR}/${r}.reason"
        echo "  $r: UNREACHABLE (ls-remote rc=$ls_rc)"
        continue
    fi

    printf '%s\n' "$ls_out" | awk 'NF>=2 {print $2"\t"$1}' > "$shas_file"

    fetch_out=$(git fetch --no-tags "$r" "${branches[@]}" 2>&1)
    fetch_rc=$?
    if [ "$fetch_rc" -ne 0 ]; then
        printf 'STALE-DATA\n' > "${STATE_DIR}/${r}.state"
        printf 'fetch rc=%s: %s\n' "$fetch_rc" "$(printf '%s' "$fetch_out" | tail -1)" \
            > "${STATE_DIR}/${r}.reason"
        echo "  $r: STALE-DATA (fetch rc=$fetch_rc — 服务器答了，但对象视图没确认)"
        continue
    fi

    printf 'OK\n' > "${STATE_DIR}/${r}.state"
    printf '\n' > "${STATE_DIR}/${r}.reason"
    echo "  $r: OK"
done

echo ""

# ── Phase 2: 两两比对 ───────────────────────────────────────────────────
#
# state_for_branch <remote> <branch> ⇒ "<STATE> <sha-or-empty>"
#
# sha 只在 OK / STALE-DATA 时有值（STALE-DATA 的 sha 是「服务器说的」，
# 但对象没确认 ⇒ 不可用于算 diff ⇒ 输出里仍然不出现数字）。
state_for_branch() {
    local r="$1" b="$2" st sha
    st=$(cat "${STATE_DIR}/${r}.state" 2>/dev/null || echo "UNREACHABLE")
    if [ "$st" = "UNREACHABLE" ]; then
        echo "UNREACHABLE"
        return
    fi
    sha=$(awk -v ref="refs/heads/${b}" '$1 == ref { print $2; exit }' "${STATE_DIR}/${r}.shas" 2>/dev/null)
    if [ -z "$sha" ]; then
        echo "NO-BRANCH"
        return
    fi
    if [ "$st" = "STALE-DATA" ]; then
        echo "STALE-DATA ${sha}"
        return
    fi
    echo "OK ${sha}"
}

EXIT_CODE=0
CANNOT_TELL=0
BRANCH_MISSING=0
MAIN_DIVERGED=0

echo -e "branch\tpair\tpair\tahead\tbehind"

for branch in "${branches[@]}"; do
    for ((i = 0; i < ${#REMOTES[@]}; i++)); do
        for ((j = i + 1; j < ${#REMOTES[@]}; j++)); do
            a="${REMOTES[$i]}"
            b="${REMOTES[$j]}"

            read -r state_a sha_a <<< "$(state_for_branch "$a" "$branch")"
            read -r state_b sha_b <<< "$(state_for_branch "$b" "$branch")"

            if [ "$state_a" != "OK" ] || [ "$state_b" != "OK" ]; then
                label="NO-BRANCH"
                if [ "$state_a" = "UNREACHABLE" ] || [ "$state_b" = "UNREACHABLE" ]; then
                    label="UNREACHABLE"
                    CANNOT_TELL=1
                elif [ "$state_a" = "STALE-DATA" ] || [ "$state_b" = "STALE-DATA" ]; then
                    label="STALE-DATA"
                    CANNOT_TELL=1
                else
                    BRANCH_MISSING=1
                fi
                printf "%s\t%s\t%s\t%s\t%s\n" "$branch" "$a" "$b" "$label" "$label"
                continue
            fi

            # 服务器 tip 已经拿到，但 diff 需要对象在本地。
            if ! git cat-file -e "${sha_a}^{commit}" 2>/dev/null \
               || ! git cat-file -e "${sha_b}^{commit}" 2>/dev/null; then
                printf "%s\t%s\t%s\t%s\t%s\n" "$branch" "$a" "$b" "NO-OBJECTS" "NO-OBJECTS"
                CANNOT_TELL=1
                continue
            fi

            ab=$(git rev-list --count "${sha_a}" "^${sha_b}" 2>/dev/null)
            ba=$(git rev-list --count "${sha_b}" "^${sha_a}" 2>/dev/null)
            if [ -z "$ab" ] || [ -z "$ba" ]; then
                printf "%s\t%s\t%s\t%s\t%s\n" "$branch" "$a" "$b" "NO-OBJECTS" "NO-OBJECTS"
                CANNOT_TELL=1
                continue
            fi
            printf "%s\t%s\t%s\t%s\t%s\n" "$branch" "$a" "$b" "$ab" "$ba"
            total=$((ab + ba))

            # Strict mode: main must be byte-identical across every remote.
            if [ "$STRICT_MAIN" -eq 1 ] && [ "$branch" = "main" ]; then
                if [ "$total" -ne 0 ]; then
                    MAIN_DIVERGED=1
                fi
            elif [ "$total" -gt "$threshold" ]; then
                EXIT_CODE=1
            fi
        done
    done
done

echo ""
echo "--- remote state ---"
for r in "${REMOTES[@]}"; do
    st=$(cat "${STATE_DIR}/${r}.state" 2>/dev/null || echo "UNKNOWN")
    reason=$(cat "${STATE_DIR}/${r}.reason" 2>/dev/null || echo "")
    tips=""
    for b in "${branches[@]}"; do
        read -r s sha <<< "$(state_for_branch "$r" "$b")"
        if [ "$s" = "OK" ]; then
            tips="${tips} ${b}=${sha:0:12}"
        else
            tips="${tips} ${b}=${s}"
        fi
    done
    echo "  ${r}: ${st}${reason:+ (${reason})}"
    echo "      ${tips# }"
done
echo ""

if [ "$STRICT_MAIN" -eq 1 ]; then
    # `main` 严格一致这条判据只在 **main 确实在被检查的分支里** 时才有意义。
    # 早先的实现无条件跑这一段：调用方只点名了 develop/v4.1.0 时，
    # 它拿 develop 的 SHA 去比「main 是否一致」，比的是另一个东西，
    # 结论既无意义又会误报漂移（2026-11-16 实测）。
    main_checked=0
    for b in "${branches[@]}"; do
        [ "$b" = "main" ] && main_checked=1
    done
    if [ "$main_checked" -eq 0 ]; then
        echo "MAIN: skipped — 'main' is not among the checked branches (${branches[*]})"
        echo ""
    else
    # Dedicated, unmissable report for the release pointer.
    main_tips=""
    main_undeterminable=0
    for a in "${REMOTES[@]}"; do
        read -r s sha <<< "$(state_for_branch "$a" "main")"
        if [ "$s" = "OK" ]; then
            main_tips="$main_tips ${sha:0:12}"
        else
            main_undeterminable=1
            main_tips="$main_tips ${s}"
        fi
    done
    main_uniq=$(echo $main_tips | tr ' ' '\n' | grep . | sort -u | wc -l | tr -d ' ')
    if [ "$main_undeterminable" -eq 0 ] && [ "$MAIN_DIVERGED" -eq 0 ] && [ "$main_uniq" = "1" ]; then
        echo "MAIN: OK — identical across all remotes (${main_tips# })"
    elif [ "$main_undeterminable" -ne 0 ]; then
        echo "MAIN: UNKNOWN — 'main' could not be measured on every remote (${main_tips# })"
        echo "      'cannot tell' is not 'in sync'. Numbers for 'main' are withheld."
        echo "      Resolve with: git ls-remote <remote> refs/heads/main"
        CANNOT_TELL=1
    else
        echo "MAIN: ERROR — 'main' has DIVERGED across remotes (strict mode, zero tolerance)."
        echo "      Distinct tips: ${main_uniq}"
        echo "      'main' is the GA release pointer and must be byte-identical everywhere."
        echo "      Resolve via docs/releases/v4.1.0/MAIN_DIVERGENCE_RESOLUTION_2026-09-30.md"
        echo "      (backup branch/tag: backup/main-pre-convergence-2026-09-30,"
        echo "       archive/main-pre-convergence-2026-09-30)"
        EXIT_CODE=1
    fi
    echo ""
    fi
fi

if [ "$CANNOT_TELL" -ne 0 ]; then
    EXIT_CODE=2
elif [ "$BRANCH_MISSING" -ne 0 ]; then
    EXIT_CODE=1
fi

if [ "$EXIT_CODE" -eq 0 ]; then
    echo "RESULT: all branches within threshold ($threshold)"
elif [ "$EXIT_CODE" -eq 1 ]; then
    echo "RESULT: drift detected (threshold=$threshold, strict_main=$STRICT_MAIN)"
else
    echo "RESULT: cannot tell — at least one remote was UNREACHABLE or STALE-DATA."
    echo "        Numbers for the affected pairs are withheld on purpose:"
    echo "        a stale local ref must never be presented as a measurement."
fi

exit "$EXIT_CODE"
