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
# C5  **No step that runs a script from this repository may come before the
#     `Checkout` step.**  A step that says `bash scripts/…` is asking for a
#     file that only exists *after* the checkout; with the step ordered
#     first, the job runs it in an empty workspace and dies with
#
#         bash: scripts/ci/cache_cargo_registry.sh: No such file or directory
#         exitcode '127': command not found
#
#     That is not hypothetical: #5451 put the "Restore cargo registry
#     cache" step ahead of the checkout in all five cargo jobs, and CI run
#     9157 came back with every one of them red in 2-3 seconds — a red
#     pipeline whose message says nothing about the code under test.
#     The order that works is
#
#         Checkout -> Restore cargo registry cache -> <cargo steps> -> Save
#
#     because the checkout needs no cargo, and the restore only has to be
#     in place before the first cargo invocation.
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

# --- C5: a step that runs a repository script must come after the checkout --

# NOTE ON THE EMBEDDED PYTHON
# --------------------------
# The python body below lives in a **quoted** here-document
# (`<<'PY'`).  The bash that ships with macOS (3.2.57) mis-parses a quoted
# here-document that contains a backslash immediately in front of a single
# quote: the backslash-quote pair leaks out of the here-document and the
# script dies with "unexpected EOF while looking for matching `''".
#
# That is why the single quote in the regular expression is built with
# `chr(39)` at run time instead of being written as `\'`: the here-document
# then contains no backslash-quote pair at all.  Do not "simplify" it back.
#
# The step directories that hold executables belonging to this repository.
# A `run:` body that names one of them needs the file to be present, and the
# file only appears after the checkout.
REPO_SCRIPT_DIRS=(scripts gate tools)

STEP_ORDER_PY=$(cat <<'PY'
import re, sys

# A `run:` that names a path inside one of the repository script
# directories.  The path has to start the command or follow a shell
# metacharacter, so that `echo "see scripts/README"` does not count.
script_dirs = sys.argv[1].split(',')
# The single quote is built at run time on purpose: a backslash directly in
# front of a quote inside a quoted here-document breaks the parse of the
# bash that ships with macOS (3.2).  See the note in the shell section
# above the C5 check.
Q = chr(39)
path_re = re.compile(
    r'(?:^|[\s"' + Q + r'(=;|&])(?:\./)?(' + '|'.join(re.escape(d) for d in script_dirs) +
    r'/[A-Za-z0-9_][A-Za-z0-9_./-]*)'
)

step_re = re.compile(r'^(\s*)- name:\s*(.*?)\s*$')
job_re = re.compile(r'^  [A-Za-z0-9_-]+:\s*$')
run_block_re = re.compile(r'^(\s*)run:\s*\|\s*$')
run_inline_re = re.compile(r'^(\s*)run:\s*(\S.*)$')

rc = 0
for path in sys.argv[2:]:
    lines = open(path, encoding='utf-8').read().split('\n')
    job_hdr = [i for i, l in enumerate(lines) if job_re.match(l)]
    for j, start in enumerate(job_hdr):
        end = job_hdr[j + 1] if j + 1 < len(job_hdr) else len(lines)
        job = lines[start].strip()
        steps = []
        for i in range(start, end):
            m = step_re.match(lines[i])
            if m and len(m.group(1)) == 6:
                steps.append((i, m.group(2)))
        checkout_at = next((k for k, (_i, n) in enumerate(steps) if n == 'Checkout'), None)
        if checkout_at is None:
            continue
        for k in range(checkout_at):
            i, name = steps[k]
            stop = steps[k + 1][0] if k + 1 < len(steps) else end
            block = lines[i:stop]
            # the executable part: the `run:` body only.  Comment lines are
            # documentation and may mention any path at all.
            body = []
            r = 0
            while r < len(block):
                rm = run_block_re.match(block[r])
                if rm:
                    base = len(rm.group(1))
                    r += 1
                    while r < len(block):
                        line = block[r]
                        if line.strip() == '':
                            body.append('')
                            r += 1
                            continue
                        cur = len(line) - len(line.lstrip())
                        if cur <= base:
                            break
                        body.append(line[base + 2:] if len(line) > base + 2 else '')
                        r += 1
                    continue
                im = run_inline_re.match(block[r])
                if im:
                    body.append(im.group(2))
                r += 1
            text = '\n'.join(l for l in body if not l.strip().startswith('#'))
            hits = sorted(set(path_re.findall(text)))
            if hits:
                print("EARLY\t%s\t%s\tstep %r runs a repository script before the checkout: %s"
                      % (path, job, name, ', '.join(hits)))
                rc = 1
sys.exit(rc)
PY
)

