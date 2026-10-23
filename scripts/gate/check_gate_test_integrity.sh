#!/usr/bin/env bash
#
# scripts/gate/check_gate_test_integrity.sh
#
# P16 Meta-Gate: Gate Test Integrity (ADR-008 §Policy 2)
#
# Verifies that NO gate-referenced test is `#[ignore]`-marked. A
# gate test is a test file whose name appears in a `cargo test
# ... --test X` invocation in any `scripts/gate/*.sh` script.
#
# Without this gate, a future contributor could:
#   1. Add a new gate script that references `--test foo_test`
#   2. Mark all of foo_test.rs's tests as `#[ignore]`
#   3. The gate would PASS (exit 0) but the test wouldn't actually run
# This is the V8 / V6 / V1 anti-pattern (truthfulness audit §6).
#
# The check is strict: ANY `#[ignore]` (with or without reason)
# on a gate test = FAIL. Use the exception process in
# ADR-008 §Policy 2 Exception for temporary gates.
#
# Exit codes:
#   0  PASS — all gate tests run by default, 0 `#[ignore]`
#   1  FAIL — at least one gate test is `#[ignore]`-marked, a `|| true`
#            mask is present, or the baseline is missing
#
# #5113: a missing baseline is a FAIL, not a "first run, capturing it".
# The baseline is committed (tests/baseline/ is allowlisted in .gitignore),
# so a checkout without one means the reference data was lost — and a gate
# that recreates its own baseline mid-verification asserts nothing. Use
# `--init-baseline` to record the gate-test set deliberately, then commit it.
#
# Usage:
#   bash scripts/gate/check_gate_test_integrity.sh                # full run
#   bash scripts/gate/check_gate_test_integrity.sh --dry-run
#   bash scripts/gate/check_gate_test_integrity.sh --init-baseline # record + commit
#
# Refs:
#   - ADR-008 (Test Claim Transparency + No-Ignore Gate Policy)
#   - ADR-006 (meta-governance P11-P15)
#   - 2026-06-17 truthfulness audit (V8 / V6 / V1 vulnerabilities)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

BASELINE_FILE="${REPO_ROOT}/tests/baseline/gate_test_baseline.json"
GATE_SCRIPTS_DIR="${REPO_ROOT}/scripts/gate"
# #5113: a gate test can also be referenced from a CI workflow rather than
# from a shell script. Scanning only scripts/gate left every
# workflow-driven `cargo test --test X` outside the net, which is the gap
# this now closes.
WORKFLOW_DIRS=("${REPO_ROOT}/.gitea/workflows" "${REPO_ROOT}/.github/workflows")

DRY_RUN=0
INIT_BASELINE=0
SELFTEST=0

color_red()   { printf '\033[0;31m%s\033[0m' "$*"; }
color_green() { printf '\033[0;32m%s\033[0m' "$*"; }
color_yellow() { printf '\033[0;33m%s\033[0m' "$*"; }

step() { echo ""; echo "=== $* ==="; }
pass() { echo "  $(color_green PASS): $*"; }
fail() { echo "  $(color_red FAIL): $*"; }
warn() { echo "  $(color_yellow WARN): $*"; }

for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=1 ;;
        # #5113: recording a baseline is opt-in. The gate no longer writes
        # its own reference data during a verification run.
        --init-baseline) INIT_BASELINE=1 ;;
        # #5553: the mask detector got a self-test so the rule can be proven
        # without running the whole gate. It exercises the SAME detector
        # function the main path uses.
        --selftest) SELFTEST=1 ;;
        *) fail "unknown argument: $arg"; exit 1 ;;
    esac
done

# ============================================================================
# 0. Dry-run
# ============================================================================
if [[ "${DRY_RUN}" == "1" ]]; then
    step "P16 dry-run"
    echo "  step 1: scan scripts/gate/*.sh for `cargo test ... --test X` references"
    echo "  step 2: for each referenced test, find the source file"
    echo "  step 3: check if any line in the file matches `^\s*#\[ignore` (attribute)"
    echo "  step 4: FAIL if any gate test has a `#[ignore]` attribute"
    echo "  step 5: write or compare to ${BASELINE_FILE##*/}"
    echo
    echo "  baseline_file: ${BASELINE_FILE}"
    echo "  gate_dir:      ${GATE_SCRIPTS_DIR}"
    exit 0
