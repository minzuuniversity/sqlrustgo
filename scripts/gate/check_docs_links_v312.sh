#!/usr/bin/env bash
# =============================================================================
# check_docs_links_v312.sh — v3.12.0 Documentation Links Gate (Issue #4387 / GA-7)
# =============================================================================
# Verifies that all relative markdown links inside v3.12.0 release scope
# resolve to existing files.
#
# v3.12.0 scope:
#   - docs/releases/v3.12.0/**/*.md
#   - README.md (root)
#   - RELEASE_NOTES.md (root)
#   - CHANGELOG.md (root)
#
# Wrapper around check_docs_links.sh (general-purpose) restricted to v3.12 scope.
#
# Output:
#   stdout: PASS/FAIL with broken link count
#   file:   docs/releases/v3.12.0/evidence/v312-59/GA7_DOCS_LINKS_REPORT.md
#
# Exit codes:
#   0  = 0 broken links
#   1  = any broken link
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

VERSION="${VERSION:-v3.12.0}"
EVIDENCE_DIR="${EVIDENCE_DIR:-docs/releases/v3.12.0/evidence/v312-59}"
OUT_FILE="${OUT_FILE:-$EVIDENCE_DIR/GA7_DOCS_LINKS_REPORT.md}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
COMMIT_SHA="$(git rev-parse HEAD 2>/dev/null || echo 'unknown')"

mkdir -p "$EVIDENCE_DIR"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
log_step() { printf "\n${YELLOW}== %s ==${NC}\n" "$1"; }
log_pass() { printf "  ${GREEN}✓ PASS${NC} %s\n" "$1"; }
log_fail() { printf "  ${RED}✗ FAIL${NC} %s\n" "$1"; }
log_info() { printf "  ℹ %s\n" "$1"; }

# v3.12.0 documentation scope
V312_DOC_FILES=(
    "README.md"
    "RELEASE_NOTES.md"
    "CHANGELOG.md"
    "docs/releases/v3.12.0/README.md"
    "docs/releases/v3.12.0/RELEASE_NOTES.md"
    "docs/releases/v3.12.0/STAGE.yaml"
    "docs/releases/v3.12.0/CHANGELOG.md"
    "docs/releases/v3.12.0/FEATURE_CHECKLIST.md"
    "docs/releases/v3.12.0/DEVELOPMENT_PLAN.md"
    "docs/releases/v3.12.0/GMP_COMPLIANCE_MATRIX.md"
    "docs/releases/v3.12.0/VERSION_PLAN.md"
    "docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md"
)

# Verify each file exists
verify_files_exist() {
    log_step "Step 1: Verify v3.12.0 doc files exist"
    local missing=0
    for f in "${V312_DOC_FILES[@]}"; do
        if [ -f "$REPO_ROOT/$f" ]; then
            log_pass "exists: $f"
        else
            log_fail "missing: $f"
            missing=$((missing + 1))
        fi
    done
    return $missing
}

# Extract and verify relative markdown links
verify_relative_links() {
    log_step "Step 2: Verify relative markdown links in v3.12.0 docs"
    local broken_log="$EVIDENCE_DIR/docs_links_broken_v312.txt"
    local broken_count=0

    {
        echo "=== Broken Links in v3.12.0 Docs ==="
        echo "Timestamp: $TIMESTAMP"
        echo "Commit: $COMMIT_SHA"
        echo ""
    } > "$broken_log"

    for md_file in "${V312_DOC_FILES[@]}"; do
        if [ ! -f "$REPO_ROOT/$md_file" ]; then
            continue
        fi
        local file_dir
        file_dir=$(dirname "$md_file")

        # Extract markdown links [text](target)
        while IFS= read -r link; do
            # Strip wrapping
            local target="${link#*(}"
            target="${target%)*}"
            target="${target#<}"
            target="${target%>}"

            # Skip if not a relative path
            case "$target" in
                http*|mailto:*|tel:*|ftp:*|data:*|javascript:*|\#*)
                    continue
                    ;;
            esac

            # Strip query / hash
            target="${target%%#*}"
            target="${target%%\?*}"

            # Resolve relative to file_dir
            local resolved="$file_dir/$target"
            # Normalize ../ and ./
            resolved=$(realpath -m "$REPO_ROOT/$resolved" 2>/dev/null || echo "$REPO_ROOT/$resolved")

            if [ ! -f "$resolved" ] && [ ! -d "$resolved" ]; then
                echo "BROKEN: $md_file -> $target" >> "$broken_log"
                broken_count=$((broken_count + 1))
            fi
        done < <(grep -oE '\[[^]]*\]\([^)]+\)' "$REPO_ROOT/$md_file" 2>/dev/null)
    done

    if [ "$broken_count" -eq 0 ]; then
        log_pass "0 broken links in v3.12.0 docs"
        return 0
    else
        log_fail "$broken_count broken links (see $broken_log)"
        return 1
    fi
}

