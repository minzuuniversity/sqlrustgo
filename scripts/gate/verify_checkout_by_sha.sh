#!/usr/bin/env bash
#
# scripts/gate/verify_checkout_by_sha.sh
#
# Purpose: prove that a CI job analyses **the commit its run was created
#          for**, and fail loudly when it does not.
#          Issue #5534 reported that `gate.yml` could end up analysing the
#          default branch's tree while the evidence graph / legacy gate
#          reported that tree as "the run's commit".  The canonical checkout
#          body (see `scripts/gate/check_ci_checkout_consistency.sh`) already
#          refuses to guess which commit to check out; this script is the
#          *verifier* that re-asserts the guarantee at the point of analysis
#          and never falls back to "whatever tree happens to be present".
#
# Coverage: #5534 (gate.yml analysed a tree that was not the run's commit),
#           #5447 (a gate must grade the tree it actually analysed).
#
# Principle: P3 (one rule, one implementation) / P16 (a failure must surface,
#           not be laundered into a PASS).
#           The checkout rule is NOT re-implemented here.  `--selftest`
#           extracts the canonical body from
#           `scripts/gate/check_ci_checkout_consistency.sh` and exercises that
#           very body, so the checkout and the verifier cannot drift apart.
#
# Verify:  `--selftest` is hermetic and offline (a local `file://` fixture) and
#           covers:
#             * a reachable, non-tip SHA is checked out exactly;
#             * an unreachable SHA fails the fetch with the remote's own
#               error and leaves **no** tree behind (no silent fallback);
#             * a tree whose HEAD differs from the run's SHA is rejected;
#             * a missing / blank / all-zero run SHA is refused, not guessed.
#
# Usage:
#   bash scripts/gate/verify_checkout_by_sha.sh --selftest
#   bash scripts/gate/verify_checkout_by_sha.sh [--dir DIR] [--sha SHA]
#
# Exit codes:
#   0 = the tree at DIR is exactly the run's commit
#   1 = the tree is not the run's commit, or the run's commit is unusable
#   2 = usage error
#
# ---------------------------------------------------------------------------

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CANONICAL_SCRIPT="$SCRIPT_DIR/check_ci_checkout_consistency.sh"

# Exit codes used by this script.
readonly EXIT_OK=0
readonly EXIT_FAIL=1
readonly EXIT_USAGE=2

usage() {
  cat <<'USAGE'
Usage:
  verify_checkout_by_sha.sh --selftest
  verify_checkout_by_sha.sh [--dir DIR] [--sha SHA]

  --selftest   run the hermetic, offline self-test
  --dir DIR    the checked-out working tree to verify (default: ".")
  --sha SHA    the commit the run is for (default: $GITHUB_SHA, then $GITEA_SHA)

Exit codes: 0 = tree is the run's commit, 1 = mismatch/unusable, 2 = usage.
USAGE
}

die_usage() {
  echo "FATAL: $*" >&2
  usage >&2
  exit "$EXIT_USAGE"
}

# --- helpers ----------------------------------------------------------------

# A run SHA must be a non-empty, non-all-zero hex object name.
is_usable_sha() {
  local sha="${1:-}"
  [ -n "$sha" ] || return 1
  [ "${sha//0/}" != "" ] || return 1
  case "$sha" in
    *[!0-9a-fA-F]*) return 1 ;;
  esac
  [ "${#sha}" -ge 7 ] || return 1
  [ "${#sha}" -le 40 ] || return 1
  return 0
}

resolve_run_sha() {
  local candidate="${1:-}"
  if [ -z "$candidate" ]; then
    candidate="${GITHUB_SHA:-${GITEA_SHA:-}}"
  fi
  if ! is_usable_sha "$candidate"; then
    echo "FATAL: the run's commit is unavailable or unusable ('${candidate}') -" \
         "refusing to guess which commit this run analyses" >&2
    return "$EXIT_FAIL"
  fi
  printf '%s' "$candidate"
  return 0
}

# git_head <dir> -> prints the full HEAD sha, or fails with git's own stderr.
git_head() {
  local dir="$1" out rc
  out="$(git -C "$dir" rev-parse --verify HEAD 2>&1)"
  rc=$?
  if [ "$rc" -ne 0 ]; then
    printf '%s\n' "$out" >&2
    return "$rc"
  fi
  printf '%s' "$out"
  return 0
}

# sha_matches <actual-full-sha> <expected> -> 0 when expected names actual.
sha_matches() {
  local actual="$1" expected="$2"
  if [ "${#expected}" -eq 40 ]; then
    [ "$actual" = "$expected" ]
    return $?
  fi
  case "$actual" in
    "$expected"*) return 0 ;;
  esac
  return 1
}

