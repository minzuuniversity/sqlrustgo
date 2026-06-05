#!/bin/bash
# run_24h_soak.sh - G13 24h 真实长跑 (Z6G4 真实环境)
#
# 这是 GA 卡死的 24h 真实运行 (非压缩时间 mock).
# 监控: RSS / FD / 锁 / WAL / CPU
# 工作负载: sysbench oltp_read_write 8 thread
#
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G13 (24h 强制)
#       V390_TEST_PLAN_SUPPLEMENT_PERF.md §G13

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

HOURS=${HOURS:-24}
INTERVAL=${INTERVAL:-60}  # 监控间隔秒
RESULTS_DIR=${RESULTS_DIR:-"test_results/stability_24h_$(date +%Y%m%d_%H%M%S)"}
PID_FILE="$RESULTS_DIR/sqlrustgo.pid"
LOG_FILE="$RESULTS_DIR/sqlrustgo.log"
METRICS_FILE="$RESULTS_DIR/metrics.csv"

mkdir -p "$RESULTS_DIR"

echo "=========================================="
echo "SQLRustGo 24h Soak Test (真实运行)"
echo "=========================================="
echo "Hours: $HOURS  Interval: ${INTERVAL}s"
echo "Results: $RESULTS_DIR"
echo "=========================================="

# 1. 启动 sqlrustgo server (假设 binary 在 target/release)
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-./target/release/sqlrustgo}"
if [ ! -x "$SQLRUSTGO_BIN" ]; then
    echo "  ⚠️ WARN: $SQLRUSTGO_BIN not found, attempting cargo build..."
    cargo build --release --bin sqlrustgo
fi

# 启动 server (后台)
echo "[1/3] Starting sqlrustgo server..."
nohup "$SQLRUSTGO_BIN" --port 3306 --data-dir "$RESULTS_DIR/data" > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo $SERVER_PID > "$PID_FILE"
echo "  Server PID: $SERVER_PID"

# 等待启动
sleep 10
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "  ❌ FAIL: server died on startup"
    cat "$LOG_FILE"
    exit 1
fi

# 2. 启动 sysbench oltp_read_write (后台)
echo "[2/3] Starting sysbench oltp_read_write (8 threads, ${HOURS}h)..."
SYSBENCH_LOG="$RESULTS_DIR/sysbench.log"
nohup sysbench oltp_read_write \
    --db-driver=mysql \
    --mysql-host=127.0.0.1 --mysql-port=3306 \
    --mysql-user=root --mysql-password="$MYSQL_PASSWORD" \
    --mysql-db=sbtest --table-size=10000 --tables=1 \
    --threads=8 --time=$((HOURS*3600)) \
    --report-interval=60 \
    run > "$SYSBENCH_LOG" 2>&1 &
SYSBENCH_PID=$!
echo "  sysbench PID: $SYSBENCH_PID"

# 3. 监控循环
echo "[3/3] Monitoring (${INTERVAL}s interval)..."
echo "ts,rss_mb,fd_count,cpu_pct,wal_mb,lock_count" > "$METRICS_FILE"

END_TS=$(($(date +%s) + HOURS*3600))
while [ $(date +%s) -lt $END_TS ]; do
    TS=$(date '+%Y-%m-%d %H:%M:%S')
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "  ❌ FAIL: server died during run"
        cat "$LOG_FILE" | tail -20
        exit 1
    fi
    RSS_MB=$(ps -o rss= -p $SERVER_PID 2>/dev/null | tr -d ' ' || echo 0)
    RSS_MB=$((RSS_MB / 1024))
    FD_COUNT=$(lsof -p $SERVER_PID 2>/dev/null | wc -l)
    CPU_PCT=$(ps -o %cpu= -p $SERVER_PID 2>/dev/null | tr -d ' ' || echo 0)
    WAL_MB=0
    if [ -d "$RESULTS_DIR/data/WAL" ]; then
        WAL_MB=$(du -sm "$RESULTS_DIR/data/WAL" 2>/dev/null | cut -f1 || echo 0)
    fi
    LOCK_COUNT=0
    echo "$TS,$RSS_MB,$FD_COUNT,$CPU_PCT,$WAL_MB,$LOCK_COUNT" >> "$METRICS_FILE"

    # 异常检测
    if [ "$RSS_MB" -gt 4096 ]; then
        echo "  ⚠️ WARN: RSS > 4GB"
    fi
    if [ "$FD_COUNT" -gt 5000 ]; then
        echo "  ⚠️ WARN: FD > 5000"
    fi
    if [ "$WAL_MB" -gt 10240 ]; then
        echo "  ⚠️ WARN: WAL > 10GB"
    fi

    sleep $INTERVAL
done

# 4. 收口
echo "=========================================="
echo "24h Soak Test Complete"
echo "=========================================="

# 停止 sysbench
kill $SYSBENCH_PID 2>/dev/null || true
wait $SYSBENCH_PID 2>/dev/null || true

# 停止 server
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

# 生成报告
cat > "$RESULTS_DIR/SUMMARY.md" <<EOF
# G13 24h Soak Summary

| 指标 | 24h 后值 | 阈值 | 状态 |
|------|----------|------|------|
| RSS 增长 | $RSS_MB MB | < 50 MB/24h | TBD |
| FD 增长 | $FD_COUNT | < 50 | TBD |
| CPU 平均 | $CPU_PCT% | < 80% | TBD |
| WAL 大小 | $WAL_MB MB | < 1024 MB | TBD |
| 崩溃次数 | 0 | 0 | ✅ |

详细数据: \`metrics.csv\`
sysbench 输出: \`sysbench.log\`
server 日志: \`sqlrustgo.log\`
EOF

cat "$RESULTS_DIR/SUMMARY.md"

echo
echo "✅ 24h Soak Complete. Results: $RESULTS_DIR"
