#!/usr/bin/env bash
# check_stage_config_consistency.sh — the SSOT's own consistency gate.
#
# WHY THIS FILE EXISTS
# ---------------------
# `docs/governance/STAGE_CONFIG.yaml` calls itself the single source of
# truth for the stage framework. Issue #5440 showed it had **no executor**
# at all; issues #5437/#5438/#5439 showed that even *as a document* it
# contradicted itself:
#
#   * the same G number meant three different things in three files (#5437);
#   * five G numbers pointed at scripts that had nothing to do with what
#     the G number names (#5438);
#   * annotations glued onto paths made 3 of 5 GA required files
#     unresolvable, and `doc_artifacts` was never checked at all (#5439).
#
# This gate is the machine check that the SSOT is internally consistent.
# It does **not** run any of the gates the SSOT names — it checks the
# SSOT itself: do the paths it names exist, do the aliases it defines
# point at scripts that claim to be those aliases, are the artifact
# entries really paths?
#
# RULES (each one is a FAIL condition)
# ------------------------------------
# C1  Every entry of every stage's `required_gates` / `optional_gates`
#     that is not a cargo command must resolve to a file that exists.
#     (Catches dangling references such as the old
#     `scripts/gate/check_soak_gate.sh`.)
#
# C2  Every `gate_definitions[*].script` / `.alternative` /
#     `.additional_scripts` must resolve to a file that exists.
#
# C3  The **primary** script of a G number must **declare that G number
#     in its own header** (first 20 lines). This is what makes the alias
#     trustworthy: the script itself says "I am the G7 gate".
#     (#5438's mis-mappings were exactly the cases where the script did
#     not claim to be the gate it was pointed at.)
#
# C4  The script a G number names must be **recognisable from the G's own
#     name** — one of the significant words of `name` must appear in the
#     script path. Derived automatically from `name`, never hand-kept as
#     a second mapping table. (Rule R4 in stage_config_common.sh.)
#
# C5  A `required_files` / `doc_artifacts` entry must be a **path**, not a
#     path with an annotation glued on. `(state=GA)` is a *state
#     expectation* and belongs in `required_file_expectations`.
#
# C6  Every key of `required_file_expectations` must be a member of
#     the same stage's `required_files` — an expectation about a file
#     nobody demands is a dead expectation.
#
# C7  Every G alias must be **run by a stage**: the alias's script (and
#     every `additional_scripts` entry) must appear in the gate list
#     (`required_gates` / `optional_gates`) of the stage named by the
#     alias's `stage_min` **and of every stage after it up to GA** —
#     "must pass from X onward".
#
#     Before #5505 this was a D1 *disclosure* ("no stage gate list runs
#     this script"): 14 of the 16 aliases were executed by **nothing**,
#     so "GA is green" never covered TPC-H baseline, savepoint, upgrade,
#     audit log, time travel, QPS/TPS, sysbench, perf report or
#     compatibility. A disclosure nobody acts on is not a check ⇒ D1 is
#     now a FAIL rule.
#
# C8  A G alias placed in a stage's `optional_gates` (i.e. a
#     **non-blocking** alias) must declare a `requirement`: what has to
#     be present for it to run at all (hardware / tool / report) and how
#     a NOT-RUN is disclosed. The blocking semantics of an alias live in
#     the SSOT — never in a private column of an orchestrator script.
#
# Usage:
#   bash scripts/gate/check_stage_config_consistency.sh
#   bash scripts/gate/check_stage_config_consistency.sh --version v4.1.0
#   bash scripts/gate/check_stage_config_consistency.sh --selftest
#
# Exit codes:
#   0 = PASS (no FAIL findings)
#   1 = FAIL (at least one consistency violation)
#   2 = usage / environment error
#
# Refs: #5437 (one criterion), #5438 (G→script mapping), #5439 (artifact
#       discovery), #5440 (the SSOT needs an executor).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/gate/stage_config_common.sh
source "${SCRIPT_DIR}/stage_config_common.sh"

CONFIG="${STAGE_CONFIG_PATH_DEFAULT}"
VERSION="v4.1.0"
SELFTEST=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --config)   CONFIG="$2"; shift 2 ;;
        --version)  VERSION="$2"; shift 2 ;;
        --selftest) SELFTEST=true; shift ;;
        -h|--help)  grep -E '^#( |!)' "$0" | head -40; exit 0 ;;
        *) echo "Unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [[ ! -f "$CONFIG" ]]; then
    echo "ERROR: stage config not found: $CONFIG" >&2
    exit 2
