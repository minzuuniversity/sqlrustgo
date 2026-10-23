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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/gate/common.sh
source "$SCRIPT_DIR/common.sh"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT" || exit 2

# --- selftest: does this gate actually catch a missing override? -------------
#
# #5401's acceptance criterion 3: "一个新增 `db: &str` trait 方法的变异能被
# 该门禁抓住（有变异证据，不是靠人读代码断言）".
#
# The mutation is run against a **copy** of the tree in a temporary directory:
# the working tree is never modified, so the selftest is safe to run in CI
# next to the real check.
#
# Three arms:
#
#   A. baseline   — the tree as it is; the gate must pass.
#   B. mutated    — a new `db: &str` method on the trait, overridden by
#                   **neither** wrapper; the gate must fail and must name
#                   the method.
#   C. remediated — the same new method, overridden by **both** wrappers;
#                   the gate must pass again.
#
# Arm C is what makes arm B meaningful: a gate that fails for any reason
# whatsoever would also "catch" the mutation, and would also fail arm C.
run_selftest() {
    local rc
    local fails=0
    local probe="gate_selftest_probe"
    local probe_sig="fn ${probe}(&self, db: &str) -> bool { let _ = db; true }"

    # The scratch directory is a **global**, not a `local`: the EXIT trap
    # below runs after this function has returned, and under `set -u` a trap
    # that names an out-of-scope local aborts the script with
    # "tmp: unbound variable" — the selftest would then report a failure it
    # never measured.
    SELFTEST_TMP="$(make_temp_dir)" || return 1
    # Cleanup follows the repository convention (see
    # scripts/gate/check_docs_pointer_paths.sh): a trap that removes the
    # scratch directory this script created with `mktemp -d` a moment ago.
    # The guard keeps the removal inside the system temp dir, so this script
    # can only ever delete something it created itself.
    trap 'case "${SELFTEST_TMP:-}" in "${TMPDIR:-/tmp}"/*) rm -rf "$SELFTEST_TMP" ;; esac' EXIT

    mkdir -p "$SELFTEST_TMP/scripts/gate" "$SELFTEST_TMP/crates/storage/src" || return 1
    cp "$SCRIPT_DIR/check_storage_db_scope_coverage.sh" "$SELFTEST_TMP/scripts/gate/" || return 1
    cp "$SCRIPT_DIR/common.sh" "$SELFTEST_TMP/scripts/gate/" || return 1
    cp crates/storage/src/engine.rs "$SELFTEST_TMP/crates/storage/src/" || return 1
    cp crates/storage/src/wal_storage.rs "$SELFTEST_TMP/crates/storage/src/" || return 1
    cp crates/storage/src/mvcc_storage.rs "$SELFTEST_TMP/crates/storage/src/" || return 1

    # --- arm A: baseline ---------------------------------------------------
    ( cd "$SELFTEST_TMP" && bash scripts/gate/check_storage_db_scope_coverage.sh ) >/dev/null 2>&1
    rc=$?
    if [ "$rc" -ne 0 ]; then
        echo "SELFTEST FAIL: baseline tree must pass the gate (rc=$rc)"
        fails=$((fails + 1))
    fi

    # --- arm B: mutation ---------------------------------------------------
    # A new database-scoped method on the trait, overridden by nobody.
    awk -v sig="$probe_sig" \
        '/^pub trait StorageEngine/{print; print "    " sig; next} {print}' \
        "$SELFTEST_TMP/crates/storage/src/engine.rs" > "$SELFTEST_TMP/crates/storage/src/engine.rs.new" || return 1
    mv "$SELFTEST_TMP/crates/storage/src/engine.rs.new" "$SELFTEST_TMP/crates/storage/src/engine.rs"

    local out
    out="$(cd "$SELFTEST_TMP" && bash scripts/gate/check_storage_db_scope_coverage.sh 2>&1)"
    rc=$?
    if [ "$rc" -eq 0 ]; then
        echo "SELFTEST FAIL: a trait method that no wrapper overrides must fail the gate"
        fails=$((fails + 1))
    fi
    if ! gate_text_matches_regex "$out" "$probe"; then
        echo "SELFTEST FAIL: the gate must name the missing method '$probe'"
        fails=$((fails + 1))
    fi

    # --- arm C: remediation ------------------------------------------------
    # The same method, overridden by BOTH wrappers.
    local w
    for w in wal_storage mvcc_storage; do
        awk -v sig="$probe_sig" \
            '/^impl<.*StorageEngine for/{print; print "    " sig; next} {print}' \
            "$SELFTEST_TMP/crates/storage/src/$w.rs" > "$SELFTEST_TMP/crates/storage/src/$w.rs.new" || return 1
        mv "$SELFTEST_TMP/crates/storage/src/$w.rs.new" "$SELFTEST_TMP/crates/storage/src/$w.rs"
    done

    out="$(cd "$SELFTEST_TMP" && bash scripts/gate/check_storage_db_scope_coverage.sh 2>&1)"
    rc=$?
    if [ "$rc" -ne 0 ]; then
        echo "SELFTEST FAIL: a trait method overridden by both wrappers must pass the gate"
        printf '%s\n' "$out" | head -20
        fails=$((fails + 1))
    fi

    # --- the text predicates must also be stable ----------------------------
    if ! gate_text_helpers_selftest; then
        fails=$((fails + 1))
    fi

    if [ "$fails" -eq 0 ]; then
        echo "SELFTEST: PASS (baseline green, mutation caught, remediation green, text predicates stable)"
    else
        echo "SELFTEST: $fails failure(s)"
    fi
    return "$fails"
}

if [ "${1:-}" = "--selftest" ]; then
    run_selftest
    exit $?
fi

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
        # #5401: this used to be `printf '%s\n' "$impl_body" | grep -q "fn $m("`.
        #
        # `grep -q` exits at the first match and closes the read end of the
        # pipe; the writer — which is still pushing a ~50 KB Rust impl block —
        # then takes SIGPIPE and dies with 141. Under `set -o pipefail` the
        # **pipeline** is reported as failed, so a method that *was* found was
        # reported as missing.
        #
        # Measured: 16 methods x 2 wrappers x 60 iterations = 1920 samples,
        # 32 of them returned 141, and **all 32 were false failures** (the
        # method was present — cross-checked against a pipe-free reference).
        # The whole gate therefore failed in roughly 42% of its runs, with a
        # different method each time — a red nobody could reproduce.
        #
        # `gate_text_contains_literal` asks the same question with no pipe at
        # all, so there is no writer left to be killed. One rule, one
        # implementation (scripts/gate/common.sh).
        if ! gate_text_contains_literal "$impl_body" "fn $m("; then
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
