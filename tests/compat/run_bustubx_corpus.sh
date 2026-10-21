#!/usr/bin/env bash
# BustubX-EDU B-Track Corpus Runner
#
# Runs the SQL corpus as differential tests against SQLite.
# Usage: ./run_bustustx_corpus.sh [sqlite_bin] [sqlrustgo_bin]
#
# Exit codes:
#   0  - All tests passed
#   1  - Some tests failed
#   2  - Usage error (corpus / sqlite3 missing)
#   77 - NOT RUN (the engine binary could not be located) — disclosed, never
#        counted as "passed" (#5406)

set -uo pipefail

# #5406 / #5477: the engine binary was located through a hardcoded path
# inside one developer's home directory. That path exists on exactly one
# machine, so everywhere else this runner printed "SKIP" and exited 0 —
# a caller reads that as "the corpus passed" while nothing was compared.
#
# Now: explicit env var -> CARGO_TARGET_DIR -> repo target/ -> PATH, and a
# missing binary is disclosed as NOT-RUN with exit code 77 (not 0).
source "$(dirname "$0")/../common/resolve_bin.sh"

SQLITE_BIN="${1:-${SQLITE_BIN:-sqlite3}}"
SQLRUSTGO_BIN="${2:-}"
CORPUS="$(dirname "$0")/bustubx_b_track_corpus.sql"

if [ -z "$SQLRUSTGO_BIN" ]; then
    SQLRUSTGO_BIN="$(resolve_sqlrustgo_bin "run_bustubx_corpus")" || exit $?
fi

if ! command -v "$SQLITE_BIN" >/dev/null 2>&1 && [ ! -x "$SQLITE_BIN" ]; then
    echo "ERROR: sqlite3 not found at $SQLITE_BIN"
    exit 2
fi

if [ ! -f "$CORPUS" ]; then
    echo "ERROR: Corpus not found at $CORPUS"
    exit 2
fi

echo "Running BustubX-EDU B-Track Corpus"
echo "  SQLite: $SQLITE_BIN"
echo "  SQLRustGo: $SQLRUSTGO_BIN"
echo "  Corpus: $CORPUS"
echo ""

PASS=0
FAIL=0
SKIP=0

# Parse corpus into individual test cases
# Each test starts with "-- TEST:" and ends before the next "-- TEST:" or EOF
current_test=""
current_sql=""
expected=""

run_test() {
    local test_id="$1"
    local sql="$2"
    local expected="$3"

    if [ -z "$sql" ]; then
        echo "  SKIP: $test_id (no SQL)"
        SKIP=$((SKIP + 1))
        return
    fi

    # Run on SQLite
    local sqlite_result
    sqlite_result=$(echo "$sql" | "$SQLITE_BIN" -csv -header :memory: 2>&1) || true

    # Run on SQLRustGo
    local sqlrustgo_result
    sqlrustgo_result=$(echo "$sql" | "$SQLRUSTGO_BIN" sqlite --batch --mode csv --headers true :memory: 2>&1) || true

    # Normalize and compare
    local sqlite_norm sqlrustgo_norm
    sqlite_norm=$(echo "$sqlite_result" | grep -v '^$' | tail -n +2 | LC_ALL=C sort)
    sqlrustgo_norm=$(echo "$sqlrustgo_result" | grep -v '^$' | tail -n +2 | LC_ALL=C sort)

    if [ "$sqlite_norm" = "$sqlrustgo_norm" ]; then
        echo "  PASS: $test_id"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $test_id"
        echo "    SQLite:    $(echo "$sqlite_norm" | head -3)"
        echo "    SQLRustGo: $(echo "$sqlrustgo_norm" | head -3)"
        FAIL=$((FAIL + 1))
    fi
}

# Read corpus line by line
while IFS= read -r line; do
    # Detect test start
    if [[ "$line" =~ ^--\ TEST:\ (.+)$ ]]; then
        # Run previous test if any
        if [ -n "$current_test" ] && [ -n "$current_sql" ]; then
            run_test "$current_test" "$current_sql" "$expected"
        fi
        current_test="${BASH_REMATCH[1]}"
        current_sql=""
        expected=""
    elif [[ "$line" =~ ^--\ EXPECTED:\ (.+)$ ]]; then
        expected="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ ^--\ STATUS:\ (.+)$ ]]; then
        # Skip fixed tests
        if [[ "${BASH_REMATCH[1]}" == FIXED* ]]; then
            echo "  SKIP: $current_test (already fixed)"
            SKIP=$((SKIP + 1))
            current_test=""
            current_sql=""
            expected=""
        fi
    elif [[ "$line" =~ ^--\ [A-Z] ]]; then
        # Skip other comment headers
        continue
    elif [[ "$line" =~ ^-- ]]; then
        # Skip regular comments
        continue
    elif [[ -n "$line" ]]; then
        # Accumulate SQL
        if [ -n "$current_sql" ]; then
            current_sql="$current_sql
$line"
        else
            current_sql="$line"
        fi
    fi
done < "$CORPUS"

# Run last test
if [ -n "$current_test" ] && [ -n "$current_sql" ]; then
    run_test "$current_test" "$current_sql" "$expected"
fi

echo ""
echo "Results: $PASS passed, $FAIL failed, $SKIP skipped"

if [ "$FAIL" -gt 0 ]; then
    exit 1
else
    exit 0
fi