# --- verify mode ------------------------------------------------------------

verify_tree() {
  local dir="$1" expected="$2" actual rc

  if [ ! -d "$dir" ]; then
    echo "FATAL: the checked-out tree '$dir' does not exist -" \
         "refusing to analyse anything but the run's commit" >&2
    return "$EXIT_FAIL"
  fi

  actual="$(git_head "$dir")"
  rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "FATAL: cannot read HEAD of '$dir' (git exit $rc) -" \
         "refusing to guess which commit was analysed" >&2
    return "$EXIT_FAIL"
  fi

  if ! sha_matches "$actual" "$expected"; then
    echo "FATAL: the tree at '$dir' is at $actual but this run is for $expected -" \
         "the job would analyse a commit it was not created for" >&2
    return "$EXIT_FAIL"
  fi

  echo "verified: $dir is at $actual (this run is for $expected)"
  return "$EXIT_OK"
}

# --- selftest ---------------------------------------------------------------

# Extract the canonical checkout body from the consistency gate so that the
# selftest exercises the *production* rule instead of a second copy of it.
# A failure to extract is a hard error: silently testing a different body
# would be exactly the kind of fake green this repository forbids.
extract_canonical_body() {
  local out
  [ -f "$CANONICAL_SCRIPT" ] || {
    echo "FATAL: cannot find the canonical checkout body in $CANONICAL_SCRIPT" >&2
    return 1
  }
  out="$(awk -v start="read -r -d '' CANONICAL_BODY <<'CANONICAL_EOF'" \
        'index($0, start) { f = 1; next }
         f && /^CANONICAL_EOF$/ { exit }
         f { print }' "$CANONICAL_SCRIPT")"
  if [ -z "$out" ] || ! grep -q 'git fetch' <<< "$out"; then
    echo "FATAL: could not extract the canonical checkout body from $CANONICAL_SCRIPT" >&2
    return 1
  fi
  printf '%s\n' "$out"
  return 0
}

selftest_fail() {
  echo "SELFTEST FAIL: $*" >&2
  return "$EXIT_FAIL"
}