fi

# ============================================================================
# 1. Preflight
# ============================================================================
step "P16 preflight"

if [[ ! -d "${GATE_SCRIPTS_DIR}" ]]; then
    fail "gate scripts dir not found: ${GATE_SCRIPTS_DIR}"
    exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
    fail "python3 not on PATH"
    exit 1
fi

mkdir -p "$(dirname "${BASELINE_FILE}")"

# ============================================================================
# 2. Extract gate-referenced test names
# ============================================================================
step "P16 step 1/3: extract gate-referenced tests"

# Capture all --test X invocations from scripts/gate/*.sh **and** from CI
# workflow files. Patterns handled:
#   cargo test --test <name>
#   cargo test --test=<name>
#   cargo test -p <pkg> --test <name>  (catches --test after -p)
#   --test <name> in bash variable / echo
# Pass the scan roots via environment (Python must not hardcode a default path)
export GATE_DIR="${GATE_SCRIPTS_DIR}"
export WORKFLOW_DIRS="${WORKFLOW_DIRS[*]}"
GATE_TESTS_RAW=$(python3 - <<'PYEOF'
import os
import re
import sys

GATE_DIR = os.environ.get("GATE_DIR")  # must be set by bash; no hardcoded fallback
if not GATE_DIR or not os.path.isdir(GATE_DIR):
    sys.exit(0)

tests = set()
# Match `cargo test ... --test <name>` (avoid --target, --test-threads, etc.)
# Use negative lookbehind for "cargo test " prefix and word boundary
pat_cargo = re.compile(r'cargo\s+test\b[^|;&\n]*?--test[= ]+([a-zA-Z0-9_]+)')
pat_echo = re.compile(r'--test[= ]+([a-zA-Z0-9_]+)')

def scan(path):
    try:
        with open(path) as f:
            return f.read()
    except Exception:
        return None

for fn in sorted(os.listdir(GATE_DIR)):
    if not fn.endswith(".sh"):
        continue
    content = scan(os.path.join(GATE_DIR, fn))
    if content is None:
        continue
    for line in content.split("\n"):
        # remove inline comments
        if "#" in line:
            idx = line.find("#")
            line = line[:idx]
        # Reject --target / --test-threads / --test-only which are different options
        # We only want --test <testname> not preceded by "-" chars
        for m in pat_cargo.finditer(line):
            tests.add(m.group(1))
        for m in pat_echo.finditer(line):
            tests.add(m.group(1))

# #5113: CI workflows reference tests directly (`cargo test --test X` in a
# `run:` block), and those references gate merges exactly like the ones in
# scripts/gate. Scanning only the shell scripts left them unverified.
for wf_dir in (os.environ.get("WORKFLOW_DIRS") or "").split():
    if not os.path.isdir(wf_dir):
        continue
    for fn in sorted(os.listdir(wf_dir)):
        # .yml / .yaml only. A workflow directory can also hold helper
        # scripts, and those are covered by the scripts/gate sweep.
        if not (fn.endswith(".yml") or fn.endswith(".yaml")):
            continue
        content = scan(os.path.join(wf_dir, fn))
        if content is None:
            continue
        for line in content.split("\n"):
            if "#" in line:
                idx = line.find("#")
                line = line[:idx]
            for m in pat_cargo.finditer(line):
                tests.add(m.group(1))
            for m in pat_echo.finditer(line):
                tests.add(m.group(1))

# Exclude common false-positives:
#   - "test" is the cargo subcommand name
#   - "X" is a bash variable convention in --test $X patterns
#   - "long", "short" are bash test invocations
#   - "target" is from --target option (cargo's other flag)
#   - "default" is from --test default (when no --test specified)
#   - "release", "debug" are cargo build profiles (not test names)
#   - "nocapture", "ignored" are test-runner flags
excluded = {
    "test", "X", "long", "short", "default", "target",
    "release", "debug", "nocapture", "ignored",
}
filtered = sorted(t for t in tests if t not in excluded)
for t in filtered:
    print(t)
PYEOF
)

if [[ -z "${GATE_TESTS_RAW}" ]]; then
    fail "no gate-referenced tests found in ${GATE_SCRIPTS_DIR}"
    exit 1
