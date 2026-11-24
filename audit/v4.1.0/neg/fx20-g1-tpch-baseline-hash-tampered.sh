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
# The fixture asserts the **causal chain**:
#   arm A (control)  the intact tree ⇒ the gate's verdict is computed
#                    against the ORIGINAL baseline number (the gate's output
#                    names it, either as "expected: <hash>" or as
#                    "baseline hash matches (<8 hex>...)")
#   arm B (fault)   the tampered baseline ⇒ the gate's output names the
#                    TAMPERED number as its expectation and it is red
# The baseline file therefore demonstrably drives the verdict: change the
# number, the gate's expectation changes with it.  That is the claim
# "篡改 SF1 基线数字 ⇒ 必须变红" in its checkable form.
#
# The control arm must be GREEN, and it is: measured on the frozen base the
# gate prints "G1 PASS: TPC-H 22/22 baseline verified, hash=7439ce80..." and
# exits 0, i.e. the computed hash EQUALS the baseline number in
# tests/tpch_hashes_v380.json.  So the red that follows the tamper is
# attributable to the tamper; a red control would mean the red was already
# there (R50) and the fixture would report INCONCLUSIVE.
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
# shellcheck source=audit/v4.1.0/neg/neg_two_layer.sh
. "$HERE/neg_two_layer.sh"

# restore() is the shared one (neg_two_layer.sh): it puts the scratch tree back
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
ORIG_SHORT="${ORIG_HASH:0:8}"
# One flipped hex digit: a realistic single-digit tamper of a baseline number.
TAMPERED_HASH="$(python3 -c "
h='$ORIG_HASH'
first='0' if h[0]!='0' else '1'
print(first+h[1:])
")"
TAMPERED_SHORT="${TAMPERED_HASH:0:8}"
echo "$FX_ID: baseline hash $ORIG_HASH -> $TAMPERED_HASH"

# --- arm A: control ----------------------------------------------------------
echo "----- fx20 arm A (control: intact tree) -----"
CTL_LOG="$(mktemp)"
( cd "$T" && timeout 1800 bash "$GATE" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
echo "control rc=$CTL_RC"
CTL_EXPECTED="$(grep -m1 -E '^[[:space:]]*expected:[[:space:]]*[0-9a-f]{64}' "$CTL_LOG" | grep -oE '[0-9a-f]{64}' || true)"
CTL_MATCH="$(grep -m1 -E 'baseline hash matches' "$CTL_LOG" || true)"
echo "control reported expected hash: ${CTL_EXPECTED:-<none>}"
echo "control reported match line   : ${CTL_MATCH:-<none>}"
rm -f "$CTL_LOG"
# The gate must name the ORIGINAL baseline number, one way or the other.
if [ "$CTL_EXPECTED" != "$ORIG_HASH" ] && [ -z "$CTL_MATCH" ]; then
    echo "$FX_ID: the control run never compared against the baseline file (expected='${CTL_EXPECTED:-<none>}' match='${CTL_MATCH:-<none>}' want=$ORIG_HASH) -> INCONCLUSIVE"
    restore
    exit 4
fi
case "$CTL_MATCH" in
    *"$ORIG_SHORT"*) CTL_SAW_ORIG=1 ;;
    *) CTL_SAW_ORIG=0 ;;
esac
[ "$CTL_EXPECTED" = "$ORIG_HASH" ] && CTL_SAW_ORIG=1
echo "$FX_ID: control named the original baseline number: $CTL_SAW_ORIG (control rc=$CTL_RC)"
# The control must be GREEN: this gate is green on the frozen baseline (the
# computed hash equals the baseline number), so the red that follows the
# tamper is attributable to the tamper.  A red control would mean the red
# was already there (R50).
if [ "$CTL_RC" -ne 0 ]; then
    echo "$FX_ID: the gate is ALREADY RED on the intact tree (control rc=$CTL_RC) -> the mutation cannot be attributed to the red -> INCONCLUSIVE"
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
tail -8 "$FX_RUN_DIR"/001-*.stderr 2>/dev/null || true
echo "----- end -----"

if [ "$TWO_LAYER_RC" -ne 0 ]; then
    echo "$FX_ID: could not run the two-layer check -> INCONCLUSIVE"
    restore
    rm -rf "$FX_RUN_DIR"
    exit 4
fi

GATE_TAIL="$(cat "$FX_RUN_DIR"/001-*.stdout "$FX_RUN_DIR"/001-*.stderr 2>/dev/null || true)"
GATE_EXPECTED="$(printf '%s\n' "$GATE_TAIL" | grep -m1 -E '^[[:space:]]*expected:[[:space:]]*[0-9a-f]{64}' | grep -oE '[0-9a-f]{64}' || true)"
GATE_MATCH="$(printf '%s\n' "$GATE_TAIL" | grep -m1 -E 'baseline hash matches' || true)"
echo "$FX_ID: fault run reported expected hash: ${GATE_EXPECTED:-<none>}"
echo "$FX_ID: fault run reported match line   : ${GATE_MATCH:-<none>}"

restore
rm -rf "$FX_RUN_DIR"

if [ "$FX_GATE_STATE" = "INCOMPLETE" ] || [ "$FX_GATE_RC" = "124" ]; then
    echo "$FX_ID: the gate layer is INCOMPLETE (exit=${FX_GATE_RC:-?}) - a timeout is NOT a red (R50) -> INCONCLUSIVE"
    restore
    rm -rf "$FX_RUN_DIR"
    exit 4
fi
if ! fx_is_red "$FX_GATE_STATE" "$FX_GATE_RC"; then
    echo "$FX_ID: gate layer state=$FX_GATE_STATE rc=$FX_GATE_RC (want FAIL with a non-124 rc) -> FIXTURE-FAIL"
    exit 3
fi
if [ "$FX_STAGE_VERDICT" != "FAIL" ]; then
    echo "$FX_ID: stage layer verdict=$FX_STAGE_VERDICT (want FAIL) -> FIXTURE-FAIL"
    exit 3
fi
if [ "$GATE_EXPECTED" != "$TAMPERED_HASH" ]; then
    echo "$FX_ID: the gate did not compare against the tampered baseline (reported '${GATE_EXPECTED:-<none>}', want '$TAMPERED_HASH') -> FIXTURE-FAIL"
    exit 3
fi

echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT, verdict driven by the tampered SF1 baseline ($TAMPERED_SHORT) -> FIXTURE-PASS"
exit 0
