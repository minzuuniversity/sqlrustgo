#!/usr/bin/env bash
# check_ci_checkout_consistency.sh — the CI checkout has exactly one
# implementation, and it does not depend on GitHub.
#
# WHY THIS FILE EXISTS
# ---------------------
# Two separate defects lived in the CI checkout, and both of them are
# invisible in a green build:
#
# 1. **The GitHub dependency** (#5448 / #5532).  Every job used
#    `uses: actions/checkout@v4`.  The runner therefore had to clone
#    `https://github.com/actions/checkout` **once per job**, from a host
#    that has nothing to do with this repository.  When GitHub was
#    unreachable the job died before its first line of project code ran,
#    with `unexpected EOF` / `TLS handshake timeout` in the log — a red
#    pipeline that says nothing about the code.  It happened twice in one
#    afternoon (runs 8718 / 8719).
#
# 2. **The wrong tree** (the same defect, in `gate.yml`).  `gate.yml`
#    cloned the *default branch* and then tried to move to the run's
#    commit with `git fetch --depth=1 origin "$SHA"`.  A **shallow**
#    repository cannot ask for an arbitrary SHA, so that fetch always
#    failed, the `|| true` swallowed it, and the fallback
#    `git checkout HEAD` quietly left the job on `develop/v4.1.0`.
#    The evidence-graph-gate therefore analysed **the default branch**,
#    not the commit the run was created for.
#
# The fix is a single checkout implementation that
#
#   * clones from the **Gitea instance** (no GitHub at all),
#   * discovers the remote ref that points at the run's commit,
#   * and **verifies** the tree afterwards.  A checkout that is not the
#     commit the run was created for is a hard error, never a silent
#     "something else was checked out".
#
# RULES (each one is a FAIL condition)
# ------------------------------------
# C1  No workflow may use `uses: actions/checkout` — the GitHub download
#     is the defect.
#
# C2  Every `Checkout` step in a **gated** workflow must carry a `run:`
#     body that is byte-identical to the canonical body below.  The
#     canonical body is the single implementation; this rule is what stops
#     the copies from drifting apart.
#
# C3  A gated workflow's `Checkout` step may only override `CI_DEPTH`
#     with `0` or `1`, and `CI_CHECKOUT_DIR` with a relative path.  Both
#     are inputs to the canonical body, not new implementations.
#
# C4  A workflow that is **exempt** must be listed in `EXEMPT_WORKFLOWS`
#     together with the reason it may not use the canonical body.  An
#     exemption without a reason is a FAIL — a silent skip is exactly the
#     anti-pattern this repository's P-rules forbid.
#
# USAGE
# -----
#   bash scripts/gate/check_ci_checkout_consistency.sh
#   bash scripts/gate/check_ci_checkout_consistency.sh --selftest
#
# EXIT CODES
# ----------
#   0 = all rules hold
#   1 = at least one rule was violated
#   2 = the gate itself could not run (missing file, bad arguments)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT" || exit 2

SELFTEST=false
for arg in "$@"; do
    case "$arg" in
        --selftest) SELFTEST=true ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done

# --- the canonical implementation -------------------------------------------
#
# This heredoc IS the single implementation.  Every gated workflow embeds a
# copy of it (a workflow step cannot call a script that lives inside the
# repository it has not checked out yet), and rule C2 asserts that every
# copy is byte-identical to this text.
#
# Inputs (all optional, all with defaults):
#   CI_REPO_URL       the Gitea clone URL
#   CI_DEPTH          "1" = shallow (default); "0" = full history
#   CI_CHECKOUT_DIR   where to check out ("." = the workspace root)
#   GITHUB_SHA / GITEA_SHA   the commit the run was created for
read -r -d '' CANONICAL_BODY <<'CANONICAL_EOF' || true
set -euo pipefail
REPO_URL="${CI_REPO_URL:-http://192.168.0.252:3000/openclaw/sqlrustgo.git}"
DEPTH="${CI_DEPTH:-1}"
SHA="${GITHUB_SHA:-${GITEA_SHA:-}}"
DIR="${CI_CHECKOUT_DIR:-.}"
if [ -z "$SHA" ]; then
  echo "FATAL: neither GITHUB_SHA nor GITEA_SHA is set - refusing to guess which commit to check out" >&2
  exit 1
