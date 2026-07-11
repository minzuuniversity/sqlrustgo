#!/bin/bash
# auto_rotate_log.sh - 自动 rotation 守护进程
# 每 INTERVAL 秒检查一次 log 文件大小，超过限制就 rotate

INTERVAL=${1:-60}
LOG_FILE="${2:-$(ls -td test_results/wired_soak_72h_* 2>/dev/null | head -1)/sqlrustgo.log}"
MAX_SIZE_MB="${3:-500}"

# 找这个脚本的位置 (rotator 的位置)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROTATOR="$SCRIPT_DIR/rotate_log.sh"

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Auto-rotate monitor started"
echo "  Log: $LOG_FILE"
echo "  Max size: ${MAX_SIZE_MB}MB"
echo "  Interval: ${INTERVAL}s"

while true; do
    bash "$ROTATOR" "$LOG_FILE" "$MAX_SIZE_MB" >> "${LOG_FILE}.rotation.log" 2>&1
    sleep "$INTERVAL"
done
