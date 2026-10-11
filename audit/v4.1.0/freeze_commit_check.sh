#!/usr/bin/env bash
# Freeze-commit check (iaia410-manifest/v1, SSOT section 5).
#
# Validates, at run time, that the assessed commit recorded in the
# manifest is an ancestor of HEAD (exact at run time; child commits and
# merges at replay are acceptable) and that the worktree is clean with
# respect to tracked files. Prints the evidence lines expected in
# traces/frozen_commit_check.txt.
set -euo pipefail

assessed="${1:-}"
if [[ ! "$assessed" =~ ^[0-9a-f]{40}$ ]]; then
  echo "usage: freeze_commit_check.sh <40-hex assessed commit>" >&2
  exit 2
fi
cd "$(git rev-parse --show-toplevel)"

if ! git merge-base --is-ancestor "$assessed" "$(git rev-parse HEAD)"; then
  echo "frozen-commit-check FAIL: assessed commit $assessed is not an ancestor of HEAD" >&2
  exit 1
fi
dirty="$(git status --porcelain=v1 --untracked-files=no)"
if [[ -n "$dirty" ]]; then
  echo "frozen-commit-check FAIL: tracked modifications present:" >&2
  echo "$dirty" >&2
  exit 1
fi

branch="$(git branch --show-current)"
[[ -n "$branch" ]] || branch="detached"
cargo_lock="$(sha256sum Cargo.lock | awk '{print $1}')"

echo "assessed_commit=$assessed"
echo "head_commit=$(git rev-parse HEAD)"
echo "branch_or_detached=$branch"
echo "cargo_lock_sha256=$cargo_lock"
echo "frozen-commit-check ok"
