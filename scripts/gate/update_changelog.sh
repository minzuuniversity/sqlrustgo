#!/usr/bin/env bash
# scripts/gate/update_changelog.sh
# Update CHANGELOG.md from conventional commits since a given tag.
# Works on bash 3.2 (macOS) and bash 5+ (Linux) — uses temp files
# instead of associative arrays for portability.
#
# Usage:
#   bash scripts/gate/update_changelog.sh [VERSION] [SINCE_TAG]
#   VERSION     e.g. v3.8.0
#   SINCE_TAG   e.g. v3.7.0  (defaults to the latest tag)
#
# Conventional commit prefixes: feat, fix, perf, refactor, docs, test, chore

set -eo pipefail
shopt -s nullglob 2>/dev/null || true

VERSION="${1:-v3.8.0}"
SINCE_TAG="${2:-}"
TODAY=$(date +%Y-%m-%d)
CHANGELOG="CHANGELOG.md"
WORKDIR=$(mktemp -d)
trap 'rm -rf "$WORKDIR"' EXIT

if [[ -z "$SINCE_TAG" ]]; then
    SINCE_TAG=$(git tag --list 'v*' --sort=-v:refname | head -n 1 || true)
fi

if [[ -z "$SINCE_TAG" ]]; then
    echo "No SINCE_TAG provided and no git tags found; pass SINCE_TAG explicitly." >&2
    exit 1
fi

# Bucket files
for key in feat fix perf refactor docs test chore other; do
    : > "$WORKDIR/$key"
done

while IFS='|' read -r hash subject; do
    [[ -z "$hash" ]] && continue
    prefix=$(echo "$subject" | grep -oE '^[a-z]+(\([^)]+\))?' | sed 's/(.*//' | head -n 1 || true)
    case "$prefix" in
        feat|fix|perf|refactor|docs|test|chore) bucket="$prefix" ;;
        *) bucket="other" ;;
    esac
    echo "- $subject ($hash)" >> "$WORKDIR/$bucket"
done < <(git log "${SINCE_TAG}..HEAD" --pretty=format:"%h|%s" 2>/dev/null || true)

NEW_SECTION="## [$VERSION] - $TODAY"
NEW_SECTION+=$'\n\n'

emit_section() {
    local header="$1"
    local bucket="$2"
    if [[ -s "$WORKDIR/$bucket" ]]; then
        NEW_SECTION+="$header"
        NEW_SECTION+=$'\n\n'
        NEW_SECTION+="$(cat "$WORKDIR/$bucket")"
        NEW_SECTION+=$'\n'
    fi
}

emit_section "### Added"          "feat"
emit_section "### Fixed"          "fix"
emit_section "### Performance"    "perf"
emit_section "### Changed"        "refactor"
emit_section "### Documentation"  "docs"
emit_section "### Tests"          "test"
emit_section "### Maintenance"    "chore"
emit_section "### Other"          "other"

if [[ ! -f "$CHANGELOG" ]]; then
    echo "# Changelog" > "$CHANGELOG"
fi

TMP=$(mktemp)
HEADER_END=$(grep -n "^## \[" "$CHANGELOG" 2>/dev/null | head -n 1 | cut -d: -f1 || true)
if [[ -z "$HEADER_END" ]]; then
    echo "" >> "$CHANGELOG"
    printf "%b" "$NEW_SECTION" >> "$CHANGELOG"
else
    head -n $((HEADER_END - 1)) "$CHANGELOG" > "$TMP"
    printf "%b" "$NEW_SECTION" >> "$TMP"
    tail -n +$((HEADER_END)) "$CHANGELOG" >> "$TMP"
    mv "$TMP" "$CHANGELOG"
fi

echo "Updated $CHANGELOG with section [$VERSION]"
