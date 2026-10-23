#!/usr/bin/env bash
# scripts/gate/check_storage_db_scope_coverage.sh — every database-scoped
# StorageEngine method must be overridden by every wrapper.
#
# Issue #5192: `StorageEngine` grew a family of `*_in_db` / `*_in` methods so
# a caller can name the database it means. Their trait defaults discard the
# `db` argument and answer for the *current* database:
#
#     fn get_table_info_in(&self, db: &str, table: &str) -> ... {
#         let _ = db;
#         self.get_table_info(table)
#     }
#
# A wrapper that does not override one inherits that behaviour, and read,
# write, existence checks and enumeration each resolve to a different
# database. Observed consequences: `CREATE TABLE` landing in `default`,
# `flush()` dropping the rows written to `aud`, and WAL replay skipping every
# entry because it validated the table against the wrong database.
#
# Nine separate defects in this family were found one at a time, each after
# a different investigation. This gate exists so the tenth is caught by CI
# rather than by an outage.
#
# Exit codes:
#   0 — every wrapper overrides every database-scoped method
#   1 — at least one wrapper is missing an override (listed on stderr)
#   2 — not inside a git repository / storage crate unreadable

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT" || exit 2

ENGINE="crates/storage/src/engine.rs"
[ -r "$ENGINE" ] || { echo "ERROR: cannot read $ENGINE" >&2; exit 2; }

# Wrappers only. A type that holds no inner engine (BinaryTableStorage) has
# nothing to forward to and implements these directly.
WRAPPERS="wal_storage mvcc_storage"

# Trait body = the text of the trait itself: from `pub trait StorageEngine`
# to the line that closes it (a `}` in column 0).
#
# #5401: the previous shape stopped at the first `impl ... StorageEngine for`,
# which is ~2000 lines further down — so `MemoryStorage`'s **inherent** impl
# was counted as part of the trait. The gate then demanded that the wrappers
# override `scan_via_bufferpool_in_db`, a method that is **not in the trait
# at all** (it is an inherent method of MemoryStorage) ⇒ the gate was red for
# ever, and no change to the wrappers could ever turn it green.
trait_body="$(awk '/^pub trait StorageEngine/{f=1} f{print} f && /^}$/{exit}' "$ENGINE")"

# Only methods that actually take a `db` parameter — `find_column_index_in`
# matches the name pattern but resolves column names, not databases.
# Extract method names whose signature actually takes a `db: &str`
# parameter. awk rather than sed: names contain no punctuation, but the
# capture spans parentheses, and sed would split on them.
required="$(printf '%s\n' "$trait_body" \
  | awk '{
      if (match($0, /fn [a-z_]+\((&(mut )?self, db: &str)/)) {
        s = substr($0, RSTART + 3, RLENGTH - 3)
        sub(/\(.*/, "", s)
        print s
      }
    }' | sort -u)"

if [ -z "$required" ]; then
    echo "ERROR: found no database-scoped methods in the trait — has the" >&2
    echo "       trait shape changed? Update this gate before trusting it." >&2
    exit 2
fi

fail=0
for w in $WRAPPERS; do
    f="crates/storage/src/$w.rs"
    [ -r "$f" ] || { echo "ERROR: cannot read $f" >&2; exit 2; }
    impl_body="$(awk "/^impl<.*StorageEngine for/,/^}/" "$f")"
    for m in $required; do
        if ! printf '%s\n' "$impl_body" | grep -q "fn $m("; then
            echo "FAIL: $w does not override \`$m\`" >&2
            echo "      the trait default answers for the current database." >&2
            fail=1
        fi
    done
done

if [ "$fail" -ne 0 ]; then
    echo >&2
    echo "Each of these must forward to \`inner()\` / \`inner_mut()\`." >&2
    echo "#5192 — see docs/releases/v4.1.0/ for the full record." >&2
    exit 1
fi

# #5401: the wrapper count was `printf '%s\n' "$WRAPPERS" | wc -l` — that counts
# the LINES of a single space-separated string, so it always said "1 wrappers"
# even though two wrappers are checked.
echo "PASS: $(echo $WRAPPERS | wc -w | tr -d ' ') wrappers override all $(printf '%s\n' "$required" | wc -l | tr -d ' ') database-scoped methods"
