#!/bin/bash
# run_soak.sh - Unified entrypoint for all sqlrustgo stability / soak tests.
#
# Dispatches to the canonical run_*.sh in scripts/stability/, applying
# P0 enhancements (ulimit -v cap, FD cap, RSS absolute watchdog) by
# default and providing a single place to evolve coordination logic.
#
# Usage:
#   bash scripts/stability/run_soak.sh wired 0.5     # 0.5h wired soak
#   bash scripts/stability/run_soak.sh tpch  30      # 30min TPC-H
#   bash scripts/stability/run_soak.sh 24h   24      # 24h soak
#   bash scripts/stability/run_soak.sh 5min          # 5min integration smoke
#
# Dispatches to the correct script and passes through any extra env vars.
# Default memory cap is 8 GB per run (overridable via SERVER_MEM_MB).
#
# P0 enhancement 2026-06-14: this wraps the underlying run_*.sh with a
# single source of truth for resource limits, so when 兄弟 agent adds a new
# run_*.sh, they just call this entrypoint instead of re-implementing
# ulimit/cleanup/watchdog code.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

usage() {
    cat <<EOF
Usage: bash $0 <TYPE> [ARGS...]

Types:
  wired  [HOURS=24]          Wired soak (sysbench + TPC-H 22 rotation)
  tpch   [DURATION=1800]     30min TPC-H only (sysbench-free)
  24h    [HOURS=24]          24h stability soak
  72h    [HOURS=72]          72h stability soak
  168h   [HOURS=168]         168h stability soak
  5min                       5-min integration smoke test (PR #3380)
  integration_5min           Alias for 5min
  list                       List available scripts and exit
  monitor [--watch N]        Run monitor/active_soaks.sh (cross-user view)
  help                       Show this message

All types pass through env vars (SERVER_MEM_MB, THREADS, FIXTURE, etc).

P0 enhancements applied by default (override via env if you know better):
  - SERVER_MEM_MB=8192       ulimit -v cap on sqlrustgo-mysql-server
  - SERVER_FD_LIMIT=1024     per-server FD cap
  - RSS_ALERT_MB=80% of cap auto-kill watchdog
EOF
}

TYPE="${1:-help}"
shift || true

case "$TYPE" in
    help|-h|--help) usage; exit 0 ;;
    list) ls -1 "$SCRIPT_DIR" | grep -E '^(run_|test_)' | sort; exit 0 ;;
    monitor) exec bash "$SCRIPT_DIR/../monitor/active_soaks.sh" "$@" ;;
    wired)     TARGET="run_wired_soak.sh" ;;
    tpch)      TARGET="run_tpch_30min.sh" ;;
    24h)       TARGET="run_24h_soak.sh" ;;
    72h)       TARGET="run_72h_soak.sh" ;;
    168h)      TARGET="run_168h_soak.sh" ;;
    5min|integration_5min)  TARGET="test_integration_5min.sh" ;;
    *)
        echo "FAIL: unknown type '$TYPE'" >&2
        usage
        exit 1
        ;;
esac

if [ ! -x "$SCRIPT_DIR/$TARGET" ]; then
    echo "FAIL: $SCRIPT_DIR/$TARGET not found or not executable" >&2
    exit 1
fi

# P0 default memory cap (8 GB per run) — overridable via env.
export SERVER_MEM_MB=${SERVER_MEM_MB:-8192}
export SERVER_FD_LIMIT=${SERVER_FD_LIMIT:-1024}
# RSS_ALERT_MB=0 means "auto-derive from SERVER_MEM_MB (80%)" inside the script.
export RSS_ALERT_MB=${RSS_ALERT_MB:-0}

# P1 — port + data_dir auto-allocation via run_registry.sh.
# Default: suggest from pool; user can override via PORT / DATA_DIR env.
REGISTRY="$SCRIPT_DIR/run_registry.sh"
if [ -x "$REGISTRY" ] && [ "${USE_REGISTRY:-1}" = "1" ]; then
    if [ -z "${PORT:-}" ] && [ "$TYPE" != "monitor" ] && [ "$TYPE" != "help" ] && [ "$TYPE" != "list" ]; then
        SUGGESTED_PORT=$(bash "$REGISTRY" suggest-port 2>/dev/null || true)
        if [ -n "$SUGGESTED_PORT" ]; then
            export PORT="$SUGGESTED_PORT"
            echo "  [P1] Auto-allocated port $PORT from registry pool"
        fi
    fi
fi

echo "============================================================"
echo "run_soak.sh unified entrypoint — type=$TYPE target=$TARGET"
echo "SERVER_MEM_MB=$SERVER_MEM_MB SERVER_FD_LIMIT=$SERVER_FD_LIMIT"
echo "PORT=$PORT"
echo "Other args: $*"
echo "============================================================"
# Export SCRIPT_DIR so dispatched scripts (which use ${SCRIPT_DIR}/../../target/...)
# resolve the binary path correctly when invoked from run_soak.sh.
export SCRIPT_DIR
exec bash "$SCRIPT_DIR/$TARGET" "$@"