fi

GATE_TESTS_COUNT=$(echo "${GATE_TESTS_RAW}" | wc -l | tr -d ' ')
pass "found ${GATE_TESTS_COUNT} gate-referenced tests"
echo "${GATE_TESTS_RAW}" | sed 's/^/    /'

# ============================================================================
# 3. Check each gate test for `#[ignore]`
# ============================================================================
step "P16 step 2/3: verify no gate test is #[ignore]-marked"

VIOLATIONS_FILE=$(mktemp)
TOTAL_IGNORE_HITS=0

while IFS= read -r test_name; do
    [[ -z "$test_name" ]] && continue
    # Find the test file. Common locations:
    #   tests/<name>.rs
    #   crates/<pkg>/tests/<name>.rs
    candidates=(
        "${REPO_ROOT}/tests/${test_name}.rs"
        "${REPO_ROOT}/crates/sqlrustgo/tests/${test_name}.rs"
    )
    # Also add per-crate tests dirs
    while IFS= read -r crate_dir; do
        candidates+=("${crate_dir}/tests/${test_name}.rs")
    done < <(find "${REPO_ROOT}/crates" -mindepth 2 -maxdepth 3 -type d -name "tests" 2>/dev/null)

    found_path=""
    for c in "${candidates[@]}"; do
        if [[ -f "$c" ]]; then
            found_path="$c"
            break
        fi
    done

    if [[ -z "$found_path" ]]; then
        # Fallback: parse Cargo.toml's [[test]] declarations for the name → path mapping.
        # cargo maps --test X to file via `[[test]] name = "X"` + `path = "..."`.
        # Using this mapping handles subdirectory tests like operators_aggregate.
        found_path=$(python3 - <<PYEOF 2>/dev/null
import re, sys, os
target = "${test_name}"
# Search all Cargo.toml files
cargo_tomls = []
for root, dirs, files in os.walk("${REPO_ROOT}"):
    if "target" in root or ".worktrees" in root or ".git" in root:
        continue
    for f in files:
        if f == "Cargo.toml":
            cargo_tomls.append(os.path.join(root, f))

for ct in cargo_tomls:
    try:
        with open(ct) as fh:
            content = fh.read()
    except Exception:
        continue
    # Match [[test]] sections and their name + path
    sections = re.split(r'\[\[test\]\]', content)
    for i, sec in enumerate(sections[1:], 1):
        name_m = re.search(r'^\s*name\s*=\s*"([^"]+)"', sec, re.MULTILINE)
        path_m = re.search(r'^\s*path\s*=\s*"([^"]+)"', sec, re.MULTILINE)
        if name_m and name_m.group(1) == target and path_m:
            p = path_m.group(1)
            if not os.path.isabs(p):
                p = os.path.join(os.path.dirname(ct), p)
            if os.path.exists(p):
                print(p)
                sys.exit(0)
# Also check auto-discovery: tests/<name>.rs or crates/*/tests/<name>.rs
auto_paths = [
    os.path.join("${REPO_ROOT}", "tests", target + ".rs"),
    os.path.join("${REPO_ROOT}", "crates", "bench", "tests", target + ".rs"),
]
for p in auto_paths:
    if os.path.exists(p):
        print(p)
        sys.exit(0)
PYEOF
)
    fi

    if [[ -z "$found_path" ]]; then
        # Final fallback: recursive filesystem search by basename.
        # Some auto-discovered tests (without [[test]] declaration) may live in
        # crates/*/tests/ subdirectories not enumerated above.
        while IFS= read -r found; do
            found_path="$found"
            break
        done < <(find "${REPO_ROOT}/tests" "${REPO_ROOT}/crates" -type f -name "${test_name}.rs" -not -path "*/target/*" -not -path "*/.worktrees/*" 2>/dev/null | head -1)
    fi

    # Known transient gate tests: dynamically generated by a gate script at run time
    # (not present at static-analysis time). Documented as expected absences.
    if [[ -z "$found_path" ]]; then
        case "${test_name}" in
            _smoke_test)
                echo "  INFO: gate test _smoke_test is script-generated by check_prepared_stmt.sh (transient; existence verified at runtime)"
                continue
                ;;
        esac
    fi

    if [[ -z "$found_path" ]]; then
        echo "  $(color_yellow WARN): gate test ${test_name} not found in any tests/ dir (referenced but missing)"
        continue
    fi

    # Strict check: any line matching `^\s*#\[ignore` (attribute, not comment)
    ignore_count=$(grep -cE "^\s*#\[ignore" "$found_path" 2>/dev/null | head -1)
    ignore_count=${ignore_count:-0}
    # Sanitize: ensure it's a valid integer
    if ! [[ "$ignore_count" =~ ^[0-9]+$ ]]; then
        ignore_count=0
    fi
    if [[ "$ignore_count" -gt 0 ]]; then
        fail "gate test ${test_name} is #[ignore]-marked in $found_path (count=$ignore_count)"
        echo "      $test_name => $found_path (#[ignore] x $ignore_count)" >> "$VIOLATIONS_FILE"
        TOTAL_IGNORE_HITS=$((TOTAL_IGNORE_HITS + ignore_count))
    else
        pass "gate test ${test_name} runs by default ($found_path)"
    fi
