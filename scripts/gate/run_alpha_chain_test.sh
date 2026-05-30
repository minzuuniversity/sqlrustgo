#!/bin/bash
# v3.8.0 Alpha Gate — Evidence Graph Chain Test
# 验证 gate.yml workflow 的 evidence chain 完整性

set -e

REPO="${REPO:-/home/ai/sqlrustgo}"
cd "$REPO"

TIMESTAMP=$(date +%s)
DB="${EG_DB:-/tmp/eg_alpha_${TIMESTAMP}.db}"
COMMIT_SHA=$(git rev-parse HEAD)
TASK_ID="v3.8.0-alpha"
CI_ID="alpha-check-${TIMESTAMP}"
ARTIFACT_ID="v3.8.0-alpha-artifact"

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

# Ingest nodes: use short hash for node_id (first 8 chars)
COMMIT_NODE="commit_$(echo $COMMIT_SHA | cut -c1-8)"
CI_NODE="ci_${CI_ID}"
ART_NODE="artifact_${ARTIFACT_ID}"

# Ingest nodes
echo "[2/10] Ingesting task..."
TASK_RESULT=$($INGEST_BIN task "$TASK_ID" "v3.8.0 Alpha Gate" --db "$DB")
echo "$TASK_RESULT"

echo "[3/10] Ingesting commit..."
COMMIT_RESULT=$($INGEST_BIN commit "$COMMIT_SHA" "openclaw" "alpha evidence" --db "$DB")
echo "$COMMIT_RESULT"

echo "[4/10] Ingesting CI..."
CI_RESULT=$($INGEST_BIN ci "$CI_ID" "$COMMIT_SHA" PASS --db "$DB")
echo "$CI_RESULT"

echo "[5/10] Ingesting artifact..."
ARTIFACT_RESULT=$($INGEST_BIN artifact "$ARTIFACT_ID" "$CI_ID" "binary/gate" "e3a4c28b" --db "$DB")
echo "$ARTIFACT_RESULT"

# Create links using actual node IDs
# Chain: task IMPLEMENTED_BY commit, commit PRODUCES ci, ci VERIFIED_BY artifact
echo "[6/10] Linking task→commit..."
$INGEST_BIN link "$TASK_ID" IMPLEMENTED_BY "$COMMIT_NODE" --db "$DB"

echo "[7/10] Linking commit→CI..."
$INGEST_BIN link "$COMMIT_NODE" PRODUCES "$CI_NODE" --db "$DB"

echo "[8/10] Linking CI→artifact..."
$INGEST_BIN link "$CI_NODE" VERIFIED_BY "$ART_NODE" --db "$DB"

# Evaluate: evaluate <TASK_ID>
echo "[9/10] Gate evaluate..."
RESULT=$($GATE_BIN evaluate "$TASK_ID" --db "$DB" 2>&1)
echo "Result: $RESULT"

echo "[10/10] Status..."
$GATE_BIN status --db "$DB" 2>&1

# Cleanup
echo ""
echo "=== Result ==="
# Parse result from JSON (handle compact single-line output)
echo "$RESULT" | python3 -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print('PASS' if d.get('result') == 'PASS' else f\"FAIL ({d.get('reason')})\")
    print(f\"  Missing: {d.get('missing', [])}\")
except Exception as e:
    print(f'FAIL: parse error: {e}')
"

# Keep DB for debugging if FAIL
if ! echo "$RESULT" | python3 -c "import sys,json; sys.exit(0 if json.loads(sys.stdin.read()).get('result')=='PASS' else 1)" 2>/dev/null; then
    echo "DB kept at: $DB (for debugging)"
else
    rm -f "$DB"
fi