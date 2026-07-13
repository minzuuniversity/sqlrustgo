#!/usr/bin/env bash
# scripts/gate/watchdog_memory.sh
#
# Memory-overcommit guard for the TPC-H gate.
# Polls /proc/<pid>/smaps_rollup (RSS of the wrapped process tree) every
# interval seconds. When cumulative RSS exceeds LIMIT_GB, sends SIGKILL
# (after a WARN-then-SIGTERM grace period) to the entire process group
# of the wrapped PID. Also writes a stack-dump artifact when killing.
#
# Usage:
#   watchdog_memory.sh --limit-gb 350 --interval 5 -- ./scripts/gate/run_xxx.sh
#
# Notes:
# - Memory budget per your policy: OOM at 400 GB → kill at 350 GB.
# - The wrapped program MUST be a long-running foreground process.
# - The watchdog sets a process group and tracks both PIDs and children
#   (so `cargo test` + child `tpch_*_test` binary are both killed).
#
# Author: MiniMax M3 / Z6G4, on 2026-07-13 for issue #3732 G4 gate.

set -uo pipefail

LIMIT_GB="${LIMIT_GB:-350}"
INTERVAL="${INTERVAL:-5}"
GRACE_S="${GRACE_S:-30}"
DUMP_DIR="${DUMP_DIR:-/tmp/tpch-watchdog-dumps}"
LOG_PREFIX="${LOG_PREFIX:-[watchdog]}"

# ---------- arg parsing ----------
ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --limit-gb) LIMIT_GB="$2"; shift 2 ;;
        --interval) INTERVAL="$2"; shift 2 ;;
        --grace)    GRACE_S="$2";   shift 2 ;;
        --dump-dir) DUMP_DIR="$2";  shift 2 ;;
        --) shift; break ;;
        *)  ARGS+=("$1"); shift ;;
    esac
done
[[ $# -gt 0 ]] && ARGS+=("$@")
[[ ${#ARGS[@]} -eq 0 ]] && { echo "$LOG_PREFIX no command given" >&2; exit 2; }

mkdir -p "$DUMP_DIR"

LIMIT_BYTES=$((LIMIT_GB * 1024 * 1024 * 1024))

echo "$LOG_PREFIX limit=${LIMIT_GB}G interval=${INTERVAL}s grace=${GRACE_S}s dump_dir=$DUMP_DIR"
echo "$LOG_PREFIX running: ${ARGS[*]}"

# ---------- run wrapped command in its own process group ----------
set -m
"${ARGS[@]}" &
WRAP_PID=$!
set +m

# Helper: total RSS in bytes across PID + descendants (best-effort).
total_rss_bytes() {
    local root="$1"
    local sum=0
    local pid
    while read -r pid; do
        [[ -z "$pid" ]] && continue
        local f="/proc/$pid/smaps_rollup"
        [[ -r "$f" ]] || continue
        # PSS line: "Pss:             12345 kB"  →  pull first integer
        local kb
        kb=$(awk '/^Pss:/{print $2; exit}' "$f" 2>/dev/null) || continue
        [[ "$kb" =~ ^[0-9]+$ ]] || continue
        sum=$((sum + kb * 1024))
    done < <(descendants "$root")
    echo "$sum"
}

# Recursive descent: PPID walk (no pgrep dependency).
descendants() {
    local root="$1"
    local -A seen=()
    local queue=("$root")
    while [[ ${#queue[@]} -gt 0 ]]; do
        local pid="${queue[0]}"; queue=("${queue[@]:1}")
        [[ -n "${seen[$pid]:-}" ]] && continue
        seen[$pid]=1
        echo "$pid"
        for c in /proc/[0-9]*/status; do
            [[ -r "$c" ]] || continue
            local ppid
            ppid=$(awk '/^PPid:/{print $2; exit}' "$c" 2>/dev/null) || continue
            if [[ "$ppid" == "$pid" ]]; then
                local cpid
                cpid=$(awk '/^Pid:/{print $2; exit}' "$c")
                queue+=("$cpid")
            fi
        done
    done
}

dump_stacks() {
    local reason="$1"
    local stamp
    stamp=$(date +%Y%m%d%H%M%S)
    local out="$DUMP_DIR/stacks-$stamp-$reason.txt"
    {
        echo "=== watchdog dump reason=$reason at $(date -Iseconds) ==="
        echo "=== total RSS = $(printf '%d MB' $((TOTAL_BYTES_LAST / 1024 / 1024))) limit=$(printf '%d GB' $LIMIT_GB) ==="
        echo "=== process tree ==="
        for pid in $(descendants "$WRAP_PID" 2>/dev/null); do
            echo "--- pid=$pid ---"
            cat "/proc/$pid/cmdline" 2>/dev/null | tr '\0' ' '; echo
            cat "/proc/$pid/status" 2>/dev/null | head -10
        done
    } > "$out"
    echo "$LOG_PREFIX dump → $out"
}

# ---------- monitor loop ----------
TOTAL_BYTES_LAST=0
ELAPSED_GRACE=0
START_TS=$(date +%s)
KILL_SENT=0

while kill -0 "$WRAP_PID" 2>/dev/null; do
    sleep "$INTERVAL"
    [[ $KILL_SENT -eq 1 ]] && continue  # waiting for wrapped to actually die

    TOTAL_BYTES_LAST=$(total_rss_bytes "$WRAP_PID")
    TOTAL_MB=$((TOTAL_BYTES_LAST / 1024 / 1024))
    TOTAL_GB=$(awk "BEGIN { printf \"%.1f\", $TOTAL_BYTES_LAST/1024/1024/1024 }")

    ELAPSED=$(( $(date +%s) - START_TS ))
    echo "$LOG_PREFIX elapsed=${ELAPSED}s rss=${TOTAL_MB}MB (${TOTAL_GB}G) pid=$WRAP_PID"

    if (( TOTAL_BYTES_LAST > LIMIT_BYTES )); then
        echo "$LOG_PREFIX ⚠️ RSS ${TOTAL_GB}G > ${LIMIT_GB}G"
        if (( ELAPSED_GRACE == 0 )); then
            echo "$LOG_PREFIX sending SIGTERM (grace ${GRACE_S}s)..."
            kill -TERM -"$WRAP_PID" 2>/dev/null   # negative = whole pgroup
            ELAPSED_GRACE=1
            GRACE_START_TS=$(date +%s)
        else
            GRACE_NOW=$(( $(date +%s) - GRACE_START_TS ))
            if (( GRACE_NOW >= GRACE_S )); then
                echo "$LOG_PREFIX grace exceeded; sending SIGKILL to pgroup $WRAP_PID"
                dump_stacks "oom-limit"
                kill -KILL -"$WRAP_PID" 2>/dev/null
                KILL_SENT=1
            fi
        fi
    else
        # Below limit: reset grace if we had been inside the grace window
        if (( ELAPSED_GRACE > 0 )); then
            echo "$LOG_PREFIX RSS back under limit; cancelling pending kill"
            ELAPSED_GRACE=0
        fi
    fi
done

# Final wait + cleanup
wait "$WRAP_PID" 2>/dev/null
RC=$?

if (( KILL_SENT )); then
    echo "$LOG_PREFIX wrapped process killed by watchdog; exit-code=137 (SIGKILL)"
    exit 137
fi

echo "$LOG_PREFIX wrapped process exited normally with rc=$RC"
exit "$RC"
