#!/usr/bin/env bash
#
# scripts/gate/check_no_process_env_mutation.sh
#
# Meta-gate: the **root package's** library sources must not mutate
# process-global environment variables.
#
# #5423 — the root lib test binary ran ~160 unit tests concurrently inside
# ONE process. Several of them used `std::env::set_var` /
# `std::env::remove_var` on `SQLRUSTGO_EXECUTOR_PARALLELISM` as a fixture.
# `env::set_var` rewrites the **whole process** environment table, so the
# window in which the variable was set to 4 was visible to every other test
# running at the same time — including
# `test_v410_atomic_parallel_degree_getter_clamps_to_one`, which asserts the
# default is 1 and therefore intermittently observed 4:
#
#   ---- execution_engine_tests::test_v410_atomic_parallel_degree_getter_clamps_to_one stdout ----
#   thread '...' panicked at src/execution_engine_tests.rs:1361:5:
#   assertion `left == right` failed
#     left: 4
#    right: 1
#
# The CI `test` job runs `cargo test --all-features --no-fail-fast`, which
# includes this binary ⇒ the job had a roughly **1-in-3 random red**. A
# flaky red is worse than a deterministic one: it teaches reviewers to
# re-run instead of to read.
#
# THE INVARIANT
# -------------
#
#   C1  `<repo>/src/**/*.rs` must not call `env::set_var` / `env::remove_var`
#       (directly, via a `use std::env::set_var` import, or through any
#       alias).
#
# WHY ONLY THE ROOT PACKAGE
# -------------------------
#
# Every Cargo test target is its own process, so env mutation only races
# with the *other tests of the same binary*. The root package's `src/` is the
# one binary where this actually bit us (#5423):
#
#   * `crates/storage/tests/a06_failpoint_reachability.rs` — already
#     serializes its env mutations under a local `env_lock()` (the fault
#     injection hook is read from the env at the fault point, so injection
#     is not an option there);
#   * `crates/mysql-server/src/lib.rs` — serializes under a shared
#     `ENV_LOCK`.
#
# ⇒ The rule is deliberately narrow. It does NOT claim that env mutation is
# forbidden everywhere; it forbids it in the one tree where it has no
# locking discipline and a measured 1-in-3 flake rate.
#
# THE ESCAPE ROUTE
# ----------------
#
# If a root-package test genuinely needs to observe the env-driven code
# path, it must not mutate the process environment in `src/`:
#
#   1. prefer an explicit seam — `ExecutionEngine::with_parallel_degree` /
#      `ExecutionEngine::base_with_raw` / `resolve_parallel_degree` exist
#      exactly for this (#5423);
#   2. otherwise move the test into its own integration-test binary under
#      `tests/` — a separate process, hence no sibling race.
#
# Exit codes:
#   0  [PASS] — the root package does not mutate process-global env
#   1  [FAIL] — at least one call site violates C1
#   2  usage error / a required input is missing
#
# Usage:
#   bash scripts/gate/check_no_process_env_mutation.sh
#   bash scripts/gate/check_no_process_env_mutation.sh --json <out.json>
#   bash scripts/gate/check_no_process_env_mutation.sh --selftest
#   bash scripts/gate/check_no_process_env_mutation.sh --repo-root <dir>

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
REPO_ROOT_OVERRIDE=""
JSON_OUT=""
SELFTEST=0

color_red()   { printf '\033[0;31m%s\033[0m' "$*"; }
color_green() { printf '\033[0;32m%s\033[0m' "$*"; }

step() { echo ""; echo "=== $* ==="; }
pass() { echo "  $(color_green PASS): $*"; }
fail() { echo "  $(color_red FAIL): $*"; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --json)      JSON_OUT="$2"; shift 2 ;;
        --json=*)    JSON_OUT="${1#--json=}"; shift ;;
        --selftest)  SELFTEST=1; shift ;;
        --repo-root) REPO_ROOT_OVERRIDE="$2"; shift 2 ;;
        --repo-root=*) REPO_ROOT_OVERRIDE="${1#--repo-root=}"; shift ;;
        -h|--help)
            echo "Usage: bash scripts/gate/check_no_process_env_mutation.sh [--json <out>] [--selftest] [--repo-root <dir>]"
            exit 0
            ;;
        *) echo "ERROR: unknown argument: $1" >&2; exit 2 ;;
    esac
done

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required" >&2
    exit 2
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/no-env-mutation.XXXXXX")"
trap 'rm -rf -- "${TMP_DIR}"' EXIT

# ---------------------------------------------------------------------------
# The analyzer
# ---------------------------------------------------------------------------
cat > "${TMP_DIR}/analyze.py" <<'PYEOF'
import json
import os
import re
import sys

