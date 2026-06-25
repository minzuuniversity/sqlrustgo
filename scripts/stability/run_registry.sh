#!/bin/bash
# run_registry.sh - Cross-user registry for sqlrustgo stability / soak tests.
#
# Records active runs in $REGISTRY_FILE so other users / scripts can:
#   - see what is currently running (port, type, host_io pressure, etc.)
#   - avoid port/data_dir conflicts when launching new runs
#   - compute cross-run metrics in a cron job
#
# Design (P1 enhancement 2026-06-14, after P0 verified 8MB RSS vs prior 75GB):
#   - File-based registry at $REGISTRY_FILE (default: /run/sqlrustgo-soaks.tsv)
#     - If /run is not writable, falls back to $TMPDIR/sqlrustgo-soaks-$(id -u).tsv
#   - One row per run, columns: run_id,user,host,port,data_dir,type,start_ts,
#     pid,parent_pid,log_file,memcap_mb,fdcap
#   - Use `flock` for cross-process safety (multiple users may start runs
#     concurrently, e.g. parallel_soak.sh in a loop)
#   - Dead-entry cleanup: on `register` or `list`, garbage-collect entries
#     whose pid is gone (>2 hours since start_ts also evicted)
#
# Usage:
#   run_registry.sh register <type> <port> <data_dir> <log_file> <memcap_mb> [fdcap]
#   run_registry.sh list                      # one row per active run
#   run_registry.sh unregister <run_id>      # remove on graceful exit
#   run_registry.sh suggest-port             # print next free port in [3500,3600)
#   run_registry.sh suggest-data-dir <type>  # print next free /shared-style data_dir
#   run_registry.sh help

set -euo pipefail

REGISTRY_DIR="${REGISTRY_DIR:-/run}"
if [ ! -w "$REGISTRY_DIR" ] 2>/dev/null; then
    REGISTRY_DIR="${TMPDIR:-/tmp}"
fi
REGISTRY_FILE="${REGISTRY_DIR}/sqlrustgo-soaks.tsv"
LOCK_FILE="${REGISTRY_FILE}.lock"

GC_MAX_AGE_S=${GC_MAX_AGE_S:-7200}     # 2h: evict stale entries even if pid alive
SUGGEST_PORT_MIN=${SUGGEST_PORT_MIN:-3500}
SUGGEST_PORT_MAX=${SUGGEST_PORT_MAX:-3600}

cmd="${1:-help}"
shift || true

acquire_lock() {
    # Cross-platform lock: prefer flock(1) (Linux), fall back to mkdir(1) on macOS.
    if command -v flock >/dev/null 2>&1; then
        exec 9>"$LOCK_FILE"
        flock -w 5 9 || { echo "FAIL: could not acquire $LOCK_FILE" >&2; exit 1; }
    else
        # mkdir-based lock with timeout (mkdir is atomic on POSIX).
        local i=0
        while ! mkdir "$LOCK_FILE.lockdir" 2>/dev/null; do
            i=$((i + 1))
            if [ "$i" -gt 50 ]; then
                echo "FAIL: could not acquire $LOCK_FILE.lockdir" >&2
                exit 1
            fi
            sleep 0.1
        done
    fi
}
release_lock() {
    if command -v flock >/dev/null 2>&1; then
        flock -u 9 2>/dev/null || true
        exec 9>&- 2>/dev/null || true
    else
        rmdir "$LOCK_FILE.lockdir" 2>/dev/null || true
    fi
}

# Garbage-collect: remove entries where pid is gone, or age > GC_MAX_AGE_S
gc() {
    if [ ! -f "$REGISTRY_FILE" ]; then return 0; fi
    local now
    now=$(date +%s)
    local tmp="${REGISTRY_FILE}.gc.$$"
    awk -F'\t' -v now="$now" -v max_age="$GC_MAX_AGE_S" '
        NR==1 { print; next }
        {
            pid = $8 + 0
            start_ts = $7 + 0
            if (pid == 0) next  # malformed
            if (kill(pid, 0) != 0) next  # process gone
            age = now - start_ts
            if (age > max_age) next  # too old
            print
        }
    ' "$REGISTRY_FILE" 2>/dev/null > "$tmp" || cp "$REGISTRY_FILE" "$tmp"
    mv "$tmp" "$REGISTRY_FILE"
}

