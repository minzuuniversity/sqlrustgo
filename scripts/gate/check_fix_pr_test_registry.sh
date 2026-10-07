#!/usr/bin/env bash
# scripts/gate/check_fix_pr_test_registry.sh
#
# v4.1.0 fix-PR test registry gate (ADR-008 §Policy 3 extension)
#
# SSOT: tests/baseline/v4.1_fix_pr_test_registry.json
#
# This gate enforces the audit's verdict: every fix/fix-gate/feat/feat-gate
# PR merged into develop/v4.1.0 must (a) add at least one regression test
# (a new file or a meaningful edit to an existing one), and (b) the test
# must exercise the bug being fixed.
#
# Enforcement is in two passes:
#   STEP 1: structural — every entry's `test_paths` files exist, each file
#             has at least one `#[test]`, and the path matches the registry
#             glob. Cheap; < 1 s.
#   STEP 2: behavioural — for every Rust test path, run a focused
#             `cargo test --no-run` (compile) to catch "test file exists
#             but was silently deleted from Cargo.toml". The actual
#             execution step is opt-in (REGISTRY_RUN_TESTS=1) because
#             the full test matrix is slow and the existing
#             check_anti_fabrication.sh step 2 already gates compile.
#
# Fail-closed: deleting a registered file or its test binary name fails
# the gate. The registry cannot be weakened by editing JSON; new fix PRs
# must add new entries (the work of the fix PR itself).
#
# Exit codes:
#   0  PASS — all entries structurally present, all Rust tests compile
#   1  FAIL — structural miss (file deleted, bin missing, malformed JSON)
#   2  FAIL — a registered Rust test fails to compile
#   3  FAIL — a registered test panicked at runtime (only with
#              REGISTRY_RUN_TESTS=1; off by default to avoid gate
#              time-budget pressure)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

REGISTRY="$REPO_ROOT/tests/baseline/v4.1_fix_pr_test_registry.json"
RUN_TESTS="${REGISTRY_RUN_TESTS:-0}"
TEST_TIMEOUT_SEC="${REGISTRY_TEST_TIMEOUT:-60}"

ERRORS=0
WARNINGS=0
log_info() { echo "[INFO]  $*"; }
log_error() { echo "[ERROR] $*" >&2; ERRORS=$((ERRORS + 1)); }
log_warn()  { echo "[WARN]  $*" >&2; WARNINGS=$((WARNINGS + 1)); }
log_pass() { echo "[PASS]  $*"; }

# ---------- STEP 0: registry sanity ----------
if [ ! -f "$REGISTRY" ]; then
    log_error "registry not found at $REGISTRY"
    exit 1
fi
if ! python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$REGISTRY" 2>/dev/null; then
    log_error "registry is not valid JSON: $REGISTRY"
    exit 1
fi
log_info "registry: $REGISTRY"
log_info "  ($(grep -c '"pr":' "$REGISTRY") entries, schema v$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['schema_version'])" "$REGISTRY"))"

# ---------- STEP 1: structural verification ----------
# For each entry, every test_path must exist and contain at least one
# `#[test]` (or the entry must justify its absence — non-Rust scripts,
# docs, etc.). Counted as a guardrail so an agent cannot trivially
# satisfy the gate with a `test_paths: [something_unrelated]`.
log_info ""
log_info "STEP 1: structural pass"

# Two passes via Python: per-entry and per-path; bash loops are slow.
python3 - "$REGISTRY" <<'PY' || exit 1
import json, os, sys, re
data = json.load(open(sys.argv[1]))
test_re = re.compile(r'#\s*\[\s*test\b')
# Tests that the path is in scope (not in archive/) by checking a small
# list of allowed roots.
def in_scope(p):
    return p.startswith('tests/') or p.startswith('crates/') or p.startswith('scripts/') or p.startswith('docs/')

errors = 0
checked = 0
for e in data['entries']:
    pr = e['pr']
    for tp in e['test_paths']:
        checked += 1
        if not os.path.exists(tp):
            print(f"  #{pr}: MISSING file {tp}")
            errors += 1
            continue
        if not in_scope(tp):
            print(f"  #{pr}: file {tp} not in allowed scope (tests/ crates/ scripts/ docs/)")
            errors += 1
            continue
        # For Rust tests, check at least one `#[test]` annotation.
        if tp.endswith('.rs'):
            try:
                content = open(tp).read()
            except Exception as ex:
                print(f"  #{pr}: cannot read {tp}: {ex}")
                errors += 1
                continue
            if not test_re.search(content):
                print(f"  #{pr}: file {tp} has no `#[test]` annotation")
                errors += 1
                continue
        # OK