# `std::env::set_var(...)`, `env::set_var(...)`, and the fully-qualified
# `::std::env::set_var(...)` spelling.
CALL_RE = re.compile(r"(?:^|[^A-Za-z0-9_])((?:::)?std::env|(?:::)?env)::\s*(set_var|remove_var)\s*\(")

# `use std::env::set_var;` / `use std::env::{set_var, remove_var};`
IMPORT_RE = re.compile(r"^\s*use\s+[^;]*\benv::\s*\{?[^;]*\b(set_var|remove_var)\b")

RULE = "C1"


def analyze_file(path, rel):
    """Return the C1 violations in one Rust source file."""
    out = []
    try:
        with open(path, errors="replace") as fh:
            lines = fh.readlines()
    except OSError:
        return out
    for lineno, line in enumerate(lines, start=1):
        stripped = line.strip()
        hit = CALL_RE.search(line)
        kind = None
        if hit:
            kind = hit.group(2)
        else:
            imp = IMPORT_RE.match(line)
            if imp:
                kind = imp.group(1)
        if kind:
            out.append({
                "file": rel,
                "line": lineno,
                "rule": RULE,
                "kind": kind,
                "text": stripped[:160],
            })
    return out


def scan(repo_root):
    """Scan `<repo_root>/src` — the root package's library sources only."""
    src_dir = os.path.join(repo_root, "src")
    violations = []
    scanned = []
    if not os.path.isdir(src_dir):
        return scanned, violations
    for dirpath, dirnames, filenames in os.walk(src_dir):
        dirnames[:] = sorted(d for d in dirnames if d not in ("target", ".git"))
        for fn in sorted(filenames):
            if not fn.endswith(".rs"):
                continue
            p = os.path.join(dirpath, fn)
            rel = os.path.relpath(p, repo_root)
            scanned.append(rel)
            violations.extend(analyze_file(p, rel))
    return scanned, violations


def main(argv):
    repo_root = argv[0]
    json_out = argv[1] if len(argv) > 1 else None
    scanned, violations = scan(repo_root)
    report = {
        "scanned_root": "src",
        "scanned_files": scanned,
        "scanned_count": len(scanned),
        "violations": violations,
        "failure_count": len(violations),
        "status": "PASS" if not violations else "FAIL",
    }
    if json_out:
        with open(json_out, "w") as fh:
            json.dump(report, fh, indent=2, ensure_ascii=False)
            fh.write("\n")
    else:
        json.dump(report, sys.stdout, indent=2, ensure_ascii=False)
        sys.stdout.write("\n")
    return 0 if not violations else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
PYEOF

# ---------------------------------------------------------------------------
# --selftest: the gate must be able to fail
# ---------------------------------------------------------------------------
if [[ "${SELFTEST}" -eq 1 ]]; then
    step "Self-test: synthetic fixtures"

    SELFTEST_FAILURES=0

    expect_verdict() {
        local dir="$1" expected="$2" label="$3"
        local out="${TMP_DIR}/verdict.json"
        if python3 "${TMP_DIR}/analyze.py" "${dir}" "${out}" >/dev/null 2>&1; then
            local got="PASS"
        else
            local got="FAIL"
        fi
        if [[ "${got}" == "${expected}" ]]; then
            pass "${label}: expected ${expected}, got ${got}"
            return 0
        fi
        fail "${label}: expected ${expected}, got ${got}"
        SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))
        return 1
    }

    # 1. clean tree → PASS
    mkdir -p "${TMP_DIR}/clean/src"
    cat > "${TMP_DIR}/clean/src/lib.rs" <<'EOF'
pub fn helper() -> usize {
    std::env::var("SQLRUSTGO_HOME").unwrap_or_default().len()
}
EOF
    expect_verdict "${TMP_DIR}/clean" "PASS" "clean root src"

    # 2. `std::env::set_var` in the root src → FAIL
    mkdir -p "${TMP_DIR}/bad_set/src"
    cat > "${TMP_DIR}/bad_set/src/lib.rs" <<'EOF'
pub fn arm() {
    std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", "4");
}
EOF
    expect_verdict "${TMP_DIR}/bad_set" "FAIL" "std::env::set_var in root src"

    # 3. `std::env::remove_var` in the root src → FAIL
    mkdir -p "${TMP_DIR}/bad_remove/src"
    cat > "${TMP_DIR}/bad_remove/src/lib.rs" <<'EOF'
pub fn disarm() {
    std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
}
EOF
    expect_verdict "${TMP_DIR}/bad_remove" "FAIL" "std::env::remove_var in root src"

    # 4. `use std::env::set_var;` import → FAIL
    mkdir -p "${TMP_DIR}/bad_import/src"
    cat > "${TMP_DIR}/bad_import/src/lib.rs" <<'EOF'
