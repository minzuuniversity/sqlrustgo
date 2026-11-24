#!/usr/bin/env bash
# fx_two_layer.sh — the shared "two-layer RED" mechanism for the ALPHA
# negative fixtures (Task 8 of the v4.1.0 readiness plan).
#
# WHAT THIS IS
# ------------
# A negative fixture has to prove two different things, and the plan requires
# them to be proven **by one and the same execution**:
#
#   (a) GATE LAYER   the target gate itself says FAIL (exit_code != 0),
#   (b) STAGE LAYER  the stage summary, which **independently re-parses** the
#                    authoritative gate set, concludes != PASS, and attributes
#                    the red to the target gate.
#
# The two layers are produced from ONE `scripts/gate/run_gates.py execute`
# run and ONE `results.jsonl`.  Running the gate twice (once "directly" for
# the gate layer, once "through the summary" for the stage layer) would let
# the two layers disagree — and a disagreement is exactly what a red is
# supposed to be checked for.  So: one run, two readings of the same
# evidence.
#
# The tools are the repository's **existing** tri-state ones:
#   scripts/gate/run_gates.py   plan / execute   (three-state bookkeeping)
#   scripts/gate/gate_report.py report            (independent re-parse)
# This is not a second harness: no new scheduler, no new ledger, no new
# runner (global constraint C2).
#
# EXIT-CODE DISCIPLINE (R50)
# --------------------------
# A timeout is **not** a red.  `run_gates.py` maps exit 124 (timeout) and any
# signal to INCOMPLETE, and this helper keeps that distinction: a fixture
# whose target gate ends INCOMPLETE is reported as INCONCLUSIVE, never as
# RED, and never counts towards a 7/7.
#
# USAGE (from a fixture script)
# ------------------------------
#   . "$(dirname "${BASH_SOURCE[0]}")/fx_two_layer.sh"
#   fx_two_layer_run "$T" "scripts/gate/check_xxx.sh" "fxNN-..." 1800
#   # afterwards:
#   $FX_GATE_STATE   PASS | FAIL | INCOMPLETE | MISSING
#   $FX_GATE_RC      the gate's own exit code (124 == timeout)
#   $FX_STAGE_VERDICT PASS | FAIL | INCONCLUSIVE | UNPARSEABLE
#   $FX_STAGE_COUNTS json of the summary's per-state counts
#   $FX_RUN_DIR      the run directory (results.jsonl, summary.json, logs)
#
# The caller is responsible for restoring the scratch tree afterwards.

set -uo pipefail