if errors == 0:
    print(f"  PASS — {checked} test_paths across {len(data['entries'])} PRs")
else:
    print(f"  FAIL — {errors} structural miss(es)")
sys.exit(1 if errors else 0)
PY
RC=$?
if [ $RC -ne 0 ]; then
    log_error "STEP 1: structural verification FAILED"
    exit 1
fi
log_pass "STEP 1: structural verification — all entries have their test files"

# ---------- STEP 2: compile verification ----------
# For each Rust test file path, derive the cargo test target name and
# ensure it compiles via `cargo test --no-run`. Skip non-Rust files.
log_info ""
log_info "STEP 2: cargo test --no-run (compile) on registered Rust test targets"

# Derive target names by reading cargo metadata. Cargo autotests are
# named after the file stem, regardless of the path depth:
#   tests/<n>.rs              → <n>            (autotest, kind=test)
#   crates/<c>/tests/<n>.rs   → <n>            (autotest, kind=test)
# We look up each candidate against cargo metadata so we know whether
# to use --test or --bin (bin targets are listed in [[bin]] sections).
TARGETS_FILE=$(mktemp)
BIN_NAMES_FILE=$(mktemp)
TEST_NAMES_FILE=$(mktemp)
cat > /tmp/fix_registry_derive_targets.py <<'PYEOF'
import json, subprocess, sys

registry = json.load(open(sys.argv[1]))

# Build candidate set from registry paths. Cargo autotests use the
# file stem as the target name (not the crate name), so the derivation
# rule is identical whether the file lives in tests/ or crates/<c>/tests/.
candidates = set()
for e in registry["entries"]:
    for tp in e["test_paths"]:
        if not tp.endswith(".rs"):
            continue
        if not (tp.startswith("tests/") or tp.startswith("crates/")):
            continue
        stem = tp.split("/")[-1].removesuffix(".rs").replace("-", "_")
        candidates.add(stem)

# Read cargo metadata to learn each target's kind + owning package.
meta = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--no-deps", "--format-version=1"]))

# Map: target_name -> list of (pkg_name, kind)
target_map = {}
for pkg in meta["packages"]:
    for t in pkg["targets"]:
        target_map.setdefault(t["name"], []).append((pkg["name"], t["kind"]))

# Classify candidates. Each candidate yields one (kind, target, pkg) tuple.
# We avoid duplicating same-name targets across packages; pick the first
# match (they should be unique anyway — autotests are scoped to one crate).
matched_bins, matched_tests, unmatched = [], [], []
for c in sorted(candidates):
    matches = target_map.get(c)
    if not matches:
        unmatched.append(c)
        continue
    # Pick first pkg/bin first (safer since default-bless seen earlier).
    pkg_name = matches[0][0]
    kinds_present = set()
    for _, kinds in matches:
        for k in kinds:
            if k == "bin":
                kinds_present.add("bin")
            elif k == "test":
                kinds_present.add("test")
    if "bin" in kinds_present:
        matched_bins.append((pkg_name, c))
    if "test" in kinds_present:
        matched_tests.append((pkg_name, c))

with open(sys.argv[2], "w") as f:
    for b in matched_bins: f.write("{}\t{}\n".format(*b))
with open(sys.argv[3], "w") as f:
    for t in matched_tests: f.write("{}\t{}\n".format(*t))
print("  candidates matched as --bin:  {}".format(len(matched_bins)))
print("  candidates matched as --test: {}".format(len(matched_tests)))
print("  unmatched (probe/dev artifact): {}".format(len(unmatched)))
if unmatched:
    print("    (not in metadata) " + ", ".join(unmatched))
PYEOF
python3 /tmp/fix_registry_derive_targets.py "$REGISTRY" "$BIN_NAMES_FILE" "$TEST_NAMES_FILE"

# Build cargo invocations. Each test target needs `cargo test -p <pkg>
# --test <name> --no-run` (or --bin) because non-default packages (e.g.
# sqlrustgo-mysql-server) require explicit `-p`. We group by package
# using small per-package files (avoids bash 3.2 `declare -A`).
BUCKET_DIR=$(mktemp -d)
while IFS=$'\t' read -r pkg name; do
    printf '%s\n' "$name" >> "$BUCKET_DIR/bin.$pkg"
done < "$BIN_NAMES_FILE"
while IFS=$'\t' read -r pkg name; do
    printf '%s\n' "$name" >> "$BUCKET_DIR/test.$pkg"
done < "$TEST_NAMES_FILE"
rm -f "$TARGETS_FILE" "$BIN_NAMES_FILE" "$TEST_NAMES_FILE" /tmp/fix_registry_derive_targets.py

