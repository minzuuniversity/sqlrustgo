#!/usr/bin/env bash
# fx25-criterion4-pending-rule-not-isolatable.sh — characterization fixture
# for criterion 4 (the ADR-mapping layer) of
# docs/refactor/v4.1.0-c/takeover/gen_verdict.py
# fixture-id: fx25-criterion4-pending-rule-not-isolatable
# expect: green-characterization
# fault: a task in REFACTOR_TASK_DAG.json gets an EMPTY `approved_adr_refs`
#        -- the very thing criterion 4 is about ("R1: every task must cite at
#        least one ADR").
#
# WHAT THIS DOCUMENTS (a known hole, not a capability)
# ---------------------------------------------------
# Criterion 4 reads `tasks_pending_adr_approval` out of
# `verify_handoff.py --check` and fails when the list is non-empty.  That rule
# **cannot be exercised by a tree mutation**: the handoff manifest binds the
# sha256 of the task DAG, so ANY edit of the DAG makes the checker's hash
# assertion fire first, the checker exits non-zero and prints NO pending list
# at all.  The criterion then fails for a completely different reason (the
# checker's exit code), and the pending-rule itself is never evaluated.
#
# Consequence: criterion 4 has **no isolatable negative fixture** today.
# What this fixture records is exactly that, with the measured evidence:
#   control -- the checker exits 0 and reports the pending task (C01-15);
#   fault   -- a one-field DAG edit makes the checker exit non-zero with no
#              pending list, so the rule is unreachable by mutation.
#
# Closing the hole needs a harness that feeds a synthetic DAG to the rule
# (an owner decision: it is a change to the checker's input contract, not a
# gate change).
#
# NOT a gate: it never runs in ci_gate.sh, never changes the gate denominator
# (168).  It mutates a **scratch** checkout, runs the real checker there,
# and restores the tree.

set -uo pipefail

T="${1:?usage: fx25-criterion4-pending-rule-not-isolatable.sh <scratch-tree>}"
FX_ID="fx25-criterion4-pending-rule-not-isolatable"
DAG="docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json"
CHECKER="docs/releases/v4.1.0/refactoring/tools/verify_handoff.py"

[ -d "$T" ] || { echo "$FX_ID: scratch tree '$T' is not a directory -> INCONCLUSIVE"; exit 4; }
ORIG="$(git -C "$T" rev-parse HEAD 2>/dev/null)" \
    || { echo "$FX_ID: '$T' is not a git checkout -> INCONCLUSIVE"; exit 4; }
for f in "$DAG" "$CHECKER"; do
    [ -f "$T/$f" ] || { echo "$FX_ID: $f not found -> INCONCLUSIVE"; exit 4; }
done

# The checker needs jsonschema==4.23.0.  Pick the same interpreter the
# generator picks; if none has it, the property is NOT establishable here --
# that is ENV_UNKNOWN, never a red (the repo's rule).
PY=""
for cand in "${V410_PYTHON:-}" /tmp/v410-task9-venv/bin/python python3 /usr/bin/python3; do
    [ -n "$cand" ] || continue
    if "$cand" -c "import jsonschema" >/dev/null 2>&1; then PY="$cand"; break; fi
done
[ -n "$PY" ] || { echo "$FX_ID: no interpreter with jsonschema -> INCONCLUSIVE (ENV_UNKNOWN, not a red)"; exit 4; }
echo "$FX_ID: using interpreter $PY"

restore() {
    git -C "$T" reset --hard "$ORIG" >/dev/null 2>&1 || true
    git -C "$T" clean -qfd -e audit/v4.1.0/runs/ >/dev/null 2>&1 || true
}
restore

run_checker() {   # <out.txt> -> echoes the exit code
    ( cd "$T" && timeout 600 "$PY" "$CHECKER" --check ) >"$1" 2>&1
    echo $?
}

# --- control: the intact tree ------------------------------------------------
TMP="$(mktemp -d "${TMPDIR:-/tmp}/fx25.XXXXXX")"
trap 'rm -rf "$TMP"; restore' EXIT
RC0="$(run_checker "$TMP/control.txt")"
echo "----- $FX_ID : control (intact tree) -----"
echo "control checker rc=$RC0"
grep -o '"tasks_pending_adr_approval": \[[^]]*\]' "$TMP/control.txt" | head -1 || true
if [ "$RC0" != "0" ]; then
    echo "$FX_ID: the checker does not even run clean on the intact tree -> INCONCLUSIVE"
    tail -5 "$TMP/control.txt"
    exit 4
fi
if ! grep -q '"tasks_pending_adr_approval"' "$TMP/control.txt"; then
    echo "$FX_ID: the checker printed no pending list -> INCONCLUSIVE"
    exit 4
fi

# --- fault: empty approved_adr_refs on one task -----------------------------
python3 - "$T/$DAG" <<'PY'
import json, sys
path = sys.argv[1]
doc = json.load(open(path, encoding="utf-8"))
tasks = doc.get("tasks") or []
if not tasks:
    sys.exit("no tasks in the DAG")
for t in tasks:
    if t.get("approved_adr_refs"):
        t["approved_adr_refs"] = []
        break
else:
    sys.exit("no task with a non-empty approved_adr_refs")
json.dump(doc, open(path, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
PY
[ $? -ne 0 ] && { echo "$FX_ID: could not empty approved_adr_refs -> INCONCLUSIVE"; exit 4; }

RC1="$(run_checker "$TMP/fault.txt")"
echo "----- $FX_ID : fault (one task's approved_adr_refs emptied) -----"
echo "fault checker rc=$RC1"
tail -3 "$TMP/fault.txt"

restore
if [ "$RC1" != "0" ] && ! grep -q '"tasks_pending_adr_approval"' "$TMP/fault.txt"; then
    echo "$FX_ID: the pending rule is unreachable by mutation (the manifest hash chain fires first) -> UNFALSIFIABLE (as documented): criterion 4 needs a synthetic-DAG harness"
    exit 0
fi
echo "$FX_ID: the pending rule WAS reachable by mutation -> the documented hole is closed; this characterization fixture should be replaced by a red fixture"
exit 3
