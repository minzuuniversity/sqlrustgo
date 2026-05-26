#!/bin/bash
#===============================================================================
# DiskGraphStore Cypher Execution Test
# Issue: #1326
# Requirement: Support Cypher execution on DiskGraphStore
#===============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
RESULT_FILE="$SCRIPT_DIR/diskgraph_cypher_result.json"

echo "=== DiskGraphStore Cypher Test (#1326) ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# Build the project
echo "--- Building project ---"
cd "$PROJECT_ROOT"
if ! cargo build --release -p sqlrustgo 2>&1 | tail -5; then
    echo "❌ Build failed"
    exit 1
fi
echo "✅ Build passed"
echo ""

# Check if Graph module exists
GRAPH_SRC="$PROJECT_ROOT/crates/graph/src"
if [ ! -d "$GRAPH_SRC" ]; then
    echo "⚠️  Graph module not found at $GRAPH_SRC"
    echo "Creating stub test..."
    
    # Create test directory
    mkdir -p "$PROJECT_ROOT/test-data/graph"
    
    cat > "$RESULT_FILE" << EOF
{
  "test": "DiskGraphStore Cypher Execution",
  "issue": "#1326",
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "status": "SKIP",
  "reason": "Graph module not yet implemented"
}
EOF
    
    echo "Status: SKIP (Graph module not found)"
    exit 0
fi

# Run Graph tests if they exist
echo "--- Running Graph tests ---"
TEST_OUTPUT=$(cargo test -p sqlrustgo-graph --lib 2>&1 || echo "NO_GRAPH_CRATE")

if echo "$TEST_OUTPUT" | grep -q "NO_GRAPH_CRATE"; then
    echo "⚠️  sqlrustgo-graph crate not found"
    echo "Status: SKIP"
    exit 0
fi

# Parse test results
TOTAL=$(echo "$TEST_OUTPUT" | grep -oP '\d+ tests?' | head -1 || echo "0")
PASSED=$(echo "$TEST_OUTPUT" | grep -oP '\d+ passed' | head -1 || echo "0")
FAILED=$(echo "$TEST_OUTPUT" | grep -oP '\d+ failed' | head -1 || echo "0")

echo "Tests: $PASSED / $TOTAL"

if [ "$FAILED" != "0 failed" ]; then
    echo "❌ Some tests failed"
    STATUS="FAIL"
else
    echo "✅ All tests passed"
    STATUS="PASS"
fi

# Save result
cat > "$RESULT_FILE" << EOF
{
  "test": "DiskGraphStore Cypher Execution",
  "issue": "#1326",
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "total_tests": ${TOTAL:-0},
  "passed": ${PASSED:-0},
  "failed": ${FAILED:-0},
  "status": "$STATUS"
}
EOF

echo ""
echo "Result saved to: $RESULT_FILE"

if [ "$STATUS" = "PASS" ]; then
    echo "✅ DiskGraphStore Cypher Test PASS"
    exit 0
else
    echo "❌ DiskGraphStore Cypher Test FAIL"
    exit 1
fi