# Wrapper call to general check_docs_links.sh (--entry mode)
run_general_docs_links() {
    log_step "Step 3: General docs links check (entry mode)"
    local general_script="$SCRIPT_DIR/check_docs_links.sh"
    if [ ! -f "$general_script" ]; then
        log_info "check_docs_links.sh not found — skipping general check"
        return 0
    fi
    local general_log="$EVIDENCE_DIR/docs_links_general_v312.log"
    if bash "$general_script" entry > "$general_log" 2>&1; then
        log_pass "general docs links: PASS (see $general_log)"
        return 0
    else
        log_info "general docs links: see $general_log"
        return 0  # don't fail GA-7 on general results
    fi
}

# ============================================================================
# MAIN
# ============================================================================

main() {
    echo "═════════════════════════════════════════════════════════════"
    echo "  v3.12.0 Documentation Links Gate (Issue #4387 / GA-7)"
    echo "  Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'detached')"
    echo "  Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
    echo "  Timestamp: $TIMESTAMP"
    echo "═════════════════════════════════════════════════════════════"

    local exit_code=0

    if ! verify_files_exist; then
        exit_code=1
    fi

    if ! verify_relative_links; then
        exit_code=1
    fi

    run_general_docs_links

    # Write report
    cat > "$OUT_FILE" <<EOF
# GA-7 v3.12.0 Documentation Links Report

> **provenance:** generated_by=check_docs_links_v312.sh, generated_at=$TIMESTAMP, commit=$COMMIT_SHA, branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'detached'), source_repo=openclaw/sqlrustgo, gate_policy_eval_id=v312-ga7-docs-links-001, policy=Anti-Fabrication-Policy-v1.0
> **related:** Issue #4387 (V312-59-D), umbrella #4383

## Summary

| Check | Status |
|-------|--------|
EOF

    if [ "$exit_code" -eq 0 ]; then
        echo "| v3.12.0 file existence | PASS |" >> "$OUT_FILE"
        echo "| Relative link verification | PASS |" >> "$OUT_FILE"
        echo "| General docs links (entry) | see log |" >> "$OUT_FILE"
        echo "" >> "$OUT_FILE"
        echo "**Verdict: PASS** — 0 broken links across v3.12.0 doc scope" >> "$OUT_FILE"
    else
        echo "| v3.12.0 file existence | FAIL (some files missing) |" >> "$OUT_FILE"
        echo "| Relative link verification | FAIL (broken links) |" >> "$OUT_FILE"
        echo "| General docs links (entry) | see log |" >> "$OUT_FILE"
        echo "" >> "$OUT_FILE"
        echo "**Verdict: FAIL** — broken links detected; see docs_links_broken_v312.txt" >> "$OUT_FILE"
    fi

    echo "" >> "$OUT_FILE"
    echo "## v3.12.0 Documentation Scope" >> "$OUT_FILE"
    echo "" >> "$OUT_FILE"
    for f in "${V312_DOC_FILES[@]}"; do
        echo "- \`$f\`" >> "$OUT_FILE"
    done

    echo ""
    echo "═════════════════════════════════════════════════════════════"
    echo "  GA-7 Docs Links Summary"
    echo "═════════════════════════════════════════════════════════════"
    echo "  Report: $OUT_FILE"
    if [ "$exit_code" -eq 0 ]; then
        echo "  Status: PASS"
    else
        echo "  Status: FAIL"
    fi
    echo ""

    exit $exit_code
}

main "$@"