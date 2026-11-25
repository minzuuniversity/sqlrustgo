#!/usr/bin/env bash
# fx22-arch3-added-bypass-invisible-characterization.sh —
# characterization fixture for scripts/gate/check_arch3_no_bypass.sh
# fixture-id: fx22-arch3-added-bypass-invisible-characterization
# expect: green-characterization
# fault: ADD a new DML entry point to the ExecutionEngine that performs DML
#        **without** the `VtuGuard::assert_path_for_dml` marker — i.e. a DML
#        call that bypasses ExecutionEngine's path assertion.
#
# WHAT THIS DOCUMENTS (a known hole, not a capability)
# ---------------------------------------------------
# The ledger's original target for this gate was "新增一处绕过 ExecutionEngine
# 的 DML 调用 ⇒ 必须变红".  This fixture proves that target is **FALSE**: after
# the bypass is added the gate still exits 0.
#
# Why (scripts/gate/check_arch3_no_bypass.sh): the gate measures four things,
# and none of them notices a bypass that was ADDED:
#
#   [1/4] the number of `assert_path_for_dml` lines in src/execution_engine.rs
#         + src/execution_engine_methods.rs must be >= 3   (an added bypass
#         does not change the count)
#   [2/4] the number of markers in crates/server/src/openclaw_endpoints.rs
#         must be >= 2                                   (ditto)
#   [3/4] the number of "bypass" strings — INFORMATIONAL, never a FAIL
#   [4/4] `pub fn assert_path_for_dml` must be declared in
#         crates/storage/src/vtu_guard.rs                 (ditto)
#
# The gate's detection surface is "a guard marker DISAPPEARED", not "a bypass
# APPEARED".  What it does catch is established by
# fx17-arch3-no-bypass-dml-guard-dropped.sh.
#
# The consequence for the ledger: the property "the gate catches a newly added
# bypass" is FALSE and is recorded as a **known hole** on this gate's ledger
# row, not as a `catches` claim.
#
# Part of the v4.1.0 readiness plan (Task 8).  NOT a gate: it never runs in
# ci_gate.sh, never changes the gate denominator (168).  It mutates a
# **scratch** checkout, runs the real gate there, and the scratch tree is
# restored afterwards.

set -uo pipefail

T="${1:?usage: fx22-arch3-added-bypass-invisible-characterization.sh <scratch-tree>}"
FX_ID="fx22-arch3-added-bypass-invisible-characterization"
GATE="scripts/gate/check_arch3_no_bypass.sh"
TARGET="src/execution_engine_methods.rs"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

restore() {
    git -C "$T" checkout -- . >/dev/null 2>&1 || true
    git -C "$T" clean -qfd -e audit/v4.1.0/runs/ >/dev/null 2>&1 || true
}
restore

if [ ! -f "$T/$TARGET" ]; then
    echo "$FX_ID: $TARGET not found in the scratch tree -> INCONCLUSIVE"
    exit 4
fi

# --- control: the intact tree ------------------------------------------------
CTL_LOG="$(mktemp)"
( cd "$T" && timeout 600 bash "$GATE" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
echo "----- fx22 control (intact tree) -----"
echo "control rc=$CTL_RC"
rm -f "$CTL_LOG"
if [ "$CTL_RC" -ne 0 ]; then
    echo "$FX_ID: the gate is red on the intact tree (rc=$CTL_RC) -> the characterization needs a green baseline -> INCONCLUSIVE"
    restore
    exit 4
fi

# --- inject the fault: add a bypassing DML entry point -----------------------
python3 - "$T/$TARGET" <<'PY'
import sys
path = sys.argv[1]
src = open(path, encoding="utf-8").read()
anchor = "    pub fn execute_update("
add = ('    // fx22: a NEW DML entry point that bypasses ExecutionEngine\'s\n'
       '    // VtuGuard path assertion.\n'
       '    pub fn execute_insert_raw(&mut self, insert: &InsertStatement)\n'
       '        -> SqlResult<ExecutorResult>\n'
       '    {\n'
       '        crate::engine_dml::execute_insert(self, insert)\n'
       '    }\n\n')
if anchor not in src:
    sys.exit("fx22: anchor not found")
open(path, "w", encoding="utf-8").write(src.replace(anchor, add + anchor, 1))
print("fx22: added an unguarded DML entry point (execute_insert_raw)")
PY
if [ $? -ne 0 ]; then
    echo "$FX_ID: could not inject the fault -> INCONCLUSIVE"
    restore
    exit 4
fi

# --- run the real gate against the mutated tree -------------------------------
LOG="$(mktemp)"
( cd "$T" && timeout 600 bash "$GATE" ) >"$LOG" 2>&1
RC=$?
echo "----- fx22 fault run (a bypass was ADDED) -----"
echo "fault rc=$RC"
tail -8 "$LOG"
rm -f "$LOG"

restore

# The characterization holds when the gate STAYS GREEN: an ADDED bypass is
# invisible to it.
if [ "$RC" -ne 0 ]; then
    echo "$FX_ID: the gate went RED after a bypass was ADDED (rc=$RC) -> the 'unfalsifiable' claim is WRONG (the gate does see added bypasses)"
    exit 3
fi
echo "$FX_ID: the gate stayed GREEN (rc=0) after a bypassing DML call was ADDED -> UNFALSIFIABLE (as documented): this gate only notices a REMOVED marker, not an added bypass"
exit 0
