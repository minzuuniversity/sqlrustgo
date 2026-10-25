#!/usr/bin/env bash
# R11 self check for scripts/gate/check_anti_fabrication.sh
#
# ---------------------------------------------------------------------------
# Why this script exists
# ---------------------------------------------------------------------------
# The V312-24 artifact check in check_anti_fabrication.sh validates the
# test-runner report with a `python3 -c "..."` payload. The payload carried a
# Markdown style comment that mentions the key name in backticks:
#
#     # `results` satisfied it, so a run that executed nothing looked valid.
#
# Inside a **double quoted** shell string a backtick starts a command
# substitution, so the shell ran `results` as a command. The observable
# effect of that is a line
#
#     check_anti_fabrication.sh: line 411: results: command not found
#
# in the validator's stderr log, on **every** run of the gate. The gate
# reports a failing validator with
#
#     log_error "  V312-24: test-runner report INVALID: $(head -1 ...)"
#
# so the first line of that log becomes the stated reason. The stated reason
# is then "results: command not found", which names a missing command rather
# than the actual verdict. A defect is reported as an unrelated defect, and
# the real cause of a red gate is hidden behind it.
#
# This script is the re-runnable self check the owner asked for on
# 2026-10-25 (hub issue #5 comment 198562: "R11 误标未修复的判定缺陷请补一条
# 可复跑的自检命令"). It answers one question with a machine readable
# verdict, and it never turns "could not evaluate" into "not fixed".
#
# ---------------------------------------------------------------------------
# Exit codes
# ---------------------------------------------------------------------------
#   0  PASS             every assertion held
#   10 UNFIXED          the defect reproduces (the gate really mislabels)
#   20 INCONCLUSIVE     the check could not evaluate (payload not found, the
#                       script was restructured, a fixture could not be built)
#   30 INTERNAL_ERROR   the check itself failed
#
# The three states are deliberately distinct: a check that cannot see the
# payload must not report the payload as broken, otherwise a rename or a
# refactor of the gate would be published as a regression of R11.
#
# ---------------------------------------------------------------------------
# Usage
# ---------------------------------------------------------------------------
#   bash scripts/gate/selftest_anti_fabrication.sh              check the gate
#   bash scripts/gate/selftest_anti_fabrication.sh --negative   prove the
#                                                            check can fail
#
# The negative mode injects the defect into a throw away copy of the gate and
# asserts that this script detects it. Without it, a self check that can
# never fail is indistinguishable from a self check that always passes.
#
# The check is read only, needs no network, no cargo build and no artifacts.
# It runs in well under a second.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
GATE_SCRIPT="${R11_GATE_SCRIPT:-$REPO_ROOT/scripts/gate/check_anti_fabrication.sh}"

MODE="check"
[[ "${1:-}" == "--negative" ]] && MODE="negative"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/r11-selftest.XXXXXX")"
cleanup() {
    # Guarded on purpose: only the directory this script created is removed.
    case "$TMP_DIR" in
        */r11-selftest.*) rm -rf -- "$TMP_DIR" ;;
    esac
}
trap cleanup EXIT

verdict() {
    echo "VERDICT: $1"
    echo "REASON:  $2"
    echo "r11_selftest: $1"
}

fail_inconclusive() {
    verdict "INCONCLUSIVE" "$1"
    exit 20
}

fail_internal() {
    verdict "INTERNAL_ERROR" "$1"
    exit 30
}

