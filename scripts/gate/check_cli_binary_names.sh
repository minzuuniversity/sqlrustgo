#!/usr/bin/env bash
#
# cli-bin-allow-samples — the synthetic fixtures below (a fake `app-bin`
# package, a fake `app-cli` path) are **sample paths this gate tests itself
# against**. Requiring this file to be free of them would make the C3
# detector untestable. The exemption is a visible string in the source, and
# the gate lists every exempt file in its output.
#
# scripts/gate/check_cli_binary_names.sh
#
# Meta-gate: the name of a build artifact must have **exactly one** source
# of truth, and that source must be the manifest that produces it.
#
# #5405 — `tests/e2e/sqlrustgo_cli_soak_e2e_test.rs` looked for
# `target/debug/sqlrustgo-cli`. No bin is called `sqlrustgo-cli` — that is
# the **package** name of `crates/sqlrustgo-cli`, whose `[[bin]] name` is
# `sqlrustgo`. The test therefore never found its binary, and it only
# survived because the target had never actually been executed.
#
# The typo is not the real problem. The real problem is that "what is this
# binary called" had **two** answers — the package name and the bin name —
# and the test tree was free to pick either one. This gate removes that
# freedom: the manifest is the only place a bin name may come from, and
# everywhere else is cross-checked against it.
#
# THE INVARIANTS
# --------------
#
#   C1  no two workspace packages declare the same `[[bin]] name`
#       (a duplicate name is a cargo filename collision waiting to happen)
#
#   C2  every `env!("CARGO_BIN_EXE_<name>")` names a `[[bin]]` of the
#       **package that owns the file** — cargo only sets that variable for
#       the current package's own binaries, so a name borrowed from a
#       sibling package is a compile error waiting to happen
#
#   C3  every `target/<profile>/<name>` literal in the test trees and the
#       crate sources names a binary that some workspace package actually
#       produces (or a declared external tool)
#
#   C4  every candidate name in the shared resolver
#       (`tests/common/bin_resolver.rs`) is either a workspace bin or a
#       declared external tool — and the external-tool list is read from
#       that same file, so "what is not a workspace bin" has one source
#
#   C5  a package that declares `[[bin]]` targets produces **no binary the
#       manifest does not declare** — cargo auto-discovers `src/main.rs`
#       and `src/bin/<name>.rs`, and an auto-discovered binary is named
#       after the **package**, which is exactly the name #5405 banned
#
# Exit codes:
#   0  [PASS] — every bin name traces back to the manifest that produces it
#   1  [FAIL] — at least one invariant is violated
#   2  usage error / a required input is missing
#
# Usage:
#   bash scripts/gate/check_cli_binary_names.sh
#   bash scripts/gate/check_cli_binary_names.sh --json <out.json>
#   bash scripts/gate/check_cli_binary_names.sh --selftest
#
# --selftest builds synthetic fixtures — one clean, one per violation
# class — and asserts that the analyzer agrees with the expected verdict.
# A gate that cannot fail is not a gate; this is the evidence that it can.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
ANALYZER="${SCRIPT_DIR}/check_cli_binary_names.py"
JSON_OUT=""
SELFTEST=0

color_red()   { printf '\033[0;31m%s\033[0m' "$*"; }
color_green() { printf '\033[0;32m%s\033[0m' "$*"; }
color_yellow() { printf '\033[0;33m%s\033[0m' "$*"; }

step() { echo ""; echo "=== $* ==="; }
pass() { echo "  $(color_green PASS): $*"; }
fail() { echo "  $(color_red FAIL): $*"; }
note() { echo "  $(color_yellow NOTE): $*"; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --json)      JSON_OUT="$2"; shift 2 ;;
        --json=*)    JSON_OUT="${1#--json=}"; shift ;;
        --selftest)  SELFTEST=1; shift ;;
        -h|--help)
            echo "Usage: bash scripts/gate/check_cli_binary_names.sh [--json <out>] [--selftest]"
            exit 0
            ;;
        *) echo "ERROR: unknown argument: $1" >&2; exit 2 ;;
    esac
done

if [[ ! -f "${ANALYZER}" ]]; then
    echo "ERROR: analyzer not found: ${ANALYZER}" >&2
    exit 2
fi
if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required" >&2
    exit 2
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cli-binary-names.XXXXXX")"
trap 'rm -rf -- "${TMP_DIR}"' EXIT

