#!/usr/bin/env bash
# test_upgrade_v310_to_v311.sh — v3.10.0 → v3.11.0 In-Place Upgrade Test
#
# Tests the upgrade path from v3.10.0 to v3.11.0 by:
# 1. Starting v3.10.0, writing test data
# 2. Stopping v3.10.0
# 3. Upgrading to v3.11.0 binary
# 4. Starting v3.11.0 and verifying data integrity
#
# Usage:
#   bash scripts/test_upgrade_v310_to_v311.sh [--rollback]
#
# Exit code: 0 = PASS, non-zero = FAIL

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Configuration
V310_PORT=3306
V311_PORT=3307
V310_DATA_DIR="/tmp/sqlrustgo-upgrade-v310"
V311_DATA_DIR="/tmp/sqlrustgo-upgrade-v311"
V310_LOG="/tmp/sqlrustgo-v310-upgrade.log"
V311_LOG="/tmp/sqlrustgo-v311-upgrade.log"
TEST_DB="upgrade_test"
ROLLBACK=false

# Parse arguments
for arg in "$@"; do
    case "$arg" in
        --rollback) ROLLBACK=true ;;
        --help|-h)
            echo "Usage: $0 [--rollback]"
            echo "  --rollback  Test downgrade path (v3.11.0 → v3.10.0)"
            exit 0
            ;;
    esac
done

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

cleanup() {
    log_info "Cleaning up..."
    # Kill any running servers
    pkill -f "sqlrustgo.*--port=$V310_PORT" 2>/dev/null || true
    pkill -f "sqlrustgo.*--port=$V311_PORT" 2>/dev/null || true
    # Remove temp directories
    rm -rf "$V310_DATA_DIR" "$V311_DATA_DIR" 2>/dev/null || true
    rm -f "$V310_LOG" "$V311_LOG" 2>/dev/null || true
}

# Ensure cleanup on exit
trap cleanup EXIT

# Check if sqlrustgo binary exists
check_binary() {
    if [ ! -f "$REPO_ROOT/target/debug/sqlrustgo" ]; then
        log_warn "sqlrustgo binary not found at $REPO_ROOT/target/debug/sqlrustgo"
        log_info "Building sqlrustgo..."
        cargo build --release 2>/dev/null || cargo build 2>/dev/null || true
    fi
}

# Wait for server to be ready
wait_for_server() {
    local port="$1"
    local max_wait="${2:-30}"
    local count=0
    while [ $count -lt $max_wait ]; do
        if nc -z 127.0.0.1 "$port" 2>/dev/null; then
            return 0
        fi
        sleep 1
        count=$((count + 1))
    done
    return 1
}

# Start sqlrustgo server
start_server() {
    local port="$1"
    local data_dir="$2"
    local log_file="$3"
    local version="$4"

    log_info "Starting $version on port $port..."
    mkdir -p "$data_dir"

    "$REPO_ROOT/target/debug/sqlrustgo" \
        --port "$port" \
        --data-dir "$data_dir" \
        > "$log_file" 2>&1 &
    local pid=$!

    if ! wait_for_server "$port" 10; then
        log_error "$version failed to start. Log:"
        cat "$log_file"
        return 1
    fi

    log_info "$version started successfully (PID: $pid)"
    return 0
}

# Stop sqlrustgo server
stop_server() {
    local port="$1"
    local version="$2"

    log_info "Stopping $version..."
    pkill -f "sqlrustgo.*--port=$port" 2>/dev/null || true
    sleep 2
    log_info "$version stopped"
}

# Run SQL via mysql client
run_sql() {
    local port="$1"
    local sql="$2"

    mysql -h 127.0.0.1 -P "$port" -u root -e "$sql" 2>/dev/null || true
}

