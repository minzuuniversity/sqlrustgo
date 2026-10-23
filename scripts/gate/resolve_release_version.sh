#!/usr/bin/env bash
# resolve_release_version.sh — single source of truth for "which release line
# is this checkout part of?".
#
# #5446: the postcheck job used to probe for `origin/develop/v3.8.0` and
# `origin/develop/v3.7.0`. With `fetch-depth: 1` neither remote-tracking
# reference exists, and on Gitea neither branch exists at all — so the
# detection was **v3.7.0 on every branch**, v4.1.0 included, and every
# version-sensitive gate in that job ran against the v3.7.0 contract while
# the step stayed green.
#
# The previous default (`v3.7.0`) is gone on purpose: a wrong-but-plausible
# version is worse than a visible failure, because it makes a gate check a
# release line nobody asked about. This script exits non-zero instead.
#
# Resolution order:
#   1. explicit argument (a branch name or a version)
#   2. $GITHUB_REF / $GITEA_REF      (e.g. refs/heads/develop/v4.1.0)
#   3. $GITHUB_REF_NAME / $GITEA_REF_NAME
#   4. the current git branch
#   5. the newest existing docs/releases/v* directory
#   6. the VERSION file
#
# Usage:
#   VERSION="$(bash scripts/gate/resolve_release_version.sh)"
#   VERSION="$(bash scripts/gate/resolve_release_version.sh develop/v4.1.0)"
#
# Prints the version on stdout (with the leading "v"), e.g. "v4.1.0".
# Exit codes: 0 = resolved, 1 = cannot resolve, 2 = usage error.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

usage() {
  cat >&2 <<'EOF'
Usage: resolve_release_version.sh [branch-or-version]

Prints the release version (e.g. "v4.1.0") this checkout belongs to.
EOF
}

# Extract "vX.Y.Z" from a ref-ish string, if it carries one.
version_from_ref() {
  local ref="$1" seg=""
  [ -n "$ref" ] || return 0
  seg="${ref##*/}"
  case "$seg" in
    v[0-9]*) printf '%s\n' "$seg" ;;
  esac
}

VERSION=""

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  "") ;;
  -*) echo "resolve_release_version: unknown option: $1" >&2; usage; exit 2 ;;
  *)  VERSION="$(version_from_ref "$1")" ;;
esac

if [ -z "$VERSION" ]; then
  VERSION="$(version_from_ref "${GITHUB_REF:-${GITEA_REF:-}}")"
fi
if [ -z "$VERSION" ]; then
  VERSION="$(version_from_ref "${GITHUB_REF_NAME:-${GITEA_REF_NAME:-}}")"
fi
if [ -z "$VERSION" ]; then
  BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
  VERSION="$(version_from_ref "$BRANCH")"
fi

# A branch-derived version is only usable when the release line it names
# actually exists in this tree. `develop/v4.1.0-hotfix` names no release
# directory, so it must not become the version.
if [ -n "$VERSION" ] && [ ! -d "docs/releases/$VERSION" ]; then
  echo "resolve_release_version: version '$VERSION' has no docs/releases directory; trying the next source" >&2
  VERSION=""
fi

if [ -z "$VERSION" ]; then
  latest="$(find docs/releases -maxdepth 1 -type d -name 'v[0-9]*' \
    -exec basename {} \; 2>/dev/null | sort -V | tail -1)"
  if [ -n "$latest" ] && [ -f "docs/releases/$latest/STAGE.yaml" ]; then
    VERSION="$(grep -m1 -E '^version:' "docs/releases/$latest/STAGE.yaml" \
      | sed -E 's/^version:[[:space:]]*"?([^"]*)"?[[:space:]]*$/\1/')"
    [ -n "$VERSION" ] || VERSION="$latest"
  elif [ -n "$latest" ]; then
    VERSION="$latest"
  fi
fi

if [ -z "$VERSION" ] && [ -f VERSION ]; then
  VERSION="$(grep -m1 -E '^v[0-9]' VERSION | tr -d '[:space:]' || true)"
fi

if [ -z "$VERSION" ]; then
  echo "resolve_release_version: cannot determine the release version of this checkout" >&2
  exit 1
fi

echo "$VERSION"
