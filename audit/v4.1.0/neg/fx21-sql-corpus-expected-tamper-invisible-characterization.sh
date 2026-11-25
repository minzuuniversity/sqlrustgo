#!/usr/bin/env bash
# fx21-sql-corpus-expected-tamper-invisible-characterization.sh —
# characterization fixture for scripts/gate/check_sql_corpus_gate.sh
# fixture-id: fx21-sql-corpus-expected-tamper-invisible-characterization
# expect: green-characterization
# fault: tamper the corpus' EXPECTED OUTPUT — every `-- EXPECT: rows <n>` in
#        sql_corpus/**/*.sql is rewritten so the expectation can never hold.
#
# WHAT THIS DOCUMENTS (a known hole, not a capability)
# ---------------------------------------------------
# This fixture proves that the SQL corpus gate **cannot see** a tampered
# expected output: after the tamper the gate still reports the same pass rate
# and still exits 0.
#
# Why (crates/sql-corpus/src/lib.rs):
#
#     } else if trimmed.starts_with("-- EXPECT:") {
#         ...
#         } else if expect.starts_with("rows") {
#             if let Ok(n) = expect.split_whitespace().next().unwrap_or("0").parse() {
#
# `expect` is "rows 10"; `split_whitespace().next()` yields the word **"rows"**,
# and parsing that as a number always fails ⇒ `expected_rows` stays `None` ⇒
# `success` falls back to `unwrap_or(true)`.  The expected row count is never
# compared, so the gate's pass rate measures "does the statement execute
# without error", not "does it return the expected rows".
#
# The consequence for the ledger: the property "the gate catches a tampered
# expected output" is FALSE and is recorded as a **known hole** on this gate's
# ledger row, not as a `catches` claim.  What the gate does catch (a truncated
# corpus) is established by fx19-sql-corpus-gate-corpus-truncated.sh.
#
# What this fixture ASSERTS (both, not just one):
#   * the gate still exits 0 after the tamper, and
#   * the pass rate the gate reports is UNCHANGED (control == fault) — i.e. the
#     tampered expectation has no effect on the gate's own measurement.
#     The rate is read out of the gate's own line
#     "SQL Corpus pass rate: <rate>% (threshold: 80.0%)".
#
# Part of the v4.1.0 readiness plan (Task 8).  NOT a gate: it never runs in
# ci_gate.sh, never changes the gate denominator (168).  It mutates a
# **scratch** checkout, runs the real gate there, and the scratch tree is
# restored afterwards.

set -uo pipefail

T="${1:?usage: fx21-sql-corpus-expected-tamper-invisible-characterization.sh <scratch-tree>}"
FX_ID="fx21-sql-corpus-expected-tamper-invisible-characterization"
GATE="scripts/gate/check_sql_corpus_gate.sh"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

restore() {
    git -C "$T" checkout -- . >/dev/null 2>&1 || true
    git -C "$T" clean -qfd -e audit/v4.1.0/runs/ >/dev/null 2>&1 || true
}
restore

# --- control: the intact tree ------------------------------------------------
CTL_LOG="$(mktemp)"
( cd "$T" && timeout 1800 bash "$GATE" ) >"$CTL_LOG" 2>&1
CTL_RC=$?
CTL_RATE="$(grep -m1 -oE 'pass rate: [0-9]+\.[0-9]+%' "$CTL_LOG" | grep -oE '[0-9]+\.[0-9]+' || true)"
echo "----- fx21 control (intact tree) -----"
echo "control rc=$CTL_RC pass rate=${CTL_RATE:-<none>}"
rm -f "$CTL_LOG"
if [ "$CTL_RC" -ne 0 ]; then
    echo "$FX_ID: the gate is red on the intact tree (rc=$CTL_RC) -> the characterization needs a green baseline -> INCONCLUSIVE"
    restore
    exit 4
fi

# --- inject the fault: rewrite every EXPECT directive ------------------------
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
print("fx21: rewrote %d EXPECT directives to an unreachable row count" % n)
if n == 0:
    sys.exit("fx21: no EXPECT directive found - the corpus format changed, aborting")
PY
if [ $? -ne 0 ]; then
    echo "$FX_ID: could not inject the fault -> INCONCLUSIVE"
    restore
    exit 4
fi

# --- run the real gate against the mutated tree -------------------------------
LOG="$(mktemp)"
( cd "$T" && timeout 1800 bash "$GATE" ) >"$LOG" 2>&1
RC=$?
FAULT_RATE="$(grep -m1 -oE 'pass rate: [0-9]+\.[0-9]+%' "$LOG" | grep -oE '[0-9]+\.[0-9]+' || true)"
echo "----- fx21 fault run (tampered EXPECT directives) -----"
echo "fault rc=$RC pass rate=${FAULT_RATE:-<none>}"
tail -4 "$LOG"
rm -f "$LOG"

restore

# The characterization holds when the gate STAYS GREEN **and the pass rate it
# reports does not move**.  Both are asserted: the exit code is the primary
# discriminator, the rate is the premise the header claims ("the gate still
# reports the same pass rate") and it must be machine-checked, not merely
# printed.
if [ "$RC" -ne 0 ]; then
    echo "$FX_ID: the gate went RED after the EXPECT tamper (rc=$RC) -> the 'unfalsifiable' claim is WRONG (the gate does see it)"
    exit 3
fi
if [ -z "$CTL_RATE" ] || [ -z "$FAULT_RATE" ]; then
    echo "$FX_ID: could not read the pass rate out of the gate's output (control='${CTL_RATE:-<none>}' fault='${FAULT_RATE:-<none>}') -> the premise is unverified -> INCONCLUSIVE"
    restore
    exit 4
fi
if [ "$CTL_RATE" != "$FAULT_RATE" ]; then
    echo "$FX_ID: the pass rate MOVED (control ${CTL_RATE}% -> fault ${FAULT_RATE}%) -> the gate does react to the tampered expectation -> the 'unfalsifiable' claim is WRONG"
    exit 3
fi
echo "$FX_ID: the gate stayed GREEN (rc=0) and the pass rate did not move (${FAULT_RATE}% = control) after the expected output was tampered -> UNFALSIFIABLE (as documented): this gate cannot see a tampered expected output"
exit 0
