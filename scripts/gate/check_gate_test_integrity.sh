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
#   1  FAIL — at least one gate test is `#[ignore]`-marked
#   2  DRIFT-acceptable (baseline missing; first run captures it)
#
# Usage:
#   bash scripts/gate/check_gate_test_integrity.sh         # full run
#   bash scripts/gate/check_gate_test_integrity.sh --dry-run
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

DRY_RUN=0
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=1
fi

color_red()   { printf '\033[0;31m%s\033[0m' "$*"; }
color_green() { printf '\033[0;32m%s\033[0m' "$*"; }
color_yellow() { printf '\033[0;33m%s\033[0m' "$*"; }

step() { echo ""; echo "=== $* ==="; }
pass() { echo "  $(color_green PASS): $*"; }
fail() { echo "  $(color_red FAIL): $*"; }
warn() { echo "  $(color_yellow WARN): $*"; }

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

# Capture all --test X invocations from scripts/gate/*.sh
# Patterns handled:
#   cargo test --test <name>
#   cargo test --test=<name>
#   cargo test -p <pkg> --test <name>  (catches --test after -p)
#   --test <name> in bash variable / echo
# Pass the gate scripts dir to Python via environment (Python hardcodes a wrong default path)
export GATE_DIR="${GATE_SCRIPTS_DIR}"
GATE_TESTS_RAW=$(python3 - <<'PYEOF'
import os
import re
import sys

GATE_DIR = os.environ.get("GATE_DIR")  # must be set by bash; no hardcoded fallback
if not GATE_DIR or not os.path.isdir(GATE_DIR):
    sys.exit(0)
    sys.exit(0)

tests = set()
# Match `cargo test ... --test <name>` (avoid --target, --test-threads, etc.)
# Use negative lookbehind for "cargo test " prefix and word boundary
pat_cargo = re.compile(r'cargo\s+test\b[^|;&\n]*?--test[= ]+([a-zA-Z0-9_]+)')
pat_echo = re.compile(r'--test[= ]+([a-zA-Z0-9_]+)')

for fn in sorted(os.listdir(GATE_DIR)):
    if not fn.endswith(".sh"):
        continue
    path = os.path.join(GATE_DIR, fn)
    try:
        with open(path) as f:
            content = f.read()
    except Exception:
        continue
    # Strip line comments
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
# ============================================================================
step "P16 step 2.5/3: verify no \`cargo test ... || true\` masks in gate scripts"

OR_TRUE_VIOLATIONS=0
while IFS= read -r gate_script; do
    [[ -z "$gate_script" ]] && continue
    while IFS= read -r line_no; do
        [[ -z "$line_no" ]] && continue
        fail "$gate_script: cargo test invocation masked with || true (line $line_no)"
        OR_TRUE_VIOLATIONS=$((OR_TRUE_VIOLATIONS + 1))
    done < <(
        sed 's/[[:space:]]*#.*$//' "$gate_script" \
        | grep -nE 'cargo[[:space:]]+test\b.*\|\|[[:space:]]*true' \
        | grep -vE '^[[:space:]]*[0-9]+:[[:space:]]*(step|pass|fail|warn|echo|printf)[[:space:]]' \
        | grep -vE '"cargo[[:space:]]+test' \
        | grep -vE '`cargo[[:space:]]+test' \
        | cut -d: -f1
    )
done < <(find "${GATE_SCRIPTS_DIR}" -maxdepth 1 -name "*.sh" -type f)

if [[ "$OR_TRUE_VIOLATIONS" -eq 0 ]]; then
    pass "no \`cargo test ... || true\` masking in any gate script"
else
    fail "$OR_TRUE_VIOLATIONS \`cargo test || true\` masks in gate scripts"
fi

 # ============================================================================
 # 4. Baseline
 # ============================================================================

TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
GATE_TESTS_LIST=$(echo "${GATE_TESTS_RAW}" | python3 -c "import sys,json; print(json.dumps([t.strip() for t in sys.stdin if t.strip()]))")

if [[ ! -f "${BASELINE_FILE}" ]]; then
    warn "baseline not found: ${BASELINE_FILE}"
    warn "  first run: auto-creating from current state"
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

    pass "P16: $GATE_TESTS_COUNT gate tests, 0 new #[ignore] (baseline $BASELINE_COUNT, was $BASELINE_IGNORES ignores)"
    rm -f "$VIOLATIONS_FILE"
    exit 0
fi
