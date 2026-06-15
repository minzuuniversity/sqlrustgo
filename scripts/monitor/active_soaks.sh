#!/bin/bash
# active_soaks.sh - Cross-user visibility of running sqlrustgo stability tests.
#
# Lists every sqlrustgo-mysql-server process + its parent shell, showing
# who launched it, when, how much RSS, and which soak script is wrapping it.
# Designed to be runnable by any user (no root / no sudo) — just `ps` +
# `/proc` introspection.
#
# Usage:
#   bash scripts/monitor/active_soaks.sh           # one-shot tabular
#   bash scripts/monitor/active_soaks.sh --json    # JSON for cron piping
#   bash scripts/monitor/active_soaks.sh --watch 5 # refresh every 5s
#
# Design constraints (P0 enhancement 2026-06-14):
#   - Read-only: never kills / never pkill — that is the operator's job
#   - Works across all users (no /proc/<pid>/io owned by same user)
#   - Stable output schema for downstream tooling (cron → /var/log/active_soaks.log)
#   - Fast (<2s even on 1k PIDs) — single ps + awk pipeline, no per-PID forks
#
# Pairs with the per-script ulimit -v caps (P0 enhancement): this monitor
# *observes* but does not enforce. The ulimit caps are the enforcement.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

MODE="table"
WATCH_INTERVAL=0
case "${1:-}" in
    --json)  MODE="json" ;;
    --watch)
        WATCH_INTERVAL="${2:-5}"
        if ! [[ "$WATCH_INTERVAL" =~ ^[0-9]+$ ]] || [ "$WATCH_INTERVAL" -lt 1 ]; then
            echo "FAIL: --watch N requires integer N>=1 (got '$WATCH_INTERVAL')" >&2
            exit 1
        fi
        ;;
    --help|-h)
        sed -n '2,20p' "$0"
        exit 0
        ;;
    "")      ;;
    *)      echo "FAIL: unknown arg '$1' (try --help)" >&2; exit 1 ;;
esac

# ─────────────────────────────────────────────────────────────────────
# Core: walk /proc for any PID that looks like a sqlrustgo soak run.
#
# A "soak process tree" is:
#   bash  run_*.sh  (parent shell, runs monitoring loop)
#     └─ sqlrustgo-mysql-server  (child binary)
#
# Both can be detected by their argv[0/1] without being able to read
# the child's environ. We use /proc/<pid>/comm + /proc/<pid>/cmdline
# which are world-readable on Linux.
# ─────────────────────────────────────────────────────────────────────
collect_soaks() {
    # Single ps + awk pipeline; build a tree in one pass.
    # Output format: PID PPID USER RSS_MB ETIME CMD
    ps -eo pid,ppid,user:32,rss,etime,args --sort=-rss 2>/dev/null \
      | awk '
        BEGIN { sqr=0; bash=0 }
        /sqlrustgo-mysql-server/ {
            # Trim args after the first quoted span; we only need the recognizable prefix
            gsub(/^[[:space:]]+/, "")
            printf "SQLRUSTGO\t%s\n", $0
            sqr++
        }
        /(^|\/)(run_wired_soak|run_tpch_30min|run_24h_soak|run_72h_soak|run_168h_soak|launch_parallel_soak|test_integration_5min|run_integration_5min)\.sh/ {
            gsub(/^[[:space:]]+/, "")
            printf "SOAK_SHELL\t%s\n", $0
            bash++
        }
        END { printf "# found sqlrustgo=%d soak_shells=%d\n", sqr, bash > "/dev/stderr" }
    ' 2>/dev/null
}

# Convert RSS (KB) → MB
rss_to_mb() { awk -v k="$1" 'BEGIN { printf "%.0f", k/1024 }'; }

# Render human table
render_table() {
    local lines
    lines=$(collect_soaks)
    if [ -z "$lines" ]; then
        echo "  (no active sqlrustgo soak tests detected at $(date +%H:%M:%S))"
        return
    fi
    printf "%-10s %-10s %-6s %-9s %-9s %s\n" "PID" "USER" "RSS_MB" "ELAPSED" "PORT" "COMMAND"
    printf "%-10s %-10s %-6s %-9s %-9s %s\n" "----------" "----------" "------" "---------" "---------" "--------"
    echo "$lines" | grep -E "^SQLRUSTGO\t" | awk -F'\t' '{
        # /proc/<pid>/cmdline is NUL-separated
        cmd = $2
        # Extract PORT if --port N appears
        port = "-"
        if (match(cmd, /--port[= ]+([0-9]+)/, arr)) {
            port = arr[1]
        }
        printf "%-10s %-10s %6s %-9s %-9s %s\n", $1, $3, int($4/1024), $5, port, $6
    }' 2>/dev/null
    echo "$lines" | grep -E "^SOAK_SHELL\t" | awk -F'\t' '{
        printf "%-10s %-10s %6s %-9s %-9s %s\n", $1, $3, int($4/1024), $5, "-", $6
    }' 2>/dev/null
}

# Render JSON for cron / monitoring pipelines
render_json() {
    local lines
    lines=$(collect_soaks)
    local server_count=$(echo "$lines" | grep -c "^SQLRUSTGO" || echo 0)
    local shell_count=$(echo "$lines" | grep -c "^SOAK_SHELL" || echo 0)
    printf '{"ts":"%s","server_count":%d,"shell_count":%d,"processes":[' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$server_count" "$shell_count"
    local first=1
    while IFS=$'\t' read -r tag rest; do
        [ -z "$tag" ] && continue
        # rest = "PID PPID USER RSS ETIME CMD"
        local pid=$(echo "$rest" | awk '{print $1}')
        local ppid=$(echo "$rest" | awk '{print $2}')
        local user=$(echo "$rest" | awk '{print $3}')
        local rss_kb=$(echo "$rest" | awk '{print $4}')
        local etime=$(echo "$rest" | awk '{print $5}')
        local cmd=$(echo "$rest" | cut -d' ' -f6-)
        local kind=$(echo "$tag" | tr -d '[:space:]')
        local rss_mb=$(rss_to_mb "$rss_kb")
        [ "$first" = 0 ] && printf ","
        first=0
        printf '{"kind":"%s","pid":%s,"ppid":%s,"user":"%s","rss_mb":%s,"etime":"%s","cmd":%s}' \
            "$kind" "$pid" "$ppid" "$user" "$rss_mb" "$etime" "$(printf '%s' "$cmd" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))' 2>/dev/null || echo '""')"
    done <<< "$lines"
    printf ']}\n'
}

# Main
case "$MODE" in
    table)
        echo "============================================================"
        echo "Active sqlrustgo Soak Tests — $(date '+%Y-%m-%d %H:%M:%S %Z')"
        echo "============================================================"
        render_table
        echo ""
        echo "  Tip: bash $0 --json     # JSON for cron / monitoring"
        echo "       bash $0 --watch 5  # refresh every 5s"
        if [ "$WATCH_INTERVAL" -gt 0 ]; then
            while true; do
                sleep "$WATCH_INTERVAL"
                clear
                echo "============================================================"
                echo "Active sqlrustgo Soak Tests — $(date '+%Y-%m-%d %H:%M:%S %Z') (refresh ${WATCH_INTERVAL}s)"
                echo "============================================================"
                render_table
            done
        fi
        ;;
    json)
        render_json
        ;;
esac
