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
#   5. Checks required_files + doc_artifacts exist
#   6. Writes a run log under docs/releases/<version>/logs/
#   7. Exits 0 if all PASS, 1 if any FAIL, 2 if DRIFT
#
# #5440: this script was the SSOT's only executor and **no CI job ever
# called it** — the SSOT was a declarative document with nobody consuming
# it. It is now wired into CI (.gitea/workflows/ci.yml) and is the
# authoritative entry point for a stage's gate list.
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
#   - scripts/gate/stage_config_common.sh (the resolution rules — one
#     implementation, shared with the consistency and artifact gates)
#   - docs/releases/vX.Y.Z/STAGE.yaml (per-version override, optional)
#
# This script does NOT replace the existing stage-specific scripts
# (check_alpha.sh, check_beta_gate.sh, check_rc_ga_gate.sh,
# check_alpha_v380.sh). They are listed in STAGE_CONFIG.yaml's
# required_gates for each stage. The framework is the driver; the
# existing scripts are the implementations. This avoids duplication and
# makes "what's checked at each stage" a single-file change.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
STAGE_CONFIG="${REPO_ROOT}/docs/governance/STAGE_CONFIG.yaml"

# The shared resolution rules (annotation stripping, {VER} spellings, doc
# artifact resolution, G-name matching, the one artifact check). The
# driver, the consistency gate and the artifact gate all source this
# file, so the three cannot disagree about what a path means — see
# scripts/gate/stage_config_common.sh.
# shellcheck source=scripts/gate/stage_config_common.sh
source "${SCRIPT_DIR}/stage_config_common.sh"

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
        #
        # `git tag -l ... | head -1 | grep -q .` asked the question through a
        # pipe whose last stage is `grep -q`: `head -1` closes the pipe after
        # one line, `git tag` is killed by SIGPIPE, and under
        # `set -o pipefail` the pipeline can report "no match" for a tag that
        # is right there. The tag list is now captured first and the question
        # is asked of the captured text.
        if [ -n "$(git tag -l "${ver}-rc*" 2>/dev/null | head -1)" ]; then
            echo "RC"
        elif [ -n "$(git tag -l "${ver}-beta*" 2>/dev/null | head -1)" ]; then
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
    'branch_pattern': info.get('branch_pattern', []),
    'carrier_enforced': bool(info.get('carrier_enforced', False)),
}
print(json.dumps(out))
EOF
}

STAGE_INFO=$(read_stage_info)

# Parse JSON
REQ_GATES=()
OPT_GATES=()
REQ_FILES=()
DOC_ARTIFACTS=()
BRANCH_PATTERNS=()
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
    while IFS= read -r line; do
        [[ -n "$line" ]] && DOC_ARTIFACTS+=("$line")
    done < <(echo "$STAGE_INFO" | jq -r '.doc_artifacts[]?' 2>/dev/null)
    DESCRIPTION=$(echo "$STAGE_INFO" | jq -r '.description' 2>/dev/null)
    while IFS= read -r line; do
        [[ -n "$line" ]] && BRANCH_PATTERNS+=("$line")
    done < <(echo "$STAGE_INFO" | jq -r '.branch_pattern[]?' 2>/dev/null)
