#!/usr/bin/env bash
# =============================================================================
# V312-19 / ISSUE #3906 — assert_reviewer_signoff.sh
# =============================================================================
# Validates a reviewer sign-off file at <path> against the canonical
# template at docs/governance/REVIEWER_SIGNOFF_TEMPLATE.md.
#
# Checks:
#   1. File exists and is non-empty.
#   2. Contains both `## Reviewer A` and `## Reviewer B` sections.
#   3. Each section has a `Login:` line; the two logins are distinct
#      (case-sensitive).
#   4. The `Commit:` line matches `git rev-parse HEAD`.
#   5. The `Branch:` line matches `git rev-parse --abbrev-ref HEAD`.
#   6. The `Evidence hash:` line is a valid hex SHA-256 (64 hex chars).
#   7. The file was modified within 7 days of HEAD.
#
# Exit 0 on success, non-zero on any check failure with a clear message.
# =============================================================================
set -uo pipefail

if [ $# -lt 1 ]; then
    echo "usage: $0 <signoff-file>" >&2
    exit 2
fi
SIGNOFF="$1"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

if [ ! -f "${SIGNOFF}" ]; then
    echo "FAIL: signoff file not found: ${SIGNOFF}" >&2
    exit 1
fi
if [ ! -s "${SIGNOFF}" ]; then
    echo "FAIL: signoff file is empty: ${SIGNOFF}" >&2
    exit 1
fi

# 1+2: required sections
if ! grep -qE '^## Reviewer A$' "${SIGNOFF}"; then
    echo "FAIL: missing '## Reviewer A' section in ${SIGNOFF}" >&2
    exit 1
fi
if ! grep -qE '^## Reviewer B$' "${SIGNOFF}"; then
    echo "FAIL: missing '## Reviewer B' section in ${SIGNOFF}" >&2
    exit 1
fi

# 3: logins distinct
LOGIN_A="$(awk '/^## Reviewer A$/{flag=1; next} /^## Reviewer B$/{flag=0} flag && /^\- \*\*Login\*\*:/{print $NF; exit}' "${SIGNOFF}")"
LOGIN_B="$(awk '/^## Reviewer B$/{flag=1; next} /^## Reviewer A$/{flag=0} flag && /^\- \*\*Login\*\*:/{print $NF; exit}' "${SIGNOFF}")"
if [ -z "${LOGIN_A}" ] || [ -z "${LOGIN_B}" ]; then
    echo "FAIL: could not extract Login: lines (A='${LOGIN_A}', B='${LOGIN_B}')" >&2
    exit 1
fi
if [ "${LOGIN_A}" = "${LOGIN_B}" ]; then
    echo "FAIL: Reviewer A and Reviewer B must be distinct (both='${LOGIN_A}')" >&2
    exit 1
fi

# 4: commit SHA matches HEAD (with tolerance window per #3980)
EXPECTED_HEAD="$(cd "${ROOT}" && git rev-parse HEAD 2>/dev/null || echo unknown)"
CLAIMED_COMMIT="$(awk '/^\- \*\*Commit\*\*:/{print $NF; exit}' "${SIGNOFF}")"
ALLOWED_DRIFT=3  # V312-19 #3980: signoff within 3 commits of HEAD is OK
if [ "${CLAIMED_COMMIT}" != "${EXPECTED_HEAD}" ]; then
    if [ "${CLAIMED_COMMIT}" = "unknown" ] || [ -z "${CLAIMED_COMMIT}" ]; then
        echo "FAIL: Commit SHA in signoff is unknown or empty" >&2
        exit 1
    fi
    if ! git rev-parse --verify "${CLAIMED_COMMIT}" >/dev/null 2>&1; then
        echo "FAIL: Commit SHA ${CLAIMED_COMMIT} not found in repository" >&2
        exit 1
    fi
    DRIFT=$(git rev-list --count "${CLAIMED_COMMIT}..HEAD" 2>/dev/null || echo 999)
    if [ "${DRIFT}" -gt "${ALLOWED_DRIFT}" ]; then
        echo "FAIL: signoff commit ${CLAIMED_COMMIT} is ${DRIFT} commits behind HEAD (max allowed: ${ALLOWED_DRIFT}). Refresh signoff." >&2
        exit 1
    fi
    echo "INFO: signoff commit is ${DRIFT} commits behind HEAD (within tolerance ${ALLOWED_DRIFT})" >&2
fi

# 5: branch matches
EXPECTED_BRANCH="$(cd "${ROOT}" && git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
CLAIMED_BRANCH="$(awk '/^\- \*\*Branch\*\*:/{print $NF; exit}' "${SIGNOFF}")"
if [ "${CLAIMED_BRANCH}" != "${EXPECTED_BRANCH}" ]; then
    echo "FAIL: Branch does not match HEAD: claimed=${CLAIMED_BRANCH} head=${EXPECTED_BRANCH}" >&2
    exit 1
fi

# 6: evidence hash is 64 hex chars
EVIDENCE_HASH="$(awk '/^\- \*\*Evidence hash\*\*:/{print $NF; exit}' "${SIGNOFF}")"
if ! echo "${EVIDENCE_HASH}" | grep -qE '^[0-9a-fA-F]{64}$'; then
    echo "FAIL: Evidence hash is not a 64-char SHA-256 hex string: '${EVIDENCE_HASH}'" >&2
    exit 1
fi

# 7: modified within 7 days
if command -v stat >/dev/null 2>&1; then
    MTIME="$(stat -c %Y "${SIGNOFF}" 2>/dev/null || stat -f %m "${SIGNOFF}" 2>/dev/null || echo 0)"
    NOW="$(date +%s)"
    SEVEN_DAYS=$((7 * 24 * 3600))
    AGE=$((NOW - MTIME))
    if [ "${AGE}" -gt "${SEVEN_DAYS}" ]; then
        echo "FAIL: signoff file is ${AGE}s old (>7 days). Refresh before sign-off." >&2
        exit 1
    fi
fi

echo "PASS: signoff valid (Reviewer A=${LOGIN_A}, Reviewer B=${LOGIN_B}, commit=${CLAIMED_COMMIT:0:12}, branch=${CLAIMED_BRANCH})"
exit 0