fi

# --- helpers ----------------------------------------------------------------

# The significant words of a G name, as a single space-separated line.
stage_g_words_line() {
    stage_significant_words "$1" | tr '\n' ' '
}

# Does the script's header (first 20 lines) declare the given G number?
# "G7" must appear as a standalone token, not as part of "G70".
stage_script_declares_g() {
    local script="$1" g="$2"
    [[ -f "$script" ]] || return 1
    head -20 "$script" | grep -qE "(^|[^A-Za-z0-9])${g}([^A-Za-z0-9]|$)"
}

# The G alias table with the two fields C7/C8 need and
# scripts/gate/stage_config_common.sh does not expose:
#   <G>\x1f<primary script>\x1f<stage_min>\x1f<requirement>\x1f<additional scripts>
#
# Path *resolution* still comes from stage_config_common.sh — this reader
# only reads two fields, so it cannot disagree with the shared rules
# about what a path means.
stage_config_g_alias_fields() {
    local cfg="${1:-$STAGE_CONFIG_PATH_DEFAULT}"
    python3 - "$cfg" <<'PYEOF'
import sys
try:
    import yaml
    with open(sys.argv[1]) as fh:
        d = yaml.safe_load(fh) or {}
except Exception:
    raise SystemExit(0)
for g, info in (d.get('gate_definitions') or {}).items():
    info = info or {}
    print('\x1f'.join([
        str(g),
        str(info.get('script') or ''),
        str(info.get('stage_min') or ''),
        str(info.get('requirement') or ''),
        ' '.join(info.get('additional_scripts') or []),
    ]))
PYEOF
}

# The lifecycle order of the stages: "<stage>\x1f<order>" per line.
stage_config_stage_orders() {
    local cfg="${1:-$STAGE_CONFIG_PATH_DEFAULT}"
    python3 - "$cfg" <<'PYEOF'
import sys
try:
    import yaml
    with open(sys.argv[1]) as fh:
        d = yaml.safe_load(fh) or {}
except Exception:
    raise SystemExit(0)
for s, info in (d.get('stages', {}) or {}).items():
    info = info or {}
    print('%s\x1f%s' % (s, info.get('order', '')))
PYEOF
}

# The order of one stage (empty when the stage is unknown).
stage_config_stage_order() {
    local cfg="$1" stage="$2"
    python3 - "$cfg" "$stage" <<'PYEOF'
import sys
try:
    import yaml
    with open(sys.argv[1]) as fh:
        d = yaml.safe_load(fh) or {}
except Exception:
    raise SystemExit(0)
info = (d.get('stages', {}) or {}).get(sys.argv[2]) or {}
order = info.get('order')
print('' if order is None else str(order))
PYEOF
}

# --- the check --------------------------------------------------------------

