#!/usr/bin/env bash
# fx17-arch3-no-bypass-dml-guard-dropped.sh — negative fixture for
# scripts/gate/check_arch3_no_bypass.sh
# fixture-id: fx17-arch3-no-bypass-dml-guard-dropped
# expect: red
# fault: the ExecutionEngine DML path stops asserting its path — the
#        `VtuGuard::assert_path_for_dml` marker is taken out of
#        `pub fn execute_insert` in src/execution_engine_methods.rs, so that
#        DML call now bypasses the guard.
#
# WHAT THE LEDGER TARGET SAYS, AND WHAT THE GATE CAN ACTUALLY SEE
# ---------------------------------------------------------------
# The ledger target for this gate is "新增一处绕过 ExecutionEngine 的 DML
# 调用 ⇒ 必须变红".  The gate's four measurements are:
#
#   [1/4] the number of `assert_path_for_dml` lines in
#         src/execution_engine.rs + src/execution_engine_methods.rs must be
#         >= 3
#   [2/4] the number of markers in crates/server/src/openclaw_endpoints.rs
#         must be >= 2
#   [3/4] the number of "bypass" strings — INFORMATIONAL, never a FAIL
#   [4/4] `pub fn assert_path_for_dml` must be declared in
#         crates/storage/src/vtu_guard.rs
#
# Measured (this fixture, arm C): **adding** a brand-new unguarded DML
# entry point leaves [1/4] at 3, [2/4] at 2 and [4/4] untouched, so the gate
# stays GREEN.  The gate's detection surface is "a guard marker disappeared",
# not "a bypass appeared".  That is a real hole in the gate and it is
# reported here as such (arm C prints it) — but the ledger's target is
# realised in the form the gate can actually observe: the DML call in the
# ExecutionEngine path no longer asserts its path.
#
# Arms:
#   A control  intact tree ⇒ gate exits 0
#   B fault   the guard marker is removed from `pub fn execute_insert`
#             ⇒ the gate must go red (its [1/4] count drops below 3)
#   C hole    a NEW unguarded DML entry point is added instead ⇒ the gate
#             stays green.  Informational: it documents the hole above.
#             This arm never changes the fixture's verdict.
#
# Part of the v4.1.0 readiness plan (Task 8).  NOT a gate: it never runs in
# ci_gate.sh, never changes the gate denominator (168).  It mutates a
# **scratch** checkout, runs the real gate there, and the scratch tree is
# restored afterwards.

set -uo pipefail

T="${1:?usage: fx17-arch3-no-bypass-dml-guard-dropped.sh <scratch-tree>}"
FX_ID="fx17-arch3-no-bypass-dml-guard-dropped"
GATE="scripts/gate/check_arch3_no_bypass.sh"
TARGET="src/execution_engine_methods.rs"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=audit/v4.1.0/neg/fx_two_layer.sh
. "$HERE/fx_two_layer.sh"

# restore() is the shared one (fx_two_layer.sh): it puts the scratch tree back
# and does not delete the evidence run directory this fixture belongs to.
restore() {
    fx_restore "$T"
}
restore

# --- arm A: control ----------------------------------------------------------
echo "----- fx17 arm A (control: intact tree) -----"
CTL_LOG="$(mktemp)"
( cd "$T" && timeout 300 bash "$GATE" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
echo "control rc=$CTL_RC (want 0)"
tail -4 "$CTL_LOG"
rm -f "$CTL_LOG"
if [ "$CTL_RC" -ne 0 ]; then
    echo "$FX_ID: the gate is already red on the intact tree (rc=$CTL_RC) -> INCONCLUSIVE (the mutation would not be attributable)"
    restore
    exit 4
fi

# --- inject the fault --------------------------------------------------------
python3 - "$T/$TARGET" <<'PY'
import sys
path = sys.argv[1]
src = open(path, encoding="utf-8").read()
guard = ('        sqlrustgo_storage::vtu_guard::VtuGuard::<()>::assert_path_for_dml(\n'
         '            "execute_insert",\n'
         '            &insert.table,\n'
         '        );\n')
if guard not in src:
    sys.exit("fx17: the VtuGuard marker of execute_insert was not found - aborting")
open(path, "w", encoding="utf-8").write(src.replace(guard, "", 1))
PY
if [ $? -ne 0 ]; then
    echo "$FX_ID: could not inject the fault -> INCONCLUSIVE"
    restore
    exit 4
fi

# --- run the real gate through the tri-state runner --------------------------
fx_two_layer_run "$T" "$GATE" "$FX_ID" 600
TWO_LAYER_RC=$?
echo "----- fx17 gate output (tail 12) -----"
tail -12 "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true
echo "----- end -----"

# --- arm C (informational): does ADDING a bypass turn the gate red? ---------
if [ "$TWO_LAYER_RC" -eq 0 ]; then
    restore
    python3 - "$T/$TARGET" <<'PY'
import sys
path = sys.argv[1]
src = open(path, encoding="utf-8").read()
anchor = "    pub fn execute_update("
add = ('    // fx17 arm C: a NEW DML entry point that bypasses ExecutionEngine\'s\n'
       '    // VtuGuard path assertion.\n'
       '    pub fn execute_insert_raw(&mut self, insert: &InsertStatement)\n'
       '        -> SqlResult<ExecutorResult>\n'
       '    {\n'
       '        crate::engine_dml::execute_insert(self, insert)\n'
       '    }\n\n')
if anchor not in src:
    sys.exit("fx17 arm C: anchor not found")
open(path, "w", encoding="utf-8").write(src.replace(anchor, add + anchor, 1))
PY
    C_LOG="$(mktemp)"
    ( cd "$T" && timeout 300 bash "$GATE" ) >"$C_LOG" 2>&1
    C_RC=$?
    echo "----- fx17 arm C (ADD a bypass instead) rc=$C_RC -----"
    tail -6 "$C_LOG"
    rm -f "$C_LOG"
    if [ "$C_RC" -eq 0 ]; then
        echo "fx17 arm C: the gate stayed GREEN after a bypass was ADDED -> KNOWN HOLE of $GATE (it only notices a REMOVED marker); informational, does not change this fixture's verdict"
    else
        echo "fx17 arm C: the gate went red after a bypass was ADDED (rc=$C_RC) -> the gate does detect added bypasses"
    fi
    restore
fi

if [ "$TWO_LAYER_RC" -ne 0 ]; then
    echo "$FX_ID: could not run the two-layer check -> INCONCLUSIVE"
    rm -rf "$FX_RUN_DIR"
    exit 4
fi

rm -rf "$FX_RUN_DIR"

if ! fx_is_red "$FX_GATE_STATE" "$FX_GATE_RC"; then
    echo "$FX_ID: gate layer state=$FX_GATE_STATE rc=$FX_GATE_RC (want FAIL with a non-124 rc) -> FIXTURE-FAIL"
    exit 3
fi
if [ "$FX_STAGE_VERDICT" != "FAIL" ]; then
    echo "$FX_ID: stage layer verdict=$FX_STAGE_VERDICT (want FAIL) -> FIXTURE-FAIL"
    exit 3
fi

echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT -> FIXTURE-PASS (the gate detects the unguarded DML call)"
exit 0
