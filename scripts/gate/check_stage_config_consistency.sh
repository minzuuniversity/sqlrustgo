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
# C9  Every entry of a stage's `exit_criteria` must say **who decides it**,
#     in that same stage block, exactly once:
#       * `criteria_gates: { "<criterion>": ["<gate path>", ...] }` — the
#         criterion is covered by gates. Each named gate must either be a
#         member of that stage's own `required_gates`/`optional_gates`, or
#         be named by a live Gitea Actions workflow (i.e. it has a real
#         execution point). A criterion may **not** claim coverage from a
#         gate that nothing runs.
#       * `manual_only_criteria: { "<criterion>": "<reason>" }` — no
#         executable gate covers it, and the reason must say why (naming
#         any candidate script and why it does not count).
#     FAIL conditions: a criterion disposed of in neither map; a criterion
#     disposed of twice; a map key that is not a member of `exit_criteria`
#     (a dead mapping); a criterion mapped to an empty gate list; a mapped
#     gate with no execution point for that stage; a `manual_only` entry
#     with an empty reason; an unparsable config (which would otherwise
#     make C9 pass vacuously).
#
#     Why this rule exists (看板 P1-1 / #5595): the SSOT listed 21 exit
#     criteria across the five stages and named **zero** executors for them
#     (`stage_crit=21 stage_map=0`). "the stage is ready to leave" was a
#     sentence, not a check. C9 turns it into one — and the honest answer
#     for the criteria with no gate is `manual_only`, never a fabricated
#     mapping. Adding the missing gates would change the gate list, which
#     is an owner-only action.
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
    # The head output is captured first and `grep -qE` then reads it from a
    # here-string: the old form piped it into `grep -q`, whose early exit
    # could kill `head` with SIGPIPE and — under `set -o pipefail` — turn a
    # declared gate into an undeclared one.
    grep -qE "(^|[^A-Za-z0-9])${g}([^A-Za-z0-9]|$)" <<< "$(head -20 "$script")"
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

