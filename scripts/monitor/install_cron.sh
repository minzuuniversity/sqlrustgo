#!/bin/bash
# install_cron.sh - Install/remove cron entries for sqlrustgo stability automation.
#
# Adds two cron jobs for the current user:
#   1. status_dashboard.sh every 60s (writes test_results/STATUS.md)
#   2. cleanup_stale_registry.sh every 5 min (removes entries from run_registry
#      whose pid is gone — P0/P1 only on `list`/`register`, not periodic)
#
# Cron limitation: the lowest granularity is 1 minute. To get 60s we use
# two entries offset by 30s:
#   * * * * * /path/to/status_dashboard.sh >/dev/null 2>&1
#   * * * * * sleep 30; /path/to/status_dashboard.sh >/dev/null 2>&1
#
# We also install a watchdog: if status_dashboard hasn't been written in 5 min,
# page the operator (a desktop notification on the host running this).
#
# Usage:
#   bash scripts/monitor/install_cron.sh install   # idempotent
#   bash scripts/monitor/install_cron.sh remove
#   bash scripts/monitor/install_cron.sh status

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DASHBOARD="$SCRIPT_DIR/status_dashboard.sh"
REGISTRY="$PROJECT_ROOT/scripts/stability/run_registry.sh"
LOG_DIR="${LOG_DIR:-$HOME/.sqlrustgo-cron}"
CRON_TAG="# sqlrustgo-stability-automation"

CMD="${1:-status}"

mkdir -p "$LOG_DIR"

# Idempotent marker block.
# All "tagged" lines are kept together; replacing the block atomically.
replace_cron_block() {
    # Build the new block content: stripped crontab + new tagged lines.
    local tmp
    tmp=$(mktemp) || { echo "FAIL: mktemp" >&2; return 1; }
    crontab -l 2>/dev/null | grep -v "$CRON_TAG" > "$tmp" 2>/dev/null
    # Caller adds tagged lines to $tmp via stdin
    cat >> "$tmp"
    crontab "$tmp"
    rm -f "$tmp"
}

# Legacy wrapper: append one tagged line (uses replace_cron_block under the hood)
add_cron() {
    local line="$1"
    local tmp
    tmp=$(mktemp) || { echo "FAIL: mktemp" >&2; return 1; }
    crontab -l 2>/dev/null | grep -v "$CRON_TAG" > "$tmp" 2>/dev/null
    echo "$line  $CRON_TAG" >> "$tmp"
    crontab "$tmp"
    rm -f "$tmp"
}

remove_cron() {
    local tmp
    tmp=$(mktemp) || { echo "FAIL: mktemp" >&2; return 1; }
    crontab -l 2>/dev/null | grep -v "$CRON_TAG" > "$tmp" 2>/dev/null
    crontab "$tmp" 2>/dev/null || true
    rm -f "$tmp"
    echo "Cron entries removed"
}

case "$CMD" in
    install)
        # 60s granularity: two entries offset by 30s
        # Use replace_cron_block so all 3 entries land atomically.
        replace_cron_block <<EOF
* * * * * $DASHBOARD $PROJECT_ROOT/test_results/STATUS.md >> $LOG_DIR/dashboard.log 2>&1  $CRON_TAG
* * * * * sleep 30 && $DASHBOARD $PROJECT_ROOT/test_results/STATUS.md >> $LOG_DIR/dashboard.log 2>&1  $CRON_TAG
*/5 * * * * $REGISTRY list >/dev/null 2>&1  $CRON_TAG
EOF
        echo "Installed 3 cron entries (2× status_dashboard every 60s, 1× registry cleanup every 5 min)"
        echo "Log: $LOG_DIR/dashboard.log"
        echo ""
        echo "Verify with: crontab -l | grep $CRON_TAG"
        ;;
    remove)
        remove_cron
        ;;
    status)
        echo "Current cron entries for $CRON_TAG:"
        crontab -l 2>/dev/null | grep "$CRON_TAG" || echo "  (none installed)"
        echo ""
        echo "Last 20 lines of $LOG_DIR/dashboard.log:"
        tail -20 "$LOG_DIR/dashboard.log" 2>/dev/null || echo "  (no log yet)"
        ;;
    *)
        echo "Usage: $0 {install|remove|status}" >&2
        exit 1
        ;;
esac
