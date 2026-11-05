#!/usr/bin/env bash
# fx14-stage-version-uninferable.sh — negative fixture for scripts/gate/check_stage.sh
# fixture-id: fx14-stage-version-uninferable
# expect: red
# fault: remove every input the release-line resolver can read (all
#        docs/releases/v* directories + the VERSION file), i.e. create the
#        scenario in which the release line genuinely cannot be inferred.
#
# Two arms, both must hold or the fixture reports red:
#   A fail-closed   : with nothing inferable, the driver must exit non-zero
#                     (rc=2 = INCOMPLETE). Exit 0 means it invented a version.
#   B log binding   : on an intact tree whose branch names no release line,
#                     the driver must resolve v4.1.0 and must not create any
#                     log under an archived release directory. Measured before
#                     the fix (HEAD 9f82eeb777): it resolved **v3.9.0** and
#                     wrote docs/releases/v3.9.0/logs/gate_rc_*.log.
#   C unbound target: --version naming a release line that does not exist must
#                     exit non-zero instead of creating docs/releases/<that
#                     version>/logs/ and filing a gate decision record there.
#                     (Execute mode, because that is when the log is created.)
#
# Part of SC-1 (kanban v4.1.0). NOT a gate: never runs in ci_gate.sh, never
# changes the gate denominator (168). Mutates a scratch checkout only, and the
# scratch tree is restored afterwards.

set -uo pipefail

T="${1:?usage: fx14-stage-version-uninferable.sh <scratch-tree>}"

restore() {
    git -C "$T" checkout -- . >/dev/null 2>&1 || true
    git -C "$T" clean -qfd >/dev/null 2>&1 || true
}
restore

STASH="$T/.fx14-stash"
fail=0

# --- arm A: nothing inferable -> the driver must refuse ----------------------
mv "$T/docs/releases" "$STASH-releases" 2>/dev/null || fail=1
mv "$T/VERSION" "$STASH-version" 2>/dev/null || fail=1
if [ "$fail" -ne 0 ]; then
    echo "fx14: could not set up the mutation -> INCONCLUSIVE"
    restore
    exit 4
fi

A_LOG="$(mktemp)"
cd "$T"
timeout 120 bash scripts/gate/check_stage.sh --stage ALPHA --dry-run >"$A_LOG" 2>&1
A_RC=$?
mv "$STASH-releases" "$T/docs/releases"
mv "$STASH-version" "$T/VERSION"

echo "----- fx14 arm A (uninferable version) rc=$A_RC -----"
sed -n '1,6p' "$A_LOG"
echo "----- end arm A -----"
# Measured against the pre-fix driver, "rc != 0" was NOT discriminating: the
# old code carried Version: unknown into the stage lookup and died there with
# rc=1, so a loose assertion would have called that a pass. The assertion is
# therefore about the *contract*: refuse before evaluating any stage.
A_HEAD="$(grep -m1 -E '^  Stage Gate Check: ' "$A_LOG" || true)"
A_INCOMPLETE="$(grep -c 'INCOMPLETE' "$A_LOG" || true)"
if [ -n "$A_HEAD" ]; then
    echo "fx14 arm A: driver evaluated a stage (printed '${A_HEAD}') with no inferable release line -> FIXTURE-FAIL (it guessed: rc=$A_RC)"
    fail=1
elif [ "$A_RC" -ne 2 ] || [ "$A_INCOMPLETE" -eq 0 ]; then
    echo "fx14 arm A: driver rc=$A_RC / INCOMPLETE lines=$A_INCOMPLETE (want rc=2 and a refusal line) -> FIXTURE-FAIL"
    fail=1
else
    echo "fx14 arm A: driver refused before evaluating any stage (rc=2, INCOMPLETE) -> FIXTURE-PASS"
fi
rm -f "$A_LOG"

# --- arm B: intact tree, branch names no release line ------------------------
BEFORE="$(cd "$T" && git status --porcelain | wc -l | tr -d ' ')"
B_LOG="$(mktemp)"
cd "$T"
timeout 120 bash scripts/gate/check_stage.sh --stage ALPHA --dry-run >"$B_LOG" 2>&1
B_RC=$?
echo "----- fx14 arm B rc=$B_RC (first 6 lines) -----"
sed -n '1,6p' "$B_LOG"
echo "----- end arm B -----"

VER_LINE="$(grep -m1 -E '^  Version: ' "$B_LOG" || true)"
case "$VER_LINE" in
    *"Version: v4.1.0"*)
        echo "fx14 arm B: resolved v4.1.0 -> FIXTURE-PASS (correct release line)" ;;
    "")
        echo "fx14 arm B: driver printed no Version line (rc=$B_RC) -> FIXTURE-FAIL"
        fail=1 ;;
    *)
        echo "fx14 arm B: resolved '${VER_LINE}' -> FIXTURE-FAIL (archived release line)"
        fail=1 ;;
esac

# Contamination check: nothing may appear under an archived release logs/.
STRAY="$(cd "$T" && git status --porcelain -- docs/releases | grep -v 'v4\.1\.0' || true)"
if [ -n "$STRAY" ]; then
    echo "fx14 arm B: wrote evidence outside the resolved release line:"
    printf '%s\n' "$STRAY" | head -5
    fail=1
else
    echo "fx14 arm B: no evidence written outside docs/releases/v4.1.0/ -> FIXTURE-PASS"
fi
rm -f "$B_LOG"

# --- arm C: --version pointing at a release line that does not exist ---------
C_LOG="$(mktemp)"
cd "$T"
timeout 60 bash scripts/gate/check_stage.sh --version v9.9.9 --stage ALPHA >"$C_LOG" 2>&1
C_RC=$?
echo "----- fx14 arm C (--version v9.9.9, execute mode) rc=$C_RC -----"
sed -n '1,4p' "$C_LOG"
echo "----- end arm C -----"
if [ -d "$T/docs/releases/v9.9.9" ]; then
    echo "fx14 arm C: driver created docs/releases/v9.9.9 -> FIXTURE-FAIL (evidence target unbound; rc=$C_RC)"
    fail=1
elif [ "$C_RC" -eq 0 ]; then
    echo "fx14 arm C: driver exited 0 for a release line that does not exist -> FIXTURE-FAIL"
    fail=1
else
    echo "fx14 arm C: no docs/releases/v9.9.9 created, rc=$C_RC (want 2) -> FIXTURE-PASS"
fi
rm -f "$C_LOG"

restore
if [ "$fail" -ne 0 ]; then
    echo "fx14-stage-version-uninferable: FIXTURE-FAIL"
    exit 3
fi
echo "fx14-stage-version-uninferable: FIXTURE-PASS (both arms)"
exit 0