# consistency_check <config-path> <version>
# Prints one line per finding:
#   FAIL<TAB><rule><TAB><detail>
#   DISCLOSURE<TAB><rule><TAB><detail>
# Returns 1 when at least one FAIL was printed.
consistency_check() {
    local cfg="$1" ver="$2"
    local fails=0
    local kind entry gname gscript galt gadd name words word resolved

    # ---- C1: gate entries must resolve -----------------------------------
    local stage
    while IFS= read -r stage; do
        [[ -z "$stage" ]] && continue
        while IFS=$'\x1f' read -r kind entry; do
            [[ -z "${kind:-}" || -z "${entry:-}" ]] && continue
            [[ "$entry" == cargo* ]] && continue
            resolved="$(stage_resolve_gate "$entry" "$ver")"
            if [[ -z "$resolved" ]]; then
                printf 'FAIL\tC1\t%s stage=%s gate "%s" does not resolve to any file\n' "$kind" "$stage" "$entry"
                fails=$((fails + 1))
            fi
        done < <(stage_gate_entries "$cfg" "$stage")
    done < <(stage_stage_names "$cfg")

    # ---- C2/C3/C4: the G alias table --------------------------------------
    # Read the G definitions.
    while IFS=$'\x1f' read -r gname gscript galt gadd name; do
        [[ -z "${gname:-}" ]] && continue

        # C2 — the primary script must exist.
        resolved="$(stage_resolve_gate "$gscript" "$ver")"
        if [[ -z "$resolved" ]]; then
            printf 'FAIL\tC2\t%s script "%s" does not resolve to any file\n' "$gname" "$gscript"
            fails=$((fails + 1))
        else
            # C3 — the script must declare the G number itself.
            if ! stage_script_declares_g "$resolved" "$gname"; then
                printf 'FAIL\tC3\t%s script "%s" does not declare "%s" in its header — the alias is unverified\n' \
                    "$gname" "$gscript" "$gname"
                fails=$((fails + 1))
            fi
            # C4 — the script must be recognisable from the G's own name.
            if ! stage_script_matches_name "$resolved" "$name"; then
                printf 'FAIL\tC4\t%s script "%s" shares no significant word with the name "%s"\n' \
                    "$gname" "$gscript" "$name"
                fails=$((fails + 1))
            fi
        fi

        # C2/C4 for the alternative and the additional scripts.
        for extra in "$galt" $gadd; do
            [[ -z "$extra" ]] && continue
            resolved="$(stage_resolve_gate "$extra" "$ver")"
            if [[ -z "$resolved" ]]; then
                printf 'FAIL\tC2\t%s extra script "%s" does not resolve to any file\n' "$gname" "$extra"
                fails=$((fails + 1))
            elif ! stage_script_matches_name "$resolved" "$name"; then
                printf 'FAIL\tC4\t%s extra script "%s" shares no significant word with the name "%s"\n' \
                    "$gname" "$extra" "$name"
                fails=$((fails + 1))
            fi
        done
    done < <(stage_config_gates "$cfg")

    # ---- C5/C6: the artifact entries --------------------------------------
    while IFS= read -r stage; do
        [[ -z "$stage" ]] && continue
        while IFS=$'\x1f' read -r kind entry; do
            [[ -z "${kind:-}" || -z "${entry:-}" ]] && continue
            if [[ "$entry" =~ [[:space:]]*\([^()]*\)[[:space:]]*$ ]]; then
                printf 'FAIL\tC5\tstage=%s %s "%s" carries an annotation — an artifact entry must be a path\n' \
                    "$stage" "$kind" "$entry"
                fails=$((fails + 1))
            fi
        done < <(stage_artifact_entries "$cfg" "$stage")

        # C6 — expectations must point at required files.
        while IFS=$'\x1f' read -r key expect; do
            [[ -z "${key:-}" ]] && continue
            if ! stage_artifact_entries "$cfg" "$stage" | cut -d $'\x1f' -f2 | grep -qxF "$key"; then
                printf 'FAIL\tC6\tstage=%s expectation for "%s" has no matching required_files entry\n' \
                    "$stage" "$key"
                fails=$((fails + 1))
            fi
        done < <(stage_config_expectations "$cfg" "$stage")
    done < <(stage_stage_names "$cfg")

    # ---- C7/C8: every G alias must be run by a stage ----------------------
    #
    # Index every stage's gate entries once: "<stage>\t<kind>\t<path>".
    local gate_index
    gate_index="$(mktemp "${TMPDIR:-/tmp}/stage_gate_index.XXXXXX")"
    while IFS= read -r stage; do
        [[ -z "$stage" ]] && continue
        while IFS=$'\x1f' read -r kind entry; do
            [[ -z "${kind:-}" || -z "${entry:-}" ]] && continue
            [[ "$entry" == cargo* ]] && continue
            printf '%s\t%s\t%s\n' "$stage" "$kind" "${entry%% *}" >>"$gate_index"
        done < <(stage_gate_entries "$cfg" "$stage")
    done < <(stage_stage_names "$cfg")

    # The stage order, so "from stage_min onward" is computable.
    local stage_orders=()
    while IFS=$'\x1f' read -r sname sorder; do
        [[ -z "${sname:-}" ]] && continue
        stage_orders+=("${sname}"$'\x1f'"${sorder:-}")
    done < <(stage_config_stage_orders "$cfg")

    while IFS=$'\x1f' read -r gname gscript gstage_min grequirement gextra; do
        [[ -z "${gname:-}" ]] && continue

        # ---- C7: the alias must say which stage runs it -------------------
        if [[ -z "${gstage_min:-}" ]]; then
            printf 'FAIL\tC7\t%s declares no "stage_min" — an alias no stage is required to run is an orphan\n' "$gname"
            fails=$((fails + 1))
            continue
        fi
        local min_order
        min_order="$(stage_config_stage_order "$cfg" "$gstage_min")"
        if [[ -z "$min_order" ]]; then
            printf 'FAIL\tC7\t%s names an unknown stage_min "%s"\n' "$gname" "$gstage_min"
            fails=$((fails + 1))
            continue
        fi

        # The scripts this alias covers: the primary one plus its
        # additional ones (a concern may be covered by more than one
        # gate — G10 covers audit log, time travel AND hash chain).
        local alias_scripts=()
        [[ -n "${gscript:-}" ]] && alias_scripts+=("$gscript")
        local extra_one
        for extra_one in ${gextra:-}; do
            alias_scripts+=("$extra_one")
        done

        local sc st st_order
        for sc in "${alias_scripts[@]}"; do
            for st in "${stage_orders[@]}"; do
                st_order="${st##*$'\x1f'}"
                st="${st%%$'\x1f'*}"
                [[ "$st" == "DRAFT" ]] && continue
                [[ -z "$st_order" ]] && continue
                (( st_order < min_order )) && continue
                if ! awk -F'\t' -v s="$st" -v p="$sc" \
                        '$1==s && $3==p {found=1} END{exit !found}' "$gate_index"; then
                    printf 'FAIL\tC7\t%s script "%s" is not run by stage %s (stage_min=%s — must pass from there onward)\n' \
                        "$gname" "$sc" "$st" "$gstage_min"
                    fails=$((fails + 1))
                fi
            done
        done

        # ---- C8: a non-blocking alias must declare its requirement -------
        if [[ -n "${gscript:-}" ]] && \
           awk -F'\t' -v p="$gscript" '$3==p && $2=="optional" {found=1} END{exit !found}' "$gate_index"; then
            if [[ -z "${grequirement:-}" ]]; then
                printf 'FAIL\tC8\t%s is non-blocking (listed in optional_gates) but declares no "requirement" — the prerequisite and the NOT-RUN disclosure must be in the SSOT\n' \
                    "$gname"
                fails=$((fails + 1))
            fi
        fi
    done < <(stage_config_g_alias_fields "$cfg")

    rm -f "$gate_index"

    return $((fails > 0 ? 1 : 0))
}

