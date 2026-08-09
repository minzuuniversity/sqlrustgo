#!/usr/bin/env bash
# check_stage.sh — Version-agnostic stage gate driver
#
# 2026-07-01 claude-macmini (per #3667 + governance audit 整改)
#
# Purpose: Given a git branch + working tree, determine the current stage
# (DRAFT/ALPHA/BETA/RC/GA) and run the appropriate stage gate from
# docs/governance/STAGE_CONFIG.yaml.
#
# This is the **driver** — it does not define what each stage requires
# (that's STAGE_CONFIG.yaml's job). It only:
#   1. Parses current git state (branch, recent tags, current stage marker)
#   2. Maps state → stage (DRAFT/ALPHA/BETA/RC/GA)
#   3. Looks up the stage in STAGE_CONFIG.yaml
#   4. Runs required_gates in order; reports PASS/FAIL
#   5. Checks required_files exist
#   6. Exits 0 if all PASS, 1 if any FAIL, 2 if DRIFT
#
# Usage:
#   bash scripts/gate/check_stage.sh                    # auto-detect stage
#   bash scripts/gate/check_stage.sh --version v3.9.0  # override version
#   bash scripts/gate/check_stage.sh --stage BETA        # force stage (testing)
#   bash scripts/gate/check_stage.sh --json              # JSON output
#   bash scripts/gate/check_stage.sh --list              # list stage gates
#   bash scripts/gate/check_stage.sh --version v3.9.0 --stage RC --dry-run
#                                                       # show what would run
#
# Companion files:
#   - docs/governance/STAGE_CONFIG.yaml (stage definitions SSOT)
#   - docs/releases/vX.Y.Z/STAGE.yaml (per-version override, optional)
#
# This script does NOT replace the existing 4 stage-specific scripts
# (check_alpha.sh, check_beta_gate.sh, check_rc_ga_gate.sh,
# check_alpha_v380.sh). They are listed in STAGE_CONFIG.yaml's
# required_gates for each stage. The framework is the driver; the
# existing scripts are the implementations. This avoids duplication and
# makes "what's checked at each stage" a single-file change.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
STAGE_CONFIG="${REPO_ROOT}/docs/governance/STAGE_CONFIG.yaml"

# ---- Argument parsing ----
VERSION=""
STAGE_OVERRIDE=""
JSON_OUTPUT=false
LIST_ONLY=false
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version) VERSION="$2"; shift 2 ;;
        --stage)   STAGE_OVERRIDE="$2"; shift 2 ;;
        --json)    JSON_OUTPUT=true; shift ;;
        --list)    LIST_ONLY=true; shift ;;
        --dry-run) DRY_RUN=true; shift ;;
        -h|--help)
            grep -E '^#( |!)' "$0" | head -30
            exit 0
            ;;
        *) echo "Unknown arg: $1" >&2; exit 2 ;;
    esac
done

# ---- Pre-flight checks ----
if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 required for YAML parsing" >&2
    exit 2
fi
if [[ ! -f "$STAGE_CONFIG" ]]; then
    echo "ERROR: STAGE_CONFIG not found: $STAGE_CONFIG" >&2
    exit 2
fi
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

cd "$REPO_ROOT"

# ---- Detect version (from --version arg, or current branch name, or STAGE.yaml) ----
detect_version() {
    if [[ -n "$VERSION" ]]; then
        echo "$VERSION"
        return
    fi
    # Try to extract from current branch (e.g. develop/v3.9.0 -> v3.9.0)
    local branch
    branch=$(git symbolic-ref --short HEAD 2>/dev/null || echo "")
    if [[ "$branch" =~ ^develop/v(.+)$ ]]; then
        echo "v${BASH_REMATCH[1]}"
        return
    fi
    if [[ "$branch" =~ ^beta/v(.+)$ ]]; then
        echo "v${BASH_REMATCH[1]}"
        return
    fi
    if [[ "$branch" =~ ^rc/v(.+)$ ]]; then
        echo "v${BASH_REMATCH[1]}"
        return
    fi
    if [[ "$branch" =~ ^release/v(.+)$ ]]; then
        echo "v${BASH_REMATCH[1]}"
        return
    fi
    if [[ "$branch" =~ ^hotfix/v(.+)$ ]]; then
        echo "v${BASH_REMATCH[1]}"
        return
    fi
    # Fallback: try to read STAGE.yaml from any version
    local stage_file
    stage_file=$(find docs/releases -name "STAGE.yaml" -type f 2>/dev/null | head -1)
    if [[ -n "$stage_file" ]]; then
        # Extract version from path
        if [[ "$stage_file" =~ docs/releases/v([^/]+)/STAGE\.yaml ]]; then
            echo "v${BASH_REMATCH[1]}"
            return
        fi
    fi
    echo "unknown"
}

