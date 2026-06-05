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

check_v380_mandatory_docs() {
    log_info "CHECK 6: v3.8.0 mandatory documents (11 items)..."
    local version_dir="docs/releases/v3.8.0"
    local required_docs=(
        "README.md"
        "CHANGELOG.md"
        "RELEASE_NOTES.md"
        "MIGRATION_GUIDE.md"
        "DEPLOYMENT_GUIDE.md"
        "DEVELOPMENT_GUIDE.md"
        "TEST_MANUAL.md"
        "FEATURE_MATRIX.md"
        "SECURITY_POLICY.md"
        "COMPATIBILITY_MATRIX.md"
        "SUPPORT_MATRIX.md"
    )

    local missing=0
    for doc in "${required_docs[@]}"; do
        if [[ -e "$version_dir/$doc" ]]; then
            if [[ -s "$version_dir/$doc" ]]; then
                log_pass "$version_dir/$doc: exists and non-empty"
            else
                log_error "$version_dir/$doc: EXISTS BUT EMPTY"
                missing=$((missing + 1))
            fi
        else
            log_error "$version_dir/$doc: MISSING"
            missing=$((missing + 1))
        fi
    done

    if [[ $missing -gt 0 ]]; then
        log_error "v3.8.0 mandatory docs: $missing/${#required_docs[@]} missing or empty"
    else
        log_pass "v3.8.0 mandatory docs: all ${#required_docs[@]} present"
    fi
}

check_v380_feature_matrix() {
    log_info "CHECK 7: v3.8.0 FEATURE_MATRIX.md feature count >= 50..."
    local fm="$version_dir/FEATURE_MATRIX.md"
    if [[ ! -e "$fm" ]]; then
        log_error "FEATURE_MATRIX.md: MISSING"
        return
    fi

    local count
    count=$(grep -cE "^\\| .+ \\| (✅|⚠️|❌)" "$fm" 2>/dev/null || echo 0)
    if [[ "$count" -ge 50 ]]; then
        log_pass "FEATURE_MATRIX.md: $count features (>= 50)"
    else
        log_error "FEATURE_MATRIX.md: only $count features (expected >= 50)"
    fi
}

check_v380_security_policy() {
    log_info "CHECK 8: v3.8.0 SECURITY_POLICY.md has known limitations section..."
    local sp="$version_dir/SECURITY_POLICY.md"
    if [[ ! -e "$sp" ]]; then
        log_error "SECURITY_POLICY.md: MISSING"
        return
    fi
    if grep -qE "(SEC-|known|Known|LIMITATION|limitation)" "$sp" 2>/dev/null; then
        log_pass "SECURITY_POLICY.md: has known limitations section"
    else
        log_warn "SECURITY_POLICY.md: no known limitations section found"
    fi
}

main() {
    echo "============================================"
    echo "SQLRustGo Documentation Consistency Check"
    echo "============================================"
    echo ""

    # version_dir is passed as first arg or defaults
    local version_dir="${1:-docs/releases/v3.8.0}"

    check_version_history_current
    check_changelog_version_table
    check_changelog_no_duplicates
    check_readme_exists
    check_docs_index_version_listing
    check_v380_mandatory_docs "$version_dir"
    check_v380_feature_matrix "$version_dir"
    check_v380_security_policy "$version_dir"

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
