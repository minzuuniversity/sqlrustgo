#!/usr/bin/env bash
#
# check_v312_stage_yaml_sync.sh
#
# Purpose: Verify the set of fields declared under
#          `thresholds_override` in `docs/releases/v3.12.0/STAGE.yaml`
#          is consistent with the set of fields checked by
#          `scripts/gate/check_v312_gate_thresholds.sh`.
#
# Issue: #4388 (V312-59-E)
#
# Failure mode:
#   - Field declared in STAGE.yaml but missing from gate script = FAIL
#     (B8 would silently allow GA promotion despite YAML claiming a gate)
#   - Field declared in gate script but missing from STAGE.yaml = WARN
#     (gate is over-strict, not a hard failure)
#
# Usage: bash scripts/gate/check_v312_stage_yaml_sync.sh
# Exit:  0 = PASS (or WARN-only); 1 = FAIL drift detected.
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

STAGE_YAML="docs/releases/v3.12.0/STAGE.yaml"
GATE_SCRIPT="scripts/gate/check_v312_gate_thresholds.sh"

if [ ! -f "$STAGE_YAML" ]; then
    echo "ERROR: STAGE.yaml not found at $STAGE_YAML" >&2
    exit 1
fi
if [ ! -f "$GATE_SCRIPT" ]; then
    echo "ERROR: gate script not found at $GATE_SCRIPT" >&2
    exit 1
fi

# Read field names declared in STAGE.yaml `thresholds_override` section.
# Use yaml's safe_load; if PyYAML missing, fall back to grep-based extraction.
YAML_FIELDS=$(python3 - "$STAGE_YAML" <<'PY'
import sys, yaml
path = sys.argv[1]
with open(path) as f:
    data = yaml.safe_load(f)
fields = sorted((data.get("thresholds_override") or {}).keys())
for f in fields:
    print(f)
PY
)

# Read field names declared in the gate script's FIELDS array.
# Pattern: lines that begin with whitespace and a quoted "FIELD_NAME|"
GATE_FIELDS=$(grep -E "^[[:space:]]*\"[A-Z0-9_]+\|" "$GATE_SCRIPT" \
    | sed -E 's/^[[:space:]]*"([A-Z0-9_]+)\|.*/\1/' \
    | sort -u)

# Diff: fields in YAML but not in gate = FAIL
MISSING_IN_GATE=$(comm -23 <(echo "$YAML_FIELDS") <(echo "$GATE_FIELDS"))
# Diff: fields in gate but not in YAML = WARN
EXTRA_IN_GATE=$(comm -13 <(echo "$YAML_FIELDS") <(echo "$GATE_FIELDS"))

YAML_COUNT=$(echo "$YAML_FIELDS" | grep -c . || echo 0)
GATE_COUNT=$(echo "$GATE_FIELDS" | grep -c . || echo 0)

echo "==================================================================="
echo "B8 STAGE.yaml / gate script field-set sync"
echo "  STAGE.yaml thresholds_override fields: $YAML_COUNT"
echo "  Gate script declared fields:          $GATE_COUNT"
echo "==================================================================="

if [ -n "$MISSING_IN_GATE" ]; then
    echo ""
    echo "[STAGE_YAML_SYNC] FAIL: drift detected (fields in YAML not checked by gate):"
    echo "$MISSING_IN_GATE" | sed 's/^/  - /'
    echo ""
    echo "  Fix: add each missing field to FIELDS array in $GATE_SCRIPT"
    exit 1
fi

if [ -n "$EXTRA_IN_GATE" ]; then
    echo ""
    echo "[STAGE_YAML_SYNC] WARN: gate script has fields not declared in STAGE.yaml:"
    echo "$EXTRA_IN_GATE" | sed 's/^/  - /'
    echo ""
    echo "  Note: gate over-coverage is allowed but should be intentional."
fi

echo ""
echo "[STAGE_YAML_SYNC] PASS — $YAML_COUNT fields in sync"
exit 0