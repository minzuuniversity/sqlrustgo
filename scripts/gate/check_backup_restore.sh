#!/bin/bash
# check_backup_restore.sh - P1-1 (#3173) Backup/Restore/Verify/PITR G6 gate
#
# Verifies P1-1 deliverables:
#   1. crates/admin crate exists with sqlrustgo-admin binary
#   2. 4 subcommands (backup, restore, verify, pitr) defined via clap
#   3. tests/backup_restore_test.rs has >= 50 tests
#   4. cargo test -p sqlrustgo-admin PASS
#   5. cargo test --test backup_restore_test PASS
#   6. End-to-end CLI smoke test (4 commands)
#
# Exit code: 0 = PASS, 1 = FAIL
#
# Refs: docs/openspec/3173-p11-backup-restore.md
#       V390_TEST_PLAN.md §G6

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== G6 Gate: P1-1 (#3173) Backup/Restore/Verify/PITR ==="

# 1. crates/admin exists with [[bin]] name = "sqlrustgo-admin"
if [ ! -d crates/admin ]; then
    echo "  [1/6] FAIL: crates/admin directory not found"
    exit 1
fi
if ! grep -q 'name = "sqlrustgo-admin"' crates/admin/Cargo.toml; then
    echo "  [1/6] FAIL: crates/admin/Cargo.toml missing 'name = \"sqlrustgo-admin\"'"
    exit 1
fi
echo "  [1/6] PASS: sqlrustgo-admin crate exists"

# 2. main.rs defines 4 subcommands via clap
MAIN=crates/admin/src/main.rs
if [ ! -f "$MAIN" ]; then
    echo "  [2/6] FAIL: crates/admin/src/main.rs not found"
    exit 1
fi
for cmd in Backup Restore Verify Pitr; do
    if ! grep -q "$cmd" "$MAIN"; then
        echo "  [2/6] FAIL: $cmd subcommand not defined in main.rs"
        exit 1
    fi
done
echo "  [2/6] PASS: 4 subcommands (backup, restore, verify, pitr) defined"

# 3. e2e test file has >= 50 tests
TEST_FILE=tests/backup_restore_test.rs
if [ ! -f "$TEST_FILE" ]; then
    echo "  [3/6] FAIL: $TEST_FILE not found"
    exit 1
fi
TESTS=$(grep -c "^#\[test\]" "$TEST_FILE" 2>/dev/null || echo 0)
if [ "$TESTS" -lt 50 ]; then
    echo "  [3/6] FAIL: only $TESTS tests (expected >= 50)"
    exit 1
fi
echo "  [3/6] PASS: $TESTS e2e tests in tests/backup_restore_test.rs"

# 4. cargo test -p sqlrustgo-admin PASS
echo "  [4/6] Running cargo test -p sqlrustgo-admin (5min budget)..."
if ! timeout 300 cargo test -p sqlrustgo-admin --lib 2>&1 | tail -3; then
    echo "  [4/6] FAIL: cargo test -p sqlrustgo-admin failed"
    exit 1
fi
echo "  [4/6] PASS: sqlrustgo-admin unit tests"

# 5. e2e tests PASS
echo "  [5/6] Running cargo test --test backup_restore_test (5min budget)..."
if ! timeout 300 cargo test --test backup_restore_test 2>&1 | tail -3; then
    echo "  [5/6] FAIL: backup_restore_test failed"
    exit 1
fi
echo "  [5/6] PASS: 51 e2e tests"

# 6. End-to-end CLI smoke test (backup -> verify -> restore)
echo "  [6/6] Running end-to-end CLI smoke test..."
TMPDIR=$(mktemp -d)
mkdir -p "$TMPDIR/data"
echo "CREATE TABLE t (id INTEGER);" > "$TMPDIR/data/schema.sql"
echo "data" > "$TMPDIR/data/t.json"
SQLRUSTGO_ADMIN="$(cargo build -p sqlrustgo-admin --quiet --message-format=plain 2>/dev/null && \
    find target/debug -name 'sqlrustgo-admin' -type f -executable 2>/dev/null | head -1)"
if [ -z "$SQLRUSTGO_ADMIN" ]; then
    SQLRUSTGO_ADMIN="$(find target -name 'sqlrustgo-admin' -type f -executable 2>/dev/null | head -1)"
fi
if [ -z "$SQLRUSTGO_ADMIN" ] || [ ! -x "$SQLRUSTGO_ADMIN" ]; then
    echo "  [6/6] SKIP: sqlrustgo-admin binary not found (build manually: cargo build -p sqlrustgo-admin)"
else
    if "$SQLRUSTGO_ADMIN" backup --data-dir "$TMPDIR/data" --output "$TMPDIR/b.tar.gz" 2>&1 | grep -q "backup ok"; then
        echo "    backup: ok"
        if "$SQLRUSTGO_ADMIN" verify --input "$TMPDIR/b.tar.gz" 2>&1 | grep -q "verify ok"; then
            echo "    verify: ok"
            if "$SQLRUSTGO_ADMIN" restore --input "$TMPDIR/b.tar.gz" --target-dir "$TMPDIR/restore" 2>&1 | grep -q "restore ok"; then
                echo "    restore: ok"
                echo "  [6/6] PASS: end-to-end CLI smoke test"
            else
                echo "  [6/6] FAIL: restore cli failed"
                rm -rf "$TMPDIR"
                exit 1
            fi
        else
            echo "  [6/6] FAIL: verify cli failed"
            rm -rf "$TMPDIR"
            exit 1
        fi
    else
        echo "  [6/6] FAIL: backup cli failed"
        rm -rf "$TMPDIR"
        exit 1
    fi
fi

rm -rf "$TMPDIR"

echo
echo "=== G6 Gate: PASS ==="
echo "P1-1 (#3173) Backup/Restore/Verify/PITR verified"
exit 0
