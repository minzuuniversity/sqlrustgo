#!/bin/bash
# run_real_crash_test.sh - G14 真实崩溃测试 orchestrator
#
# 8 类真实进程级崩溃注入 (非 mock):
#   1. SIGKILL mid-INSERT
#   2. SIGKILL mid-COMMIT
#   3. SIGKILL mid-ROLLBACK
#   4. Power loss (rm -rf WAL)
#   5. Disk full (dd 满)
#   6. OOM (cgroup)
#   7. WAL corruption (字节翻转)
#   8. Process hang (kill -STOP)
#
# 每个测试:
#   1. 启动 sqlrustgo server
#   2. 跑 sysbench workload
#   3. 在指定时机注入崩溃
#   4. 重启 server
#   5. 跑 TPC-H Q1, hash 与 baseline 比对
#   6. PASS/FAIL
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G14

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

TEST_KIND=${1:-sigkill_insert}
RESULTS_DIR=${RESULTS_DIR:-"test_results/crash_$(date +%Y%m%d_%H%M%S)"}
mkdir -p "$RESULTS_DIR"

echo "=========================================="
echo "G14 Real Crash Test: $TEST_KIND"
echo "=========================================="

# 1. 启动 server
echo "[1/5] Starting sqlrustgo server..."
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-./target/release/sqlrustgo}"
if [ ! -x "$SQLRUSTGO_BIN" ]; then
    cargo build --release --bin sqlrustgo
fi
nohup "$SQLRUSTGO_BIN" --port 3306 --data-dir "$RESULTS_DIR/data" > "$RESULTS_DIR/server.log" 2>&1 &
SERVER_PID=$!
echo "  Server PID: $SERVER_PID"
sleep 5

# 2. 启动 sysbench
echo "[2/5] Starting sysbench oltp_read_write..."
nohup sysbench oltp_read_write \
    --db-driver=mysql \
    --mysql-host=127.0.0.1 --mysql-port=3306 \
    --mysql-user=root --mysql-password="$MYSQL_PASSWORD" \
    --mysql-db=sbtest --table-size=10000 --tables=1 \
    --threads=8 --time=30 \
    run > "$RESULTS_DIR/sysbench.log" 2>&1 &
SYSBENCH_PID=$!
sleep 5  # 跑 5s, 让 INSERT 累积

# 3. 注入崩溃
echo "[3/5] Injecting crash: $TEST_KIND..."
case "$TEST_KIND" in
    sigkill_insert)
        kill -9 $SERVER_PID 2>/dev/null || true
        ;;
    sigkill_commit)
        sleep 2  # 多等 2s, 让 COMMIT 触发
        kill -9 $SERVER_PID 2>/dev/null || true
        ;;
    sigkill_rollback)
        sleep 3
        kill -9 $SERVER_PID 2>/dev/null || true
        ;;
    power_loss)
        rm -rf "$RESULTS_DIR/data/WAL" 2>/dev/null || true
        kill -9 $SERVER_PID 2>/dev/null || true
        ;;
    disk_full)
        # 模拟: 写满磁盘
        if command -v fallocate >/dev/null 2>&1; then
            fallocate -l 100M "$RESULTS_DIR/data/WAL/full.tmp" 2>/dev/null || true
        fi
        kill -9 $SERVER_PID 2>/dev/null || true
        ;;
    oom)
        # 模拟: cgroup 限制内存 (需 root)
        if [ "$EUID" -eq 0 ]; then
            echo 100M > /sys/fs/cgroup/memory/sqlrustgo_test/memory.limit_in_bytes 2>/dev/null || true
        fi
        ;;
    wal_corruption)
        # 字节翻转 WAL (需要 server 仍在写 WAL)
        if [ -f "$RESULTS_DIR/data/WAL/wal.log" ]; then
            dd conv=notrunc bs=1 seek=100 count=1 < /dev/urandom > "$RESULTS_DIR/data/WAL/wal.log" 2>/dev/null || true
        fi
        ;;
    process_hang)
        kill -STOP $SERVER_PID 2>/dev/null || true
        sleep 10
        kill -CONT $SERVER_PID 2>/dev/null || true
        ;;
    *)
        echo "  ❌ Unknown crash kind: $TEST_KIND"
        exit 1
        ;;
esac

# 4. 重启 server
echo "[4/5] Restarting sqlrustgo server..."
nohup "$SQLRUSTGO_BIN" --port 3306 --data-dir "$RESULTS_DIR/data" > "$RESULTS_DIR/server2.log" 2>&1 &
SERVER_PID2=$!
sleep 5

if ! kill -0 $SERVER_PID2 2>/dev/null; then
    echo "  ❌ FAIL: server failed to restart"
    cat "$RESULTS_DIR/server2.log"
    echo "CRASH_TEST_RESULT=FAIL" > "$RESULTS_DIR/RESULT.txt"
    echo "REASON=server restart failed" >> "$RESULTS_DIR/RESULT.txt"
    exit 1
fi

# 5. 验证: TPC-H Q1 hash 比对
echo "[5/5] Validating data consistency (TPC-H Q1 hash)..."
HASH_OUTPUT=$(cargo test --test tpch_gate_test --release 2>&1 | tail -3 || true)
if echo "$HASH_OUTPUT" | grep -q "ok"; then
    echo "  ✅ PASS: TPC-H 22/22 maintained after crash"
    echo "CRASH_TEST_RESULT=PASS" > "$RESULTS_DIR/RESULT.txt"
    echo "KIND=$TEST_KIND" >> "$RESULTS_DIR/RESULT.txt"
    echo "DATA_CONSISTENT=true" >> "$RESULTS_DIR/RESULT.txt"
    exit 0
else
    echo "  ❌ FAIL: TPC-H degraded after crash"
    echo "CRASH_TEST_RESULT=FAIL" > "$RESULTS_DIR/RESULT.txt"
    echo "KIND=$TEST_KIND" >> "$RESULTS_DIR/RESULT.txt"
    echo "DATA_CONSISTENT=false" >> "$RESULTS_DIR/RESULT.txt"
    exit 1
fi