# ---------------------------------------------------------------------------
# --selftest: the gate must be able to fail
# ---------------------------------------------------------------------------
if [[ "${SELFTEST}" -eq 1 ]]; then
    step "Self-test: synthetic fixtures"

    # --- fixture builder ---------------------------------------------------
    make_fixture() {
        local dir="$1"
        mkdir -p "${dir}/crates/app/src" "${dir}/crates/app/tests" \
                 "${dir}/crates/other/src" "${dir}/tests/common" "${dir}/scripts"
        cat > "${dir}/Cargo.toml" <<'EOF'
[workspace]
members = ["crates/app", "crates/other"]

[package]
name = "root"
version = "0.1.0"
EOF
        cat > "${dir}/crates/app/Cargo.toml" <<'EOF'
[package]
name = "app"
version = "0.1.0"

[[bin]]
name = "app-bin"
path = "src/main.rs"
EOF
        cat > "${dir}/crates/other/Cargo.toml" <<'EOF'
[package]
name = "other"
version = "0.1.0"

[[bin]]
name = "other-bin"
path = "src/main.rs"
EOF
        cat > "${dir}/tests/common/bin_resolver.rs" <<'EOF'
pub const EXTERNAL_TOOL_BINS: &[&str] = &["psql"];
pub const APP_BIN: &str = "app-bin";
pub const OTHER_BIN: &str = "other-bin";
pub const APP_ENV: &str = "APP_BIN_ENV";
pub const PSQL_ENV: &str = "PSQL_BIN_ENV";

pub fn resolve_binary(_env_var: &str, _names: &[&str]) -> Option<u32> { None }
pub fn resolve_external_tool(_env_var: &str, _names: &[&str]) -> Option<u32> { None }

pub fn call() {
    let _ = resolve_binary(APP_ENV, &[APP_BIN]);
    let _ = resolve_external_tool(PSQL_ENV, &["psql"]);
}
EOF
        # The test lives in the package that owns `app-bin`, because cargo
        # only sets CARGO_BIN_EXE_<name> for the current package's bins.
        cat > "${dir}/crates/app/tests/app_test.rs" <<'EOF'
#[test]
fn uses_the_right_binary() {
    let _ = env!("CARGO_BIN_EXE_app-bin");
    let _ = "target/debug/app-bin";
}
EOF
    }

    # expect_verdict <fixture-dir> <PASS|FAIL> <label>
    expect_verdict() {
        local dir="$1" expected="$2" label="$3"
        local out="${TMP_DIR}/verdict.json"
        if python3 "${ANALYZER}" --repo-root "${dir}" --json "${out}" >/dev/null 2>&1; then
            local got="PASS"
        else
            local got="FAIL"
        fi
        if [[ "${got}" == "${expected}" ]]; then
            pass "${label}: expected ${expected}, got ${got}"
            return 0
        fi
        fail "${label}: expected ${expected}, got ${got}"
        return 1
    }

    SELFTEST_FAILURES=0

    # 1. clean tree → PASS
    make_fixture "${TMP_DIR}/clean"
    expect_verdict "${TMP_DIR}/clean" PASS "clean tree" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    # 2. duplicate [[bin]] name across two packages → FAIL (C1)
    make_fixture "${TMP_DIR}/dup"
    sed -i.bak 's/name = "other-bin"/name = "app-bin"/' "${TMP_DIR}/dup/crates/other/Cargo.toml"
    rm -f "${TMP_DIR}/dup/crates/other/Cargo.toml.bak"
    expect_verdict "${TMP_DIR}/dup" FAIL "C1 duplicate bin name" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    # 3. CARGO_BIN_EXE borrowing another package's bin → FAIL (C2)
    make_fixture "${TMP_DIR}/c2"
    sed -i.bak 's/CARGO_BIN_EXE_app-bin/CARGO_BIN_EXE_other-bin/' "${TMP_DIR}/c2/crates/app/tests/app_test.rs"
    rm -f "${TMP_DIR}/c2/tests/app_test.rs.bak"
    expect_verdict "${TMP_DIR}/c2" FAIL "C2 cross-package CARGO_BIN_EXE" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    # 4. a target/<profile>/<name> literal that names nothing → FAIL (C3)
    make_fixture "${TMP_DIR}/c3"
    sed -i.bak 's|target/debug/app-bin|target/debug/app-cli|' "${TMP_DIR}/c3/crates/app/tests/app_test.rs"
    rm -f "${TMP_DIR}/c3/tests/app_test.rs.bak"
    expect_verdict "${TMP_DIR}/c3" FAIL "C3 unknown target literal" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    # 5. a resolver candidate that is neither a bin nor an external tool → FAIL (C4)
    make_fixture "${TMP_DIR}/c4"
    sed -i.bak 's/&\[APP_BIN\]/\&["app-cli"]/' "${TMP_DIR}/c4/tests/common/bin_resolver.rs"
    rm -f "${TMP_DIR}/c4/tests/common/bin_resolver.rs.bak"
    expect_verdict "${TMP_DIR}/c4" FAIL "C4 unknown resolver candidate" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    # 6. an external tool that is not on the declared external-tool list → FAIL (C4)
    make_fixture "${TMP_DIR}/c4ext"
    # Only the *call site* is rewritten — the declaration of
    # EXTERNAL_TOOL_BINS must keep saying "psql", otherwise the fixture
    # would stay self-consistent and prove nothing.
    sed -i.bak 's/&\["psql"\])/\&["mysql"])/' "${TMP_DIR}/c4ext/tests/common/bin_resolver.rs"
    rm -f "${TMP_DIR}/c4ext/tests/common/bin_resolver.rs.bak"
    expect_verdict "${TMP_DIR}/c4ext" FAIL "C4 undeclared external tool" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    # 7. a package that declares a [[bin]] AND still lets cargo auto-discover
    #    an extra binary from `src/main.rs` → FAIL (C5)
    make_fixture "${TMP_DIR}/c5"
    sed -i.bak 's|path = "src/main.rs"|path = "src/local_bin.rs"|' "${TMP_DIR}/c5/crates/app/Cargo.toml"
    rm -f "${TMP_DIR}/c5/crates/app/Cargo.toml.bak"
    cat > "${TMP_DIR}/c5/crates/app/src/local_bin.rs" <<'EOF'
