#!/usr/bin/env bash
# scripts/gate/stage_config_common.sh — shared resolution rules for the
# stage-control framework.
#
# WHY THIS FILE EXISTS
# ---------------------
# The framework has one SSOT (`docs/governance/STAGE_CONFIG.yaml`) and one
# driver (`scripts/gate/check_stage.sh`). Issue #5440 showed the SSOT had
# **no executor at all**. The first attempt to give it one immediately
# produced **two** implementations of "how do I resolve a gate path" and
# "how do I resolve an artifact name" — and two implementations of a rule
# is precisely how a rule drifts.
#
# ⇒ Every rule about resolving a path in this framework lives HERE.
# The driver and the consistency gate both source this file, so they cannot
# disagree about what a path means.
#
# RULES
# -----
# R1  A **gate path** is either
#       - a cargo command (starts with "cargo"), or
#       - a path to a shell script under `scripts/gate/`.
#     `{VER}` expands to the version without the leading "v" (4.1.0);
#     `{VERSION}` expands to the version with the leading "v" (v4.1.0).
#
#     The repository names per-version gates inconsistently —
#     `check_alpha_v410.sh` versus `check_alpha_v3.12.0.sh` — so a path
#     that does not resolve is also tried with the dot-stripped, dash and
#     underscore spellings of `{VER}`:
#         check_alpha_v4.1.0.sh  →  check_alpha_v410.sh
#                                  check_alpha_v4-1-0.sh
#                                  check_alpha_v4_1_0.sh
#
# R2  A **required file** is a repository-relative path.
#     The stage config annotates some of them (`STAGE.yaml (state=GA)`);
#     the annotation states an *expectation about the state*, it is not
#     part of the path ⇒ it is stripped before the path is resolved.
#
#     Before this rule existed, **3 of the 5** GA required files were
#     reported MISS although the files were right there in the release
#     directory — the annotation alone made them unresolvable.
#
# R3  A **doc artifact** is a file name resolved **relative to the
#     version's release directory** (`docs/releases/v{VER}/`), NOT
#     repository-relative. Before this rule existed, *every* doc artifact
#     was reported MISS for exactly that reason.
#
# R4  A **G number** is an **alias**, never the criterion. The criterion is
#     the script path. The alias is nevertheless checkable: the script a G
#     number names must be recognisable from the G's own name — i.e. one
#     of the significant words of the G's `name` must appear in the script
#     path. That is what catches "G7 (24h Soak) → the full-gate
#     aggregator" and "G8 (Crash Matrix) → the drift detector"
#     (issue #5438) without hand-maintaining a second mapping table.
#
#     "Significant word" = a lowercased, alphanumeric-only word of length
#     >= 2 that is not a stop word and not a pure number. The match is
#     "some token of the script path starts with the word", so `arch`
#     matches the token `arch3`.
#
# Refs: #5437 (one criterion), #5438 (G→script mapping), #5439 (artifact
#       discovery), #5440 (the SSOT needs an executor).

# --- stop words for R4 ------------------------------------------------------
# Generic English words plus the abbreviations that would otherwise make an
# unrelated script match: "sem" is the generic *semantic* prefix (the
# SEM-1 savepoint gate is not the architecture semantic-debt gate), and
# "int" is kept because the INT-2/INT-3 gates really are `check_int_debt.sh`.
STAGE_STOPWORDS="test tests testing gate gates script scripts check the a an of for and or with at in on to plus real new full main path paths scenario scenarios case cases matrix entry baseline regression integration unit pending blocked acceptance version versions level levels per via only all any each every must should can not no same other into out over under during before after since while within without across through up down once still own such than that this these those there here when where which who what how why is are be been was were has have had do does did but if then its it from by as we our you they sem"

# --- helpers ----------------------------------------------------------------

# The repository root, derived from this file's location.
stage_repo_root() {
    cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd
}

# Expand `{VER}` / `{VERSION}` in a path. Echoes the expanded path.
stage_expand_vars() {
    local path="$1" ver="$2"
    printf '%s\n' "$path" | sed -e "s|{VERSION}|${ver}|g" -e "s|{VER}|${ver#v}|g"
}

# Strip a trailing `(...)` annotation from a path-like string.
#   "docs/releases/v{VER}/STAGE.yaml (state=GA)" → "docs/releases/v{VER}/STAGE.yaml"
stage_strip_annotation() {
    printf '%s\n' "$1" | sed -E 's/[[:space:]]*\([^()]*\)[[:space:]]*$//'
}

