#!/usr/bin/env bash
#
# scripts/gate/check_test_manifest_honesty.sh
#
# Meta-gate: the Cargo test manifest must describe **exactly** the set of
# test binaries cargo builds.
#
# #5449 — the "number of test targets" was split in three:
#
#   1. 449 `[[test]]` declarations in Cargo.toml,
#   2. 15 top-level `tests/*.rs` files that cargo *auto-discovers* and runs
#      but that nobody declared (they were invisible to every manifest-based
#      gate, and to the reader of Cargo.toml),
#   3. 6 sub-directory files that cargo neither auto-discovers nor has a
#      `[[test]]` declaration for — **never compiled, never executed**.
#      34 `#[test]` functions lived in them.
#
# Plus a fourth, silent case that `cargo metadata` revealed:
#
#   4. `tests/sequence_test.rs` (top level) was **silently dropped** by
#      cargo: an explicit `[[test]] name = "sequence_test"` pointing at
#      `tests/integration/sql/sequence_test.rs` claimed the *name*, so
#      cargo's auto-discovery of `tests/sequence_test.rs` was discarded
#      without any diagnostic. 10 `#[test]` functions never ran.
#
# The existing `check_test_target_execution.sh` cannot see any of this: it
# compares the *declared* targets against the *executed* ones, so a file
# that is neither declared nor executed is invisible to it.
#
# This gate closes the gap statically, without needing a cargo test log:
#
#   C1  every declared `[[test]]` path exists on disk
#   C2  no two declared targets share a name
#   C3  every test-bearing file under `tests/` is either a declared target or
#       reachable from one (via `#[path = "..."]` or `mod x;`) — no orphans
#   C4  every top-level `tests/*.rs` has its **own** explicit `[[test]]`
#       declaration, so nothing depends on cargo's auto-discovery
#       (auto-discovery is what silently dropped `tests/sequence_test.rs`)
#   C5  the number of declared targets equals the number of test targets
#       cargo actually builds (`cargo metadata --no-deps`)
#
# Exit codes:
#   0  [PASS] — the manifest and cargo agree, no orphan test file
#   1  [FAIL] — at least one invariant is violated
#   2  usage error / a required input is missing
#
# C5 needs `cargo` on PATH. When cargo is unavailable the check reports
# NOT-RUN with a reason code and the remaining checks still decide the
# verdict — "could not measure" is never silently reported as "measured
# and fine".
#
# Usage:
#   bash scripts/gate/check_test_manifest_honesty.sh
#   bash scripts/gate/check_test_manifest_honesty.sh --json <out.json>

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
CARGO_TOML="${REPO_ROOT}/Cargo.toml"
JSON_OUT=""

color_red()   { printf '\033[0;31m%s\033[0m' "$*"; }
color_green() { printf '\033[0;32m%s\033[0m' "$*"; }
color_yellow() { printf '\033[0;33m%s\033[0m' "$*"; }

step() { echo ""; echo "=== $* ==="; }
pass() { echo "  $(color_green PASS): $*"; }
fail() { echo "  $(color_red FAIL): $*"; }
note() { echo "  $(color_yellow NOTE): $*"; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --json)   JSON_OUT="$2"; shift 2 ;;
        --json=*) JSON_OUT="${1#--json=}"; shift ;;
        -h|--help)
            echo "Usage: bash scripts/gate/check_test_manifest_honesty.sh [--json <out>]"
            exit 0
            ;;
        *) echo "ERROR: unknown argument: $1" >&2; exit 2 ;;
    esac
done

if [[ ! -f "${CARGO_TOML}" ]]; then
    echo "ERROR: Cargo.toml not found: ${CARGO_TOML}" >&2
    exit 2
fi
if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required" >&2
    exit 2
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/test-manifest-honesty.XXXXXX")"
trap 'rm -rf -- "${TMP_DIR}"' EXIT

# ---------------------------------------------------------------------------
# 1. Static analysis: declarations, orphans, top-level coverage
# ---------------------------------------------------------------------------
step "Step 1/3: static manifest analysis"