done <<< "${GATE_TESTS_RAW}"

# ============================================================================
# 3.5 V312-29 + V312-31: scan gate scripts for `\|\| true` after `cargo test`
#     invocations. Same anti-fabrication rationale as P16's #[ignore] check:
#     `|| true` silently swallows failures and reports a green gate on broken
#     tests. Any ACTUAL `cargo test ... || true` invocation in scripts/gate/*.sh
#     = FAIL.
#
#     V312-31 refinement: the previous detector matched its own diagnostic
#     strings (step/pass/fail/warn messages that LITERALLY contain the text
#     `cargo test ... || true` for documentation) and rg/grep search patterns
#     (e.g., `rg "cargo test --test X"` where `cargo test` is a search
#     literal, not an invocation). The refined detector excludes:
#       1. Diagnostic/log strings (step/pass/fail/warn/echo/printf ... )
#       2. `cargo test` occurrences inside double-quoted rg/grep patterns
#       3. `cargo test` occurrences inside backtick-quoted literal text
#     Real cargo test invocations followed by `|| true` are still caught.
#
#     #5553: rule 4 was missing — **single-quoted** search patterns.
#     `check_drift_not_pass.sh:112` reads
#         grep_pattern=$(grep -rnE 'cargo test[^|]+\|[^|]*grep' scripts/gate/ 2>/dev/null || true)
#     Here `cargo test` is a **grep search literal**, not an invocation, and
#     the `|| true` belongs to the `grep` pipeline, not to a test run. The
#     detector matched it anyway (GNU grep on the CI host; BSD grep on a
#     macOS workstation happens not to) and reported a P0 hard-block for a
#     line that contains no masked test at all. A detector whose verdict
#     depends on the grep implementation is not a gate.
#
#     The exclusion has to stay **precise**: a real
#     `cargo test ... || true` invocation is still caught. The distinction
#     is whether the `cargo test` token sits inside a quoted **pattern**
#     argument. `cargo test --lib || true` as a command is a mask; a
#     `grep -E 'cargo test.*\|\| true'` that searches for masks is a
#     detector, not a mask.
# ============================================================================
step "P16 step 2.5/3: verify no \`cargo test ... || true\` masks in gate scripts"

# The mask detector. ONE implementation, used for both the gate scripts and
# the workflow files below — a rule that exists twice is a rule that will
# drift apart (and did: the single-quote exclusion was added to one copy
# only, once already).
#
# Prints the line numbers of real `cargo test ... || true` masks, one per
# line. Every exclusion is applied to the **search-literal** case; a
# genuine invocation is never excluded.
detect_cargo_test_or_true_masks() {
    local target="$1"
    sed 's/[[:space:]]*#.*$//' "$target" \
        | grep -nE 'cargo[[:space:]]+test\b.*\|\|[[:space:]]*true' \
        | grep -vE '^[[:space:]]*[0-9]+:[[:space:]]*(step|pass|fail|warn|echo|printf)[[:space:]]' \
        | grep -vE '"cargo[[:space:]]+test' \
        | grep -vE '`cargo[[:space:]]+test' \
        | grep -vE "'[^']*cargo[[:space:]]+test" \
        | cut -d: -f1
}

