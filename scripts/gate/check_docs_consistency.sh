#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

ERRORS=0

log_info() { echo "[INFO] $*"; }
log_error() { echo "[ERROR] $*" >&2; ERRORS=$((ERRORS + 1)); }
log_pass() { echo "[PASS] $*"; }
log_warn() { echo "[WARN] $*"; }

check_version_history_current() {
    log_info "CHECK 1: VERSION_HISTORY.md current version..."
    local vh_current
    vh_current=$(grep '^> \*\*当前版本' docs/releases/VERSION_HISTORY.md 2>/dev/null | \
        sed -E 's/.*v([0-9]+\.[0-9]+\.[0-9]+).*/\1/' | head -1 || true)
    local latest_ga_tag="3.7.0"
    if [[ "$vh_current" == "$latest_ga_tag" ]]; then
        log_pass "VERSION_HISTORY.md current: v$vh_current"
    else
        log_error "VERSION_HISTORY.md: current is v$vh_current, expected v$latest_ga_tag"
    fi
}

check_changelog_version_table() {
    log_info "CHECK 2: CHANGELOG.md version history table..."
    for changelog in docs/releases/v3.*/CHANGELOG.md; do
        [[ -e "$changelog" ]] || continue
        local version
        version=$(basename "$(dirname "$changelog")")
        if grep -q "| $version |" "$changelog" 2>/dev/null; then
            log_pass "$changelog: includes $version"
        else
            log_error "$changelog: missing $version entry"
        fi
    done
}

check_changelog_no_duplicates() {
    log_info "CHECK 3: CHANGELOG.md no duplicate commits..."
    for changelog in docs/releases/v3.*/CHANGELOG.md; do
        [[ -e "$changelog" ]] || continue
        local commits
        commits=$(grep -oE '`[0-9a-f]+`' "$changelog" 2>/dev/null | tr -d '`' | sort || true)
        if [[ -z "$commits" ]]; then continue; fi
        local duplicates
        duplicates=$(echo "$commits" | uniq -d | tr '\n' ' ' || true)
        if [[ -n "$duplicates" ]]; then
            log_error "$changelog: duplicate commits: $duplicates"
        else
            log_pass "$changelog: no duplicates"
        fi
    done
}

check_readme_exists() {
    log_info "CHECK 4: README.md existence for v3.5.0, v3.6.0..."
    for version in v3.5.0 v3.6.0; do
        local readme="docs/releases/$version/README.md"
        if [[ -e "$readme" ]]; then
            log_pass "$readme: EXISTS"
        else
            log_error "$readme: MISSING"
        fi
    done
}

check_docs_index_version_listing() {
    log_info "CHECK 5: docs/README.md version listing..."
    local first_version
    first_version=$(grep '### v' docs/README.md | head -1 | sed -E 's/.*### v([0-9]+\.[0-9]+\.[0-9]+).*/\1/' || true)
    local latest_ga_tag="3.7.0"

    if [[ -z "$first_version" ]]; then
        log_warn "Cannot determine first version in docs/README.md"
        return
    fi

    if [[ "$first_version" == "$latest_ga_tag" ]]; then
        log_pass "docs/README.md: first current version is v$first_version"
    else
        log_error "docs/README.md: first current is v$first_version, expected v$latest_ga_tag"
    fi
}

main() {
    echo "============================================"
    echo "SQLRustGo Documentation Consistency Check"
    echo "============================================"
    echo ""

    check_version_history_current
    check_changelog_version_table
    check_changelog_no_duplicates
    check_readme_exists
    check_docs_index_version_listing

    echo ""
    echo "============================================"
    if [[ $ERRORS -eq 0 ]]; then
        log_pass "All checks passed"
        exit 0
    else
        log_error "Failed with $ERRORS error(s)"
        exit 1
    fi
}

main "$@"
