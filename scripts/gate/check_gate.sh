#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
PHASE="${2:-}"

show_usage() {
    cat <<EOF
Usage: check_gate.sh <VERSION> <PHASE>

Examples:
  check_gate.sh v3.4.0 beta
  check_gate.sh v3.3.0 rc
  check_gate.sh v2.9.0 ga

Phases: alpha, beta, rc, ga
EOF
}

if [ -z "$VERSION" ] || [ -z "$PHASE" ]; then
    show_usage
    exit 1
fi

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PHASE_UPPER=$(echo "$PHASE" | tr '[:lower:]' '[:upper:]')
GATE_DOC="$PROJECT_ROOT/docs/releases/${VERSION}/${PHASE_UPPER}_GATE_CHECKLIST.md"
VERSION_NUM=$(echo "$VERSION" | sed 's/v//' | sed 's/\.//g')
GATE_SCRIPT="$PROJECT_ROOT/scripts/gate/check_${PHASE}_v${VERSION_NUM}.sh"

echo "=== Generic Gate Runner ==="
echo "Version: $VERSION"
echo "Phase: $PHASE"
echo "Gate doc: $GATE_DOC"
echo "Gate script: $GATE_SCRIPT"
echo ""

if [ ! -f "$GATE_DOC" ]; then
    echo "ERROR: Gate doc not found: $GATE_DOC"
    exit 1
fi

if [ ! -f "$GATE_SCRIPT" ]; then
    echo "ERROR: Gate script not found: $GATE_SCRIPT"
    echo "Creating stub script..."
    cat > "$GATE_SCRIPT" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
echo "Gate script not yet implemented for this version/phase"
exit 1
STUB
    chmod +x "$GATE_SCRIPT"
    exit 1
fi

echo "Executing gate script..."
bash "$GATE_SCRIPT"

GATE_RESULT=$?

if [ $GATE_RESULT -eq 0 ]; then
    echo ""
    echo "Gate PASSED: $VERSION $PHASE"
    REPORT_FILE="$PROJECT_ROOT/gate_results_${VERSION#v}_${PHASE}.json"
    cat > "$REPORT_FILE" <<EOF
{
    "gate": "$PHASE",
    "version": "$VERSION",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "status": "PASSED",
    "script": "$(basename "$GATE_SCRIPT")"
}
EOF
    echo "Report: $REPORT_FILE"
else
    echo ""
    echo "Gate FAILED: $VERSION $PHASE"
fi

exit $GATE_RESULT
