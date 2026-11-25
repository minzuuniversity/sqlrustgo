#!/usr/bin/env bash
# fx19-sql-corpus-gate-corpus-truncated.sh — negative fixture for
# scripts/gate/check_sql_corpus_gate.sh
# fixture-id: fx19-sql-corpus-gate-corpus-truncated
# expect: red
# fault: truncate the SQL corpus — every sql_corpus/**/*.sql file is cut to
#        zero length, so the corpus no longer contains a single case.
#
# WHAT THIS ESTABLISHES, AND WHAT IT DOES NOT
# -------------------------------------------
# This fixture establishes exactly one property: **a truncated corpus makes the
# gate red** (the corpus measures 0.0% ⇒ "FAIL: pass rate 0.0% < 80.0%
# threshold").
#
# It does NOT establish "tampering the corpus' expected output makes the gate
# red" — that property is FALSE, and the proof is a separate characterization
# fixture:
#     audit/v4.1.0/neg/fx21-sql-corpus-expected-tamper-invisible-characterization.sh
# It is recorded on this gate's ledger row as a **known hole** (the
# machine-readable form), not as a `catches` claim.  Reason (measured):
# crates/sql-corpus/src/lib.rs parses `-- EXPECT: rows <n>` with
# `expect.split_whitespace().next()` — i.e. it takes the word "rows" and tries
# to parse THAT as a number.  The parse fails, `expected_rows` stays None, and
# `success` falls back to `unwrap_or(true)`.  The expected row count is
# therefore never compared: this gate's pass rate measures "does the statement
# execute without error", not "does it return the expected rows".
#
# WHY THIS IS CHEAP
# -----------------
# The gate runs `cargo test --release -p sqlrustgo-sql-corpus --test corpus_test
# test_sql_corpus_all` and reads "N% pass rate" out of the output.  The
# corpus is *data*: changing a `.sql` file does not change any Rust source,
# so cargo has nothing to rebuild and the release binary is reused.
#
# The corpus format (crates/sql-corpus/src/lib.rs) is:
#   `-- === CASE: <name>`   starts a case
#   `-- EXPECT: rows <n>`   the case succeeds iff the statement returned n
#                           rows (`success = rows_returned == expected_rows`)
#
# Arms:
#   A control  intact tree ⇒ the gate exits 0 with a real pass rate
#   B fault   the corpus is truncated ⇒ the gate must go red
#
# Part of the v4.1.0 readiness plan (Task 8).  NOT a gate: it never runs in
# ci_gate.sh, never changes the gate denominator (168).  It mutates a
# **scratch** checkout, runs the real gate there, and the scratch tree is
# restored afterwards.

set -uo pipefail

T="${1:?usage: fx19-sql-corpus-gate-corpus-truncated.sh <scratch-tree>}"
FX_ID="fx19-sql-corpus-gate-corpus-truncated"
GATE="scripts/gate/check_sql_corpus_gate.sh"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=audit/v4.1.0/neg/neg_two_layer.sh
. "$HERE/neg_two_layer.sh"

# restore() is the shared one (neg_two_layer.sh): it puts the scratch tree back
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
# The gate has two fail-closed answers to "the corpus measures 0%":
#   * it cannot extract a pass rate at all -> "could not extract pass rate"
#   * it extracts 0.0%                       -> "pass rate 0.0% < 80.0% threshold"
# (the corpus harness prints "Final Summary: N files, M cases, X% pass rate",
#  so a truncated corpus measures 0.0% rather than nothing).  Both are RED.
CLOSED=0
case "$GATE_TAIL" in
    *"could not extract pass rate"*) CLOSED=1 ;;
    *"pass rate"*"threshold"*) CLOSED=1 ;;
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
    echo "$FX_ID: the gate did not answer the truncated corpus with a rate verdict -> FIXTURE-FAIL"
    exit 3
fi

echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT, corpus truncated -> the gate refused to measure -> FIXTURE-PASS (the gate detects the truncated corpus)"
exit 0