# ---------------------------------------------------------------------------
# Step 1 - locate the validator payload in the gate script
# ---------------------------------------------------------------------------
# The payload is the `python3 -c "..."` block that opens the test-runner
# report. It is found by the report path it mentions, so a reworded comment
# or a moved line does not break the check.
#
# The extraction is done in python because the payload spans many lines and
# the closing quote rule is not expressible with grep alone.
extract_payload() {
    python3 - "$1" "$2" <<'PY'
import sys

path, out_path = sys.argv[1], sys.argv[2]
src = open(path, encoding='utf-8').read()

marker = "d = json.load(open('${REPO_ROOT}/target/test-runner-report.json'))"
idx = src.find(marker)
if idx < 0:
    sys.exit(3)

start = src.rfind('python3 -c "', 0, idx)
if start < 0:
    sys.exit(3)
start += len('python3 -c "')

# The payload ends at the double quote that closes the shell string, which
# is the one right before the redirection of the validator.
tail = src.find('2>/tmp/test-runner-schema.log', idx)
if tail < 0:
    tail = src.find('2>/tmp/test-runner-schema.out', idx)
if tail < 0:
    sys.exit(3)
end = src.rfind('"', start, tail)
if end <= start:
    sys.exit(3)

payload = src[start:end]
if '"' in payload:
    # A double quote inside the payload would end the shell string early;
    # this script cannot model that safely, so it refuses to guess.
    sys.exit(4)
open(out_path, 'w', encoding='utf-8').write(payload)
PY
}

if [[ ! -f "$GATE_SCRIPT" ]]; then
    fail_inconclusive "gate script not found at $GATE_SCRIPT"
fi

PAYLOAD_FILE="$TMP_DIR/payload.py"
extract_payload "$GATE_SCRIPT" "$PAYLOAD_FILE"
rc=$?
case $rc in
    0) : ;;
    3) fail_inconclusive "the test-runner validator payload was not found in $GATE_SCRIPT; the gate was restructured and this check cannot evaluate it" ;;
    4) fail_inconclusive "the validator payload contains a double quote; this check cannot model the shell quoting of that form" ;;
    *) fail_internal "extraction failed with rc=$rc" ;;
esac

if [[ ! -s "$PAYLOAD_FILE" ]]; then
    fail_inconclusive "the extracted validator payload is empty"
fi

# ---------------------------------------------------------------------------
# Step 2 - static assertion: no shell command substitution inside the payload
# ---------------------------------------------------------------------------
# Inside `python3 -c "..."` a backtick or `$(` is executed by the shell. The
# payload may expand `${REPO_ROOT}` on purpose, but it must not contain a
# command substitution, because that is the R11 defect itself.
if grep -q '`' "$PAYLOAD_FILE"; then
    verdict "UNFIXED" "the validator payload contains a backtick, so the shell runs it as a command substitution; the gate will report the noise of that command as the reason for a red verdict"
    exit 10
fi
if grep -q '\$(' "$PAYLOAD_FILE"; then
    verdict "UNFIXED" "the validator payload contains a command substitution, so the shell executes it while the validator runs"
    exit 10
fi

# ---------------------------------------------------------------------------
# Step 3 - dynamic assertions: the validator must judge, and must be silent
# ---------------------------------------------------------------------------
# The payload is run through the same shell form the gate uses, so the shell
# quoting is exercised for real.
run_validator() {
    local report_path="$1"
    local workdir="$2"
    mkdir -p "$workdir/target"
    cp "$report_path" "$workdir/target/test-runner-report.json"

    # The payload is embedded in a double quoted shell string, exactly like
    # the gate does it. ${REPO_ROOT} is substituted the same way.
    {
        printf '#!/usr/bin/env bash\n'
        printf 'REPO_ROOT=%s\n' "'$workdir'"
        printf 'python3 -c "'
        sed "s|\${REPO_ROOT}|$workdir|g" "$PAYLOAD_FILE"
        printf '" 2>%s/stderr.log >%s/stdout.log\n' "$workdir" "$workdir"
    } > "$workdir/run.sh"

    bash "$workdir/run.sh"
    echo $? > "$workdir/rc"
}

FIXTURE_VALID="$TMP_DIR/valid.json"
FIXTURE_EMPTY="$TMP_DIR/empty_results.json"
FIXTURE_MISSING_KEY="$TMP_DIR/missing_key.json"

cat > "$FIXTURE_VALID" <<'JSON'
{"started_at":"t0","finished_at":"t1","config":{},"summary":{"total_duration_ms":42},"results":[{"name":"a","status":"passed"}]}
JSON
cat > "$FIXTURE_EMPTY" <<'JSON'
{"started_at":"t0","finished_at":"t1","config":{},"summary":{"total_duration_ms":0},"results":[]}
JSON
cat > "$FIXTURE_MISSING_KEY" <<'JSON'
{"started_at":"t0","finished_at":"t1","config":{},"summary":{"total_duration_ms":0}}
JSON

