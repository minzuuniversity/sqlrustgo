#!/bin/bash
#===============================================================================
# AI Deviation Accuracy Test (G8 Gate)
# Issue: #1361
# Requirement: AI Deviation Accuracy ≥ 90%
#===============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Configuration
MIN_ACCURACY=90
TEST_DATA_DIR="$PROJECT_ROOT/test-data/deviation"
RESULT_FILE="$SCRIPT_DIR/deviation_accuracy_result.json"

echo "=== AI Deviation Accuracy Test (G8 Gate) ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Requirement: Accuracy ≥ $MIN_ACCURACY%"
echo ""

# Check if test data exists
if [ ! -d "$TEST_DATA_DIR" ]; then
    echo "⚠️  Test data directory not found: $TEST_DATA_DIR"
    echo "Creating sample test data..."
    mkdir -p "$TEST_DATA_DIR"
    
    # Create sample test cases
    cat > "$TEST_DATA_DIR/test_cases.json" << 'TESTDATA'
[
  {
    "id": "dev_001",
    "description": "批次 #A001 温度超标，设定值 20-25°C，实际 28°C",
    "expected_root_cause": "温控系统故障"
  },
  {
    "id": "dev_002", 
    "description": "批次 #A002 湿度超标，设定值 45-55%，实际 62%",
    "expected_root_cause": "除湿设备故障"
  },
  {
    "id": "dev_003",
    "description": "批次 #A003 压力异常，设定值 1.0-1.2 bar，实际 0.8 bar",
    "expected_root_cause": "压力传感器漂移"
  }
]
TESTDATA
fi

# Load test cases
TEST_CASES=$(cat "$TEST_DATA_DIR/test_cases.json")
TOTAL=$(echo "$TEST_CASES" | jq length)
PASS=0
FAIL=0

echo "Loaded $TOTAL test cases"
echo ""

# Run each test case
for i in $(seq 0 $((TOTAL - 1))); do
    CASE=$(echo "$TEST_CASES" | jq ".[$i]")
    ID=$(echo "$CASE" | jq -r ".id")
    DESC=$(echo "$CASE" | jq -r ".description")
    EXPECTED=$(echo "$CASE" | jq -r ".expected_root_cause")
    
    echo "--- Test Case: $ID ---"
    echo "Description: $DESC"
    echo "Expected Root Cause: $EXPECTED"
    
    # Call AI Deviation Investigator API
    RESPONSE=$(curl -s -X POST "http://localhost:8080/api/v1/gmp/deviation/investigate" \
        -H "Content-Type: application/json" \
        -d "{\"description\": \"$DESC\"}" || echo "{}")
    
    ACTUAL=$(echo "$RESPONSE" | jq -r ".root_cause // empty")
    
    if [ -z "$ACTUAL" ]; then
        echo "Result: FAIL (no response)"
        FAIL=$((FAIL + 1))
    elif echo "$ACTUAL" | grep -q "$EXPECTED"; then
        echo "Result: PASS"
        PASS=$((PASS + 1))
    else
        echo "Result: FAIL (expected: $EXPECTED, got: $ACTUAL)"
        FAIL=$((FAIL + 1))
    fi
    echo ""
done

# Calculate accuracy
ACCURACY=$((PASS * 100 / TOTAL))

echo "=== Results ==="
echo "Total: $TOTAL"
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo "Accuracy: $ACCURACY%"

# Save result
cat > "$RESULT_FILE" << EOF
{
  "test": "AI Deviation Accuracy",
  "gate": "G8",
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "total": $TOTAL,
  "passed": $PASS,
  "failed": $FAIL,
  "accuracy": $ACCURACY,
  "threshold": $MIN_ACCURACY,
  "status": "$([ $ACCURACY -ge $MIN_ACCURACY ] && echo PASS || echo FAIL)"
}
EOF

echo ""
echo "Result saved to: $RESULT_FILE"

# Exit with appropriate code
if [ $ACCURACY -ge $MIN_ACCURACY ]; then
    echo "✅ G8 PASS"
    exit 0
else
    echo "❌ G8 FAIL (accuracy $ACCURACY% < $MIN_ACCURACY%)"
    exit 1
fi