python3 - "${CARGO_TOML}" "${REPO_ROOT}" > "${TMP_DIR}/static.json" <<'PYEOF'
import json
import os
import re
import sys

cargo_toml, repo_root = sys.argv[1], sys.argv[2]

# --- declared [[test]] targets -------------------------------------------
declared = []
in_test = False
name = None
path = None


def flush():
    global name, path
    if in_test and name is not None:
        declared.append((name, path))
    name = None
    path = None


with open(cargo_toml) as fh:
    for line in fh:
        stripped = line.strip()
        if stripped.startswith("#"):
            continue
        if stripped == "[[test]]":
            flush()
            in_test = True
            continue
        if in_test:
            if re.match(r"^\s*\[\[?[A-Za-z]", line):
                flush()
                in_test = False
                continue
            m = re.match(r'^\s*name\s*=\s*"([^"]*)"', line)
            if m:
                name = m.group(1)
                continue
            m = re.match(r'^\s*path\s*=\s*"([^"]*)"', line)
            if m:
                path = m.group(1)
                continue
flush()

decl_paths = []
for n, p in declared:
    decl_paths.append(os.path.normpath(p if p else "tests/%s.rs" % n))
decl_path_set = set(decl_paths)

# --- C1: declared path exists --------------------------------------------
missing_paths = sorted({p for p in decl_path_set if not os.path.isfile(os.path.join(repo_root, p))})

# --- C2: duplicate names --------------------------------------------------
seen = {}
duplicate_names = []
for n, p in declared:
    if n in seen:
        duplicate_names.append({"name": n, "first": seen[n], "second": p})
    else:
        seen[n] = p

# --- reachability from the declared targets -------------------------------
PATH_RE = re.compile(r'#\[\s*path\s*=\s*"([^"]+)"\s*\]')
MOD_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;",
    re.M,
)

reachable = set()
stack = list(decl_path_set)
while stack:
    cur = stack.pop()
    if cur in reachable:
        continue
    reachable.add(cur)
    full = os.path.join(repo_root, cur)
    if not os.path.isfile(full):
        continue
    with open(full, errors="replace") as fh:
        src = fh.read()
    base = os.path.dirname(cur)
    for m in PATH_RE.finditer(src):
        stack.append(os.path.normpath(os.path.join(base, m.group(1))))
    for m in MOD_RE.finditer(src):
        mod = m.group(1)
        for cand in (os.path.join(base, mod + ".rs"), os.path.join(base, mod, "mod.rs")):
            if os.path.isfile(os.path.join(repo_root, cand)):
                stack.append(os.path.normpath(cand))

# --- every .rs under tests/ ----------------------------------------------
all_rs = []
tests_root = os.path.join(repo_root, "tests")
for dirpath, _dirnames, filenames in os.walk(tests_root):
    for fn in filenames:
        if fn.endswith(".rs"):
            full = os.path.join(dirpath, fn)
            all_rs.append(os.path.normpath(os.path.relpath(full, repo_root)))
all_rs.sort()

TEST_RE = re.compile(r"#\[\s*(?:tokio::)?test\s*\]")

orphans = []
for f in all_rs:
    if f in decl_path_set or f in reachable:
        continue
    with open(os.path.join(repo_root, f), errors="replace") as fh:
        src = fh.read()
    n = len(TEST_RE.findall(src))
    if n:
        orphans.append({"path": f, "test_count": n})

# --- C4: top-level tests/*.rs must be explicitly declared -----------------
top_level = [f for f in all_rs if "/" not in f[len("tests/"):]]
top_level_undeclared = sorted(f for f in top_level if f not in decl_path_set)

# --- top-level files that cargo's auto-discovery would drop ---------------
# A file whose *stem* collides with the name of an explicitly declared target
# is silently discarded by cargo. Detect it before cargo does it for us.
declared_names = {n for n, _ in declared}
shadowed = []
for f in top_level:
    stem = os.path.basename(f)[:-3]
    if stem in declared_names and f not in decl_path_set:
        shadowed.append({"path": f, "stem": stem})