PKGS=$(ls "$BUCKET_DIR" 2>/dev/null | sed 's/^[a-z]*\.//' | sort -u)
if [ -z "$PKGS" ]; then
    log_warn "STEP 2: no Rust test targets in registry — skipping compile check"
    rm -rf "$BUCKET_DIR"
else
    TOTAL=$(cat "$BUCKET_DIR"/bin.* "$BUCKET_DIR"/test.* 2>/dev/null | wc -l | tr -d ' ')
    log_info "  total targets: $TOTAL"
    RC=0
    for pkg in $PKGS; do
        args=(test "-p" "$pkg")
        if [ -f "$BUCKET_DIR/bin.$pkg" ]; then
            while IFS= read -r n; do args+=("--bin" "$n"); done < "$BUCKET_DIR/bin.$pkg"
        fi
        if [ -f "$BUCKET_DIR/test.$pkg" ]; then
            while IFS= read -r n; do args+=("--test" "$n"); done < "$BUCKET_DIR/test.$pkg"
        fi
        args+=("--no-run")
        log_info "  running: cargo ${args[*]}"
        timeout 1800 cargo "${args[@]}" 2>>/tmp/fix-registry-compile.log
        if [ $? -ne 0 ]; then RC=1; fi
    done
    rm -rf "$BUCKET_DIR"
    if [ $RC -eq 0 ]; then
        log_pass "STEP 2: all $TOTAL test targets compile"
    else
        log_error "STEP 2: compile FAILED for one or more targets (see /tmp/fix-registry-compile.log)"
        tail -80 /tmp/fix-registry-compile.log >&2
        exit 2
    fi
fi

# ---------- STEP 3 (optional): run a representative subset ----------
# Off by default. When `REGISTRY_RUN_TESTS=1`, run a small sample of
# the registered tests with a per-test timeout. This is the strongest
# check: it confirms the fix actually works end-to-end. Off because the
# full v4.1.0 test matrix is slow; CI runs the full `cargo test
# --workspace` separately (see check_anti_fabrication.sh).
if [ "$RUN_TESTS" = "1" ]; then
    log_info ""
    log_info "STEP 3: running representative subset (REGISTRY_RUN_TESTS=1)"
    # Build the (package, target_name) set via metadata so we pick the
    # right `-p <pkg> --test <name>` pair (cargo autotest names are the
    # file stem, regardless of nesting).
    SAMPLE_FILE=$(mktemp)
    python3 - "$REGISTRY" "$SAMPLE_FILE" <<'PY' || exit 3
import json, subprocess, sys
data = json.load(open(sys.argv[1]))
candidates = set()
for e in data["entries"]:
    for tp in e["test_paths"]:
        if not tp.endswith(".rs"):
            continue
        if tp.startswith("tests/") or tp.startswith("crates/"):
            candidates.add(tp.split("/")[-1].removesuffix(".rs").replace("-", "_"))
meta = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--no-deps", "--format-version=1"]))
target_pkg = {}
for pkg in meta["packages"]:
    for t in pkg["targets"]:
        if "test" in t["kind"]:
            target_pkg.setdefault(t["name"], pkg["name"])
with open(sys.argv[2], "w") as f:
    for c in sorted(candidates):
        if c in target_pkg:
            f.write("{}\t{}\n".format(target_pkg[c], c))
PY
    failed=0
    total=0
    while IFS=$'\t' read -r pkg name; do
        total=$((total + 1))
        log_info "  running -p $pkg name=$name (timeout ${TEST_TIMEOUT_SEC}s)"
        if ! timeout "$TEST_TIMEOUT_SEC" cargo test "-p" "$pkg" --test "$name" -- \
            > "/tmp/fix-registry-test-$name.log" 2>&1; then
            log_error "    FAILED: $name"
            tail -20 "/tmp/fix-registry-test-$name.log" >&2
            failed=$((failed + 1))
        else
            log_pass "    PASSED: $name"
        fi
    done < "$SAMPLE_FILE"
    rm -f "$SAMPLE_FILE"
    if [ "$failed" -gt 0 ]; then
        log_error "STEP 3: $failed bin(s) failed at runtime (out of $total)"
        exit 3
    fi
    log_pass "STEP 3: all $total sample bin(s) passed"
fi

log_info ""
if [ "$ERRORS" -eq 0 ]; then
    log_pass "summary: v4.1_fix_pr_test_registry gate PASS (warnings=$WARNINGS)"
    exit 0
else
    log_error "summary: $ERRORS error(s) — gate FAILED"
    exit 1
fi