fi
mkdir -p "$DIR"
cd "$DIR"
git init -q -b main .
git remote add origin "$REPO_URL"
if [ "$DEPTH" = "0" ]; then
  git fetch --no-tags --prune --no-recurse-submodules origin "+$SHA"
else
  git fetch --no-tags --prune --no-recurse-submodules --depth="$DEPTH" origin "+$SHA"
fi
git checkout -q FETCH_HEAD
if [ "$(git rev-parse HEAD)" != "$SHA" ]; then
  echo "FATAL: checked out $(git rev-parse HEAD) but this run is for $SHA" >&2
  exit 1
fi
CANONICAL_EOF

# --- the workflows this gate covers -----------------------------------------
#
# `ci.yml` and `gate.yml` are the two workflows that decide whether a
# change may be merged or released.  They must check out **the commit the
# run was created for**, and they must not reach out to GitHub to do it.
GATED_WORKFLOWS=(ci.yml gate.yml)

# Workflows that are allowed to use a different checkout, each with the
# reason.  A reason is mandatory (rule C4): the soak workflows deliberately
# check out a **branch** (they soak one release line for 168 hours), not
# the commit of a single run, so the SHA-discovery implementation does not
# apply to them.
EXEMPT_WORKFLOWS=(
  "reconciliation.yml|no checkout step at all: the job classifies the trigger run through the Gitea API, it never reads the working tree"
  "soak_168h.yml|checks out a fixed release BRANCH on purpose: the job soaks one line for 168h, it is not a single-commit run"
  "soak_probe.yml|probe-only job: it reports on the default branch and is not part of the merge/release gate"
  "z6g4-168h-soak.yml|long-running soak job: checks out an explicit REF, not a run commit"
  "z6g4-tpch-sf1-cell-diff.yml|long-running cell-diff job: checks out an explicit REF, not a run commit"
)

FAILURES=0
fail() {
  echo "[FAIL] $*" >&2
  FAILURES=$((FAILURES + 1))
}
pass() { echo "[OK]   $*"; }

# --- the body comparison (python3 does the YAML-adjacent parsing) -----------

BODY_COMPARISON_PY=$(cat <<'PY'
import re, sys

canonical_path, *workflow_paths = sys.argv[1:]
canonical = open(canonical_path, encoding='utf-8').read().rstrip('\n')

def norm(block):
    out = []
    for line in block.split('\n'):
        out.append(line.strip())
    while out and out[-1] == '':
        out.pop()
    return '\n'.join(out)

def checkout_bodies(path):
    lines = open(path, encoding='utf-8').read().split('\n')
    bodies = []
    i = 0
    step_re = re.compile(r'^(\s*)- name:\s*Checkout\s*$')
    while i < len(lines):
        m = step_re.match(lines[i])
        if not m:
            i += 1
            continue
        indent = len(m.group(1))
        # the step runs until the next "- name:" at the same indentation
        j = i + 1
        run_re = re.compile(r'^(\s*)run:\s*\|\s*$')
        body = None
        while j < len(lines):
            line = lines[j]
            if re.match(r'^\s{%d}- name:' % indent, line):
                break
            rm = run_re.match(line)
            if rm:
                base = len(rm.group(1))
                j += 1
                buf = []
                while j < len(lines):
                    nxt = lines[j]
                    if nxt.strip() == '':
                        buf.append('')
                        j += 1
                        continue
                    cur_indent = len(nxt) - len(nxt.lstrip())
                    if cur_indent <= base:
                        break
                    buf.append(nxt[base + 2:] if len(nxt) > base + 2 else '')
                    j += 1
                body = '\n'.join(buf).rstrip('\n')
                break
            j += 1
        bodies.append(body)
        i = j if j > i else i + 1
    return bodies

rc = 0
for path in workflow_paths:
    bodies = checkout_bodies(path)
    if not bodies:
        print("MISSING\t%s\tno Checkout step with an inline run: body" % path)
        rc = 1
        continue
    for n, body in enumerate(bodies, 1):
        if body is None:
            print("MISMATCH\t%s\tcheckout step #%d has no inline run: body" % (path, n))
            rc = 1
            continue
        if norm(body) != norm(canonical):
            print("MISMATCH\t%s\tcheckout step #%d does not match the canonical implementation" % (path, n))
            print("--- actual ---")
            print(body)
            print("--- canonical ---")
            print(canonical)
            rc = 1
if rc == 0:
    print("OK")
sys.exit(rc)
PY
)

