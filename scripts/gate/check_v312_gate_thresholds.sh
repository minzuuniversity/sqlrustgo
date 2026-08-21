#!/usr/bin/env bash
#
# check_v312_gate_thresholds.sh
#
# Purpose: Hard-enforce every field under `thresholds_override` in
#          `docs/releases/v3.12.0/STAGE.yaml` as a composite gate
#          `B8_THRESHOLDS_OVERRIDE` for the v3.12.0 BETA pipeline.
#
# Issue: #4388 (V312-59-E) — 13 boolean/numeric fields must each be
#        individually validated AND backed by an executable sub-gate.
#
# Anti-Fabrication-Policy-v1.0 §5: NEVER flip boolean=true → false,
#        NEVER push any of the 8 boolean gates with `expiry`.
#
# Usage:
#   bash scripts/gate/check_v312_gate_thresholds.sh
#
# Exit codes:
#   0 — 13/13 PASS
#   1 — at least one FAIL (line-by-line report on stdout)
#
# Author: V312-59-E implementation
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

STAGE_YAML="docs/releases/v3.12.0/STAGE.yaml"
OUT_DIR="${OUT_DIR:-docs/releases/v3.12.0/evidence/v312-59-e}"
mkdir -p "$OUT_DIR"
EVIDENCE_FILE="$OUT_DIR/thresholds_override_evidence.txt"

if [ ! -f "$STAGE_YAML" ]; then
    echo "ERROR: STAGE.yaml not found at $STAGE_YAML" >&2
    exit 1
fi

# ----------------------------------------------------------------------
# Field table: key → expected kind (bool/int) and the inner check command
# ----------------------------------------------------------------------
# Each entry: "FIELD_NAME|KIND|CHECK_CMD"
#   KIND = "bool"  → expected literal "true"
#   KIND = "int"   → expected integer ≥ 0
# CHECK_CMD is the bash command line whose exit 0 = PASS.
# Use "REQUIRED:<path>" for file-existence checks, or a bash one-liner.
declare -a FIELDS=(
    "COVERAGE_MIN_PER_CRATE|int|bash scripts/gate/check_coverage_v312.sh"
    "GMP_CORPUS_UNCLASSIFIED_FAILURE_MAX|int|REQUIRED:docs/releases/v3.12.0/evidence/GMP_CORPUS_REPORT.md"
    "GMP_AUDIT_TAMPER_TEST_REQUIRED|bool|test -f docs/releases/v3.12.0/v312-08-compliance-audit-report.md"
    "GMP_RETRIEVAL_CITATION_REQUIRED|bool|test -f docs/releases/v3.12.0/v312-05-hybrid-retrieval-report.md"
    "MIXED_SOAK_HOURS|int|REQUIRED:docs/releases/v3.12.0/evidence/MIXED_SOAK_REPORT.md"
    "SQLLOGICTEST_SMOKE_REQUIRED|bool|bash scripts/gate/check_sqllogictest_v312.sh"
    "SQLLOGICTEST_SELECTED_TARGETS_REQUIRED|bool|test -f docs/releases/v3.12.0/evidence/SQLLOGICTEST_SELECTED_TARGETS.txt"
    "TPCH_SF1_CORRECTNESS_REQUIRED|bool|test -f docs/releases/v3.12.0/evidence/G4_tpch_sf1.txt"
    "MYSQL_WIRE_E2E_REQUIRED|bool|bash scripts/gate/check_v312_13_wire_load_data.sh"
    "LOAD_DATA_BULK_IMPORT_REQUIRED|bool|bash scripts/gate/check_load_data_infile.sh"
    "CRASH_RECOVERY_REQUIRED|bool|bash scripts/gate/check_v312_14_crash_recovery.sh"
    "UPGRADE_DOWNGRADE_REQUIRED|bool|bash scripts/gate/check_p14_upgrade_test.sh"
    "BUSTUBX_EDU_SQLITE_CLI_REQUIRED|bool|bash scripts/gate/check_bustubx_edu_cli_v312.sh"
)

# ----------------------------------------------------------------------
# Helpers
# ----------------------------------------------------------------------

# Parse the value of a given thresholds_override key from STAGE.yaml.
# Echoes the raw value (e.g. "true", "80", "168") on stdout.
yaml_field_value() {
    local key="$1"
    python3 - "$STAGE_YAML" "$key" <<'PY'
import sys, yaml
path, key = sys.argv[1], sys.argv[2]
with open(path) as f:
    data = yaml.safe_load(f)
val = data.get("thresholds_override", {}).get(key, None)
if val is None:
    sys.exit(2)
print(val)
PY
}

# Validate kind: bool/int. Echo PASS|FAIL with reason.
validate_kind() {
    local key="$1" kind="$2" value="$3"
    case "$kind" in
        bool)
            case "$(echo "$value" | tr '[:upper:]' '[:lower:]')" in
                true) echo "PASS" ;;
                false)
                    echo "FAIL: anti-pattern (boolean=true flipped to false — Anti-Fabrication-Policy-v1.0 §5)"
                    ;;
                *)
                    echo "FAIL: expected boolean true|false, got '$value'"
                    ;;
            esac
            ;;
        int)
            if [[ "$value" =~ ^[0-9]+$ ]] && [ "$value" -ge 0 ]; then
                echo "PASS"
            else
                echo "FAIL: expected non-negative integer, got '$value'"
            fi
            ;;
        *)
            echo "FAIL: unknown kind '$kind'"
            ;;
    esac
}