# --- selftest ---------------------------------------------------------------
#
# A gate that cannot tell a clean document from a broken one is not a gate.
# The selftest builds small synthetic stage configs — one clean, one per
# violation class — and asserts the checker returns PASS on the clean one
# and FAIL on each broken one.
run_selftest() {
    local tmp
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/stage_consistency_selftest.XXXXXX")"
    local rc=0
    local name

    cat >"$tmp/clean.yaml" <<'YAMLEOF'
framework:
  stages: [DRAFT, ALPHA, RC, GA]
stages:
  DRAFT:
    order: 1
    required_gates:
      - "scripts/gate/check_arch_invariants.sh"
      - "cargo build --all-features"
    required_files:
      - "docs/releases/v{VER}/VERSION_PLAN.md"
    doc_artifacts:
      - "VERSION_PLAN.md"
    required_file_expectations:
      "docs/releases/v{VER}/VERSION_PLAN.md": "must exist"
  ALPHA:
    order: 2
    required_gates:
      - "scripts/gate/check_g1_tpch_baseline.sh"
  RC:
    order: 4
    required_gates:
      - "scripts/gate/check_g1_tpch_baseline.sh"
    optional_gates:
      - "scripts/gate/check_p13_soak_test.sh"
  GA:
    order: 5
    required_gates:
      - "scripts/gate/check_g1_tpch_baseline.sh"
      - "scripts/gate/check_p13_soak_test.sh"
    optional_gates:
      - "scripts/gate/check_chaos_soak.sh"
    required_files:
      - "docs/releases/v{VER}/STAGE.yaml"
    doc_artifacts:
      - "STAGE.yaml"
gate_definitions:
  G1:
    name: "TPC-H 22/22 baseline (regression)"
    script: "scripts/gate/check_g1_tpch_baseline.sh"
    type: regression
    stage_min: ALPHA
  G7:
    name: "24h Soak (compressed-time)"
    script: "scripts/gate/check_p13_soak_test.sh"
    type: long-running
    stage_min: RC
    requirement: "长时 soak 主机 / 真实时长；NOT-RUN 必须披露，不得记为 PASS"
YAMLEOF

    # C1: a gate entry that names a script nobody has.
    sed 's#scripts/gate/check_arch_invariants.sh#scripts/gate/check_does_not_exist.sh#' \
        "$tmp/clean.yaml" >"$tmp/c1.yaml"
    # C2: a G alias that points at a script nobody has.
    sed 's#script: "scripts/gate/check_p13_soak_test.sh"#script: "scripts/gate/check_absent_gate.sh"#' \
        "$tmp/clean.yaml" >"$tmp/c2.yaml"
    # C3: the G alias points at a script that does not declare the G number.
    sed 's#script: "scripts/gate/check_p13_soak_test.sh"#script: "scripts/gate/check_drift_not_pass.sh"#' \
        "$tmp/clean.yaml" >"$tmp/c3.yaml"
    # C4: the script claims the G number, but its path is unrelated to the
    #     G's name — the alias says "Upgrade" and runs "stability".
    sed 's#name: "24h Soak (compressed-time)"#name: "zzz Nomatch"#' \
        "$tmp/clean.yaml" >"$tmp/c4.yaml"
    # C5: an annotation glued onto an artifact path.
    sed 's#- "docs/releases/v{VER}/STAGE.yaml"#- "docs/releases/v{VER}/STAGE.yaml (state=GA)"#' \
        "$tmp/clean.yaml" >"$tmp/c5.yaml"
    # C6: an expectation about a file nobody demands.
    sed 's#      - "docs/releases/v{VER}/VERSION_PLAN.md"#      - "docs/releases/v{VER}/OTHER.md"#' \
        "$tmp/clean.yaml" >"$tmp/c6.yaml"
    # C7: a G alias that no stage runs — the #5505 defect (D1 was a
    #     disclosure here; it is a FAIL rule now).
    sed 's#^      - "scripts/gate/check_p13_soak_test.sh"$#      - "scripts/gate/check_chaos_soak.sh"#' \
        "$tmp/clean.yaml" >"$tmp/c7.yaml"
    # C8: a non-blocking alias that does not say what it needs to run.
    sed '/^    requirement: /d' \
        "$tmp/clean.yaml" >"$tmp/c8.yaml"

    # The clean sample must PASS.
    if consistency_check "$tmp/clean.yaml" "$VERSION" >"$tmp/clean.out" 2>&1; then
        echo "  [OK]   clean sample -> PASS"
    else
        echo "  [FAIL] clean sample -> FAIL (a clean document must not be reported broken)"
        sed 's/^/         /' "$tmp/clean.out"
        rc=1
    fi

    for case in c1 c2 c3 c4 c5 c6 c7 c8; do
        if consistency_check "$tmp/$case.yaml" "$VERSION" >"$tmp/$case.out" 2>&1; then
            echo "  [FAIL] $case sample -> PASS (the gate cannot detect this violation)"
            rc=1
        else
            echo "  [OK]   $case sample -> FAIL (detected)"
        fi
    done

    # The failing samples must name the right rule.
    local expect_rule
    for case in c1 c2 c3 c4 c5 c6 c7 c8; do
        case "$case" in
            c1) expect_rule="C1" ;;
            c2) expect_rule="C2" ;;
            c3) expect_rule="C3" ;;
            c4) expect_rule="C4" ;;
            c5) expect_rule="C5" ;;
            c6) expect_rule="C6" ;;
            c7) expect_rule="C7" ;;
            c8) expect_rule="C8" ;;
        esac
        if grep -q "^FAIL	${expect_rule}	" "$tmp/$case.out"; then
            echo "  [OK]   $case sample -> reported as $expect_rule"
        else
            echo "  [FAIL] $case sample -> did not report $expect_rule"
            sed 's/^/         /' "$tmp/$case.out"
            rc=1
        fi
    done

    rm -rf "$tmp"
    return $rc
}

if [[ "$SELFTEST" == true ]]; then
    echo "=== stage config consistency gate — selftest ==="
    if run_selftest; then
        echo "SELFTEST PASS"
        exit 0
    fi
    echo "SELFTEST FAIL"
    exit 1
fi

echo "=== Stage Config Consistency Gate ==="
echo "config: $CONFIG"
echo "version: $VERSION"
echo ""

out="$(consistency_check "$CONFIG" "$VERSION")"
rc=$?
printf '%s\n' "$out"

fails=$(printf '%s\n' "$out" | grep -c '^FAIL	' || true)
disc=$(printf '%s\n' "$out" | grep -c '^DISCLOSURE	' || true)

echo ""
echo "----------------------------------------"
echo "  FAIL findings:        $fails"
echo "  DISCLOSURE findings:  $disc"
if [[ $rc -eq 0 ]]; then
    echo "  RESULT: PASS"
else
    echo "  RESULT: FAIL"
fi
echo "----------------------------------------"
exit $rc
