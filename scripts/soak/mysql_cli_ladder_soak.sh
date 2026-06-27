#!/bin/bash
# MySQL CLI Ladder Soak Test
# Tests: 10/20/30/40/60 min × 1/2/4/8/16 threads = 25 runs
# Client: MariaDB 10.6 mysql CLI (--protocol=TCP, -N -B)

set -euo pipefail

BINARY="${CARGO_TARGET_DIR:-./target}/debug/sqlrustgo-mysql-server"
DATA_DIR_BASE="${DATA_DIR_BASE:-/tmp/soak_test_$$}"
REPORT_DIR="${REPORT_DIR:-./docs/releases/v3.9.0/soak_results/}"

DURATIONS=(600 1200 1800 2400 3600)   # 10/20/30/40/60 min in seconds
THREADS=(1 2 4 8 16)
QPS_PER_THREAD=5  # simple SELECT 1 per thread per second
ITERATIONS=100    # iterations per thread before sleep

mkdir -p "$REPORT_DIR"

echo "=============================================="
echo "MySQL CLI Ladder Soak Test"
echo "Binary: $BINARY"
echo "Results: $REPORT_DIR"
echo "=============================================="

run_one() {
    local duration=$1      # seconds
    local threads=$2
    local qps=$3
    local label=$4

    local data_dir=$(mktemp -d -p /tmp "soak_${label}_XXXX")
    local port=$(python3 -c "import socket; s=socket.socket(); s.bind(('127.0.0.1',0)); print(s.getsockname()[1]); s.close()")
    local log_file="$REPORT_DIR/${label}.log"
    local csv_file="$REPORT_DIR/${label}.csv"
    local start_time=$(date +%s)
    local end_time=$((start_time + duration))
    local pid=""

    echo "[$label] Starting server port=$port data_dir=$data_dir"

    $BINARY serve \
        --host 127.0.0.1 \
        --port $port \
        --data-dir "$data_dir" \
        --log-level warn \
        > "$log_file" 2>&1 &
    pid=$!
    sleep 2

    # Check server is up
    if ! mysql -h 127.0.0.1 -P $port -u root -N -B -e "SELECT 1;" > /dev/null 2>&1; then
        echo "[$label] FAIL: server not ready"
        kill $pid 2>/dev/null; wait $pid 2>/dev/null; rm -rf "$data_dir"
        return 1
    fi

    # Get server PID info for RSS/FD tracking
    get_rss() { ps -o rss= -p $pid 2>/dev/null | tr -d ' ' || echo "0"; }
    get_fd()  { ls /proc/$pid/fd 2>/dev/null | wc -l || echo "0"; }

    echo "[$label] Running $threads threads × ${qps}qps for ${duration}s"

    # Spawn threads
    declare -a pids
    total_queries=0
    total_errors=0
    start_rss=$(get_rss)
    start_fd=$(get_fd)

    for ((t=1; t<=threads; t++)); do
        (
            local queries=0
            local errors=0
            while [ $(date +%s) -lt $end_time ]; do
                for ((i=1; i<=ITERATIONS; i++)); do
                    if [ $(date +%s) -ge $end_time ]; then break 2; fi
                    result=$(mysql -h 127.0.0.1 -P $port -u root -N -B -e "SELECT $queries AS n;" 2>&1)
                    if [ $? -ne 0 ] || echo "$result" | grep -qi "ERROR\|error\|failed\|panic"; then
                        errors=$((errors+1))
                    fi
                    queries=$((queries+1))
                done
                # Sleep to throttle to target QPS
                sleep 0.2
            done
            echo "THREAD_DONE:$queries:$errors" >> /tmp/soak_thread_${label}.out
        ) &
        pids+=($!)
    done

    # Monitor loop
    samples=0
    > "$csv_file"
    echo "timestamp,elapsed_s,queries_done,errors,rss_mb,fd_count,qps" > "$csv_file"
    while [ $(date +%s) -lt $end_time ]; do
        elapsed=$(($(date +%s) - start_time))
        rss=$(get_rss)
        fd=$(get_fd)

        # Sum queries from thread output files
        queries_done=0
        errors_done=0
        for f in /tmp/soak_thread_${label}.out; do
            if [ -f "$f" ]; then
                while IFS=: read -r q e; do
                    queries_done=$((queries_done+q))
                    errors_done=$((errors_done+e))
                done < "$f"
            fi
        done

        # Threads still alive?
        alive=0
        for p in "${pids[@]}"; do
            if kill -0 $p 2>/dev/null; then alive=$((alive+1)); fi
        done

        qps=$(python3 -c "print(f'{queries_done/max($elapsed,1):.1f}')" 2>/dev/null || echo "0")
        echo "$(date +%s),$elapsed,$queries_done,$errors_done,$rss,$fd,$qps" >> "$csv_file"
        samples=$((samples+1))
        sleep 5
    done

    # Wait for threads
    for p in "${pids[@]}"; do wait $p 2>/dev/null; done

    # Final counts
    final_queries=0
    final_errors=0
    for f in /tmp/soak_thread_${label}.out; do
        if [ -f "$f" ]; then
            while IFS=: read -r q e; do
                final_queries=$((final_queries+q))
                final_errors=$((final_errors+e))
            done < "$f"
            rm -f "$f"
        fi
    done

    end_rss=$(get_rss)
    end_fd=$(get_fd)
    actual_duration=$(($(date +%s) - start_time))
    rss_growth=$((end_rss - start_rss))
    final_qps=$(python3 -c "print(f'{$final_queries/max($actual_duration,1):.1f}')" 2>/dev/null || echo "0")

    # Kill server
    kill $pid 2>/dev/null; wait $pid 2>/dev/null; rm -rf "$data_dir"

    # Write summary
    cat > "$REPORT_DIR/${label}_summary.txt" << EOF
Label:      $label
Duration:   ${actual_duration}s (target: ${duration}s)
Threads:    $threads
QPS:        ${final_qps}
Queries:    $final_queries
Errors:     $final_errors
Error rate: $(python3 -c "print(f'{$final_errors/max($final_queries,1)*100:.2f}%')" 2>/dev/null || echo "N/A")
RSS start:  ${start_rss} KB
RSS end:    ${end_rss} KB
RSS growth: ${rss_growth} KB
FD start:   $start_fd
FD end:     $end_fd
PASS:      $([ $final_errors -eq 0 ] && echo "YES" || echo "NO (errors>0)")
EOF

    cat "$REPORT_DIR/${label}_summary.txt"
    echo "---"
}

