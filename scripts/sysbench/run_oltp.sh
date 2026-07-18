#!/usr/bin/env bash
# run_oltp.sh — Run Sysbench OLTP mixed workload
#
# Usage:
#   bash scripts/sysbench/run_oltp.sh [DURATION] [THREADS]
#
# Runs sysbench oltp_read_write for DURATION (default: 2h).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

DURATION="${1:-7200}"  # 2 hours
THREADS="${2:-8}"
HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-3307}"
USER="${USER:-root}"
DB="${DB:-sbtest}"

log_info() { echo -e "\033[0;32m[INFO]\033[0m $*"; }
log_error() { echo -e "\033[0;31m[ERROR]\033[0m $*"; }

check_sysbench() {
    if ! command -v sysbench &>/dev/null; then
        log_error "sysbench not installed"
        echo "Install: brew install sysbench (macOS) or apt install sysbench (Linux)"
        exit 1
    fi
}

prepare() {
    log_info "Preparing OLTP schema..."
    mysql -h "$HOST" -P "$PORT" -u "$USER" -e "CREATE DATABASE IF NOT EXISTS $DB;"
    sysbench oltp_read_write \
        --db-driver=mysql \
        --mysql-host="$HOST" \
        --mysql-port="$PORT" \
        --mysql-user="$USER" \
        --mysql-db="$DB" \
        --table-size=100000 \
        --tables=8 \
        prepare
}

run_test() {
    log_info "Running OLTP test: ${DURATION}s, ${THREADS} threads..."
    sysbench oltp_read_write \
        --db-driver=mysql \
        --mysql-host="$HOST" \
        --mysql-port="$PORT" \
        --mysql-user="$USER" \
        --mysql-db="$DB" \
        --table-size=100000 \
        --tables=8 \
        --threads="$THREADS" \
        --time="$DURATION" \
        run
}

main() {
    check_sysbench
    prepare
    run_test
}

main