fn main() {}
EOF
    # The auto-discovered entry point has to exist for the violation to be
    # real: a declared [[bin]] whose path points elsewhere, plus a
    # `src/main.rs` that cargo would turn into a second binary.
    cat > "${TMP_DIR}/c5/crates/app/src/main.rs" <<'EOF'
fn main() {}
EOF
    expect_verdict "${TMP_DIR}/c5" FAIL "C5 undeclared auto-discovered binary" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    echo ""
    if [[ "${SELFTEST_FAILURES}" -eq 0 ]]; then
        pass "CLI binary name gate self-test: 7/7 fixtures behaved as expected"
        exit 0
    fi
    fail "CLI binary name gate self-test: ${SELFTEST_FAILURES} fixture(s) misbehaved"
    exit 1
fi

# ---------------------------------------------------------------------------
# Real run
# ---------------------------------------------------------------------------
step "Analyzing bin names against the manifests"

RESULT_JSON="${TMP_DIR}/report.json"
python3 "${ANALYZER}" --repo-root "${REPO_ROOT}" --json "${RESULT_JSON}"
ANALYZER_EXIT=$?
# 0 = no violation, 1 = violation, >=2 = the analyzer itself failed.
if [[ "${ANALYZER_EXIT}" -ge 2 ]]; then
    echo "ERROR: analyzer failed (exit ${ANALYZER_EXIT})" >&2
    exit 2
fi

read_field() {
    python3 - "${RESULT_JSON}" "$1" <<'PYEOF'
import json
import sys
with open(sys.argv[1]) as fh:
    data = json.load(fh)
key = sys.argv[2]
if key == "bad_target_literals":
    for e in data.get("target_literals", []):
        if not e.get("known"):
            print("%s -> %s" % (e["file"], e["name"]))
elif key == "bad_cargo_bin_exe":
    for e in data.get("cargo_bin_exe", []):
        if not e.get("ok"):
            print("%s -> %s (owner=%s, declared in %s)" % (
                e["file"], e["name"], e.get("owner"), ",".join(e.get("declared_in", []))))
elif key == "duplicate_bins":
    for e in data.get("duplicate_bins", []):
        print("%s -> %s" % (e["name"], ",".join(e["packages"])))
elif key == "bad_resolver_candidates":
    for e in data.get("resolver_candidates", []):
        if not e.get("known"):
            print("%s() -> %s" % (e["call"], e["name"]))
elif key == "c3_exempt":
    for e in data.get("c3_exempt", []):
        print(e)
elif key == "implicit_bins":
    for e in data.get("implicit_bins", []):
        print("%s -> declares %s but also produces %s" % (
            e["package"],
            ",".join(e.get("declared_bins", [])),
            ",".join(e.get("implicit_entries", []))))
else:
    print(data.get(key, ""))
PYEOF
}

