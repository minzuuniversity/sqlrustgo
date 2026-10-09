#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

ERRORS=0

log_info() { echo "[INFO] $*"; }
log_error() { echo "[ERROR] $*" >&2; ERRORS=$((ERRORS + 1)); }
log_pass() { echo "[PASS] $*"; }
log_warn() { echo "[WARN] $*"; }

check_version_history_current() {
    log_info "CHECK 1: VERSION_HISTORY.md current version..."
    # The document tracks two different things on adjacent lines: the branch
    # under development ("当前版本", e.g. v4.0.0 while it is still being
    # built) and the newest shipped release ("最新 GA"). Only the latter is
    # comparable to a release tag — checking the development line against a
    # GA tag reports a mismatch for as long as the next version exists, which
    # is precisely the state this repo is in.
    local vh_ga
    vh_ga=$(grep '^> \*\*最新 GA' docs/releases/VERSION_HISTORY.md 2>/dev/null | \
        sed -E 's/.*v([0-9]+\.[0-9]+\.[0-9]+).*/\1/' | head -1 || true)
    if [[ -z "$vh_ga" ]]; then
        log_warn "VERSION_HISTORY.md has no '最新 GA' line; skipping CHECK 1"
        return 0
    fi
    # The expected version was hardcoded to "3.9.0" while the newest GA tag
    # had moved on, so this check failed no matter what the document said —
    # and the only way to silence it was to edit the doc backwards. Derive the
    # reference from the tags instead.
    local latest_ga_tag
    latest_ga_tag=$(git tag -l 'v*-ga' --sort=-v:refname 2>/dev/null | head -1 | \
        sed -E 's/^v([0-9]+\.[0-9]+\.[0-9]+).*/\1/')
    if [[ -z "$latest_ga_tag" ]]; then
        log_warn "no v*-ga tag found; skipping the VERSION_HISTORY comparison"
        return 0
    fi
    if [[ "$vh_ga" == "$latest_ga_tag" ]]; then
        log_pass "VERSION_HISTORY.md latest GA: v$vh_ga (matches tag)"
    else
        log_error "VERSION_HISTORY.md: latest GA is v$vh_ga, latest GA tag is v$latest_ga_tag"
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
        # A commit is legitimately cited many times: the GA tag line names it
        # in full, the evidence line abbreviates it, and the tag note
        # abbreviates it again. Those are references to one commit, not
        # repeated release entries, so counting occurrences reported every
        # well-formed CHANGELOG as broken. What actually matters is whether
        # one commit was shipped as two separate entries — so count how many
        # *distinct* places claim it, and normalise abbreviations to full
        # SHAs first so a full/abbreviated pair is not mistaken for two.
        local duplicates
        duplicates=$(python3 - "$changelog" <<'PYEOF'
import re
import subprocess
import sys

path = sys.argv[1]
with open(path, encoding="utf-8", errors="replace") as fh:
    lines = fh.read().split("\n")

full = {}
def resolve(sha):
    if sha in full:
        return full[sha]
    out = None
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--verify", "--quiet", sha + "^{commit}"],
            capture_output=True, text=True, timeout=10,
        )
    except Exception:
        out = None
    full[sha] = out.stdout.strip() if (out and out.returncode == 0 and out.stdout.strip()) else sha
    return full[sha]

# The defect this guards against is a commit shipped under two different
# versions: one of those releases then contains none of its own code.
# So the unit of comparison is the version section, not the line — a commit
# cited by the GA tag line, the evidence line and the tag note is one commit
# described three times, which is normal and must not be reported. Sections
# are delimited by markdown headings that name a version.
TICK = chr(96)  # backtick; spelled this way to keep the heredoc balanced
SHA_RE = re.compile(TICK + r"([0-9a-f]{7,40})" + TICK)
VERSION_HEADING_RE = re.compile(r"^#{1,6}\s+(?:\[)?v?[0-9]+\.[0-9]+\.[0-9]+")

sections = {}   # version -> {sha -> [line numbers]}
current = "(header)"
sections.setdefault(current, {})

for idx, line in enumerate(lines, 1):
    if VERSION_HEADING_RE.match(line):
        current = line.lstrip("# ").strip()[:40]
        sections.setdefault(current, {})
        continue
    for sha in SHA_RE.findall(line):
        sections[current].setdefault(resolve(sha), []).append(idx)

# Which version sections claim each commit.
claimants = {}
for version, shas in sections.items():
    for sha, where in shas.items():
        claimants.setdefault(sha, []).append((version, where))

dupes = []
for sha, claims in claimants.items():
    if len(claims) < 2:
        continue
    # One section listing the same commit on several lines is prose. The
    # defect needs the commit to appear under two *different* versions.
    versions = {v for v, _ in claims}
    if len(versions) > 1:
        dupes.append(sha[:12] + " in " + ", ".join(sorted(versions)))

print("; ".join(sorted(dupes)))
PYEOF
)
        if [[ -n "${duplicates// /}" ]]; then
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
    local latest_ga_tag="3.9.0"

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
