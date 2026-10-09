#!/usr/bin/env bash
# scripts/gate/check_path_casefold_conflict.sh — reject tracked paths that
# differ only by case.
#
# Issue #5197: `docs/releases/v2.9.0/` tracked both `OPencode_STARTUP.md`
# and `OPENCODE_STARTUP.md` with byte-identical content. The duplicate came
# from a `post-resync cleanup` merge, not from authorship.
#
# Why this must be a gate rather than a one-off cleanup: on a
# case-insensitive filesystem (APFS, NTFS) the two names are the *same*
# file. `git checkout` then either errors or silently lets one overwrite
# the other, so a contributor on macOS or Windows cannot reliably get a
# working tree at all. Linux hides it — which is why it survived.
#
# The damage is invisible on the CI filesystem and fatal on a developer's.
# That asymmetry is exactly what a gate is for.
#
# Detection is over `git ls-files` (the index), not the working tree:
# on a case-insensitive volume the working tree has already lost one of
# the two by the time you look, so scanning the filesystem finds nothing
# to complain about.
#
# Exit codes:
#   0 — no casefold conflict
#   1 — at least one conflict (listed on stderr)
#   2 — not inside a git repository

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT" || exit 2

if ! git rev-parse --git-dir >/dev/null 2>&1; then
    echo "ERROR: not inside a git repository" >&2
    exit 2
fi

conflicts="$(
    git ls-files \
        | awk '{ key = tolower($0); print key "\t" $0 }' \
        | LC_ALL=C sort \
        | awk -F'\t' '
            $1 == prev_key && $2 != prev_path {
                if (!reported) { print prev_path; reported = 1 }
                print $2
            }
            { prev_key = $1; prev_path = $2 }
        '
)"

if [ -n "$conflicts" ]; then
    echo "FAIL: tracked paths that differ only by case." >&2
    echo "      On macOS/Windows these are one file, so checkout breaks" >&2
    echo "      there while passing on Linux. Keep the intended name:" >&2
    echo >&2
    echo "$conflicts" | sed 's/^/        /' >&2
    echo >&2
    echo "      git rm the duplicate, then re-checkout the survivor — on a" >&2
    echo "      case-insensitive volume deleting one removes both." >&2
    exit 1
fi

echo "PASS: no tracked paths differ only by case ($(git ls-files | wc -l | tr -d ' ') paths checked)"