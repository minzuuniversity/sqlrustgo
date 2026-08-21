#!/usr/bin/env bash
# V312-57 smoke lib/sqlite_oracle.sh — SQLite oracle comparison helper.
#
# Provides `oracle_run <sql_file> <db_path>` which runs the same SQL on
# sqlrustgo (binary from $SQLRUSTGO_BIN) and on the system sqlite3 binary,
# then compares the multiset of CSV data rows (order-insensitive) and
# reports PASS/FAIL with diff.
#
# Used by week06/* fixtures to gate JOIN / GROUP BY / aggregate behavior
# against an independently-implemented reference (SQLite 3.51+).
#
# Exit codes:
#   0  - row multisets match (oracle PASS)
#   1  - row multisets differ (oracle FAIL)
#   2  - sqlite3 binary missing (oracle BLOCKED)

set -uo pipefail

SQLITE_BIN="${SQLITE_BIN:-/usr/bin/sqlite3}"

# oracle_run <sql_file> <db_path> [<mode>]
#   <mode> defaults to csv (header row is dropped before diff)
oracle_run() {
    local sql_file="$1"
    local db_path="$2"
    local mode="${3:-csv}"

    if [ ! -x "$SQLITE_BIN" ]; then
        echo "ORACLE_BLOCKED: sqlite3 binary not found at $SQLITE_BIN"
        return 2
    fi

    # sqlrustgo side: force --headers true so the output starts with a
    # header row, mirroring sqlite3's -header flag. Without this, sqlrustgo
    # omits the header and `tail -n +2` would silently drop the first
    # data row.
    local ours
    ours=$("$SQLRUSTGO_BIN" sqlite --batch --mode "$mode" --headers true "$db_path" < "$sql_file" 2>&1) || true

    # sqlite3 side: use -csv flag for comma-separated output AND -header
    # so both sides have a leading header line that gets dropped during
    # normalization. (Default sqlite3 mode uses `|` separator, which
    # would produce a meaningless diff.)
    local theirs
    theirs=$(cat "$sql_file" | "$SQLITE_BIN" -csv -header :memory: 2>&1) || true

    # Normalize: drop empty lines, strip the header (line 1) on both
    # sides uniformly, sort for multiset comparison.
    local ours_norm theirs_norm
    ours_norm=$(echo "$ours" | grep -v '^$' | tail -n +2 | LC_ALL=C sort)
    theirs_norm=$(echo "$theirs" | grep -v '^$' | tail -n +2 | LC_ALL=C sort)

    local diff_out
    diff_out=$(diff <(echo "$ours_norm") <(echo "$theirs_norm") || true)
    if [ -z "$diff_out" ]; then
        return 0
    fi
    echo "ORACLE_FAIL: row multisets differ"
    echo "--- diff (ours < theirs) ---"
    echo "$diff_out" | head -40
    echo "--- sqlrustgo raw (first 5 rows) ---"
    echo "$ours" | grep -v '^$' | head -5
    echo "--- sqlite3 raw (first 5 rows) ---"
    echo "$theirs" | grep -v '^$' | head -5
    return 1
}

# oracle_scalar <sql_file> <db_path>
#   Convenience wrapper for single-value SELECT results (e.g. COUNT(*)).
oracle_scalar() {
    local sql_file="$1"
    local db_path="$2"
    oracle_run "$sql_file" "$db_path" csv
}