run_validator "$FIXTURE_VALID" "$TMP_DIR/valid"
if [[ -s "$TMP_DIR/valid/stderr.log" ]]; then
    if grep -q "command not found" "$TMP_DIR/valid/stderr.log"; then
        verdict "UNFIXED" "running the validator produced 'command not found' on stderr: $(head -1 "$TMP_DIR/valid/stderr.log")"
        exit 10
    fi
    fail_internal "the validator wrote to stderr on a valid fixture: $(head -1 "$TMP_DIR/valid/stderr.log")"
fi
if [[ "$(cat "$TMP_DIR/valid/rc")" != "0" ]]; then
    fail_internal "the validator rejected a valid fixture with rc=$(cat "$TMP_DIR/valid/rc")"
fi
if ! grep -q "test(s) ran, none crashed" "$TMP_DIR/valid/stdout.log"; then
    fail_internal "the validator did not report the passing fixture: $(head -1 "$TMP_DIR/valid/stdout.log")"
fi

run_validator "$FIXTURE_EMPTY" "$TMP_DIR/empty"
if [[ "$(cat "$TMP_DIR/empty/rc")" == "0" ]]; then
    fail_internal "the validator accepted a report with an empty results list"
fi
if ! grep -q "results is empty" "$TMP_DIR/empty/stdout.log"; then
    fail_internal "the validator did not name the empty results list: $(head -1 "$TMP_DIR/empty/stdout.log")"
fi

run_validator "$FIXTURE_MISSING_KEY" "$TMP_DIR/missing"
if [[ "$(cat "$TMP_DIR/missing/rc")" == "0" ]]; then
    fail_internal "the validator accepted a report without the results key"
fi
if ! grep -q "missing key: results" "$TMP_DIR/missing/stdout.log"; then
    fail_internal "the validator did not name the missing key: $(head -1 "$TMP_DIR/missing/stdout.log")"
fi

# ---------------------------------------------------------------------------
# Step 4 - negative mode: the check must be able to fail
# ---------------------------------------------------------------------------
if [[ "$MODE" == "negative" ]]; then
    BROKEN_GATE="$TMP_DIR/broken_gate.sh"
    python3 - "$GATE_SCRIPT" "$BROKEN_GATE" <<'PY'
import sys

src = open(sys.argv[1], encoding='utf-8').read()
# Put the backticks back, exactly where the defect was.
needle = "# `results` satisfied it"
if needle in src:
    broken = src.replace(needle, needle, 1)
else:
    broken = src.replace(
        "# The previous check only verified that these keys EXIST. An empty",
        "# The previous check only verified that these keys EXIST. An empty\n"
        "# `results` satisfied it, so a run that executed nothing looked valid.",
        1,
    )
open(sys.argv[2], 'w', encoding='utf-8').write(broken)
PY

    if ! cmp -s "$GATE_SCRIPT" "$BROKEN_GATE"; then
        # The defect was injected. The check must now report UNFIXED.
        R11_GATE_SCRIPT="$BROKEN_GATE" "$0" > "$TMP_DIR/negative.out" 2>&1
        neg_rc=$?
        if [[ "$neg_rc" != "10" ]]; then
            verdict "INTERNAL_ERROR" "the negative mode injected the defect but the check did not report UNFIXED (rc=$neg_rc): $(head -3 "$TMP_DIR/negative.out" | tr '\n' ' ')"
            exit 30
        fi
        echo "negative mode: the injected defect was detected as expected"
        echo "VERDICT: PASS"
        echo "REASON:  the check reports UNFIXED on a gate that carries the defect and PASS on a gate that does not"
        echo "r11_selftest: PASS"
        exit 0
    fi
    fail_internal "the negative mode could not inject the defect into a copy of the gate"
fi

verdict "PASS" "the validator payload carries no shell command substitution, the validator judges a valid, an empty and an incomplete report correctly, and it stays silent on stderr"
exit 0