print(json.dumps({
    "declared_count": len(declared),
    "declared_paths": sorted(decl_path_set),
    "missing_paths": missing_paths,
    "duplicate_names": duplicate_names,
    "orphans": orphans,
    "orphan_case_count": sum(o["test_count"] for o in orphans),
    "top_level_count": len(top_level),
    "top_level_undeclared": top_level_undeclared,
    "shadowed": shadowed,
    "total_rs": len(all_rs),
}, indent=2))
PYEOF

if [[ ! -f "${TMP_DIR}/static.json" ]]; then
    fail "static analysis produced no result"
    exit 2
fi

read_static() {
    python3 - "${TMP_DIR}/static.json" "$1" <<'PYEOF'
import json
import sys
with open(sys.argv[1]) as fh:
    data = json.load(fh)
value = data.get(sys.argv[2])
if isinstance(value, list):
    for item in value:
        if isinstance(item, dict):
            print(" ".join("%s=%s" % (k, v) for k, v in item.items()))
        else:
            print(item)
elif isinstance(value, int):
    print(value)
PYEOF
}

DECLARED_COUNT="$(read_static declared_count)"
ORPHAN_CASES="$(read_static orphan_case_count)"
TOPLEVEL_COUNT="$(read_static top_level_count)"
TOTAL_RS="$(read_static total_rs)"

echo "  declared [[test]] targets:      ${DECLARED_COUNT}"
echo "  .rs files under tests/:          ${TOTAL_RS}"
echo "  top-level tests/*.rs:            ${TOPLEVEL_COUNT}"
echo "  orphan test files:               $(read_static orphans | wc -l | tr -d '[:space:]') (${ORPHAN_CASES} #[test] functions)"

FAILURES=0

# --- C1 --------------------------------------------------------------------
MISSING_PATHS="$(read_static missing_paths)"
if [[ -n "${MISSING_PATHS}" ]]; then
    fail "C1: declared target(s) point at a file that does not exist:"
    while IFS= read -r p; do
        [[ -z "${p}" ]] && continue
        echo "    ${p}"
    done <<< "${MISSING_PATHS}"
    FAILURES=$((FAILURES + 1))
else
    pass "C1: every declared target exists on disk"
fi

# --- C2 --------------------------------------------------------------------
DUP_NAMES="$(read_static duplicate_names)"
if [[ -n "${DUP_NAMES}" ]]; then
    fail "C2: duplicate [[test]] target name(s):"
    while IFS= read -r d; do
        [[ -z "${d}" ]] && continue
        echo "    ${d}"
    done <<< "${DUP_NAMES}"
    FAILURES=$((FAILURES + 1))
else
    pass "C2: no duplicate target names"
fi

# --- C3 --------------------------------------------------------------------
ORPHANS="$(read_static orphans)"
if [[ -n "${ORPHANS}" ]]; then
    fail "C3: test file(s) that no declared target compiles — they never run:"
    while IFS= read -r o; do
        [[ -z "${o}" ]] && continue
        echo "    ${o}"
    done <<< "${ORPHANS}"
    FAILURES=$((FAILURES + 1))
else
    pass "C3: no orphan test file (every test file is declared or reachable)"
fi

# --- C4 --------------------------------------------------------------------
TOPLEVEL_UNDECLARED="$(read_static top_level_undeclared)"
if [[ -n "${TOPLEVEL_UNDECLARED}" ]]; then
    fail "C4: top-level tests/*.rs without an explicit [[test]] declaration:"
    while IFS= read -r p; do
        [[ -z "${p}" ]] && continue
        echo "    ${p}"
    done <<< "${TOPLEVEL_UNDECLARED}"
    FAILURES=$((FAILURES + 1))
else
    pass "C4: every top-level tests/*.rs has its own [[test]] declaration"
fi