else
    # Fallback: use python
    REQ_GATES=($(echo "$STAGE_INFO" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(g) for g in d.get('required_gates', [])]" 2>/dev/null))
    OPT_GATES=($(echo "$STAGE_INFO" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(g) for g in d.get('optional_gates', [])]" 2>/dev/null))
    REQ_FILES=($(echo "$STAGE_INFO" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(f) for f in d.get('required_files', [])]" 2>/dev/null))
    DOC_ARTIFACTS=($(echo "$STAGE_INFO" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(f) for f in d.get('doc_artifacts', [])]" 2>/dev/null))
    DESCRIPTION=$(echo "$STAGE_INFO" | python3 -c "import json,sys; print(json.load(sys.stdin).get('description', ''))" 2>/dev/null)
    BRANCH_PATTERNS=($(echo "$STAGE_INFO" | python3 -c "import json,sys; d=json.load(sys.stdin); [print(p) for p in d.get('branch_pattern', [])]" 2>/dev/null))
fi
CARRIER_ENFORCED="$(echo "$STAGE_INFO" | python3 -c "import json,sys; print('true' if json.load(sys.stdin).get('carrier_enforced') else 'false')" 2>/dev/null)"

# ---- Run log ----
# #5441: governance requires every gate run to leave its log in
# `docs/releases/<version>/logs/`, and `.gitignore` used to exclude exactly
# that directory — so the required evidence chain could not be committed by
# construction. The log name follows docs/governance/GATE_CONDITIONS.md:
# `logs/gate_<stage>_<commit>_<timestamp>.log`.
LOG_DIR="docs/releases/${VERSION}/logs"
RUN_LOG=""
if [[ "$DRY_RUN" != true ]]; then
    mkdir -p "$LOG_DIR/gates"
    COMMIT_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")"
    TIMESTAMP="$(date -u +%Y%m%d_%H%M%SZ)"
    RUN_LOG="${LOG_DIR}/gate_$(printf '%s' "$STAGE" | tr '[:upper:]' '[:lower:]')_${COMMIT_SHA}_${TIMESTAMP}.log"
    : >"$RUN_LOG"
fi

# say — print a line to stdout AND append it to the run log.
say() {
    printf '%s\n' "$*"
    if [[ -n "$RUN_LOG" ]]; then
        printf '%s\n' "$*" >>"$RUN_LOG"
    fi
}

# ---- Header ----
say "=================================================================="
say "  Stage Gate Check: $STAGE"
say "  Version: $VERSION"
say "  Branch: $(git symbolic-ref --short HEAD 2>/dev/null || echo 'detached')"
say "  Commit: $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
say "  Description: $DESCRIPTION"
say "  Mode:    $([[ "$DRY_RUN" == true ]] && echo 'DRY-RUN (nothing executed)' || echo 'EXECUTE')"
say "=================================================================="
say ""

# ---- Branch pattern (the stage's own carrier rule) ----
# #5436 —— `STAGE_CONFIG.yaml` 的 `branch_pattern` 此前是**一句没有任何代码
# 读它的话**：全仓没有任何脚本消费这个字段。于是「GA 只允许在
# main / release/* / hotfix/* 上发生」这条规则**从未被强制**，
# 只要在 develop/v4.1.0 上把清单勾上，就可以「宣布 GA」——
# 而 GA 的定义本身并不允许这件事在这个分支上发生。
#
# 现在这条声明**活起来**了：当前分支必须匹配当前阶段的 `branch_pattern`。
# GA 的模式里**没有** `develop/v*` —— 所以要把 v4.1.0 的 `current_stage`
# 推到 GA，就必须先切出合法的承载分支（`release/v4.1.0`）或合入 `main`。
# 这不是给 GA 开后门，而是把「后门本来就开着」这件事关上。
BRANCH="$(git symbolic-ref --short HEAD 2>/dev/null || echo '')"
BRANCH_OK=false
for pat in "${BRANCH_PATTERNS[@]}"; do
    if [[ -n "$pat" && "$BRANCH" == $pat ]]; then
        BRANCH_OK=true
        break
    fi
done
say "--- Branch Pattern ---"
say "  stage:     $STAGE"
say "  pattern:   ${BRANCH_PATTERNS[*]:-<none declared>}"
say "  branch:    ${BRANCH:-<detached>}"
say "  enforced:  $CARRIER_ENFORCED"
if [ "$CARRIER_ENFORCED" != true ]; then
    say "  [INFO] carrier rule is declared but not enforced for this stage (development stage)"
elif [ "$BRANCH_OK" = true ]; then
    say "  [PASS] branch matches the stage's carrier rule"
else
    say "  [FAIL] branch '${BRANCH:-<detached>}' does not match the stage's carrier rule"
    say "         this stage cannot be carried out on this branch (#5436)"
    BRANCH_VIOLATION=true
fi
say ""

# ---- Required files + doc artifacts ----
# #5439: the artifact check is the **one** implementation, in
# scripts/gate/stage_config_common.sh (stage_artifact_status). This driver
# and scripts/gate/check_release_artifacts.sh both call it, so the two
# cannot report different verdicts for the same stage.
FILE_PASS=0
FILE_FAIL=0
FILE_FAIL_LIST=()
ART_PASS=0
ART_FAIL=0
ART_FAIL_LIST=()
if [[ ${#REQ_FILES[@]} -gt 0 || ${#DOC_ARTIFACTS[@]} -gt 0 ]]; then
    say "--- Required files & doc artifacts ---"
    while IFS=$'\t' read -r status kind path; do
        [[ -z "${status:-}" ]] && continue
        case "$kind" in
            required_file)
                if [[ "$status" == "OK" ]]; then
                    say "  [OK]  $path"
                    FILE_PASS=$((FILE_PASS + 1))
                else
                    say "  [MISS] $path"
                    FILE_FAIL=$((FILE_FAIL + 1))
                    FILE_FAIL_LIST+=("$path")
                fi
                ;;
            doc_artifact)
                if [[ "$status" == "OK" ]]; then
                    say "  [OK]  $path"
                    ART_PASS=$((ART_PASS + 1))
                else
                    say "  [MISS] $path"
                    ART_FAIL=$((ART_FAIL + 1))
                    ART_FAIL_LIST+=("$path")
                fi
                ;;
        esac
    done < <(stage_artifact_status "$VERSION" "$STAGE")
    say ""
fi

# ---- Run required gates ----
GATE_PASS=0
GATE_FAIL=0
GATE_FAIL_LIST=()

if [[ ${#REQ_GATES[@]} -gt 0 ]]; then
    say "--- Required gates (${#REQ_GATES[@]}) ---"
    for g in "${REQ_GATES[@]}"; do
        # The resolution tries the dot-stripped / dash / underscore
        # spellings of {VER} — the repository names per-version gates
        # inconsistently (check_alpha_v410.sh vs
        # check_alpha_v3.12.0.sh), and an exact match reported the ALPHA
        # gate of every v4.x release as "not found".
        g_expanded="$(stage_expand_vars "$g" "$VERSION")"
        script_path="$(stage_resolve_gate "$g" "$VERSION")"
        if [[ "$g_expanded" =~ ^cargo ]]; then
            label="$g_expanded"
            if [[ "$DRY_RUN" == true ]]; then
                say "  [DRY]  $g_expanded"
                continue
            fi
            if eval "$g_expanded" >"${LOG_DIR}/gates/${label//[^A-Za-z0-9._-]/_}.log" 2>&1; then
                say "  [PASS] $label"
                GATE_PASS=$((GATE_PASS + 1))
            else
                say "  [FAIL] $label"
                GATE_FAIL=$((GATE_FAIL + 1))
                GATE_FAIL_LIST+=("$g_expanded")
            fi
        elif [[ -n "$script_path" ]]; then
            label=$(basename "$g_expanded")
            if [[ "$DRY_RUN" == true ]]; then
                say "  [DRY]  $g_expanded -> $script_path"
                continue
            fi
            if bash "$script_path" >"${LOG_DIR}/gates/${label//[^A-Za-z0-9._-]/_}.log" 2>&1; then
                say "  [PASS] $label"
                GATE_PASS=$((GATE_PASS + 1))
            else
                say "  [FAIL] $label"
                GATE_FAIL=$((GATE_FAIL + 1))
                GATE_FAIL_LIST+=("$g_expanded")
            fi
        else
            say "  [MISS] $g_expanded (script not found — a gate that names a missing script cannot pass)"
            GATE_FAIL=$((GATE_FAIL + 1))
            GATE_FAIL_LIST+=("$g_expanded (NOT FOUND)")
        fi
    done
    say ""
fi

# ---- Optional gates (informational) ----
if [[ ${#OPT_GATES[@]} -gt 0 ]]; then
    say "--- Optional gates (${#OPT_GATES[@]}, informational) ---"
    for g in "${OPT_GATES[@]}"; do
        g_expanded="$(stage_expand_vars "$g" "$VERSION")"
        script_path="$(stage_resolve_gate "$g" "$VERSION")"
        label=$(basename "$g_expanded")
        if [[ -n "$script_path" ]]; then
            say "  [INFO] $label (optional)"
        else
            say "  [INFO] $label (optional, script not found)"
        fi
    done
    say ""
fi

# ---- JSON output ----
if [[ "$JSON_OUTPUT" == true ]]; then
    # Build failed lists as comma-separated strings for safe interpolation
    # Disable set -u around array expansion to avoid unbound errors when empty
    set +u
    file_fail_csv=$(IFS=,; echo "${FILE_FAIL_LIST[*]+"${FILE_FAIL_LIST[*]}"}")
    art_fail_csv=$(IFS=,; echo "${ART_FAIL_LIST[*]+"${ART_FAIL_LIST[*]}"}")
    gate_fail_csv=$(IFS=,; echo "${GATE_FAIL_LIST[*]+"${GATE_FAIL_LIST[*]}"}")
    set -u
    # Use heredoc to avoid bash variable expansion conflicts in python -c
    json="$(python3 - "${VERSION}" "${STAGE}" "${DESCRIPTION}" "${FILE_PASS}" "${FILE_FAIL}" "${file_fail_csv}" "${ART_PASS}" "${ART_FAIL}" "${art_fail_csv}" "${GATE_PASS}" "${GATE_FAIL}" "${gate_fail_csv}" <<'PYEOF'
import json, sys
(version, stage, description, file_pass, file_fail, file_fail_csv,
 art_pass, art_fail, art_fail_csv, gate_pass, gate_fail, gate_fail_csv) = sys.argv[1:13]
file_failed = [f for f in file_fail_csv.split(",") if f] if file_fail_csv else []
art_failed = [f for f in art_fail_csv.split(",") if f] if art_fail_csv else []
gate_failed = [f for f in gate_fail_csv.split(",") if f] if gate_fail_csv else []
result = {
    "version": version,
    "stage": stage,
    "description": description,
    "files": {"pass": int(file_pass), "fail": int(file_fail), "failed": file_failed},
    "doc_artifacts": {"pass": int(art_pass), "fail": int(art_fail), "failed": art_failed},
    "gates": {"pass": int(gate_pass), "fail": int(gate_fail), "failed": gate_failed},
    "overall": "PASS" if (int(file_fail) + int(art_fail) + int(gate_fail)) == 0 else "FAIL",
}
print(json.dumps(result, indent=2, ensure_ascii=False))
PYEOF
)"
    say "$json"
fi

# ---- Summary ----
# #5436 —— 分支承载规则不满足 ⇒ 整个阶段判定无效（不是「少一项」）。
BRANCH_FAIL=0
if [[ "${BRANCH_VIOLATION:-false}" == true ]]; then
    BRANCH_FAIL=1
fi
TOTAL_FAIL=$((FILE_FAIL + ART_FAIL + GATE_FAIL + BRANCH_FAIL))
say "=================================================================="
say "  Stage $STAGE ($VERSION): $(($FILE_PASS + ART_PASS + GATE_PASS)) PASS, $TOTAL_FAIL FAIL"
[[ $BRANCH_FAIL -gt 0 ]] && say "  branch carrier rule: FAIL (the current branch cannot carry this stage)"
say "  gate logs: $LOG_DIR/gates/"
[[ -n "$RUN_LOG" ]] && say "  run log:   $RUN_LOG" || true
say "=================================================================="

# Exit code
if [[ $TOTAL_FAIL -eq 0 ]]; then
    exit 0
else
    exit 1
fi