# fx_two_layer_run <scratch-tree> <gate-path> <run-id> <timeout-sec>
fx_two_layer_run() {
    local tree="$1" gate="$2" run_id="$3" timeout="${4:-1800}"

    FX_RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/fxtwolayer.XXXXXX")"
    FX_GATE_STATE="MISSING"; FX_GATE_RC=""; FX_STAGE_VERDICT="UNPARSEABLE"; FX_STAGE_COUNTS="{}"

    local plan_all="$FX_RUN_DIR/plan_all.jsonl"
    local plan_one="$FX_RUN_DIR/plan_target.jsonl"

    # 1. the authoritative ALPHA set, derived by the runner from
    #    STAGE_CONFIG.yaml (never hand-written here).
    python3 "$tree/scripts/gate/run_gates.py" plan \
        --stage ALPHA --version v4.1.0 \
        --plan "$plan_all" --run-id "$run_id" >"$FX_RUN_DIR/plan.log" 2>&1
    if [ $? -ne 0 ] || [ ! -s "$plan_all" ]; then
        echo "fx_two_layer: could not build the authoritative ALPHA plan" >&2
        return 4
    fi

    # 2. scope it to this fixture's target gate.  The scope is declared here
    #    (one gate per fixture); it is NOT a way to shrink the expectation
    #    set to get a green — the expected set handed to the summariser is
    #    this very same file, so the summary's verdict is about this gate.
    grep -F "\"gate\": \"$gate\"" "$plan_all" > "$plan_one"
    if [ ! -s "$plan_one" ]; then
        echo "fx_two_layer: $gate is not in the ALPHA stage list" >&2
        return 4
    fi
    cp "$plan_one" "$FX_RUN_DIR/expected.jsonl"

    # 3. THE run.  Both layers are read off this one results.jsonl.
    #    NOTE: `--repo-root` is a top-level option of both tools, so it has to
    #    come BEFORE the subcommand.
    python3 "$tree/scripts/gate/run_gates.py" --repo-root "$tree" execute \
        --plan "$plan_one" \
        --results "$FX_RUN_DIR/results.jsonl" \
        --timeout-sec "$timeout" >"$FX_RUN_DIR/execute.log" 2>&1
    FX_EXEC_RC=$?

    # 4. gate layer: the target gate's own row.
    FX_GATE_STATE="$(python3 - "$FX_RUN_DIR/results.jsonl" "$gate" <<'PY'
import json, sys
try:
    rows = [json.loads(l) for l in open(sys.argv[1], encoding="utf-8") if l.strip()]
except OSError:
    print("MISSING"); raise SystemExit(0)
for r in rows:
    if r.get("gate") == sys.argv[2]:
        print(r.get("state", "MISSING")); raise SystemExit(0)
print("MISSING")
PY
)"
    FX_GATE_RC="$(python3 - "$FX_RUN_DIR/results.jsonl" "$gate" <<'PY'
import json, sys
try:
    rows = [json.loads(l) for l in open(sys.argv[1], encoding="utf-8") if l.strip()]
except OSError:
    print(""); raise SystemExit(0)
for r in rows:
    if r.get("gate") == sys.argv[2]:
        print("" if r.get("exit_code") is None else r.get("exit_code")); raise SystemExit(0)
print("")
PY
)"

    # 5. stage layer: the summariser re-parses the authoritative set and the
    #    SAME results.jsonl.  It does not trust the runner's self-verdict.
    python3 "$tree/scripts/gate/gate_report.py" --repo-root "$tree" \
        --plan "$plan_one" \
        --results "$FX_RUN_DIR/results.jsonl" \
        --authoritative-file "$FX_RUN_DIR/expected.jsonl" \
        --catalog "$tree/docs/releases/v4.1.0/GATE_CATALOG.jsonl" \
        --out "$FX_RUN_DIR/summary.json" >"$FX_RUN_DIR/report.log" 2>&1
    FX_REPORT_RC=$?
    FX_STAGE_VERDICT="$(python3 - "$FX_RUN_DIR/summary.json" <<'PY'
import json, sys
try:
    print(json.load(open(sys.argv[1], encoding="utf-8")).get("verdict", "UNPARSEABLE"))
except Exception:
    print("UNPARSEABLE")
PY
)"
    FX_STAGE_COUNTS="$(python3 - "$FX_RUN_DIR/summary.json" <<'PY'
import json, sys
try:
    print(json.dumps(json.load(open(sys.argv[1], encoding="utf-8")).get("counts", {})))
except Exception:
    print("{}")
PY
)"

    echo "FX-GATE-LAYER: gate=$gate state=$FX_GATE_STATE exit_code=$FX_GATE_RC"
    echo "FX-STAGE-LAYER: verdict=$FX_STAGE_VERDICT counts=$FX_STAGE_COUNTS"
    return 0
}

# fx_is_red <state> <rc> — a gate layer counts as RED only on a real FAIL
# with a real exit code.  124 (timeout) and signals are INCONCLUSIVE.
fx_is_red() {
    local state="${1:-}" rc="${2:-}"
    [ "$state" = "FAIL" ] || return 1
    case "$rc" in
        ''|*[!0-9]*) return 1 ;;
    esac
    [ "$rc" -ne 0 ] || return 1
    [ "$rc" -ne 124 ] || return 1
    return 0
}