# ---------------------------------------------------------------------------
# 1b. C6 — no machine-specific home-directory path in the test tree
# ---------------------------------------------------------------------------
step "Step 1b/3: machine-specific path scan"

# #5406 / #5477: a test that resolves a binary (or a fixture) through a
# developer's home directory runs on exactly one machine. When the path does
# not resolve, the usual reaction is to degrade to "skip" — which turns the
# test into a fake green.
#
# THE INVARIANT
# -------------
#
#   **The test tree must not contain an absolute path rooted in a user's
#   home directory** (`/home/<user>/…` or `/Users/<user>/…`).
#
# Why this and not a narrower rule:
#
#   * A home-directory path is *by construction* machine-specific. It is the
#     single most reliable marker of "this only ever worked on one box".
#   * The earlier version of this check matched only `/home/<x>/sqlrustgo`
#     — i.e. it looked for the *checkout* under the home directory. That is
#     a string heuristic, not an invariant: it missed `/home/<u>/tpch_baseline/sf1`
#     (a fixture path), `/home/<u>/tpch-dbgen-master/dbgen` (a tool path),
#     and every layout where the checkout is not directly under the home
#     directory. Measured on the pre-change tree: 16 hits, of which the old
#     regex saw 4. The check was reporting "PASS" while 12 machine-specific
#     paths were still in the tree.
#   * Machine-specific-ness is a property of the *path*, not of what the
#     path names (checkout vs. fixture vs. tool). So the criterion must be
#     about the path.
#
# THE EXCEPTION
# -------------
#
# A file may opt out with the marker `c6-allow-home-paths` in its first 20
# lines. The marker exists for exactly one reason: **the checker and the
# shared resolver need sample paths to test themselves against**. Those
# files are *about* machine-specific paths; requiring them to contain none
# would make it impossible to test the detector at all.
#
# The exception is explicit and auditable (it is a visible marker in the
# source, not a hidden path list), it is scoped per-file, and every exempt
# file is listed in the gate output so a reviewer can see them.

C6_EXEMPT_MARKER="c6-allow-home-paths"
C6_HITS=""
C6_EXEMPT_LIST=""

while IFS= read -r file; do
    [[ -z "${file}" ]] && continue
    rel="${file#"${REPO_ROOT}/"}"
    # The exemption marker must appear near the top of the file, so that a
    # stray marker deep inside a real test cannot silently exempt it.
    if head -20 "${file}" | grep -qF "${C6_EXEMPT_MARKER}"; then
        C6_EXEMPT_LIST="${C6_EXEMPT_LIST}${rel}"$'
'
        continue
    fi
    # Report the file once, with the offending lines.
    file_hits="$(grep -nE '(^|[^A-Za-z0-9_])/(Users|home)/[A-Za-z0-9_.-]+' "${file}" 2>/dev/null || true)"
    if [[ -n "${file_hits}" ]]; then
        while IFS= read -r hit; do
            [[ -z "${hit}" ]] && continue
            C6_HITS="${C6_HITS}${rel}:${hit}"$'
'
        done <<< "${file_hits}"
    fi
done < <(find "${REPO_ROOT}/tests" -type f \( -name '*.rs' -o -name '*.sh' \) 2>/dev/null | sort)

if [[ -n "${C6_HITS}" ]]; then
    fail "C6: the test tree contains a machine-specific home-directory path:"
    while IFS= read -r hit; do
        [[ -z "${hit}" ]] && continue
        echo "    ${hit}"
    done <<< "${C6_HITS}"
    echo "    (fix: resolve the path at run time — tests/common/bin_resolver.rs for"
    echo "     binaries, tests/common/sf1_fixture.rs for the SF=1 fixture)"
    FAILURES=$((FAILURES + 1))
else
    pass "C6: no machine-specific home-directory path in the test tree"
fi

if [[ -n "${C6_EXEMPT_LIST}" ]]; then
    note "C6 exemptions (files that legitimately contain sample paths):"
    while IFS= read -r ex; do
        [[ -z "${ex}" ]] && continue
        echo "    ${ex}"
    done <<< "${C6_EXEMPT_LIST}"
