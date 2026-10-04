#!/usr/bin/env bash
# #4956 A/B runner.
#
# One server per (binary, round). Fresh data dir per run so the two sides
# never share a snapshot/WAL state. A/B order alternates per round so a
# monotonic machine drift (thermal, page cache, background agent) cannot
# masquerade as a side effect.
set -uo pipefail

BIN="$1"; ROUND="$2"; PORT="$3"; SCENARIO="$4"; THREADS="$5"; OPS="$6"; ROWS="$7"
RUN=/tmp/perf4956/run_${SCENARIO}_${THREADS}t_${ROWS}r
rm -rf "$RUN"; mkdir -p "$RUN"

"$BIN" serve --host 127.0.0.1 --port "$PORT" --data-dir "$RUN/data" \
      --auth-mode none --storage file --wal-sync every \
      --server-threads 32 > "$RUN/server.log" 2>&1 &
SRV=$!

# Wait for the port to accept connections.
for _ in $(seq 1 100); do
  if nc -z 127.0.0.1 "$PORT" 2>/dev/null; then break; fi
  sleep 0.2
done
sleep 1.0

# Warmup: same binary, discarded. Lets the page cache and the connection
# pool settle so round 1 is not systematically worse for whichever side
# happened to run first.
/tmp/perf4956/driver/target/release/perf4956 "$PORT" "$SCENARIO" 1 50 "$ROWS" > /dev/null 2>&1

OUT=$(/tmp/perf4956/driver/target/release/perf4956 "$PORT" "$SCENARIO" "$THREADS" "$OPS" "$ROWS" 2>&1 | grep '^RESULT|')
echo "$OUT"

kill "$SRV" 2>/dev/null
wait "$SRV" 2>/dev/null
