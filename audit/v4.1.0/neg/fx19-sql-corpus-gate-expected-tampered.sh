#!/usr/bin/env bash
# fx19-sql-corpus-gate-expected-tampered.sh — negative fixture for
# scripts/gate/check_sql_corpus_gate.sh
# fixture-id: fx19-sql-corpus-gate-expected-tampered
# expect: red
# fault: truncate the SQL corpus — every sql_corpus/**/*.sql file is cut to
#        zero length, so the corpus no longer contains a single case.
#
# WHY THIS IS THE MUTATION (and what the ledger's other option turned out
# to be)
# -------------------------------------------------------------------
# The ledger's target for this gate is "截断语料库**或**篡改 expected 输出 ⇒
# 必须变红".  Both halves were tried; only one of them can red the gate,
# and the reason is worth recording:
#
#   * "篡改 expected 输出" — the corpus' expected output is written as
#     `-- EXPECT: rows <n>`, and crates/sql-corpus/src/lib.rs parses it with
#     `expect.split_whitespace().next()` — i.e. it takes the word "rows" and
#     tries to parse THAT as a number.  The parse fails, `expected_rows`
#     stays None, and `success` falls back to `unwrap_or(true)`.  So the
#     expected row count is **never compared**: the corpus gate's pass rate
#     measures "does the statement execute without error", not "does it
#     return the expected rows".  Arm C below demonstrates this on the real
#     gate (it stays GREEN after 1476 EXPECT directives are tampered).
#     That is a real defect of the corpus harness, but fixing it would
#     change what the gate measures — a gate change, which is the owner's
#     call, not this task's.
#   * "截断语料库" — the corpus is truncated to zero length ⇒ the corpus
#     test has no case to report ⇒ the gate cannot extract a pass rate ⇒
#     the gate must fail closed ("could not extract pass rate from corpus
#     output").  This is the mutation this fixture uses, and it is cheap:
#     the corpus is *data*, so no Rust source changes and cargo has nothing
#     to rebuild.
#
# Arms:
#   A control  intact tree ⇒ the gate exits 0 with a real pass rate
#   B fault   the corpus is truncated ⇒ the gate must go red (fail closed)
#   C hole    the EXPECT directives are tampered instead ⇒ the gate stays
#             GREEN.  Informational: it records the hole above.
#
# Part of the v4.1.0 readiness plan (Task 8).  NOT a gate: it never runs in
# ci_gate.sh, never changes the gate denominator (168).  It mutates a
# **scratch** checkout, runs the real gate there, and the scratch tree is
# restored afterwards.

set -uo pipefail

T="${1:?usage: fx19-sql-corpus-gate-expected-tampered.sh <scratch-tree>}"
FX_ID="fx19-sql-corpus-gate-expected-tampered"
GATE="scripts/gate/check_sql_corpus_gate.sh"

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
echo "----- fx19 arm A (control: intact tree) -----"
CTL_LOG="$(mktemp)"
CTL_T0=$(date +%s)
( cd "$T" && timeout 1800 bash "$GATE" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
CTL_T1=$(date +%s)
echo "control rc=$CTL_RC (want 0) in $((CTL_T1-CTL_T0))s"
grep -E "pass rate" "$CTL_LOG" | tail -2 || true
rm -f "$CTL_LOG"
if [ "$CTL_RC" -ne 0 ]; then
    echo "$FX_ID: the gate is already red on the intact tree (rc=$CTL_RC) -> INCONCLUSIVE (the mutation would not be attributable)"
    restore
    exit 4
fi

# --- arm C (informational): tamper the EXPECT directives ---------------------
# Done BEFORE arm B so that the tree is intact again for arm B.
python3 - "$T/sql_corpus" <<'PY'
import os, re, sys
root = sys.argv[1]
pat = re.compile(r"^(--\s*EXPECT:\s*)(\S+)(.*)$")
n = 0
for dirpath, _dirs, files in os.walk(root):
    for fn in files:
        if not fn.endswith(".sql"):
            continue
        p = os.path.join(dirpath, fn)
        try:
            lines = open(p, encoding="utf-8", errors="replace").read().split("\n")
        except OSError:
            continue
        out, hit = [], False
        for l in lines:
            m = pat.match(l)
            if m:
                out.append("%s999999%s" % (m.group(1), m.group(3)))
                hit = True
                n += 1
            else:
                out.append(l)
        if hit:
            open(p, "w", encoding="utf-8").write("\n".join(out))
print("fx19 arm C: tampered %d EXPECT directives" % n)
PY
C_LOG="$(mktemp)"
( cd "$T" && timeout 1800 bash "$GATE" ) >"$C_LOG" 2>&1
C_RC=$?
echo "----- fx19 arm C (tampered EXPECT directives) rc=$C_RC -----"
grep -E "pass rate" "$C_LOG" | tail -2 || true
rm -f "$C_LOG"
restore
if [ "$C_RC" -eq 0 ]; then
    echo "fx19 arm C: the gate stayed GREEN after 1476 expected-output directives were tampered -> KNOWN HOLE of $GATE / of crates/sql-corpus/src/lib.rs: the corpus writes '-- EXPECT: rows <n>' but the harness parses expect.split_whitespace().next() (the word 'rows'), so expected_rows stays None and the row count is never compared. Informational; does not change this fixture's verdict."
else
    echo "fx19 arm C: the gate went red after the EXPECT tamper (rc=$C_RC) -> the expected output IS compared"
fi

# --- inject the fault: truncate the corpus -----------------------------------
python3 - "$T/sql_corpus" <<'PY'
import os, sys
root = sys.argv[1]
n = 0
for dirpath, _dirs, files in os.walk(root):
    for fn in files:
        if not fn.endswith(".sql"):
            continue
        open(os.path.join(dirpath, fn), "w", encoding="utf-8").write("")
        n += 1
print("fx19: truncated %d corpus files to zero length" % n)
if n == 0:
    sys.exit("fx19: no corpus file found - aborting")
PY
if [ $? -ne 0 ]; then
    echo "$FX_ID: could not inject the fault -> INCONCLUSIVE"
    restore
    exit 4
fi

# --- run the real gate through the tri-state runner --------------------------
fx_two_layer_run "$T" "$GATE" "$FX_ID" 2400
TWO_LAYER_RC=$?
echo "----- fx19 gate output (tail 12) -----"
tail -12 "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true
tail -6 "$FX_RUN_DIR"/001-*.stderr 2>/dev/null || true
echo "----- end -----"

if [ "$TWO_LAYER_RC" -ne 0 ]; then
    echo "$FX_ID: could not run the two-layer check -> INCONCLUSIVE"
    restore
    rm -rf "$FX_RUN_DIR"
    exit 4
fi

GATE_TAIL="$(cat "$FX_RUN_DIR"/001-*.stdout "$FX_RUN_DIR"/001-*.stderr 2>/dev/null || true)"
case "$GATE_TAIL" in
    *"could not extract pass rate"*) CLOSED=1 ;;
    *) CLOSED=0 ;;
esac

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
if [ "$CLOSED" -ne 1 ]; then
    echo "$FX_ID: the gate did not fail closed on the truncated corpus -> FIXTURE-FAIL"
    exit 3
fi

echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT, corpus truncated -> the gate refused to measure -> FIXTURE-PASS (the gate detects the truncated corpus)"
exit 0