check_repo_scripts_after_checkout() {
  local dir="${1:-.gitea/workflows}" f
  local joined
  joined="$(IFS=,; echo "${REPO_SCRIPT_DIRS[*]}")"
  local out rc
  out="$(mktemp "${TMPDIR:-/tmp}/ci_step_order.XXXXXX")" || return 1
  python3 -c "$STEP_ORDER_PY" "$joined" "$dir"/*.yml >"$out" 2>&1
  rc=$?
  if [ $rc -ne 0 ]; then
    while IFS=$'\t' read -r kind path job detail; do
      [ "$kind" = "EARLY" ] || continue
      fail "$path: $job — $detail"
    done <"$out"
    grep -v $'^EARLY\t' "$out" | sed 's/^/    /' >&2
  else
    pass "C5 no step runs a repository script before its checkout step"
  fi
  rm -f "$out"
  return 0
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

  # 5. A step that runs a repository script BEFORE the checkout must be
  #    caught.  This is the #5451 / CI run 9157 defect: the restore step
  #    ran `bash scripts/ci/cache_cargo_registry.sh` in an empty workspace
  #    and every cargo job died with exit 127.
  local wfdir3="$tmp/wf3/.gitea/workflows"
  mkdir -p "$wfdir3"
  cat >"$wfdir3/ci.yml" <<'EARLY'
name: early
on: [push]
jobs:
  early:
    runs-on: [hp-z6g4]
    steps:
      - name: Restore cargo registry cache (issue #5451)
        run: bash scripts/ci/cache_cargo_registry.sh restore
      - name: Checkout
        run: |
          git init -q -b main .
EARLY
  local before3="$FAILURES"
  check_repo_scripts_after_checkout "$tmp/wf3/.gitea/workflows" >/dev/null 2>&1
  if [ "$FAILURES" -le "$before3" ]; then
    echo "  selftest FAIL: a repository script running before the checkout was not caught" >&2
    return 1
  fi
  FAILURES="$before3"

  # 5b. The same script AFTER the checkout must NOT be flagged.
  cat >"$wfdir3/ci.yml" <<'LATE'
name: late
on: [push]
jobs:
  late:
    runs-on: [hp-z6g4]
    steps:
      - name: Checkout
        run: |
          git init -q -b main .
      - name: Restore cargo registry cache (issue #5451)
        # scripts/ci/cache_cargo_registry.sh is mentioned in this comment
        # on purpose: a comment is not an invocation.
        run: bash scripts/ci/cache_cargo_registry.sh restore
LATE
  local before4="$FAILURES"
  check_repo_scripts_after_checkout "$tmp/wf3/.gitea/workflows" >/dev/null 2>&1
  if [ "$FAILURES" -gt "$before4" ]; then
    echo "  selftest FAIL: a repository script running AFTER the checkout was flagged" >&2
    return 1
  fi
  FAILURES="$before4"
  echo "  selftest OK: a repository script before the checkout is detected, and one after it is not"

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
check_repo_scripts_after_checkout

if [ "$FAILURES" -gt 0 ]; then
  echo ""
  echo "=== $FAILURES CHECK(S) FAILED ==="
  exit 1
fi
echo "=== ALL CHECKS PASS ==="
exit 0
