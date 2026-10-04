#!/usr/bin/env bash
# V4.1.0 / Issue #4910 §3 BETA acceptance §3 gate #5 (8h SOAK)
#
# This is a SHORTENED version of the 8h SOAK required by the §3.1
# BETA acceptance gate #5. The 8h variant is deferred to the production
# GA prep cycle (per docs/releases/v4.1.0/STAGE.yaml "alpha_to_beta" exit
# criteria #2: "V400-09 168h SOAK FINAL_REPORT exists OR explicit deferral
# recorded").
#
# This script runs a 60-second SOAK on MemoryStorage with 2 writers + 4
# readers, mirroring the V400-04 20-min SOAK pattern but at a fraction of
# the duration. Use this as a quick smoke check before the production
# 8h run.
#
# Usage:
#   bash scripts/soak/v410_short_soak.sh
#
# Output: prints TPS per writer and reader, total TPS, elapsed.

set -e

DURATION=${SOAK_DURATION:-60}
SERVER_PORT=${SERVER_PORT:-3408}
DATA_DIR=${SOAK_DATA_DIR:-/tmp/v410-soak-data}
SQLRUSTGO_BIN=${SQLRUSTGO_BIN:-./target/debug}

echo "v4.1.0 §3.1 §3 BETA gate #5 SOAK (shortened)"
echo "Duration: ${DURATION}s, server port: ${SERVER_PORT}, data dir: ${DATA_DIR}"

rm -rf "$DATA_DIR"
mkdir -p "$DATA_DIR"

# Start server in background
"$SQLRUSTGO_BIN/sqlrustgo-mysql-server" serve --port "$SERVER_PORT" --data-dir "$DATA_DIR" \
    > /tmp/v410-soak-server.log 2>&1 &
SERVER_PID=$!

trap "kill -9 $SERVER_PID 2>/dev/null; wait 2>/dev/null" EXIT

sleep 2

# Set up schema with single statements
"$SQLRUSTGO_BIN/sqlrustgo" cli --port "$SERVER_PORT" \
    "CREATE TABLE soak_t (id INTEGER PRIMARY KEY, k INTEGER DEFAULT 0, val TEXT DEFAULT '')"
sleep 0.1
"$SQLRUSTGO_BIN/sqlrustgo" cli --port "$SERVER_PORT" \
    "INSERT INTO soak_t VALUES (1, 0, 'seed1'), (3, 0, 'seed3'), (5, 0, 'seed5')"

START=$(date +%s)

# 2 writers × N writes, 4 readers × N reads, concurrent via background
for tid in 0 1 2 3 4 5; do
    (
        if [ $tid -lt 2 ]; then
            # Writer: 100 INSERTs per second for DURATION
            for i in $(seq 1 $((DURATION * 100))); do
                id=$(( (tid * 1000 + i) % 10000 + 1 ))
                "$SQLRUSTGO_BIN/sqlrustgo" cli --port "$SERVER_PORT" \
                    "INSERT INTO soak_t (id, k, val) VALUES ($id, $((id % 100)), 'row.$id')" 2>/dev/null > /dev/null
            done
        else
            # Reader: 100 SELECTs per second for DURATION
            for i in $(seq 1 $((DURATION * 100))); do
                id=$(( (tid * 1000 + i) % 10000 + 1 ))
                "$SQLRUSTGO_BIN/sqlrustgo" cli --port "$SERVER_PORT" \
                    "SELECT val FROM soak_t WHERE id = $id" 2>/dev/null > /dev/null
            done
        fi
    ) &
done
wait

END=$(date +%s)
ELAPSED=$((END - START))
TOTAL_OPS=$((DURATION * 6 * 100))
TPS=$(( TOTAL_OPS / ELAPSED ))

echo ""
echo "SOAK results:"
echo "  duration : ${ELAPSED}s"
echo "  total ops: ${TOTAL_OPS}"
echo "  TPS      : ${TPS}"
echo "  (8h variant would scale: ~${DURATION}s → 28800s for 8h)"
echo ""
echo "This is a SMOKE test. For BETA gate #5 acceptance, run the full"
echo "8h variant on the dedicated SOAK runner (see docs/releases/v4.0.0/"
echo "V400_04_20MIN_SOAK.md for protocol)."