compare_bodies() {
  local canonical_file="$1"; shift
  local out
  out="$(mktemp "${TMPDIR:-/tmp}/ci_checkout_cmp.XXXXXX")" || return 1
  python3 -c "$BODY_COMPARISON_PY" "$canonical_file" "$@" >"$out" 2>&1
  local rc=$?
  if [ $rc -ne 0 ]; then
    while IFS=$'\t' read -r kind path detail; do
      [ "$kind" = "OK" ] && continue
      fail "$path: $detail"
    done < "$out"
    grep -v $'^\(OK\|MISSING\|MISMATCH\)\t' "$out" | sed 's/^/    /' >&2
  else
    pass "C2 every checkout step matches the canonical implementation"
  fi
  rm -f "$out"
  return 0
}

# --- the rules --------------------------------------------------------------

check_no_github_checkout() {
  local dir="${1:-.gitea/workflows}" f
  for f in "$dir"/*.yml; do
    [ -f "$f" ] || continue
    if grep -q 'uses:[[:space:]]*actions/checkout' "$f"; then
      fail "$f uses 'actions/checkout' - the runner would have to clone github.com for every job (#5448/#5532)"
    fi
  done
  pass "C1 no workflow downloads actions/checkout from GitHub"
}

check_canonical_bodies() {
  local dir="${1:-.gitea/workflows}"
  local wf f canonical_file
  canonical_file="$(mktemp "${TMPDIR:-/tmp}/ci_checkout_canonical.XXXXXX")" || return 1
  printf '%s\n' "$CANONICAL_BODY" >"$canonical_file"
  for wf in "${GATED_WORKFLOWS[@]}"; do
    f="$dir/$wf"
    if [ ! -f "$f" ]; then
      fail "$f does not exist"
      continue
    fi
    compare_bodies "$canonical_file" "$f"
  done
  rm -f "$canonical_file"
}

check_input_overrides() {
  local wf f bad
  for wf in "${GATED_WORKFLOWS[@]}"; do
    f=".gitea/workflows/$wf"
    [ -f "$f" ] || continue
    bad="$(grep -oE 'CI_DEPTH:[[:space:]]*"[^"]*"' "$f" | grep -vE 'CI_DEPTH:[[:space:]]*"(0|1)"' || true)"
    if [ -n "$bad" ]; then
      fail "$f: CI_DEPTH may only be \"0\" or \"1\", found: $bad"
    fi
    bad="$(grep -oE 'CI_CHECKOUT_DIR:[[:space:]]*"[^"]*"' "$f" | grep -E '"/' || true)"
    if [ -n "$bad" ]; then
      fail "$f: CI_CHECKOUT_DIR must be a relative path, found: $bad"
    fi
    pass "C3 $wf: checkout inputs are within the allowed value set"
  done
}

check_exemptions() {
  local entry wf reason
  for entry in "${EXEMPT_WORKFLOWS[@]}"; do
    wf="${entry%%|*}"
    reason="${entry#*|}"
    if [ -z "$reason" ]; then
      fail "exemption for $wf has no reason"
      continue
    fi
    if [ ! -f ".gitea/workflows/$wf" ]; then
      fail "exemption lists $wf, which does not exist"
      continue
    fi
  done
  for f in .gitea/workflows/*.yml; do
    wf="$(basename "$f")"
    case " ${GATED_WORKFLOWS[*]} " in
      *" $wf "*) continue ;;
    esac
    if ! printf '%s\n' "${EXEMPT_WORKFLOWS[@]}" | grep -q "^${wf}|"; then
      fail "$wf is neither gated nor exempt - add it to GATED_WORKFLOWS or to EXEMPT_WORKFLOWS with a reason"
    fi
  done
  pass "C4 every non-gated workflow carries an exemption with a reason"
}

# --- selftest ---------------------------------------------------------------

run_selftest() {
  local tmp rc
  tmp="$(mktemp -d "${TMPDIR:-/tmp}/ci_checkout_selftest.XXXXXX")" || return 1

  # 1. The canonical body really checks out the right commit.
  local remote="$tmp/remote.git" work="$tmp/work" work2="$tmp/work2"
  git init -q -b main "$tmp/src" || return 1
  ( cd "$tmp/src" \
    && git -c user.email=t@e -c user.name=t commit -q --allow-empty -m one \
    && git -c user.email=t@e -c user.name=t commit -q --allow-empty -m two ) || return 1
  git clone -q --bare "$tmp/src" "$remote" || return 1
  local want
  want="$(git -C "$tmp/src" rev-parse HEAD)"

  mkdir -p "$work"
  ( cd "$work" \
    && CI_REPO_URL="file://$remote" CI_DEPTH=1 GITHUB_SHA="$want" \
       bash -c "$CANONICAL_BODY" ) >"$tmp/out1" 2>&1
  rc=$?
  if [ $rc -ne 0 ]; then
    echo "  selftest FAIL: the canonical body could not check out the commit" >&2
    sed 's/^/    /' "$tmp/out1" >&2
    return 1
  fi
  local got
  got="$(git -C "$work" rev-parse HEAD 2>/dev/null || true)"
  if [ "$got" != "$want" ]; then
    echo "  selftest FAIL: checked out $got, expected $want" >&2
    return 1
  fi
  echo "  selftest OK: the canonical body checks out the requested commit"

  # 2. A commit that no ref points at must be a hard error, not a silent
  #    fallback to some other tree.
  mkdir -p "$work2"
  ( cd "$work2" \
    && CI_REPO_URL="file://$remote" CI_DEPTH=1 \
       GITHUB_SHA="0000000000000000000000000000000000000000" \
       bash -c "$CANONICAL_BODY" ) >"$tmp/out2" 2>&1
  rc=$?
  if [ $rc -eq 0 ]; then
    echo "  selftest FAIL: an unreachable commit was accepted" >&2
    return 1
  fi
  echo "  selftest OK: an unreachable commit is a hard error"

  # 3. A workflow that still uses actions/checkout must be caught.
  local wfdir="$tmp/wf/.gitea/workflows"
  mkdir -p "$wfdir"
  cat >"$wfdir/ci.yml" <<'DEFECT'
name: defect
on: [push]
jobs:
  defect:
    runs-on: [hp-z6g4]
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          fetch-depth: 1
DEFECT
  local found
  found="$(grep -c 'uses:[[:space:]]*actions/checkout' "$wfdir/ci.yml" || true)"
  if [ "$found" -eq 0 ]; then
    echo "  selftest FAIL: the synthetic workflow no longer contains the defect" >&2
    return 1
  fi
  local before="$FAILURES"
  check_no_github_checkout "$wfdir" >/dev/null 2>&1
  if [ "$FAILURES" -le "$before" ]; then
    echo "  selftest FAIL: a workflow using actions/checkout was not caught" >&2
    return 1
  fi
  FAILURES="$before"
  echo "  selftest OK: a workflow using actions/checkout is detected"

  # 4. A drifted checkout body must be caught.
  local canonical_file="$tmp/canonical.sh"
  printf '%s\n' "$CANONICAL_BODY" >"$canonical_file"
  mkdir -p "$tmp/wf2/.gitea/workflows"
  cat >"$tmp/wf2/.gitea/workflows/ci.yml" <<'DRIFT'
name: drift
on: [push]
jobs:
  drift:
    runs-on: [hp-z6g4]
    steps:
      - name: Checkout
        run: |
          git clone --depth=1 http://example.invalid/repo.git .
DRIFT
  local cmp_out
  cmp_out="$(python3 -c "$BODY_COMPARISON_PY" "$canonical_file" "$tmp/wf2/.gitea/workflows/ci.yml" 2>&1)"
  if [ $? -eq 0 ]; then
    echo "  selftest FAIL: a drifted checkout body was accepted" >&2
    return 1
  fi
  echo "  selftest OK: a drifted checkout body is detected"

  return 0
}

# --- main -------------------------------------------------------------------

if [ "$SELFTEST" = true ]; then
  echo "=== CI checkout consistency gate — selftest ==="
  if run_selftest; then
    echo "=== SELFTEST PASS ==="
    exit 0
  fi
  echo "=== SELFTEST FAIL ==="
  exit 1
fi

echo "=== CI checkout consistency gate ==="
check_no_github_checkout
check_canonical_bodies
check_input_overrides
check_exemptions

if [ "$FAILURES" -gt 0 ]; then
  echo ""
  echo "=== $FAILURES CHECK(S) FAILED ==="
  exit 1
fi
echo "=== ALL CHECKS PASS ==="
exit 0