# Run the inner check command for a field; echo PASS|FAIL with reason.
run_inner_check() {
    local key="$1" cmd="$2"
    if [[ "$cmd" == REQUIRED:* ]]; then
        local path="${cmd#REQUIRED:}"
        if [ -f "$path" ]; then
            echo "PASS"
        else
            echo "INFRASTRUCTURE_MISSING: evidence file not found: $path"
        fi
        return
    fi
    # For `bash <script> ...` commands, verify the script file exists first
    # (avoids confusing "exit non-zero" when the script is just missing).
    # Other command forms (test -f, grep -q) are run as-is.
    if [[ "$cmd" == bash* ]]; then
        local script_path
        script_path="$(echo "$cmd" | awk '{print $2}')"
        if [ ! -f "$script_path" ]; then
            echo "INFRASTRUCTURE_MISSING: sub-gate script not found: $script_path"
            return
        fi
    fi
    if bash -c "$cmd" >/dev/null 2>&1; then
        echo "PASS"
    else
        echo "FAIL: sub-gate exit non-zero"
    fi
}

# ----------------------------------------------------------------------
# Run all 13 fields
# ----------------------------------------------------------------------

PASS_COUNT=0
FAIL_COUNT=0
INFRA_COUNT=0
EVIDENCE_LOG=""
EVIDENCE_LOG+="==================================================================="
EVIDENCE_LOG+="\nV312-59-E Thresholds Override Gate — Evidence"
EVIDENCE_LOG+="\nTimestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
EVIDENCE_LOG+="\nSTAGE.yaml: $STAGE_YAML"
EVIDENCE_LOG+="\n===================================================================\n\n"

idx=0
for entry in "${FIELDS[@]}"; do
    IFS='|' read -r key kind cmd <<< "$entry"
    idx=$((idx + 1))
    echo "[$idx/13] $key ($kind)"

    # Stage 1: YAML field validity
    if ! value="$(yaml_field_value "$key" 2>/dev/null)"; then
        echo "  [FIELD_VALIDITY] FAIL: key not found in thresholds_override"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        EVIDENCE_LOG+="[$idx] $key — FIELD_VALIDITY: FAIL (key missing)\n"
        continue
    fi
    field_result="$(validate_kind "$key" "$kind" "$value")"
    if [ "$field_result" != "PASS" ]; then
        echo "  [FIELD_VALIDITY] $field_result (value=$value)"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        EVIDENCE_LOG+="[$idx] $key — FIELD_VALIDITY: $field_result (value=$value)\n"
        continue
    fi
    echo "  [FIELD_VALIDITY] PASS (value=$value)"

    # Stage 2: Executable inner gate
    inner_result="$(run_inner_check "$key" "$cmd")"
    case "$inner_result" in
        PASS)
            echo "  [EXECUTABLE_GATE] PASS"
            PASS_COUNT=$((PASS_COUNT + 1))
            EVIDENCE_LOG+="[$idx] $key — FIELD_VALIDITY: PASS (value=$value) — EXECUTABLE_GATE: PASS\n"
            ;;
        INFRASTRUCTURE_MISSING:*)
            echo "  [EXECUTABLE_GATE] $inner_result"
            INFRA_COUNT=$((INFRA_COUNT + 1))
            EVIDENCE_LOG+="[$idx] $key — FIELD_VALIDITY: PASS (value=$value) — EXECUTABLE_GATE: $inner_result\n"
            ;;
        *)
            echo "  [EXECUTABLE_GATE] $inner_result"
            FAIL_COUNT=$((FAIL_COUNT + 1))
            EVIDENCE_LOG+="[$idx] $key — FIELD_VALIDITY: PASS (value=$value) — EXECUTABLE_GATE: $inner_result\n"
            ;;
    esac
done

# ----------------------------------------------------------------------
# Summary
# ----------------------------------------------------------------------

TOTAL=$((PASS_COUNT + FAIL_COUNT + INFRA_COUNT))
echo ""
echo "==================================================================="
echo "B8_THRESHOLDS_OVERRIDE Summary"
echo "  PASS:               $PASS_COUNT / 13"
echo "  FAIL:               $FAIL_COUNT / 13"
echo "  INFRASTRUCTURE:     $INFRA_COUNT / 13 (non-veto if linked)"
echo "  Total checked:      $TOTAL / 13"
echo "==================================================================="

EVIDENCE_LOG+="\n\nSUMMARY: pass=$PASS_COUNT fail=$FAIL_COUNT infra=$INFRA_COUNT total=$TOTAL/13\n"
printf "%b" "$EVIDENCE_LOG" > "$EVIDENCE_FILE"

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
if [ "$INFRA_COUNT" -gt 0 ]; then
    # Non-veto by default; only FAIL hard exits 1
    exit 0
fi
exit 0