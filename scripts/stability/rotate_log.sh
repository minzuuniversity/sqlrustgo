#!/bin/bash
# rotate_log.sh - 自动 log rotation
# 当 log 大于 500MB 时 truncate (清空), 保留最近的 1 个 backup
#
# Usage:
#   bash rotate_log.sh <log_file> [max_size_mb]
#
# 默认监控 sqlrustgo.log（500MB 限制）

LOG_FILE="${1:-$(ls -td test_results/wired_soak_72h_* 2>/dev/null | head -1)/sqlrustgo.log}"
MAX_SIZE_MB="${2:-500}"

[ ! -f "$LOG_FILE" ] && exit 0

SIZE_MB=$(stat -f %z "$LOG_FILE" 2>/dev/null | awk '{print int($1/1024/1024)}')

if [ "${SIZE_MB:-0}" -gt "$MAX_SIZE_MB" ]; then
    # 旋转: current → .1, 然后清空当前
    if [ -f "$LOG_FILE.1" ]; then
        # 删除旧 .1 (只要 1 个 backup)
        rm -f "$LOG_FILE.1"
    fi
    mv "$LOG_FILE" "$LOG_FILE.1"
    touch "$LOG_FILE"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Rotated: $LOG_FILE was ${SIZE_MB}MB, now .1 (saved), current truncated"
fi