PKG_COUNT="$(read_field package_count)"
BIN_COUNT="$(python3 -c "import json,sys; print(len(json.load(open(sys.argv[1]))['workspace_bins']))" "${RESULT_JSON}")"
C2_COUNT="$(python3 -c "import json,sys; print(len(json.load(open(sys.argv[1]))['cargo_bin_exe']))" "${RESULT_JSON}")"
C3_COUNT="$(python3 -c "import json,sys; print(len(json.load(open(sys.argv[1]))['target_literals']))" "${RESULT_JSON}")"

echo "  workspace packages:                ${PKG_COUNT}"
echo "  declared [[bin]] names:            ${BIN_COUNT}"
echo "  CARGO_BIN_EXE references checked:  ${C2_COUNT}"
echo "  target/<profile>/<name> literals:  ${C3_COUNT}"

FAILURES=0

DUPS="$(read_field duplicate_bins)"
if [[ -n "${DUPS}" ]]; then
    fail "C1: two packages declare the same [[bin]] name:"
    while IFS= read -r d; do
        [[ -z "${d}" ]] && continue
        echo "    ${d}"
    done <<< "${DUPS}"
    FAILURES=$((FAILURES + 1))
else
    pass "C1: every [[bin]] name is unique across the workspace"
fi

BAD_C2="$(read_field bad_cargo_bin_exe)"
if [[ -n "${BAD_C2}" ]]; then
    fail "C2: CARGO_BIN_EXE_<name> does not belong to the owning package:"
    while IFS= read -r h; do
        [[ -z "${h}" ]] && continue
        echo "    ${h}"
    done <<< "${BAD_C2}"
    FAILURES=$((FAILURES + 1))
else
    pass "C2: every CARGO_BIN_EXE_<name> belongs to the package that owns it"
fi

BAD_C3="$(read_field bad_target_literals)"
if [[ -n "${BAD_C3}" ]]; then
    fail "C3: a target/<profile>/<name> literal names a binary nothing produces:"
    while IFS= read -r h; do
        [[ -z "${h}" ]] && continue
        echo "    ${h}"
    done <<< "${BAD_C3}"
    FAILURES=$((FAILURES + 1))
else
    pass "C3: every target/<profile>/<name> literal names a real binary"
fi

BAD_C4="$(read_field bad_resolver_candidates)"
if [[ -n "${BAD_C4}" ]]; then
    fail "C4: the shared resolver names a binary nothing produces:"
    while IFS= read -r h; do
        [[ -z "${h}" ]] && continue
        echo "    ${h}"
    done <<< "${BAD_C4}"
    FAILURES=$((FAILURES + 1))
else
    pass "C4: every resolver candidate is a workspace bin or a declared external tool"
fi

# C5 — a package that declares `[[bin]]` targets must not also produce a
# binary that the manifest never declared (cargo's auto-discovery of
# `src/main.rs` / `src/bin/<name>.rs`).
IMPLICIT="$(read_field implicit_bins)"
if [[ -n "${IMPLICIT}" ]]; then
    fail "C5: a package produces a binary its manifest does not declare:"
    while IFS= read -r h; do
        [[ -z "${h}" ]] && continue
        echo "    ${h}"
    done <<< "${IMPLICIT}"
    FAILURES=$((FAILURES + 1))
else
    pass "C5: every package produces exactly the binaries its manifest declares"
fi

EXEMPT="$(read_field c3_exempt)"
if [[ -n "${EXEMPT}" ]]; then
    note "C3 exemptions (files that deliberately carry sample paths):"
    while IFS= read -r e; do
        [[ -z "${e}" ]] && continue
        echo "    ${e}"
    done <<< "${EXEMPT}"
fi

if [[ -n "${JSON_OUT}" ]]; then
    cp "${RESULT_JSON}" "${JSON_OUT}"
fi

echo ""
if [[ "${FAILURES}" -eq 0 ]]; then
    pass "CLI binary name gate: ${BIN_COUNT} bin names, all traced to their manifest"
    exit 0
fi

fail "CLI binary name gate: ${FAILURES} invariant(s) violated"
exit 1