register() {
    local type="${1:?type required}"
    local port="${2:?port required}"
    local data_dir="${3:?data_dir required}"
    local log_file="${4:?log_file required}"
    local memcap_mb="${5:?memcap_mb required}"
    local fdcap="${6:-1024}"
    local user=$(id -un)
    local host=$(hostname -s)
    local start_ts=$(date +%s)
    local pid=$$
    local ppid=$PPID
    local run_id="${user}-${host}-${start_ts}-${pid}"
    acquire_lock
    gc
    # Avoid clobber: if same pid already registered, refresh
    if [ -f "$REGISTRY_FILE" ] && grep -q "	${pid}	" "$REGISTRY_FILE" 2>/dev/null; then
        # Refresh existing
        awk -F'\t' -v pid="${pid}" -v OFS='\t' \
            'BEGIN{header=1} {if($8==pid || header){print; header=0}}' \
            "$REGISTRY_FILE" > "$REGISTRY_FILE.tmp" || true
    else
        # Append new (with header if file doesn't exist)
        if [ ! -f "$REGISTRY_FILE" ]; then
            printf 'run_id\tuser\thost\tport\tdata_dir\ttype\tstart_ts\tpid\tppid\tlog_file\tmemcap_mb\tfdcap\n' \
                > "$REGISTRY_FILE"
        fi
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$run_id" "$user" "$host" "$port" "$data_dir" "$type" "$start_ts" \
        "$pid" "$ppid" "$log_file" "$memcap_mb" "$fdcap" \
        >> "$REGISTRY_FILE"
    release_lock
    echo "$run_id"
}

unregister() {
    local run_id="${1:-}"
    acquire_lock
    if [ -n "$run_id" ]; then
        awk -F'\t' -v run_id="$run_id" '$1 != run_id' "$REGISTRY_FILE" > "$REGISTRY_FILE.tmp" || true
    else
        # Unregister by current pid
        awk -F'\t' -v pid="$$" '$8 != pid' "$REGISTRY_FILE" > "$REGISTRY_FILE.tmp" || true
    fi
    mv "$REGISTRY_FILE.tmp" "$REGISTRY_FILE"
    release_lock
}

list_runs() {
    acquire_lock
    gc
    release_lock
    if [ ! -f "$REGISTRY_FILE" ] || [ ! -s "$REGISTRY_FILE" ]; then
        echo "(registry empty)"
        return
    fi
    # Pretty: print with column widths
    column -t -s $'\t' "$REGISTRY_FILE" 2>/dev/null || cat "$REGISTRY_FILE"
}

suggest_port() {
    acquire_lock
    local used_ports
    if [ -f "$REGISTRY_FILE" ]; then
        used_ports=$(awk -F'\t' 'NR>1 {print $4}' "$REGISTRY_FILE" | sort -n)
    else
        used_ports=""
    fi
    release_lock
    for p in $(seq "$SUGGEST_PORT_MIN" "$SUGGEST_PORT_MAX"); do
        if ! echo "$used_ports" | grep -q "^${p}$" && ! lsof -i ":$p" >/dev/null 2>&1; then
            echo "$p"
            return 0
        fi
    done
    echo "FAIL: no free port in [$SUGGEST_PORT_MIN,$SUGGEST_PORT_MAX)" >&2
    return 1
}

suggest_data_dir() {
    local type="${1:-soak}"
    local base="${REGISTRY_DIR}/sqlrustgo-data"
    mkdir -p "$base" 2>/dev/null || base="${TMPDIR:-/tmp}/sqlrustgo-data"
    mkdir -p "$base" 2>/dev/null || { echo "FAIL: cannot create $base" >&2; return 1; }
    acquire_lock
    local used_dirs
    if [ -f "$REGISTRY_FILE" ]; then
        used_dirs=$(awk -F'\t' 'NR>1 {print $5}' "$REGISTRY_FILE" | sort -u)
    else
        used_dirs=""
    fi
    release_lock
    # Suggest a name based on type + ISO timestamp, never collide
    local stamp
    stamp=$(date +%Y%m%d_%H%M%S)
    local i=0
    while true; do
        local candidate
        if [ "$i" -eq 0 ]; then
            candidate="$base/${type}_${stamp}"
        else
            candidate="$base/${type}_${stamp}_$i"
        fi
        if ! echo "$used_dirs" | grep -qF "$candidate" && [ ! -d "$candidate" ]; then
            echo "$candidate"
            return 0
        fi
        i=$((i + 1))
    done
}

case "$cmd" in
    register)      register "$@" ;;
    unregister)    unregister "$@" ;;
    list)          list_runs "$@" ;;
    suggest-port)  suggest_port "$@" ;;
    suggest-data-dir) suggest_data_dir "$@" ;;
    help|-h|--help)
        sed -n '2,30p' "$0"
        ;;
    *)
        echo "FAIL: unknown command '$cmd'" >&2
        exit 1
        ;;
esac
