#!/bin/bash
# V312-37: Anti-Ignore gate (G19)
# Threshold: active entries <= 47, total_allowed <= 73
# Exit 0 = PASS, Exit 1 = FAIL
set -e

REGISTRY="tests/baseline/ignore_registry.json"
ACTIVE_MAX=47
TOTAL_ALLOWED_MAX=73

if [ ! -f "$REGISTRY" ]; then
    echo "FAIL: $REGISTRY not found" >&2
    exit 1
fi

# Read counts via python3 (avoid jq dependency)
read_counts() {
    python3 <<PYEOF
import json
with open("$REGISTRY") as f:
    data = json.load(f)
total_allowed = data.get("total_allowed", 0)
# Count entries that are not marked as RETIRED as active
entries = data.get("ignored_tests", [])
active = sum(1 for e in entries if "RETIRED" not in e.get("line", ""))
print(f"{active} {total_allowed}")
PYEOF
}

read_counts > /tmp/anti_ignore_counts.txt
ACTIVE=$(awk '{print $1}' /tmp/anti_ignore_counts.txt)
TOTAL_ALLOWED=$(awk '{print $2}' /tmp/anti_ignore_counts.txt)
rm -f /tmp/anti_ignore_counts.txt

echo "ignore_registry.json: total_allowed=$TOTAL_ALLOWED (max=$TOTAL_ALLOWED_MAX), active=$ACTIVE (max=$ACTIVE_MAX)"

if [ "$ACTIVE" -gt "$ACTIVE_MAX" ]; then
    echo "FAIL: active entries $ACTIVE > $ACTIVE_MAX" >&2
    exit 1
fi

if [ "$TOTAL_ALLOWED" -gt "$TOTAL_ALLOWED_MAX" ]; then
    echo "FAIL: total_allowed $TOTAL_ALLOWED > $TOTAL_ALLOWED_MAX" >&2
    exit 1
fi

exit 0