fi

# ---------------------------------------------------------------------------
# 2. C5 — cross-check against cargo's own view of the target list
# ---------------------------------------------------------------------------
step "Step 2/3: cross-check against cargo metadata"

C5_STATUS="SKIPPED"
C5_DETAIL="cargo not on PATH"
C5_CARGO_COUNT=""

if command -v cargo >/dev/null 2>&1; then
    if cargo metadata --no-deps --format-version 1 --manifest-path "${CARGO_TOML}" \
            > "${TMP_DIR}/metadata.json" 2> "${TMP_DIR}/metadata.err"; then
        C5_CARGO_COUNT="$(python3 - "${TMP_DIR}/metadata.json" "${CARGO_TOML}" <<'PYEOF'
import json
import os
import sys
with open(sys.argv[1]) as fh:
    meta = json.load(fh)
root_manifest = os.path.realpath(sys.argv[2])
count = 0
for pkg in meta.get("packages", []):
    # Only the root package: the workspace members bring their own test
    # targets, which have nothing to do with the root Cargo.toml manifest.
    if os.path.realpath(pkg.get("manifest_path", "")) != root_manifest:
        continue
    for target in pkg.get("targets", []):
        kinds = target.get("kind", [])
        if "test" in kinds and not any(k in kinds for k in ("bench", "example")):
            count += 1
print(count)
PYEOF
)"
        C5_STATUS="MEASURED"
        C5_DETAIL="cargo reports ${C5_CARGO_COUNT} test targets"
        echo "  ${C5_DETAIL}, manifest declares ${DECLARED_COUNT}"
    else
        C5_STATUS="NOT-RUN"
        C5_DETAIL="cargo metadata failed: $(head -3 "${TMP_DIR}/metadata.err" | tr '\n' ' ')"
    fi
else
    echo "  cargo not found on PATH"
fi

if [[ "${C5_STATUS}" == "MEASURED" ]]; then
    if [[ "${C5_CARGO_COUNT}" == "${DECLARED_COUNT}" ]]; then
        pass "C5: declared targets == cargo's test targets (${DECLARED_COUNT})"
    else
        fail "C5: declared ${DECLARED_COUNT} targets but cargo builds ${C5_CARGO_COUNT}"
        echo "    a declared target that cargo silently drops (name collision with an"
        echo "    auto-discovered file) or an auto-discovered file that was never"
        echo "    declared will show up here"
        FAILURES=$((FAILURES + 1))
    fi
else
    note "C5: NOT-RUN (${C5_DETAIL}) — the count could not be measured"
fi

# ---------------------------------------------------------------------------
# 3. Verdict
# ---------------------------------------------------------------------------
step "Step 3/3: verdict"

if [[ -n "${JSON_OUT}" ]]; then
    python3 - "${TMP_DIR}/static.json" "${JSON_OUT}" "${FAILURES}" "${C5_STATUS}" "${C5_CARGO_COUNT}" "${C5_DETAIL}" <<'PYEOF'
import json
import sys
static_path, out_path, failures, c5_status, c5_count, c5_detail = sys.argv[1:7]
with open(static_path) as fh:
    data = json.load(fh)
data["status"] = "PASS" if int(failures) == 0 else "FAIL"
data["failure_count"] = int(failures)
data["c5_status"] = c5_status
data["c5_cargo_target_count"] = c5_count
data["c5_detail"] = c5_detail
with open(out_path, "w") as fh:
    json.dump(data, fh, indent=2, ensure_ascii=False)
    fh.write("\n")
PYEOF
fi

echo ""
if [[ "${FAILURES}" -eq 0 ]]; then
    pass "test manifest honesty gate: ${DECLARED_COUNT} declared targets, 0 orphan test files"
    exit 0
fi

fail "test manifest honesty gate: ${FAILURES} invariant(s) violated"
exit 1
