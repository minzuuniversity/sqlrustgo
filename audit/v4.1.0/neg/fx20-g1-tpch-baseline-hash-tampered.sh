#!/usr/bin/env bash
# fx20-g1-tpch-baseline-hash-tampered.sh — negative fixture for
# scripts/gate/check_g1_tpch_baseline.sh
# fixture-id: fx20-g1-tpch-baseline-hash-tampered
# expect: red
# fault: tamper the SF1 baseline number — `tpc_h_hash_sha256` in
#        tests/tpch_hashes_v380.json gets its first hex digit flipped.
#
# WHAT THE FIXTURE PROVES, PRECISELY
# ----------------------------------
# The ledger's target for G1 is "喂空结果集或篡改 SF1 基线数字 ⇒ 必须变红".
# On the frozen base this gate is **already red** for an unrelated reason:
# the base tree's TPC-H aggregate hash (be2e9d2e…) does not match the
# v3.8.0 baseline (7439ce80…), so step 4 of the gate reports "hash
# mismatch" before this fixture touches anything.  A fixture that only
# asserted "rc != 0" would therefore prove nothing.
#
# So this fixture asserts the **causal chain** instead:
#   arm A (control)  the intact tree ⇒ the gate reports the ORIGINAL
#                    baseline number as its expectation
#   arm B (fault)   the tampered baseline ⇒ the gate reports the TAMPERED
#                    number as its expectation, and stays red
# The baseline file therefore demonstrably drives the verdict: change the
# number, the gate's expectation changes with it.  That is the claim
# "篡改 SF1 基线数字 ⇒ 必须变红" in its checkable form.
#
# The TPC-H fixture data (8 .tbl files) is required by the gate and is
# present in this environment (~/sqlrustgo-tpch/data).  Without it the gate
# reports INCOMPLETE (exit 2), which this fixture reports as INCONCLUSIVE
# rather than as a red — a missing fixture is not a drift (R50).
#
# Part of the v4.1.0 readiness plan (Task 8).  NOT a gate: it never runs in
# ci_gate.sh, never changes the gate denominator (168).  It mutates a
# **scratch** checkout, runs the real gate there, and the scratch tree is
# restored afterwards.

set -uo pipefail

T="${1:?usage: fx20-g1-tpch-baseline-hash-tampered.sh <scratch-tree>}"
FX_ID="fx20-g1-tpch-baseline-hash-tampered"
GATE="scripts/gate/check_g1_tpch_baseline.sh"
HASH_FILE="tests/tpch_hashes_v380.json"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=audit/v4.1.0/neg/fx_two_layer.sh
. "$HERE/fx_two_layer.sh"

# restore() is the shared one (fx_two_layer.sh): it puts the scratch tree back
# and does not delete the evidence run directory this fixture belongs to.
restore() {
    fx_restore "$T"
}
restore

ORIG_HASH="$(python3 -c "
import json
print(json.load(open('$T/$HASH_FILE', encoding='utf-8'))['tpc_h_hash_sha256'])
")"
if [ -z "$ORIG_HASH" ]; then
    echo "$FX_ID: could not read the baseline hash -> INCONCLUSIVE"
    exit 4
fi
# One flipped hex digit: a realistic single-digit tamper of a baseline number.
TAMPERED_HASH="$(python3 -c "
h='$ORIG_HASH'
first='0' if h[0]!='0' else '1'
print(first+h[1:])
")"
echo "$FX_ID: baseline hash $ORIG_HASH -> $TAMPERED_HASH"

# --- arm A: control ----------------------------------------------------------
echo "----- fx20 arm A (control: intact tree) -----"
CTL_LOG="$(mktemp)"
( cd "$T" && timeout 1800 bash "$GATE" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
echo "control rc=$CTL_RC"
CTL_EXPECTED="$(grep -m1 -E '^\s*expected:\s*[0-9a-f]{64}' "$CTL_LOG" | grep -oE '[0-9a-f]{64}' || true)"
echo "control reported expected hash: ${CTL_EXPECTED:-<none>}"
rm -f "$CTL_LOG"
if [ "$CTL_EXPECTED" != "$ORIG_HASH" ]; then
    echo "$FX_ID: the control run did not compare against the baseline file (expected=$CTL_EXPECTED want=$ORIG_HASH) -> INCONCLUSIVE"
    restore
    exit 4
fi

# --- inject the fault --------------------------------------------------------
python3 - "$T/$HASH_FILE" "$TAMPERED_HASH" <<'PY'
import json, sys
path, new = sys.argv[1], sys.argv[2]
d = json.load(open(path, encoding="utf-8"))
d["tpc_h_hash_sha256"] = new
json.dump(d, open(path, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
PY
if [ $? -ne 0 ]; then
    echo "$FX_ID: could not inject the fault -> INCONCLUSIVE"
    restore
    exit 4
fi

# --- run the real gate through the tri-state runner --------------------------
fx_two_layer_run "$T" "$GATE" "$FX_ID" 2400
TWO_LAYER_RC=$?
echo "----- fx20 gate output (tail 16) -----"
tail -16 "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true
echo "----- end -----"

if [ "$TWO_LAYER_RC" -ne 0 ]; then
    echo "$FX_ID: could not run the two-layer check -> INCONCLUSIVE"
    restore
    rm -rf "$FX_RUN_DIR"
    exit 4
fi

GATE_TAIL="$(cat "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true)"
GATE_EXPECTED="$(printf '%s\n' "$GATE_TAIL" | grep -m1 -E '^\s*expected:\s*[0-9a-f]{64}' | grep -oE '[0-9a-f]{64}' || true)"
echo "$FX_ID: fault run reported expected hash: ${GATE_EXPECTED:-<none>}"

restore
rm -rf "$FX_RUN_DIR"

if ! fx_is_red "$FX_GATE_STATE" "$FX_GATE_RC"; then
    echo "$FX_ID: gate layer state=$FX_GATE_STATE rc=$FX_GATE_RC (want FAIL with a non-124 rc) -> FIXTURE-FAIL"
    exit 3
fi
if [ "$FX_STAGE_VERDICT" != "FAIL" ]; then
    echo "$FX_ID: stage layer verdict=$FX_STAGE_VERDICT (want FAIL) -> FIXTURE-FAIL"
    exit 3
fi
if [ "$GATE_EXPECTED" != "$TAMPERED_HASH" ]; then
    echo "$FX_ID: the gate did not compare against the tampered baseline (reported '$GATE_EXPECTED', want '$TAMPERED_HASH') -> FIXTURE-FAIL"
    exit 3
fi

echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT, verdict driven by the tampered SF1 baseline -> FIXTURE-PASS"
exit 0