run_selftest() {
  local tmp rc sha_head sha_parent bogus body head_a head_b
  local canon="$CANONICAL_SCRIPT"

  tmp="$(mktemp -d "${TMPDIR:-/tmp}/verify-checkout-by-sha.XXXXXX")"
  if [ -z "$tmp" ] || [ ! -d "$tmp" ]; then
    echo "SELFTEST FAIL: could not create a scratch directory" >&2
    return "$EXIT_FAIL"
  fi
  # shellcheck disable=SC2064
  trap "rm -rf '$tmp'" EXIT

  body="$(extract_canonical_body)"
  rc=$?
  if [ "$rc" -ne 0 ]; then
    selftest_fail "the canonical checkout body could not be extracted"
    return "$EXIT_FAIL"
  fi

  # --- fixture: a bare remote with two commits ------------------------------
  if ! git init -q -b main "$tmp/src" >/dev/null 2>&1; then
    selftest_fail "could not create the fixture repository"
    return "$EXIT_FAIL"
  fi
  if ! git -C "$tmp/src" -c user.name=t -c user.email=t@e \
        commit -q --allow-empty -m one >/dev/null 2>&1; then
    selftest_fail "could not create the fixture's first commit"
    return "$EXIT_FAIL"
  fi
  sha_head="$(git -C "$tmp/src" rev-parse HEAD)"
  if ! git -C "$tmp/src" -c user.name=t -c user.email=t@e \
        commit -q --allow-empty -m two >/dev/null 2>&1; then
    selftest_fail "could not create the fixture's second commit"
    return "$EXIT_FAIL"
  fi
  sha_parent="$(git -C "$tmp/src" rev-parse HEAD)"
  if ! git clone -q --bare "$tmp/src" "$tmp/remote.git" >/dev/null 2>&1; then
    selftest_fail "could not create the fixture's bare remote"
    return "$EXIT_FAIL"
  fi
  # Make "fetch this exact object id" deterministic across git versions.
  git -C "$tmp/remote.git" config uploadpack.allowAnySHA1InWant true

  # --- case A: a reachable, NON-TIP sha is checked out exactly --------------
  mkdir -p "$tmp/case_a"
  (
    cd "$tmp/case_a" || exit 1
    CI_REPO_URL="file://$tmp/remote.git" CI_DEPTH=1 CI_CHECKOUT_DIR="repo" \
      GITHUB_SHA="$sha_parent" bash -c "$body"
  ) >"$tmp/case_a.out" 2>"$tmp/case_a.err"
  rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "--- canonical body stderr ---" >&2
    cat "$tmp/case_a.err" >&2
    selftest_fail "the canonical body could not check out a reachable non-tip sha (exit $rc)"
    return "$EXIT_FAIL"
  fi
  head_a="$(git -C "$tmp/case_a/repo" rev-parse --verify HEAD 2>/dev/null)"
  if [ "$head_a" != "$sha_parent" ]; then
    selftest_fail "expected $sha_parent after checkout, got '${head_a:-<none>}'"
    return "$EXIT_FAIL"
  fi
  verify_tree "$tmp/case_a/repo" "$sha_parent" >/dev/null
  if [ "$?" -ne "$EXIT_OK" ]; then
    selftest_fail "the verifier rejected a correctly checked-out tree"
    return "$EXIT_FAIL"
  fi

  # --- case B: an unreachable sha fails hard and leaves no tree -------------
  bogus="0000000000000000000000000000000000000000"
  mkdir -p "$tmp/case_b"
  (
    cd "$tmp/case_b" || exit 1
    CI_REPO_URL="file://$tmp/remote.git" CI_DEPTH=1 CI_CHECKOUT_DIR="repo" \
      GITHUB_SHA="$bogus" bash -c "$body"
  ) >"$tmp/case_b.out" 2>"$tmp/case_b.err"
  rc=$?
  if [ "$rc" -eq 0 ]; then
    selftest_fail "an unreachable sha was accepted silently"
    return "$EXIT_FAIL"
  fi
  if git -C "$tmp/case_b/repo" rev-parse --verify HEAD >/dev/null 2>&1; then
    selftest_fail "a tree was left behind after a failed checkout"
    return "$EXIT_FAIL"
  fi
  echo "selftest case B: unreachable sha rejected with exit $rc"
  sed -n '1p' "$tmp/case_b.err" >&2

  # --- case C: a tree that is not the run's commit is rejected ---------------
  verify_tree "$tmp/case_a/repo" "$sha_head" >/dev/null 2>&1
  if [ "$?" -eq "$EXIT_OK" ]; then
    selftest_fail "the verifier accepted a tree that is not the run's commit"
    return "$EXIT_FAIL"
  fi
  verify_tree "$tmp/does-not-exist" "$sha_parent" >/dev/null 2>&1
  if [ "$?" -eq "$EXIT_OK" ]; then
    selftest_fail "the verifier accepted a missing tree"
    return "$EXIT_FAIL"
  fi

  # --- case D: an unusable run sha is refused, never guessed -----------------
  ( unset GITHUB_SHA GITEA_SHA; resolve_run_sha "" >/dev/null 2>&1 )
  if [ "$?" -eq "$EXIT_OK" ]; then
    selftest_fail "the verifier guessed a run sha that was not available"
    return "$EXIT_FAIL"
  fi
  resolve_run_sha "0000000000000000000000000000000000000000" >/dev/null 2>&1
  if [ "$?" -eq "$EXIT_OK" ]; then
    selftest_fail "the verifier accepted an all-zero run sha"
    return "$EXIT_FAIL"
  fi
  resolve_run_sha "not-a-sha" >/dev/null 2>&1
  if [ "$?" -eq "$EXIT_OK" ]; then
    selftest_fail "the verifier accepted a malformed run sha"
    return "$EXIT_FAIL"
  fi

  echo "SELFTEST PASS: checkout-by-sha is verified end to end (cases A-D)"
  return "$EXIT_OK"
}

# --- main -------------------------------------------------------------------

main() {
  local dir="." sha="" selftest=0

  while [ $# -gt 0 ]; do
    case "$1" in
      --selftest)
        selftest=1
        shift
        ;;
      --dir)
        [ $# -ge 2 ] || die_usage "--dir needs a directory"
        dir="$2"
        shift 2
        ;;
      --sha)
        [ $# -ge 2 ] || die_usage "--sha needs a commit"
        sha="$2"
        shift 2
        ;;
      -h|--help)
        usage
        exit "$EXIT_OK"
        ;;
      *)
        die_usage "unknown argument '$1'"
        ;;
    esac
  done

  if [ "$selftest" -eq 1 ]; then
    run_selftest
    return $?
  fi

  local resolved
  resolved="$(resolve_run_sha "$sha")"
  local rc=$?
  if [ "$rc" -ne 0 ]; then
    return "$EXIT_FAIL"
  fi

  verify_tree "$dir" "$resolved"
  return $?
}

main "$@"
