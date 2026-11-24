#!/usr/bin/env bash
# fx19-sql-corpus-gate-expected-tampered.sh — negative fixture for
# scripts/gate/check_sql_corpus_gate.sh
# fixture-id: fx19-sql-corpus-gate-expected-tampered
# expect: red
# fault: tamper the corpus' expected output — every `-- EXPECT: <n>` line in
#        sql_corpus/**.sql is rewritten to an unreachable row count, so the
#        corpus no longer agrees with its own expectations.
#
# WHY THIS IS CHEAP
# -----------------
# The gate runs `cargo test --release -p sqlrustgo-sql-corpus --test corpus_test
# test_sql_corpus_all` and reads "N% pass rate" out of the output.  The
# corpus is *data*: changing a `.sql` file does not change any Rust source,
# so cargo has nothing to rebuild and the release binary is reused.  The
# measurement therefore costs one corpus run, not one build.
#
# The corpus format (crates/sql-corpus/src/lib.rs) is:
#   `-- === CASE: <name>`   starts a case
#   `-- EXPECT: <n>`        the case succeeds iff the statement returned n
#                           rows (`success = rows_returned == expected_rows`)
# 1476 of the 1723 cases carry an EXPECT directive, so tampering all of them
# drives the pass rate far below the gate's 80% threshold — which is the
# failure the ledger's target asks for ("篡改 expected 输出 ⇒ 必须变红").
#
# Arms:
#   A control  intact tree ⇒ the gate exits 0 (so the red in arm B is
#              attributable to the mutation)
#   B fault   the EXPECT directives are tampered ⇒ the gate must go red
#
# Part of the v4.1.0 readiness plan (Task 8).  NOT a gate: it never runs in
# ci_gate.sh, never changes the gate denominator (168).  It mutates a
# **scratch** checkout, runs the real gate there, and the scratch tree is
# restored afterwards.

set -uo pipefail

T="${1:?usage: fx19-sql-corpus-gate-expected-tampered.sh <scratch-tree>}"
FX_ID="fx19-sql-corpus-gate-expected-tampered"
GATE="scripts/gate/check_sql_corpus_gate.sh"
# An unreachable row count: no corpus statement returns a million rows.
BAD_EXPECT=999999

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

# --- inject the fault --------------------------------------------------------
python3 - "$T/sql_corpus" "$BAD_EXPECT" <<'PY'
import os, re, sys
root, bad = sys.argv[1], sys.argv[2]
pat = re.compile(r"^(--\s*EXPECT:\s*)(\S+)(.*)$")
n_files = n_lines = 0
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
                out.append("%s%s%s" % (m.group(1), bad, m.group(3)))
                hit = True
                n_lines += 1
            else:
                out.append(l)
        if hit:
            open(p, "w", encoding="utf-8").write("\n".join(out))
            n_files += 1
print("fx19: tampered %d EXPECT directives across %d corpus files" % (n_lines, n_files))
if n_lines == 0:
    sys.exit("fx19: no EXPECT directive found - the corpus format changed, aborting")
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
echo "----- end -----"

if [ "$TWO_LAYER_RC" -ne 0 ]; then
    echo "$FX_ID: could not run the two-layer check -> INCONCLUSIVE"
    restore
    rm -rf "$FX_RUN_DIR"
    exit 4
fi

GATE_TAIL="$(cat "$FX_RUN_DIR"/001-*.stdout 2>/dev/null || true)"
case "$GATE_TAIL" in
    *"pass rate"*) RATE_SEEN=1 ;;
    *) RATE_SEEN=0 ;;
esac

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
if [ "$RATE_SEEN" -ne 1 ]; then
    echo "$FX_ID: the gate output carries no pass rate at all -> FIXTURE-FAIL (the verdict must be about the rate)"
    exit 3
fi

echo "$FX_ID: gate rc=$FX_GATE_RC (FAIL, not a timeout) + stage verdict=$FX_STAGE_VERDICT -> FIXTURE-PASS (the gate detects the tampered expected output)"
exit 0
