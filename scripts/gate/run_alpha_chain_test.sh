#!/bin/bash
# v3.8.0 Alpha Gate — Evidence Graph Chain Test
# 用途：验证 gate.yml 实际执行 evidence chain

set -e

REPO="${REPO:-/home/ai/sqlrustgo}"
cd "$REPO"

TIMESTAMP=$(date +%s)
DB="/tmp/eg_alpha_${TIMESTAMP}.db"
COMMIT_SHA=$(git rev-parse HEAD)
TASK_NAME="v3.8.0-alpha"
CI_NAME="alpha-check-${TIMESTAMP}"
ARTIFACT_NAME="v3.8.0-alpha-artifact"

echo "=== Alpha Evidence Chain Test ==="
echo "DB: $DB"
echo "Commit: $COMMIT_SHA"
echo ""

# Build
echo "[1/10] Building graph-cli..."
cargo build -p graph-cli --release 2>&1 | tail -2
GATE_BIN="$REPO/target/release/gate"
INGEST_BIN="$REPO/target/release/ingest"

if [ ! -f "$GATE_BIN" ] || [ ! -f "$INGEST_BIN" ]; then
    echo "FAIL: Binary not found"
    exit 1
fi

# Ingest nodes
echo "[2/10] Ingesting task..."
$INGEST_BIN task "$TASK_NAME" --branch develop/v3.8.0 --db "$DB"

echo "[3/10] Ingesting commit..."
$INGEST_BIN commit "$COMMIT_SHA" --author "openclaw" --db "$DB"

echo "[4/10] Ingesting CI..."
$INGEST_BIN ci "$CI_NAME" --status PASS --db "$DB"

echo "[5/10] Ingesting artifact..."
$INGEST_BIN artifact "$ARTIFACT_NAME" --db "$DB"

# Create links
echo "[6/10] Linking task→commit..."
$INGEST_BIN link task --task "$TASK_NAME" --commit "$COMMIT_SHA" --db "$DB"

echo "[7/10] Linking commit→CI..."
$INGEST_BIN link ci --commit "$COMMIT_SHA" --ci "$CI_NAME" --db "$DB"

echo "[8/10] Linking CI→artifact..."
$INGEST_BIN link artifact --ci "$CI_NAME" --artifact "$ARTIFACT_NAME" --db "$DB"

# Evaluate
echo "[9/10] Gate evaluate..."
RESULT=$($GATE_BIN evaluate --task "$TASK_NAME" --db "$DB" 2>&1)
echo "Result: $RESULT"

echo "[10/10] Status..."
$GATE_BIN status --db "$DB" 2>&1

# Cleanup
echo ""
echo "=== Result ==="
echo "$RESULT" | grep -q '"result":"PASS"' && echo "PASS" || echo "FAIL"

# Keep DB for debugging if FAIL
if ! echo "$RESULT" | grep -q '"result":"PASS"'; then
    echo "DB kept at: $DB"
else
    rm -f "$DB"
fi