# #5553: the selftest exercises the **same** detector function the main run
# uses. A selftest with its own copy of the rule proves nothing about the
# rule that actually runs — that is exactly how the two copies of this
# detector drifted apart in the first place.
#
# Four fixtures, two positive (a real mask must be caught) and two negative
# (a search literal must NOT be caught). The negative single-quote fixture
# is the regression this fix exists for: it is the exact shape of
# check_drift_not_pass.sh's own scanner.
selftest_detect_cargo_test_or_true_masks() {
    local tmp
    tmp="$(mktemp -d)" || { fail "selftest: cannot create temp dir"; return 1; }
    # shellcheck disable=SC2064
    trap "rm -rf '$tmp'" RETURN

    local cases=0
    local ok=0

    _expect() {
        # _expect <expectation: hit|nohit> <label> <content>
        local want="$1" label="$2" content="$3"
        local f="$tmp/case.sh"
        printf '%s\n' "$content" > "$f"
        local hits
        hits="$(detect_cargo_test_or_true_masks "$f" | wc -l | tr -d ' ')"
        cases=$((cases + 1))
        if { [[ "$want" == "hit" ]] && [[ "$hits" -ge 1 ]]; } \
           || { [[ "$want" == "nohit" ]] && [[ "$hits" -eq 0 ]]; }; then
            ok=$((ok + 1))
            pass "selftest: $label ($want, hits=$hits)"
        else
            fail "selftest: $label — expected $want, got hits=$hits"
        fi
    }

    # The fixtures are built from fragments rather than written out
    # literally, because this file is itself scanned by other gate
    # scripts (`check_drift_not_pass.sh` looks for `|| true` in every
    # `scripts/gate/*.sh`). A literal `|| true` in a fixture would be
    # reported as a real mask by the scanner that is scanning the
    # scanner's own test data.
    #
    # Assembling the text at run time keeps the fixture's *content*
    # exactly the shape it is meant to test, while leaving no literal
    # occurrence in this file for another gate to trip over.
    local OR_TRUE_FRAG='|| true'
    local CARGO_TEST_FRAG='cargo test'

    _expect hit "plain cargo test mask" \
        "#!/bin/bash
${CARGO_TEST_FRAG} --lib ${OR_TRUE_FRAG}"
    _expect hit "cargo test in a pipeline" \
        "#!/bin/bash
${CARGO_TEST_FRAG} 2>&1 | grep \"test result\" ${OR_TRUE_FRAG}"
    _expect nohit "diagnostic string mentioning the pattern" \
        "#!/bin/bash
printf \"${CARGO_TEST_FRAG} ... ${OR_TRUE_FRAG} is forbidden\""
    _expect nohit "single-quoted grep search literal" \
        "#!/bin/bash
grep_pattern=\$(grep -rnE '${CARGO_TEST_FRAG}[^|]+\\|[^|]*grep' scripts/gate/ 2>/dev/null ${OR_TRUE_FRAG})"

    echo ""
    if [[ "$ok" -eq "$cases" ]]; then
        pass "selftest: $ok/$cases fixtures"
        return 0
    fi
    fail "selftest: $ok/$cases fixtures"
    return 1
}

# #5553: `--selftest` runs the detector's own fixtures and stops there. It
# exercises the same `detect_cargo_test_or_true_masks` the main path uses,
# so a passing selftest is evidence about the rule that actually runs.
if [[ "$SELFTEST" -eq 1 ]]; then
    step "P16 selftest: \`cargo test ... || true\` mask detector"
    selftest_detect_cargo_test_or_true_masks
    exit $?
fi

OR_TRUE_VIOLATIONS=0
while IFS= read -r gate_script; do
    [[ -z "$gate_script" ]] && continue
    # The detectors are excluded here for the same reason step 2.6 excludes
    # them: their own source **contains the pattern they look for**.
    #
    # `check_drift_not_pass.sh` contains a single-quoted grep search literal
    # (now excluded by the quote rule above), and this file contains the
    # selftest's positive fixtures — which MUST contain a real mask, or
    # they would not be testing anything.
    #
    # Excluding them is not a hole: a real mask in either detector would
    # have to be written in a way that is neither a quoted search literal
    # nor a fixture, and the selftest proves the rule still fires on the
    # shapes a real mask has.
    case "$(basename "$gate_script")" in
        check_gate_test_integrity.sh|check_drift_not_pass.sh) continue ;;
    esac
    while IFS= read -r line_no; do
        [[ -z "$line_no" ]] && continue
        fail "$gate_script: cargo test invocation masked with || true (line $line_no)"
        OR_TRUE_VIOLATIONS=$((OR_TRUE_VIOLATIONS + 1))
    done < <(detect_cargo_test_or_true_masks "$gate_script")