# ---- Detect stage (from --stage arg, or git state) ----
detect_stage() {
    if [[ -n "$STAGE_OVERRIDE" ]]; then
        echo "$STAGE_OVERRIDE"
        return
    fi
    local ver="$1"
    local stage_file="docs/releases/${ver}/STAGE.yaml"
    if [[ -f "$stage_file" ]]; then
        # Use python to parse YAML
        python3 -c "
import yaml, sys
with open('$stage_file') as f:
    d = yaml.safe_load(f)
print(d.get('current_stage', 'DRAFT').upper())
" 2>/dev/null || echo "DRAFT"
        return
    fi
    # No STAGE.yaml — infer from branch
    local branch
    branch=$(git symbolic-ref --short HEAD 2>/dev/null || echo "")
    if [[ "$branch" == "main" ]]; then
        echo "GA"
    elif [[ "$branch" =~ ^release/ ]]; then
        echo "GA"
    elif [[ "$branch" =~ ^rc/ ]]; then
        echo "RC"
    elif [[ "$branch" =~ ^beta/ ]]; then
        echo "BETA"
    elif [[ "$branch" =~ ^hotfix/ ]]; then
        echo "GA"
    elif [[ "$branch" =~ ^develop/ ]]; then
        # Check if any RC tag exists
        if git tag -l "${ver}-rc*" 2>/dev/null | head -1 | grep -q .; then
            echo "RC"
        elif git tag -l "${ver}-beta*" 2>/dev/null | head -1 | grep -q .; then
            echo "BETA"
        else
            echo "ALPHA"
        fi
    else
        echo "DRAFT"
    fi
}

# ---- List mode ----
if [[ "$LIST_ONLY" == true ]]; then
    echo "=== Stage Control Framework — Stage Gates (from STAGE_CONFIG.yaml) ==="
    python3 <<EOF
import yaml
with open("$STAGE_CONFIG") as f:
    cfg = yaml.safe_load(f)
for stage, info in cfg['stages'].items():
    print(f"\n## {stage} (order {info['order']})")
    print(f"   {info['description']}")
    print(f"   Required gates ({len(info['required_gates'])}):")
    for g in info['required_gates']:
        print(f"     - {g}")
EOF
    exit 0
fi

# ---- Main logic ----
VERSION=$(detect_version)
STAGE=$(detect_stage "$VERSION")

# Substitute {VERSION} placeholder in gate paths
expand_version() {
    local path="$1"
    # Two placeholders:
    #   {VERSION} expands to "v3.9.0" (with v prefix)
    #   {VER} expands to "3.9.0" (just the version, for use after a literal v)
    local v_full="$VERSION"
    local v_short="${VERSION#v}"  # strip leading v
    printf '%s\n' "$path" | sed -e "s|{VERSION}|$v_full|g" -e "s|{VER}|$v_short|g"
}

# Read stage config
read_stage_info() {
    python3 - "$STAGE_CONFIG" "$STAGE" <<'EOF'
import yaml, sys, json
cfg_path, stage = sys.argv[1], sys.argv[2]
with open(cfg_path) as f:
    cfg = yaml.safe_load(f)
info = cfg['stages'].get(stage, {})
out = {
    'stage': stage,
    'description': info.get('description', ''),
    'required_gates': info.get('required_gates', []),
    'optional_gates': info.get('optional_gates', []),
    'required_files': info.get('required_files', []),
    'doc_artifacts': info.get('doc_artifacts', []),
}
print(json.dumps(out))
EOF
}

