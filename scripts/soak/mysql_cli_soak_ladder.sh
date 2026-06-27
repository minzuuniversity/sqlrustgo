#!/bin/bash
# MySQL CLI Ladder Soak Test
# Uses: mysql CLI persistent pipe, one connection per thread
# Tests: 10/20/30/40/60 min × 1/2/4/8/16 threads
set -euo pipefail

BINARY="${BINARY:-./target/release/sqlrustgo-mysql-server}"
OUT_DIR="${OUT_DIR:-./docs/releases/v3.9.0/soak_results/}"
mkdir -p "$OUT_DIR"

DURATIONS=(600 1200 1800 2400 3600)  # 10/20/30/40/60 min in seconds
THREADS=(1 2 4 8 16)

log() { echo "[$(date '+%H:%M:%S')] $*"; }

start_server() {
    DATA_DIR=$(mktemp -d)
    PORT=$(python3 -c "import socket; s=socket.socket(); s.bind(('127.0.0.1',0)); print(s.getsockname()[1]); s.close()")
    LOG_FILE="$OUT_DIR/${LABEL}.log"

    $BINARY serve --host 127.0.0.1 --port $PORT \
        --data-dir "$DATA_DIR" --log-level warn \
        > "$LOG_FILE" 2>&1 &
    SERVER_PID=$!

    # Wait for server ready
    for i in $(seq 1 30); do
        if mysql -h 127.0.0.1 -P $PORT -u root -N -B -e "SELECT 1" > /dev/null 2>&1; then
            break
        fi
        sleep 0.5
    done

    echo "$PORT $SERVER_PID $DATA_DIR $LOG_FILE"
}

stop_server() {
    local port=$1 pid=$2 data_dir=$3
    kill $pid 2>/dev/null || true
    wait $pid 2>/dev/null || true
    rm -rf "$data_dir"
}

get_rss() { ps -o rss= -p $1 2>/dev/null | tr -d ' ' || echo 0; }
get_fd() { ls /proc/$1/fd 2>/dev/null | wc -l || echo 0; }

# One thread: mysql pipe loop
# Uses named pipe to keep mysql open and send queries continuously
run_thread() {
    local tid=$1 port=$2 end_time=$3
    local fifo="/tmp/soak_fifo_$$_${tid}"
    mkfifo "$fifo"

    # mysql reads from fifo, writes to /dev/null
    mysql -h 127.0.0.1 -P $port -u root -N -B \
        < "$fifo" > /dev/null 2>&1 &
    MYSQL_PID=$!

    local q=0 errors=0 cnt=$tid
    while [ $(date +%s) -lt $end_time ]; do
        echo "SELECT $cnt;" > "$fifo"
        cnt=$((cnt + 16))
        q=$((q + 1))
    done

    echo "SELECT 1;" > "$fifo"  # flush
    rm -f "$fifo"
    kill $MYSQL_PID 2>/dev/null || true
    echo "$q $errors"
}

# One soak run
run_soak() {
    local dur_min=$1 threads=$2 label=$3

    log "=== $label: ${dur_min}m, $threads threads ==="

    IFS=' ' read -r PORT SERVER_PID DATA_DIR LOG_FILE <<< "$(start_server)"
    log "Server port=$PORT pid=$SERVER_PID"

    local rss_start=$(get_rss $SERVER_PID)
    local fd_start=$(get_fd $SERVER_PID)
    local start_time=$(date +%s)
    local end_time=$((start_time + dur_min * 60))

    # Launch threads
    declare -a pids
    for t in $(seq 0 $((threads - 1))); do
        run_thread $t $PORT $end_time &
        pids+=($!)
    done

    # Monitor every 30s
    while true; do
        alive=0
        for p in "${pids[@]}"; do
            if kill -0 $p 2>/dev/null; then alive=$((alive+1)); fi
        done
        [ $alive -eq 0 ] && break

        elapsed=$(($(date +%s) - start_time))
        rss=$(get_rss $SERVER_PID)
        fd=$(get_fd $SERVER_PID)
        log "  t=${elapsed}s threads=$alive rss=${rss}KB fd=${fd}"
        sleep 30
    done

    # Collect
    total_q=0 total_err=0
    for p in "${pids[@]}"; do
        IFS=' ' read -r q err < <(wait $p 2>/dev/null || echo "0 0")
        total_q=$((total_q + q))
        total_err=$((total_err + err))
    done

    local actual_dur=$(($(date +%s) - start_time))
    local rss_end=$(get_rss $SERVER_PID)
    local fd_end=$(get_fd $SERVER_PID)
    local qps=$(python3 -c "print(f'{$total_q/max($actual_dur,1):.1f}')" 2>/dev/null || echo "N/A")

    stop_server $PORT $SERVER_PID "$DATA_DIR"

    local passed=$([ $total_err -eq 0 ] && echo YES || echo NO)
    log "RESULT: q=$total_q err=$total_err qps=$qps rss_delta=$((rss_end - rss_start))KB fd_delta=$((fd_end - fd_start)) PASS=$passed"

    cat > "$OUT_DIR/${label}_summary.txt" << EOF
Label:         $label
Duration:      ${actual_dur}s (target ${dur_min}m)
Threads:       $threads
Total queries: $total_q
Total errors:  $total_err
Error rate:   $total_err/max($total_q,1)*100:.4f}%
Final QPS:     $qps
RSS start:    ${rss_start} KB
RSS end:      ${rss_end} KB
RSS growth:   $((rss_end - rss_start)) KB
FD start:     $fd_start
FD end:       $fd_end
PASS:         $passed
EOF
}

# Main
total=0 passed=0
for dur in "${DURATIONS[@]}"; do
    dur_min=$((dur / 60))
    for t in "${THREADS[@]}"; do
        total=$((total + 1))
        label="soak_${dur_min}m_${t}t"
        if run_soak $dur_min $t "$label"; then
            passed=$((passed + 1))
        fi
        log "Progress: $passed/$total passed"
    done
done

log "=== FINAL: $passed/$total PASSED ==="
log "Results in: $OUT_DIR"
