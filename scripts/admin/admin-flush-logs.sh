#!/usr/bin/env bash
# admin-flush-logs.sh — Flush sqlrustgo log files
#
# Usage:
#   bash scripts/admin/admin-flush-logs.sh [--host HOST] [--port PORT]

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

mysql -h "$HOST" -P "$PORT" -u root -e "FLUSH LOGS;" 2>/dev/null && echo "Logs flushed" || echo "Flush failed"
