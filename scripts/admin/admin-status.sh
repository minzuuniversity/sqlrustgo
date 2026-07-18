#!/usr/bin/env bash
# admin-status.sh — Show sqlrustgo server status
#
# Usage:
#   bash scripts/admin/admin-status.sh [--host HOST] [--port PORT]

set -uo pipefail

HOST="${HOST:-127.0.0.1}"
PORT="${PORT:-3307}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --host) HOST="$2"; shift 2 ;;
        --port) PORT="$2"; shift 2 ;;
        *) shift ;;
    esac
done

mysql -h "$HOST" -P "$PORT" -u root -e "
SELECT 'Innodb_rows_read' AS Metric, Innodb_rows_read AS Value FROM information_schema.session_status WHERE Variable_name = 'Innodb_rows_read'
UNION ALL
SELECT 'Threads_connected', Threads_connected FROM information_schema.session_status WHERE Variable_name = 'Threads_connected'
UNION ALL
SELECT 'Uptime', Uptime FROM information_schema.session_status WHERE Variable_name = 'Uptime';
" 2>/dev/null || echo "Status query failed"