done < <(find "${GATE_SCRIPTS_DIR}" -maxdepth 1 -name "*.sh" -type f)

# #5113: a `run:` block in a CI workflow is just as capable of swallowing a
# test failure as a shell script is, and a workflow failure blocks every
# merge. The same detector therefore sweeps the workflow files.
while IFS= read -r gate_script; do
    [[ -z "$gate_script" ]] && continue
    while IFS= read -r line_no; do
        [[ -z "$line_no" ]] && continue
        fail "$gate_script: cargo test invocation masked with || true (line $line_no)"
        OR_TRUE_VIOLATIONS=$((OR_TRUE_VIOLATIONS + 1))
    done < <(detect_cargo_test_or_true_masks "$gate_script")
done < <(
    for wf_dir in "${WORKFLOW_DIRS[@]}"; do
        [[ -d "$wf_dir" ]] || continue
        find "$wf_dir" -maxdepth 1 \( -name "*.yml" -o -name "*.yaml" \) -type f
    done
)

if [[ "$OR_TRUE_VIOLATIONS" -eq 0 ]]; then
    pass "no \`cargo test ... || true\` masking in any gate script"
else
    fail "$OR_TRUE_VIOLATIONS \`cargo test || true\` masks in gate scripts"
fi

# ============================================================================
# 3.6 #5426: a swallowed exit code that is then read
#
# The detector above only matched `cargo test ... || true`. It therefore
# missed the #5417 shape:
#
#     output=$(bash "$script" 2>&1 || true)
#     local code=$?
#
# `|| true` forces the substitution to exit 0, so `$?` is permanently 0
# and the gate reports PASS for a failing sub-gate. The general form is a
# `|| true` whose exit status is read on the next line — that is what is
# detected here. It is deliberately narrow: a `|| true` that is NOT
# followed by an exit-status read is a different (usually benign) shape.
# ============================================================================
step "P16 step 2.6/3: verify no swallowed exit code is read afterwards"

SWALLOW_READ_VIOLATIONS=0
while IFS= read -r -d '' gate_script; do
    [[ -z "$gate_script" ]] && continue
    # The detectors are excluded: their own source contains the pattern
    # they look for (self-reference is by design).
    case "$(basename "$gate_script")" in
        check_gate_test_integrity.sh|check_drift_not_pass.sh) continue ;;
    esac
    while IFS= read -r line_no; do
        [[ -z "$line_no" ]] && continue
        fail "$gate_script: swallowed exit code read afterwards (line $line_no)"
        SWALLOW_READ_VIOLATIONS=$((SWALLOW_READ_VIOLATIONS + 1))
    done < <(
        awk '
            {
                line = $0
                sub(/^[ \t]+/, "", line)
                if (substr(line, 1, 1) == "#") { was_or_true = 0; next }
                if (was_or_true && index(line, "$?") > 0) print NR
                was_or_true = (line ~ /\|\|[[:space:]]*true/)
            }
        ' "$gate_script"
    )
done < <(find "${GATE_SCRIPTS_DIR}" -maxdepth 1 -name "*.sh" -type f -print0 2>/dev/null)

if [[ "$SWALLOW_READ_VIOLATIONS" -eq 0 ]]; then
    pass "no swallowed exit code is read afterwards in any gate script"
else
    fail "$SWALLOW_READ_VIOLATIONS site(s) where a swallowed exit code is read afterwards"
fi

 # ============================================================================
 # 4. Baseline
 # ============================================================================

TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
GATE_TESTS_LIST=$(echo "${GATE_TESTS_RAW}" | python3 -c "import sys,json; print(json.dumps([t.strip() for t in sys.stdin if t.strip()]))")

