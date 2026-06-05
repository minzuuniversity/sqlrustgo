#!/bin/bash
# Sysbench OLTP Read Write Test for SQLRustGo
# Usage: ./oltp_read_write.sh [threads] [time]
#
# Per G12: oltp_read_write 基准 (混合读写 80/20)
# Refs: V390_TEST_PLAN_SUPPLEMENT_PERF.md §G12

set -e

THREADS=${1:-8}
TIME=${2:-60}
HOST=${HOST:-127.0.0.1}
PORT=${PORT:-3306}
USER=${USER:-root}
PASSWORD=${PASS...date +%Y%m%d_%H%M%S)_oltp_read_write"
mkdir -p "$RESULTS_DIR"

echo "=========================================="
echo "SQLRustGo Sysbench OLTP Read Write"
echo "=========================================="
echo "Threads: $THREADS  Time: ${TIME}s  Table size: $TABLE_SIZE"
echo "=========================================="

# Prepare
echo "[1/3] Preparing tables..."
sysbench oltp_insert \
  --db-driver=mysql \
  --mysql-host=$HOST --mysql-port=$PORT \
  --mysql-user=$USER --mysql-password=$PASSWORD \
  --mysql-db=$DB --table-size=$TABLE_SIZE --tables=1 \
  prepare 2>/dev/null || echo "  (table may already exist)"

# Warmup
echo "[2/3] Warmup (10s)..."
sysbench oltp_read_write \
  --db-driver=mysql \
  --mysql-host=$HOST --mysql-port=$PORT \
  --mysql-user=$USER --mysql-password=$PASSWORD \
  --mysql-db=$DB --table-size=$TABLE_SIZE --tables=1 \
  --threads=$THREADS --time=10 \
  run > /dev/null 2>&1 || true

# Benchmark
echo "[3/3] Running benchmark (${TIME}s)..."
OUTPUT_FILE="$RESULTS_DIR/oltp_read_write_t${THREADS}.log"
sysbench oltp_read_write \
  --db-driver=mysql \
  --mysql-host=$HOST --mysql-port=$PORT \
  --mysql-user=$USER --mysql-password=$PASSWORD \
  --mysql-db=$DB --table-size=$TABLE_SIZE --tables=1 \
  --threads=$THREADS --time=$TIME \
  --report-interval=5 \
  run 2>&1 | tee "$OUTPUT_FILE"

echo
echo "✅ Result saved: $OUTPUT_FILE"