# Resolve a gate path to a file that exists, or echo nothing.
#
# A gate entry may carry trailing arguments
# (`scripts/gate/check_rc_ga_gate.sh ga`); the arguments belong to the
# **command**, not to the **path**, so the existence check looks at the
# first whitespace-delimited token.
#
# Tries, in order:
#   1. the expanded path as written,
#   2. the same path with the dot-stripped / dash / underscore spellings
#      of `{VER}` (R1).
stage_resolve_gate() {
    local raw="$1" ver="$2"
    local expanded path_only
    expanded="$(stage_expand_vars "$raw" "$ver")"
    path_only="${expanded%% *}"
    if [[ -f "$path_only" ]]; then
        printf '%s\n' "$path_only"
        return 0
    fi
    # Only the {VER}-derived component needs the alternative spellings.
    if [[ "$path_only" == *"${ver#v}"* ]]; then
        local short="${ver#v}"
        local -a variants=(
            "${path_only//$short/${short//./}}"
            "${path_only//$short/${short//./-}}"
            "${path_only//$short/${short//./_}}"
        )
        local v
        for v in "${variants[@]}"; do
            if [[ -f "$v" ]]; then
                printf '%s\n' "$v"
                return 0
            fi
        done
    fi
    return 0
}

# Resolve a required file (repository-relative, annotation stripped).
stage_resolve_required_file() {
    local raw="$1" ver="$2"
    local stripped
    stripped="$(stage_strip_annotation "$raw")"
    stage_expand_vars "$stripped" "$ver"
}

# Resolve a doc artifact (relative to the version's release directory).
stage_resolve_doc_artifact() {
    local raw="$1" ver="$2"
    local stripped
    stripped="$(stage_strip_annotation "$raw")"
    printf 'docs/releases/v%s/%s\n' "${ver#v}" "$stripped"
}

# The significant words of a G number's name (R4).
# Echoes one word per line, lowercased.
#
# Implemented in python rather than awk: the awk form of this filter needs
# a multi-line pattern, which is a syntax error in POSIX awk (BSD awk on
# macOS refuses it), and a rule that only runs on one platform is not a
# rule.
stage_significant_words() {
    local name="$1"
    python3 - "$name" "$STAGE_STOPWORDS" <<'PYEOF'
import re, sys
name, stop = sys.argv[1], set(sys.argv[2].split())
words = set()
for w in re.split(r"[^0-9A-Za-z]+", name.lower()):
    if len(w) >= 2 and not w.isdigit() and w not in stop:
        words.add(w)
for w in sorted(words):
    print(w)
PYEOF
}

# Does the script path contain one of the significant words (R4)?
# "some token of the script path starts with the word"
stage_script_matches_name() {
    local script="$1" name="$2"
    local word
    while IFS= read -r word; do
        [[ -z "$word" ]] && continue
        if printf '%s\n' "$script" | tr '[:upper:]' '[:lower:]' \
            | tr -cs '[:alnum:]' '\n' | grep -q "^${word}"; then
            return 0
        fi
    done < <(stage_significant_words "$name")
    return 1
}

# The version's current stage, read from its STAGE.yaml.
# Echoes the stage name (upper case) or "unknown".
stage_current_stage() {
    local ver="$1"
    local f="docs/releases/v${ver#v}/STAGE.yaml"
    [[ -f "$f" ]] || { echo "unknown"; return 0; }
    python3 - "$f" <<'PYEOF' 2>/dev/null || echo "unknown"
import sys
try:
    import yaml
except Exception:
    print("unknown"); raise SystemExit(0)
try:
    with open(sys.argv[1]) as fh:
        d = yaml.safe_load(fh) or {}
    print(str(d.get("current_stage", "unknown")).upper())
except Exception:
    print("unknown")
PYEOF
}

# --- SSOT readers -----------------------------------------------------------
#
# The **criterion** (which artifacts a stage demands) lives in
# docs/governance/STAGE_CONFIG.yaml. The **rules** for turning a config
# entry into a path live in this file. These readers are the single place
# where the two meet, so the driver (check_stage.sh), the artifact gate
# (check_release_artifacts.sh) and the consistency gate
# (check_stage_config_consistency.sh) cannot drift apart.

STAGE_CONFIG_PATH_DEFAULT="docs/governance/STAGE_CONFIG.yaml"

# The stage names in lifecycle order.
stage_stage_names() {
    local cfg="${1:-$STAGE_CONFIG_PATH_DEFAULT}"
    python3 - "$cfg" <<'PYEOF' 2>/dev/null || true
import sys
try:
    import yaml
    with open(sys.argv[1]) as fh:
        d = yaml.safe_load(fh) or {}
except Exception:
    raise SystemExit(0)
for s in (d.get('framework', {}) or {}).get('stages', []) or []:
    print(s)
PYEOF
}