# Quick smoke first
echo ""
echo "=== Smoke: 60 seconds, 1 thread ==="
run_one 60 1 5 "smoke_60s_1t"

# Ladder
total=0; passed=0
for dur in "${DURATIONS[@]}"; do
    dur_min=$((dur/60))
    for threads in "${THREADS[@]}"; do
        label="soak_${dur_min}m_${threads}t"
        total=$((total+1))
        echo ""
        echo "=== [$total/25] $label ==="
        if run_one $dur $threads $QPS_PER_THREAD "$label"; then
            passed=$((passed+1))
        else
            echo "FAILED: $label"
        fi
    done
done

echo ""
echo "=============================================="
echo "RESULTS: $passed/$total PASSED"
echo "=============================================="
echo ""
echo "Summary table:"
printf "%-6s %6s" "Dur" "Thrds"
for t in "${THREADS[@]}"; do printf " %8s" "t=$t"; done
echo ""
for dur in "${DURATIONS[@]}"; do
    dur_min=$((dur/60))
    printf "%4dm   " $dur_min
    for threads in "${THREADS[@]}"; do
        label="soak_${dur_min}m_${threads}t"
        summary="$REPORT_DIR/${label}_summary.txt"
        if [ -f "$summary" ]; then
            errors=$(grep "^Errors:" "$summary" | awk '{print $2}')
            rss=$(grep "^RSS growth:" "$summary" | awk '{print $3}')
            pass=$(grep "^PASS:" "$summary" | awk '{print $2}')
            if [ "$pass" = "YES" ]; then
                printf " %7s" "OK(${rss}KB)"
            else
                printf " %7s" "ERR(${errors})"
            fi
        else
            printf " %7s" "SKIP"
        fi
    done
    echo ""
done