STAGE_INFO=$(read_stage_info)

# Parse JSON
REQ_GATES=()
OPT_GATES=()
REQ_FILES=()
DESCRIPTION=""
if command -v jq >/dev/null 2>&1; then
    # Use jq (faster, cleaner)
    while IFS= read -r line; do
        [[ -n "$line" ]] && REQ_GATES+=("$line")
    done < <(echo "$STAGE_INFO" | jq -r '.required_gates[]?' 2>/dev/null)
    while IFS= read -r line; do
        [[ -n "$line" ]] && OPT_GATES+=("$line")
    done < <(echo "$STAGE_INFO" | jq -r '.optional_gates[]?' 2>/dev/null)
    while IFS= read -r line; do
        [[ -n "$line" ]] && REQ_FILES+=("$line")
    done < <(echo "$STAGE_INFO" | jq -r '.required_files[]?' 2>/dev/null)
    DESCRIPTION=$(echo "$STAGE_INFO" | jq -r '.description' 2>/dev/null)
else
    # Fallback: use python
    REQ_GATES=($(echo "$STAGE_INFO" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(g) for g in d.get('required_gates', [])]" 2>/dev/null))
    OPT_GATES=($(echo "$STAGE_INFO" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(g) for g in d.get('optional_gates', [])]" 2>/dev/null))
    REQ_FILES=($(echo "$STAGE_INFO" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(f) for f in d.get('required_files', [])]" 2>/dev/null))
    DESCRIPTION=$(echo "$STAGE_INFO" | python3 -c "import json,sys; print(json.load(sys.stdin).get('description', ''))" 2>/dev/null)
fi

# Header
echo "=================================================================="
echo "  Stage Gate Check: $STAGE"
echo "  Version: $VERSION"
echo "  Branch: $(git symbolic-ref --short HEAD 2>/dev/null || echo 'detached')"
echo "  Description: $DESCRIPTION"
echo "=================================================================="
echo ""

# ---- Check required files ----
FILE_PASS=0
FILE_FAIL=0
FILE_FAIL_LIST=()
if [[ ${#REQ_FILES[@]} -gt 0 ]]; then
    echo "--- Required files ---"
    for f in "${REQ_FILES[@]}"; do
        f_expanded=$(expand_version "$f")
        if [[ -f "$f_expanded" ]]; then
            echo "  [OK]  $f_expanded"
            FILE_PASS=$((FILE_PASS + 1))
        else
            echo "  [MISS] $f_expanded"
            FILE_FAIL=$((FILE_FAIL + 1))
            FILE_FAIL_LIST+=("$f_expanded")
        fi
    done
    echo ""
fi

# ---- Run required gates ----
GATE_PASS=0
GATE_FAIL=0
GATE_FAIL_LIST=()

if [[ ${#REQ_GATES[@]} -gt 0 ]]; then
    echo "--- Required gates (${#REQ_GATES[@]}) ---"
    for g in "${REQ_GATES[@]}"; do
        # Strip trailing args (e.g. "check_rc_ga_gate.sh ga" -> "check_rc_ga_gate.sh")
        # for the -f existence check below.
        # 2026-07-11: Apply {VER}/{VERSION} expansion to script_path too,
        # otherwise per-version gate paths like check_alpha_v{VER}.sh fail
        # the existence check (and incorrectly mark as SKIP/NOT FOUND).
        g_expanded=$(expand_version "$g")
        script_path="${g_expanded%% *}"
        if [[ "$g_expanded" =~ ^cargo ]]; then
            label="$g_expanded"
            if [[ "$DRY_RUN" == true ]]; then
                echo "  [DRY]  $g_expanded"
                continue
            fi
            if eval "$g_expanded" >/tmp/stage_gate_$$.log 2>&1; then
                echo "  [PASS] $label"
                GATE_PASS=$((GATE_PASS + 1))
            else
                echo "  [FAIL] $label"
                GATE_FAIL=$((GATE_FAIL + 1))
                GATE_FAIL_LIST+=("$g_expanded")
            fi
        # Strip trailing args to check if the script file exists
        # (e.g. "scripts/gate/check_rc_ga_gate.sh ga" -> check "scripts/gate/check_rc_ga_gate.sh")
        elif [[ -f "$script_path" ]] || [[ -f "${REPO_ROOT}/${script_path}" ]]; then
            label=$(basename "$g_expanded")
            if [[ "$DRY_RUN" == true ]]; then
                echo "  [DRY]  $g_expanded"
                continue
            fi
            if bash "$g_expanded" >/tmp/stage_gate_$$.log 2>&1; then
                echo "  [PASS] $label"
                GATE_PASS=$((GATE_PASS + 1))
            else
                echo "  [FAIL] $label"
                GATE_FAIL=$((GATE_FAIL + 1))
                GATE_FAIL_LIST+=("$g_expanded")
            fi
        else
            echo "  [SKIP] $g_expanded (not found, may need install)"
            GATE_FAIL=$((GATE_FAIL + 1))
            GATE_FAIL_LIST+=("$g_expanded (NOT FOUND)")
        fi
    done
    echo ""
fi

# ---- Optional gates (informational) ----
if [[ ${#OPT_GATES[@]} -gt 0 ]]; then
    echo "--- Optional gates (${#OPT_GATES[@]}, informational) ---"
    for g in "${OPT_GATES[@]}"; do
        g_expanded=$(expand_version "$g")
        script_path="${g_expanded%% *}"
        label=$(basename "$g_expanded")
        if [[ -f "$script_path" ]] || [[ -f "${REPO_ROOT}/${script_path}" ]]; then
            echo "  [INFO] $label (optional)"
        else
            echo "  [INFO] $label (optional, not installed)"
        fi
    done
    echo ""
fi

# ---- JSON output ----
if [[ "$JSON_OUTPUT" == true ]]; then
    # Build failed lists as comma-separated strings for safe interpolation
    # Disable set -u around array expansion to avoid unbound errors when empty
    set +u
    file_fail_csv=$(IFS=,; echo "${FILE_FAIL_LIST[*]+"${FILE_FAIL_LIST[*]}"}")
    gate_fail_csv=$(IFS=,; echo "${GATE_FAIL_LIST[*]+"${GATE_FAIL_LIST[*]}"}")
    set -u
    # Use heredoc to avoid bash variable expansion conflicts in python -c
    python3 - "${VERSION}" "${STAGE}" "${DESCRIPTION}" "${FILE_PASS}" "${FILE_FAIL}" "${file_fail_csv}" "${GATE_PASS}" "${GATE_FAIL}" "${gate_fail_csv}" <<'PYEOF'
import json, sys
version, stage, description, file_pass, file_fail, file_fail_csv, gate_pass, gate_fail, gate_fail_csv = sys.argv[1:10]
file_failed = [f for f in file_fail_csv.split(",") if f] if file_fail_csv else []
gate_failed = [f for f in gate_fail_csv.split(",") if f] if gate_fail_csv else []
result = {
    "version": version,
    "stage": stage,
    "description": description,
    "files": {"pass": int(file_pass), "fail": int(file_fail), "failed": file_failed},
    "gates": {"pass": int(gate_pass), "fail": int(gate_fail), "failed": gate_failed},
    "overall": "PASS" if (int(file_fail) + int(gate_fail)) == 0 else "FAIL",
}
print(json.dumps(result, indent=2, ensure_ascii=False))
PYEOF
fi

# ---- Summary ----
TOTAL_FAIL=$((FILE_FAIL + GATE_FAIL))
echo "=================================================================="
echo "  Stage $STAGE ($VERSION): $(($FILE_PASS + $GATE_PASS)) PASS, $TOTAL_FAIL FAIL"
echo "=================================================================="

# Cleanup
rm -f /tmp/stage_gate_$$.log

# Exit code
if [[ $TOTAL_FAIL -eq 0 ]]; then
    exit 0
else
    exit 1
fi