# The raw gate entries of a stage: "<kind>\t<entry>" where kind is
# required | optional. Cargo commands are included verbatim — the caller
# decides what to do with them.
stage_gate_entries() {
    local cfg="${1:-$STAGE_CONFIG_PATH_DEFAULT}" stage="$2"
    python3 - "$cfg" "$stage" <<'PYEOF' 2>/dev/null || true
import sys
try:
    import yaml
    with open(sys.argv[1]) as fh:
        d = yaml.safe_load(fh) or {}
except Exception:
    raise SystemExit(0)
info = (d.get('stages', {}) or {}).get(sys.argv[2]) or {}
for kind, key in (('required', 'required_gates'), ('optional', 'optional_gates')):
    for e in (info.get(key) or []):
        print('%s\x1f%s' % (kind, e))
PYEOF
}

# The raw artifact entries of a stage: "<kind>\t<entry>" where kind is
# required_file | doc_artifact.
stage_artifact_entries() {
    local cfg="${1:-$STAGE_CONFIG_PATH_DEFAULT}" stage="$2"
    python3 - "$cfg" "$stage" <<'PYEOF' 2>/dev/null || true
import sys
try:
    import yaml
    with open(sys.argv[1]) as fh:
        d = yaml.safe_load(fh) or {}
except Exception:
    raise SystemExit(0)
info = (d.get('stages', {}) or {}).get(sys.argv[2]) or {}
for kind, key in (('required_file', 'required_files'), ('doc_artifact', 'doc_artifacts')):
    for e in (info.get(key) or []):
        print('%s\x1f%s' % (kind, e))
PYEOF
}

# The G alias table, one line per G:
#   <G number>\t<primary script>\t<alternative>\t<additional scripts>\t<name>
stage_config_gates() {
    local cfg="${1:-$STAGE_CONFIG_PATH_DEFAULT}"
    python3 - "$cfg" <<'PYEOF' 2>/dev/null || true
import sys
try:
    import yaml
    with open(sys.argv[1]) as fh:
        d = yaml.safe_load(fh) or {}
except Exception:
    raise SystemExit(0)
for g, info in (d.get('gate_definitions') or {}).items():
    info = info or {}
    alt = info.get('alternative') or ''
    adds = ' '.join(info.get('additional_scripts') or [])
    print('%s\x1f%s\x1f%s\x1f%s\x1f%s' % (g, info.get('script') or '', alt, adds, info.get('name') or ''))
PYEOF
}

# The state expectations of a stage: "<required_files key>\t<expectation>"
stage_config_expectations() {
    local cfg="${1:-$STAGE_CONFIG_PATH_DEFAULT}" stage="$2"
    python3 - "$cfg" "$stage" <<'PYEOF' 2>/dev/null || true
import sys
try:
    import yaml
    with open(sys.argv[1]) as fh:
        d = yaml.safe_load(fh) or {}
except Exception:
    raise SystemExit(0)
info = (d.get('stages', {}) or {}).get(sys.argv[2]) or {}
for k, v in (info.get('required_file_expectations') or {}).items():
    print('%s\x1f%s' % (k, v))
PYEOF
}

# --- The one artifact check -------------------------------------------------
#
# stage_artifact_status <version> <stage>
#
# Prints one line per required artifact:
#   OK<TAB><kind><TAB><resolved path>
#   MISS<TAB><kind><TAB><resolved path>
#
# Returns 0 when every artifact is present, 1 when at least one is missing.
#
# Both check_stage.sh and check_release_artifacts.sh call THIS function, so
# "what a stage demands" has exactly one implementation.
stage_artifact_status() {
    local ver="$1" stage="$2"
    local cfg="${STAGE_CONFIG_PATH_DEFAULT}"
    local rc=0 kind entry resolved
    while IFS=$'\x1f' read -r kind entry; do
        [[ -z "${kind:-}" || -z "${entry:-}" ]] && continue
        case "$kind" in
            required_file) resolved="$(stage_resolve_required_file "$entry" "$ver")" ;;
            doc_artifact) resolved="$(stage_resolve_doc_artifact "$entry" "$ver")" ;;
            *) continue ;;
        esac
        if [[ -f "$resolved" ]]; then
            printf 'OK\t%s\t%s\n' "$kind" "$resolved"
        else
            printf 'MISS\t%s\t%s\n' "$kind" "$resolved"
            rc=1
        fi
    done < <(stage_artifact_entries "$cfg" "$stage")
    return $rc
}