# C9 findings for one config: "<rule>\t<detail>" per finding.
#
# The exit-criteria mapping is judged here (rather than through the shared
# stage_config_common.sh readers) because it needs all three maps of a
# stage at once: exit_criteria, criteria_gates and manual_only_criteria.
stage_config_c9_findings() {
    local cfg="${1:-$STAGE_CONFIG_PATH_DEFAULT}"
    python3 - "$cfg" "$REPO_ROOT" <<'PYEOF'
import os
import sys

try:
    import yaml
except Exception as exc:  # pragma: no cover - environment problem
    print('C9\tpyyaml is unavailable (%s) — C9 cannot judge the exit criteria' % exc)
    raise SystemExit(0)

path, repo_root = sys.argv[1], sys.argv[2]
try:
    with open(path, encoding='utf-8') as fh:
        doc = yaml.safe_load(fh) or {}
except Exception as exc:
    print('C9\tthe stage config could not be parsed (%s) — exit criteria cannot be judged' % exc)
    raise SystemExit(0)

# Scripts that a live Gitea Actions workflow names: those have an execution
# point even when a stage block does not list them (check_stage.sh is the
# framework driver itself, and it is what enforces the carrier rule).
live = set()
wf_dir = os.path.join(repo_root, '.gitea', 'workflows')
if os.path.isdir(wf_dir):
    for fn in sorted(os.listdir(wf_dir)):
        if not fn.endswith(('.yml', '.yaml')):
            continue
        try:
            with open(os.path.join(wf_dir, fn), encoding='utf-8', errors='replace') as fh:
                text = fh.read()
        except Exception:
            continue
        for line in text.splitlines():
            for tok in line.replace('"', ' ').replace("'", ' ').split():
                if tok.endswith(('.sh', '.py')):
                    live.add(os.path.basename(tok))


def gate_key(value):
    # Comparable form of a gate entry: basename of its first token, so
    # "scripts/gate/check_rc_ga_gate.sh ga" still matches the plain path.
    return os.path.basename(str(value).split()[0])


stages = doc.get('stages') or {}
for stage, info in stages.items():
    info = info or {}
    crits = [str(c) for c in (info.get('exit_criteria') or [])]
    gates_map = info.get('criteria_gates') or {}
    manual_map = info.get('manual_only_criteria') or {}
    stage_gates = {gate_key(g) for g in (info.get('required_gates') or [])}
    stage_gates |= {gate_key(g) for g in (info.get('optional_gates') or [])}

    if not isinstance(gates_map, dict) or not isinstance(manual_map, dict):
        print('C9\tstage=%s criteria_gates / manual_only_criteria must be mappings' % stage)
        continue

    for crit in crits:
        in_gates = crit in gates_map
        in_manual = crit in manual_map
        if in_gates and in_manual:
            print('C9\tstage=%s criterion %r is disposed of twice (both criteria_gates and manual_only_criteria)' % (stage, crit))
        elif not in_gates and not in_manual:
            print('C9\tstage=%s criterion %r names no executor — put it in criteria_gates (with the gate that decides it) or in manual_only_criteria (with the reason)' % (stage, crit))

    for crit in crits:
        if crit not in gates_map:
            continue
        mapped = gates_map[crit]
        names = mapped if isinstance(mapped, list) else ([mapped] if mapped else [])
        if not names:
            print('C9\tstage=%s criterion %r is mapped to zero gates — an empty mapping claims coverage without an executor' % (stage, crit))
            continue
        for gate in names:
            if gate_key(gate) not in stage_gates and gate_key(gate) not in live:
                print('C9\tstage=%s criterion %r claims coverage from "%s", which that stage does not run and no live workflow names' % (stage, crit, gate))

    for crit in gates_map:
        if str(crit) not in crits:
            print('C9\tstage=%s criteria_gates key %r is not a member of exit_criteria — a dead mapping' % (stage, crit))
    for crit, reason in manual_map.items():
        if str(crit) not in crits:
            print('C9\tstage=%s manual_only_criteria key %r is not a member of exit_criteria — a dead mapping' % (stage, crit))
        if not str(reason or '').strip():
            print('C9\tstage=%s manual_only_criteria for %r has no reason — saying "human" without saying why is not a disclosure' % (stage, crit))
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
            # The entry list is captured first and `grep -qxF` then reads it from a
            # here-string. The old form piped it into `grep -q`, whose early
            # exit could kill `cut` with SIGPIPE and — under
            # `set -o pipefail` — turn a present entry into an absent one.
            if ! grep -qxF "$key" <<< "$(stage_artifact_entries "$cfg" "$stage" | cut -d $'\x1f' -f2)"; then
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

    # ---- C9: every exit criterion must name its executor ------------------
    while IFS=$'\t' read -r rule detail; do
        [[ -z "${rule:-}" ]] && continue
        printf 'FAIL\tC9\t%s\n' "$detail"
        fails=$((fails + 1))
    done < <(stage_config_c9_findings "$cfg")

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
    exit_criteria:
      - "Design doc approved"
    criteria_gates: {}
    manual_only_criteria:
      "Design doc approved": "approval is a human action; no gate in this stage can decide it"
  ALPHA:
    order: 2
    required_gates:
      - "scripts/gate/check_g1_tpch_baseline.sh"
    exit_criteria:
      - "Coverage >= 50%"
    criteria_gates:
      "Coverage >= 50%": ["scripts/gate/check_coverage_v312.sh"]
  RC:
    order: 4
    required_gates:
      - "scripts/gate/check_g1_tpch_baseline.sh"
    optional_gates:
      - "scripts/gate/check_p13_soak_test.sh"
    exit_criteria:
      - "All 5 dimensions PASS"
      - "Tag rcN cut"
    criteria_gates:
      "All 5 dimensions PASS": ["scripts/gate/check_stage.sh"]
    manual_only_criteria:
      "Tag rcN cut": "tag cutting is a human release action"
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

    # C9a: an exit criterion whose mapping is simply gone.
    sed -e '/^      "Coverage >= 50%": \[/d' \
        -e '/^    criteria_gates:$/d' \
        "$tmp/clean.yaml" >"$tmp/c9a.yaml"
    # C9b: a criterion disposed of twice — the DRAFT manual-only criterion is
    #       also handed a gate.
    sed 's#^    criteria_gates: {}#    criteria_gates:\n      "Design doc approved": ["scripts/gate/check_stage.sh"]#' \
        "$tmp/clean.yaml" >"$tmp/c9b.yaml"
    # C9c: the mapping key no longer matches the criterion (a dead mapping).
    sed 's#^      "Coverage >= 50%": \[#      "Coverage >= 55%": [#' \
        "$tmp/clean.yaml" >"$tmp/c9c.yaml"
    # C9d: the mapping names a gate that this stage does not run and that no
    #       live workflow mentions — coverage claimed from a non-executor.
    sed 's#"All 5 dimensions PASS": \["scripts/gate/check_stage.sh"\]#"All 5 dimensions PASS": ["scripts/gate/check_beta_gate.sh"]#' \
        "$tmp/clean.yaml" >"$tmp/c9d.yaml"
    # C9e: manual_only without a reason.
    sed 's#"Tag rcN cut": "tag cutting is a human release action"#"Tag rcN cut": ""#' \
        "$tmp/clean.yaml" >"$tmp/c9e.yaml"
    # C9f: mapped to an empty gate list (claims coverage, names no executor).
    sed 's#"Coverage >= 50%": \["scripts/gate/check_coverage_v312.sh"\]#"Coverage >= 50%": []#' \
        "$tmp/clean.yaml" >"$tmp/c9f.yaml"

    # The clean sample must PASS.
    if consistency_check "$tmp/clean.yaml" "$VERSION" >"$tmp/clean.out" 2>&1; then
        echo "  [OK]   clean sample -> PASS"
    else
        echo "  [FAIL] clean sample -> FAIL (a clean document must not be reported broken)"
        sed 's/^/         /' "$tmp/clean.out"
        rc=1
    fi

    for case in c1 c2 c3 c4 c5 c6 c7 c8 c9a c9b c9c c9d c9e c9f; do
        if consistency_check "$tmp/$case.yaml" "$VERSION" >"$tmp/$case.out" 2>&1; then
            echo "  [FAIL] $case sample -> PASS (the gate cannot detect this violation)"
            rc=1
        else
            echo "  [OK]   $case sample -> FAIL (detected)"
        fi
    done

    # The failing samples must name the right rule.
    local expect_rule
    for case in c1 c2 c3 c4 c5 c6 c7 c8 c9a c9b c9c c9d c9e c9f; do
        case "$case" in
            c1) expect_rule="C1" ;;
            c2) expect_rule="C2" ;;
            c3) expect_rule="C3" ;;
            c4) expect_rule="C4" ;;
            c5) expect_rule="C5" ;;
            c6) expect_rule="C6" ;;
            c7) expect_rule="C7" ;;
            c8) expect_rule="C8" ;;
            c9a|c9b|c9c|c9d|c9e|c9f) expect_rule="C9" ;;
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