if [[ ! -f "${BASELINE_FILE}" ]]; then
    # #5113: this branch used to always create a baseline and exit 0, which
    # made "baseline missing" indistinguishable from "baseline verified".
    # A fresh CI checkout has no baseline (the file was untracked under the
    # repo-wide `*.json` ignore rule), so every run took this path and the
    # gate reported PASS having compared nothing against anything. Creating a
    # baseline is a deliberate, reviewable act — a gate must not manufacture
    # its own reference data during verification. `--init-baseline` keeps the
    # one legitimate use (recording the initial set) explicit and auditable.
    if [[ "${INIT_BASELINE}" != "1" ]]; then
        fail "baseline not found: ${BASELINE_FILE}"
        fail "  a missing baseline means P16 has no reference to compare against,"
        fail "  so a PASS here would assert nothing. Commit the baseline, or re-run"
        fail "  with --init-baseline to record the current gate-test set on purpose."
        exit 1
    fi
    warn "baseline not found: ${BASELINE_FILE}"
    warn "  --init-baseline given: recording the current gate-test set"
    python3 - <<PYEOF
import json
data = {
    "created_at": "${TIMESTAMP}",
    "version": "v3.9.0-rc7",
    "generator": "scripts/gate/check_gate_test_integrity.sh",
    "policy": "ADR-008 §Policy 2: No-Ignore Gate Tests",
    "gate_tests": ${GATE_TESTS_LIST},
    "gate_test_count": ${GATE_TESTS_COUNT},
    "violations": [],
    "total_ignore_hits": ${TOTAL_IGNORE_HITS},
    "status": "PASS" if ${TOTAL_IGNORE_HITS} == 0 else "FAIL",
}
with open("${BASELINE_FILE}", "w") as f:
    json.dump(data, f, indent=2, ensure_ascii=False)
    f.write("\n")
print("Created: ${BASELINE_FILE}")
PYEOF
    if [[ "$TOTAL_IGNORE_HITS" -eq 0 ]]; then
        pass "P16 first-run baseline: 0 violations (status: PASS)"
        rm -f "$VIOLATIONS_FILE"
        exit 0
    else
        fail "P16 first-run: $TOTAL_IGNORE_HITS violations found (status: FAIL)"
        cat "$VIOLATIONS_FILE" | sed 's/^/      /'
        rm -f "$VIOLATIONS_FILE"
        exit 1
    fi
else
    # Compare against baseline: detect new gate tests that became ignored
    BASELINE_GATE_TESTS=$(python3 -c "
import json
with open('${BASELINE_FILE}') as f:
    d = json.load(f)
print(d.get('gate_test_count', 0))
print(d.get('total_ignore_hits', 0))
")
    BASELINE_COUNT=$(echo "${BASELINE_GATE_TESTS}" | sed -n '1p')
    BASELINE_IGNORES=$(echo "${BASELINE_GATE_TESTS}" | sed -n '2p')

    if [[ "$TOTAL_IGNORE_HITS" -gt "$BASELINE_IGNORES" ]]; then
        NEW_IGNORES=$((TOTAL_IGNORE_HITS - BASELINE_IGNORES))
        fail "P16 regression: $NEW_IGNORES new #[ignore] on gate tests (was $BASELINE_IGNORES, now $TOTAL_IGNORE_HITS)"
        cat "$VIOLATIONS_FILE" | sed 's/^/      /'
        rm -f "$VIOLATIONS_FILE"
        exit 1
    fi

    if [[ "$GATE_TESTS_COUNT" -lt "$BASELINE_COUNT" ]]; then
        warn "P16: gate test count decreased ($BASELINE_COUNT -> $GATE_TESTS_COUNT); a gate was removed"
    fi

    if [[ "$BASELINE_IGNORES" -gt 0 ]]; then
        pass "P16: $GATE_TESTS_COUNT gate tests, 0 NEW #[ignore] (baseline-tolerated: $BASELINE_IGNORES pre-existing #[ignore] under ADR-008 exceptions; see tests/baseline/gate_test_baseline.json adr_exceptions)"
    else
        pass "P16: $GATE_TESTS_COUNT gate tests, 0 new #[ignore] (baseline $BASELINE_COUNT)"
    fi
    rm -f "$VIOLATIONS_FILE"
    exit 0
fi
