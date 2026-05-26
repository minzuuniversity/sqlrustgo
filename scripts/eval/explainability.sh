#!/bin/bash
#===============================================================================
# AI Explainability Test (G9 Gate)
# Issue: #1362
# Requirement: 100% with evidence chain
#===============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
REQUIRED_EVIDENCE_RATE=100
TEST_DATA_DIR="$PROJECT_ROOT/test-data/compliance"
RESULT_FILE="$SCRIPT_DIR/explainability_result.json"

echo "=== AI Explainability Test (G9 Gate) ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Requirement: Evidence chain rate ≥ $REQUIRED_EVIDENCE_RATE%"
echo ""

# Check if test data exists
if [ ! -d "$TEST_DATA_DIR" ]; then
    echo "⚠️  Test data directory not found: $TEST_DATA_DIR"
    echo "Creating sample test data..."
    mkdir -p "$TEST_DATA_DIR"
    
    # Create sample compliance test cases
    cat > "$TEST_DATA_DIR/compliance_cases.json" << 'TESTDATA'
[
  {
    "id": "comp_001",
    "batch_id": "BATCH001",
    "description": "批次 #BATCH001 偏差调查报告",
    "rules": ["温度超标", "超时"]
  },
  {
    "id": "comp_002",
    "batch_id": "BATCH002", 
    "description": "批次 #BATCH002 常规放行检查",
    "rules": ["参数正常"]
  }
]
TESTDATA
fi

# Load test cases
TEST_CASES=$(cat "$TEST_DATA_DIR/compliance_cases.json")
TOTAL=$(echo "$TEST_CASES" | jq length)
WITH_EVIDENCE=0
WITHOUT_EVIDENCE=0

echo "Loaded $TOTAL test cases"
echo ""

# Run each test case
for i in $(seq 0 $((TOTAL - 1))); do
    CASE=$(echo "$TEST_CASES" | jq ".[$i]")
    ID=$(echo "$CASE" | jq -r ".id")
    BATCH=$(echo "$CASE" | jq -r ".batch_id")
    DESC=$(echo "$CASE" | jq -r ".description")
    
    echo "--- Test Case: $ID ---"
    echo "Batch: $BATCH"
    echo "Description: $DESC"
    
    # Call AI Compliance Judge API
    RESPONSE=$(curl -s -X POST "http://localhost:8080/api/v1/gmp/compliance/judge" \
        -H "Content-Type: application/json" \
        -d "{\"batch_id\": \"$BATCH\", \"description\": \"$DESC\"}" || echo "{}")
    
    # Check if response has evidence chain
    EVIDENCE=$(echo "$RESPONSE" | jq -r ".evidence_chain // empty")
    JUSTIFICATION=$(echo "$RESPONSE" | jq -r ".justification // empty")
    
    if [ -n "$EVIDENCE" ] && [ -n "$JUSTIFICATION" ]; then
        echo "Evidence Chain: ✅ Found"
        echo "Evidence: $EVIDENCE"
        echo "Justification: $JUSTIFICATION"
        WITH_EVIDENCE=$((WITH_EVIDENCE + 1))
    else
        echo "Evidence Chain: ❌ Missing"
        WITHOUT_EVIDENCE=$((WITHOUT_EVIDENCE + 1))
    fi
    echo ""
done

# Calculate rate
EVIDENCE_RATE=$((WITH_EVIDENCE * 100 / TOTAL))

echo "=== Results ==="
echo "Total: $TOTAL"
echo "With Evidence: $WITH_EVIDENCE"
echo "Without Evidence: $WITHOUT_EVIDENCE"
echo "Evidence Rate: $EVIDENCE_RATE%"

# Save result
cat > "$RESULT_FILE" << EOF
{
  "test": "AI Explainability",
  "gate": "G9",
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "total": $TOTAL,
  "with_evidence": $WITH_EVIDENCE,
  "without_evidence": $WITHOUT_EVIDENCE,
  "evidence_rate": $EVIDENCE_RATE,
  "threshold": $REQUIRED_EVIDENCE_RATE,
  "status": "$([ $EVIDENCE_RATE -ge $REQUIRED_EVIDENCE_RATE ] && echo PASS || echo FAIL)"
}
EOF

echo ""
echo "Result saved to: $RESULT_FILE"

# Exit with appropriate code
if [ $EVIDENCE_RATE -ge $REQUIRED_EVIDENCE_RATE ]; then
    echo "✅ G9 PASS"
    exit 0
else
    echo "❌ G9 FAIL (evidence rate $EVIDENCE_RATE% < $REQUIRED_EVIDENCE_RATE%)"
    exit 1
fi