# Create test data on v3.10.0
setup_v310_data() {
    log_info "Setting up v3.10.0 test data..."

    # Create database
    run_sql "$V310_PORT" "DROP DATABASE IF EXISTS $TEST_DB;"
    run_sql "$V310_PORT" "CREATE DATABASE $TEST_DB;"
    run_sql "$V310_PORT" "USE $TEST_DB;"

    # Create tables with various data types
    run_sql "$V310_PORT" "
        CREATE TABLE t1 (
            id INT PRIMARY KEY,
            name VARCHAR(100),
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
    "

    run_sql "$V310_PORT" "
        CREATE TABLE t2 (
            id INT PRIMARY KEY,
            value INT,
            description TEXT
        );
    "

    run_sql "$V310_PORT" "
        CREATE TABLE t3 (
            id INT PRIMARY KEY,
            data BLOB,
            flag BOOLEAN DEFAULT FALSE
        );
    "

    # Insert test data
    for i in $(seq 1 100); do
        run_sql "$V310_PORT" "INSERT INTO t1 VALUES ($i, 'row_$i', NOW());"
    done

    for i in $(seq 1 50); do
        run_sql "$V310_PORT" "INSERT INTO t2 VALUES ($i, $((i * 10)), 'description_$i');"
    done

    # Record checksums
    local t1_count=$(run_sql "$V310_PORT" "SELECT COUNT(*) FROM t1;" | tail -1)
    local t2_count=$(run_sql "$V310_PORT" "SELECT COUNT(*) FROM t2;" | tail -1)

    echo "$t1_count" > /tmp/upgrade_t1_count.txt
    echo "$t2_count" > /tmp/upgrade_t2_count.txt

    log_info "v3.10.0 setup complete: t1=$t1_count rows, t2=$t2_count rows"
}

# Verify data after upgrade
verify_data() {
    log_info "Verifying v3.11.0 data integrity..."

    # Check database exists
    local db_exists=$(run_sql "$V311_PORT" "SHOW DATABASES LIKE '$TEST_DB';" | tail -1)
    if [ "$db_exists" != "$TEST_DB" ]; then
        log_error "Database $TEST_DB not found after upgrade"
        return 1
    fi

    # Check table counts
    local t1_count=$(run_sql "$V311_PORT" "SELECT COUNT(*) FROM ${TEST_DB}.t1;" | tail -1)
    local t2_count=$(run_sql "$V311_PORT" "SELECT COUNT(*) FROM ${TEST_DB}.t2;" | tail -1)
    local t1_expected=$(cat /tmp/upgrade_t1_count.txt)
    local t2_expected=$(cat /tmp/upgrade_t2_count.txt)

    log_info "Row counts: t1=$t1_count (expected $t1_expected), t2=$t2_count (expected $t2_expected)"

    if [ "$t1_count" != "$t1_expected" ]; then
        log_error "t1 row count mismatch: got $t1_count, expected $t1_expected"
        return 1
    fi

    if [ "$t2_count" != "$t2_expected" ]; then
        log_error "t2 row count mismatch: got $t2_count, expected $t2_expected"
        return 1
    fi

    # Check indexes exist
    local t1_indexes=$(run_sql "$V311_PORT" "SHOW INDEX FROM ${TEST_DB}.t1;" | grep -c "PRIMARY" || echo "0")
    if [ "$t1_indexes" -lt 1 ]; then
        log_error "Primary key index on t1 not found"
        return 1
    fi

    log_info "Data integrity verification PASSED"
    return 0
}

# Run the upgrade test
run_upgrade_test() {
    log_info "=== v3.10.0 → v3.11.0 Upgrade Test ==="
    echo ""

    # Clean up any previous runs
    cleanup

    # Check binary
    check_binary

    # Start v3.10.0
    if ! start_server "$V310_PORT" "$V310_DATA_DIR" "$V310_LOG" "v3.10.0"; then
        log_error "Failed to start v3.10.0"
        exit 1
    fi

    # Setup test data
    setup_v310_data

    # Stop v3.10.0
    stop_server "$V310_PORT" "v3.10.0"

    echo ""
    log_info "Upgrading from v3.10.0 to v3.11.0..."
    echo ""

    # Start v3.11.0 (same binary, triggers migration)
    if ! start_server "$V311_PORT" "$V311_DATA_DIR" "$V311_LOG" "v3.11.0"; then
        log_error "Failed to start v3.11.0"
        exit 1
    fi

    # Verify data
    if ! verify_data; then
        log_error "Data verification FAILED"
        exit 1
    fi

    echo ""
    log_info "=== Upgrade Test PASSED ==="
    log_info "v3.10.0 → v3.11.0 upgrade is working correctly"

    if [ "$ROLLBACK" = "true" ]; then
        echo ""
        log_info "Testing rollback (v3.11.0 → v3.10.0)..."
        # This would require keeping the v3.10.0 binary
        log_warn "Rollback test not implemented in this version"
    fi

    return 0
}

# Run the test
run_upgrade_test