use std::env::set_var;

pub fn arm() {
    set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", "4");
}
EOF
    expect_verdict "${TMP_DIR}/bad_import" "FAIL" "use std::env::set_var import in root src"

    # 5. env mutation under `crates/` → PASS (the rule is scoped to the root
    #    package on purpose — `crates/*` are separate test binaries and carry
    #    their own locking discipline)
    mkdir -p "${TMP_DIR}/crates_ok/src" "${TMP_DIR}/crates_ok/crates/storage/tests"
    cat > "${TMP_DIR}/crates_ok/src/lib.rs" <<'EOF'
pub fn helper() -> usize { 1 }
EOF
    cat > "${TMP_DIR}/crates_ok/crates/storage/tests/one.rs" <<'EOF'
pub fn arm() {
    std::env::set_var("SQLRUSTGO_FAULT_POINT", "AfterWALAppend");
}
EOF
    expect_verdict "${TMP_DIR}/crates_ok" "PASS" "env mutation under crates/ is out of scope"

    # 6. the analyzer must actually see a violation, not just exit non-zero
    python3 "${TMP_DIR}/analyze.py" "${TMP_DIR}/bad_set" "${TMP_DIR}/detected.json" >/dev/null 2>&1
    DETECTED=$(python3 -c "import json;print(json.load(open('${TMP_DIR}/detected.json'))['failure_count'])" 2>/dev/null || echo 0)
    if [[ "${DETECTED}" == "1" ]]; then
        pass "analyzer reports the exact violation count (1)"
    else
        fail "analyzer reported ${DETECTED} violations, expected 1"
        SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))
    fi

    # 7. `env::set_var` without the `std::` prefix → FAIL
    mkdir -p "${TMP_DIR}/bad_bare/src"
    cat > "${TMP_DIR}/bad_bare/src/lib.rs" <<'EOF'
pub fn arm() {
    env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", "4");
}
EOF
    expect_verdict "${TMP_DIR}/bad_bare" "FAIL" "env::set_var without std:: prefix in root src"

    echo ""
    if [[ "${SELFTEST_FAILURES}" -eq 0 ]]; then
        pass "self-test: all checks passed"
    else
        fail "self-test: ${SELFTEST_FAILURES} check(s) failed"
        exit 1
    fi
    exit 0
fi

# ---------------------------------------------------------------------------
# Real run
# ---------------------------------------------------------------------------
SCAN_ROOT="${REPO_ROOT_OVERRIDE:-$REPO_ROOT}"
if [[ ! -d "${SCAN_ROOT}/src" ]]; then
    echo "ERROR: no src/ directory under ${SCAN_ROOT}" >&2
    exit 2
fi

step "Scanning the root package's sources for process-global env mutation"
echo "  repo root: ${SCAN_ROOT}"
echo "  scope:     ${SCAN_ROOT}/src/**/*.rs"

REPORT="${TMP_DIR}/report.json"
python3 "${TMP_DIR}/analyze.py" "${SCAN_ROOT}" "${REPORT}" >/dev/null 2>&1
STATUS=$?

if [[ -n "${JSON_OUT}" ]]; then
    cp "${REPORT}" "${JSON_OUT}"
fi

SCANNED=$(python3 -c "import json;print(json.load(open('${REPORT}'))['scanned_count'])" 2>/dev/null || echo 0)
FAILURES=$(python3 -c "import json;print(json.load(open('${REPORT}'))['failure_count'])" 2>/dev/null || echo 0)

echo "  scanned ${SCANNED} file(s)"

if [[ "${STATUS}" -eq 0 ]]; then
    pass "C1 — the root package does not mutate process-global environment variables"
    echo ""
    echo "=== RESULT: [PASS] ==="
    exit 0
fi

echo ""
echo "  C1 violations:"
python3 - "${REPORT}" <<'PYEOF'
import json, sys
report = json.load(open(sys.argv[1]))
for v in report["violations"]:
    print("    %s:%d  [%s] %s" % (v["file"], v["line"], v["kind"], v["text"]))
PYEOF
echo ""
fail "C1 — ${FAILURES} process-global env mutation(s) in the root package"
echo ""
echo "  Why this matters (#5423): the root lib test binary runs its tests"
echo "  concurrently in ONE process; env mutation is visible to every"
echo "  sibling test and produced a ~1-in-3 random red in CI."
echo ""
echo "  Fix: use an explicit seam — ExecutionEngine::with_parallel_degree /"
echo "        ExecutionEngine::base_with_raw / resolve_parallel_degree —"
echo "        or move the test into its own binary under tests/."
echo ""
echo "=== RESULT: [FAIL] ==="
exit 1
