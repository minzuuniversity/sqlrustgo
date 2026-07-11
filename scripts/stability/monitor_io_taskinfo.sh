#!/bin/bash
# monitor_io_taskinfo.sh - Disk I/O via taskinfo (sudo required)
# Uses python3 for unit-safe parsing

INTERVAL_MIN=${1:-5}
SERVER_PID=${2:-$(lsof -ti :13306 2>/dev/null | head -1)}

if [ -z "$SERVER_PID" ]; then
    echo "FATAL: no server PID found" >&2
    exit 1
fi

DIR=$(ls -td test_results/wired_soak_72h_* 2>/dev/null | head -1)
REPORT_FILE="$DIR/io_reports.log"

if ! echo "details8488" | sudo -S true 2>/dev/null; then
    echo "FATAL: sudo failed" >&2
    exit 1
fi

PREV_READ=0
PREV_WRITE=0

echo "Writing IO reports to $REPORT_FILE every ${INTERVAL_MIN}m (server PID $SERVER_PID)" | tee -a "$REPORT_FILE"

while true; do
    TS=$(date '+%Y-%m-%d %H:%M:%S')
    
    # taskinfo + python parse for unit safety
    PARSED=$(echo "details8488" | sudo -S taskinfo $SERVER_PID 2>/dev/null | python3 -c "
import sys, re
for line in sys.stdin:
    if line.startswith('bytes read:'):
        # bytes read: 14.35 MiB written: 2.19 GiB logical writes: 3.44 GiB
        m = re.search(r'bytes read: ([\d.]+) (\w+)\s+written: ([\d.]+) (\w+)', line)
        if m:
            mult = {'B':1, 'KiB':1024, 'MiB':1048576, 'GiB':1073741824, 'TiB':1024**4}
            rv, ru, wv, wu = float(m.group(1)), m.group(2), float(m.group(3)), m.group(4)
            print(f'{int(rv * mult[ru])} {int(wv * mult[wu])}')
            break
")
    
    if [ -z "$PARSED" ]; then
        echo "[$TS] taskinfo or parsing failed" >> "$REPORT_FILE"
        sleep $((INTERVAL_MIN * 60))
        continue
    fi
    
    BYTES_READ=$(echo "$PARSED" | awk '{print $1}')
    BYTES_WRITTEN=$(echo "$PARSED" | awk '{print $2}')
    
    READ_DELTA=$((BYTES_READ - PREV_READ))
    WRITE_DELTA=$((BYTES_WRITTEN - PREV_WRITE))
    [ "$READ_DELTA" -lt 0 ] && READ_DELTA=0
    [ "$WRITE_DELTA" -lt 0 ] && WRITE_DELTA=0
    
    PREV_READ=$BYTES_READ
    PREV_WRITE=$BYTES_WRITTEN
    
    INTERVAL_S=$((INTERVAL_MIN * 60))
    
    {
        echo "=========================================================="
        echo "[$TS] IO Report (sudo taskinfo, PID $SERVER_PID)"
        echo "=========================================================="
        echo "  Cumulative: read=$(numfmt --to=iec $BYTES_READ 2>/dev/null || python3 -c "print(f'{$BYTES_READ/1024/1024:.2f} MiB')")  written=$(numfmt --to=iec $BYTES_WRITTEN 2>/dev/null || python3 -c "print(f'{$BYTES_WRITTEN/1024/1024/1024:.2f} GiB')")"
        echo "  Delta (${INTERVAL_MIN}m):"
        echo "    read:  $(python3 -c "print(f'{$READ_DELTA/1024/1024:.2f} MiB')")  ($(python3 -c "print(f'{$READ_DELTA/1024/$INTERVAL_S:.1f} KiB/s')"))"
        echo "    write: $(python3 -c "print(f'{$WRITE_DELTA/1024/1024:.2f} MiB')")  ($(python3 -c "print(f'{$WRITE_DELTA/1024/$INTERVAL_S:.1f} KiB/s')"))"
        echo ""
    } | tee -a "$REPORT_FILE"
    
    sleep $INTERVAL_S
done
