#!/bin/bash
# V312-35: per-crate coverage measurement (L1_8)
# Threshold: L1_8 average line coverage >= 80%
# Exit 0 = PASS, Exit 1 = FAIL
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# The coverage-percentage extraction rule lives in common.sh — one rule, one
# implementation (the 10-principles R5 rule uses the same one).
# shellcheck source=scripts/gate/common.sh
source "$SCRIPT_DIR/common.sh"

# L1_8 = 8 core crates per GATE_CONDITIONS.md
L1_8_CRATES=(
    "sqlrustgo-parser"
    "sqlrustgo-planner"
    "sqlrustgo-executor"
    "sqlrustgo-transaction"
    "sqlrustgo-storage"
    "sqlrustgo-catalog"
    "sqlrustgo-optimizer"
    "sqlrustgo-types"
)

THRESHOLD=80.0
TOTAL=0
COUNT=0
FAILED_CRATES=()

# Per-crate feature requirements, DERIVED from the crate's own sources.
#
# A crate whose integration tests reference a feature-gated module needs
# that feature ENABLED when the tests are compiled. Integration tests link
# the library WITHOUT `cfg(test)`, so a `#[cfg(any(test, feature = "..."))]`
# module simply does not exist for them.
#
# sqlrustgo-storage gates two such items:
#     crates/storage/src/lib.rs:28  #[cfg(any(test, feature = "fault-injection"))]
#                                  pub mod fault_injection;
#     crates/storage/src/lib.rs:92  #[cfg(feature = "bin_storage_default")]
#                                  pub use binary_storage_v2::BinaryTableStorageV2;
# Without the features, `crates/storage/tests/fault_coordination.rs`,
# `crates/storage/tests/a06_failpoint_reachability.rs` and
# `crates/storage/tests/no_namespace_db_refusal_5266.rs` fail to compile
# with
#     error[E0432]: unresolved import `sqlrustgo_storage::fault_injection`
# and the crate measures ZERO coverage.
#
# That zero is not merely a missing number: this script's own comment
# (below) records that a crate which fails to measure RAISES the reported
# average, so the L1_8 figure was being computed over a silently reduced
# sample. The number was not just missing — it was misleading.
#
# The features are OFF in production (crates/storage/Cargo.toml:30-33),
# so enabling them for the coverage measurement changes nothing about the
# shipped artifact; it only lets the crash-test hooks compile.
#
# The list is DERIVED, not hand-maintained: see derive_crate_features
# below. A hand-maintained list goes stale the moment a new
# `#[cfg(feature = "...")]` module is added, and the failure mode is
# SILENT — the gate measures a reduced sample and reports an inflated
# average.

# Derive the features a crate's integration tests need, from the crate's
# own sources.
#
# A `#[cfg(feature = "X")]` is only a problem for an integration test if
# the guarded item is a MODULE-LEVEL public item — that is what a test
# can `use`. A feature that only gates methods inside an already-public
# module cannot produce an "unresolved import", so it is excluded.
#
# The derivation is printed, so a reader can see WHY a feature was passed
# rather than having to trust a list.
derive_crate_features() {
    local crate="$1"
    local crate_dir
    case "$crate" in
        sqlrustgo-*) crate_dir="crates/${crate#sqlrustgo-}" ;;
        *) crate_dir="" ;;
    esac
    # workspace member names are not always <dir>-<name>; find it
    if [ -z "$crate_dir" ] || [ ! -d "$crate_dir" ]; then
        crate_dir=$(find crates -maxdepth 1 -type d -name "*" 2>/dev/null \
            | while read -r d; do
                if grep -qx "name = \"$crate\"" "$d/Cargo.toml" 2>/dev/null; then
                    echo "$d"
                    break
                fi
            done)
    fi
    [ -z "$crate_dir" ] && return 0
    [ -d "$crate_dir" ] || return 0

    python3 - "$crate_dir" <<'PYEOF'
import os
import re
import sys

crate_dir = sys.argv[1]
found = []

for root in (os.path.join(crate_dir, "src"), os.path.join(crate_dir, "tests")):
    if not os.path.isdir(root):
        continue
    for dirpath, _dirs, files in os.walk(root):
        for fn in files:
            if not fn.endswith(".rs"):
                continue
            path = os.path.join(dirpath, fn)
            try:
                lines = open(path, encoding="utf-8").read().split("\n")
            except OSError:
                continue
            for i, line in enumerate(lines):
                m = re.search(r"#\s*\[cfg\(([^\]]*)\)\]", line)
                if not m:
                    continue
                feats = re.findall(r'feature\s*=\s*"([^"]+)"', m.group(1))
                if not feats:
                    continue
                # A module-level public item is what a test can import.
                for j in range(i + 1, min(i + 4, len(lines))):
                    pm = re.match(r"\s*pub\s+(mod|use)\b", lines[j])
                    if pm:
                        for f in feats:
                            found.append((f, "%s:%d" % (path, i + 1)))
                        break

if found:
    print(" ".join(sorted(set(f for f, _ in found))))
    for f, ev in sorted(set(found)):
        print("      %s <- %s" % (f, ev), file=sys.stderr)
PYEOF
}

features_for_crate() {
    local crate="$1"
    local out
    out=$(derive_crate_features "$crate" 2>/dev/null)
    echo "$out" | tr -d '\n' | tr -s ' ' | sed 's/^ *//;s/ *$//'
}

# The crash-consistency tests in sqlrustgo-storage drive a real server
# process: `crates/storage/tests/fault_coordination.rs` locates the
# `sqlrustgo-mysql-server` binary at a machine-specific path, and
# `crates/storage/tests/crash_{commit_fsync,wal_index}_b1.rs` use the same
# helper. They also need the server to reach an exact fault point within
# a timeout.
#
# They are EXCLUDED from the coverage measurement:
#
#   * they are machine-dependent (a pre-built binary at a fixed path), and
#   * a failure in ANY of them makes `cargo llvm-cov` produce NO coverage
#     table for the whole crate, so the crate measures zero and the L1_8
#     average is computed over a silently reduced sample.
#
# The crash semantics themselves are covered by a dedicated gate —
# scripts/gate/check_g14_real_crash.sh — which is the right instrument for
# a "does the data survive SIGKILL" question. A line-coverage gate is not.
#
# The excluded FUNCTION NAMES are extracted from the excluded files at run
# time, so a new crash test is covered without touching this list. cargo
# cannot exclude a whole test TARGET, only whole packages, and libtest's
# `--skip` filters by function name — hence the extraction.
EXCLUDED_TEST_FILES=(
    "crates/storage/tests/crash_commit_fsync_b1.rs"
    "crates/storage/tests/crash_wal_index_b1.rs"
    "crates/storage/tests/fault_coordination.rs"
)

# Test files whose assertions are wall-clock bounds.
#
# A test that asserts "this operation takes less than N seconds" cannot
# produce a meaningful result under `cargo llvm-cov`: the instrumentation
# makes the code 2-10x slower, so the assertion fails for a reason that
# has nothing to do with the code under test — and the failure produces
# NO coverage table at all, which zeroes the whole crate's coverage and
# makes the L1_8 average misleading.
#
# Measured 2026-10-22: with the crash tests excluded but the performance
# test still running,
#     write_path_scaling_5181::report_single_row_delete_cost
#     -> "a single-row delete at 20000 rows costs 0.00226832s; that is
#         too slow for an OLTP point update"   (the bound is 0.002s)
# and the crate produced NO coverage table. With that file excluded as
# well, the crate measures 81.31%.
#
# Note this test also fails WITHOUT instrumentation on the base commit
# (0.00227s against the same 0.002s bound) — it is a pre-existing
# performance debt, tracked separately. What this exclusion fixes is that
# the debt must not also destroy the crate's coverage measurement.
#
# The whole FILE is excluded rather than individual functions: the wall
# clock is measured in a helper the test calls (e.g. `per_row_delete`),
# so the assertion is not visible inside the test function's own body.
PERF_BOUND_TEST_FILES=(
    "crates/storage/tests/write_path_scaling_5181.rs"
    "crates/storage/tests/e2e_binlog_replication_test.rs"
    "crates/storage/tests/b22_concurrent_flush_ab.rs"
)

excluded_test_names() {
    local file
    for file in "${EXCLUDED_TEST_FILES[@]}" "${PERF_BOUND_TEST_FILES[@]}"; do
        [ -f "$file" ] || continue
        python3 - "$file" <<'PYEOF'
import io
import re
import sys

path = sys.argv[1]
TEST_ATTR = re.compile(r"^\s*#\s*\[\s*(test|sqlrustgo_test)\s*\]")
FN = re.compile(r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)")
lines = io.open(path, encoding="utf-8").read().split("\n")
for i, line in enumerate(lines):
    if not TEST_ATTR.match(line):
        continue
    for j in range(i + 1, min(i + 4, len(lines))):
        m = FN.match(lines[j])
        if m:
            print(m.group(1))
            break
PYEOF
    done
}

for crate in "${L1_8_CRATES[@]}"; do
    echo "Measuring coverage for $crate ..."
    # `--no-fail-fast` so one failing test doesn't abort the whole measurement.
    # Exit status is still checked below — a crate that will not build or whose
    # tests will not run is a measurement failure, not a zero-coverage crate.
    #
    # The crate's required features are passed explicitly: a crate whose
    # integration tests need a feature-gated module cannot even COMPILE
    # without it (see the derivation above).
    crate_features="$(features_for_crate "$crate")"
    feature_args=()
    if [ -n "$crate_features" ]; then
        feature_args=(--features "$crate_features")
        echo "  features: $crate_features"
    fi
    # Environment-dependent tests are excluded from the coverage measurement.
    #
    # `--skip` is a LIBTEST argument, so it must go AFTER the `--` separator
    # (`cargo llvm-cov test [OPTIONS] [TESTNAME] [-- <args>...]`). Putting it
    # before the `--` makes cargo reject the whole invocation, which is how
    # the first attempt at this fix failed.
    #
    # `--test-threads=1` is NOT optional here. Under `cargo llvm-cov` the
    # code runs 2-10x slower, and the default parallel test execution then
    # has the threads competing for CPU: measurements of the same crates
    # came out different from run to run (sqlrustgo-planner measured
    # 88.91% and PASSED with --test-threads=1, and FAILED with the default
    # thread count on the same checkout). A coverage number that depends on
    # machine load is not a measurement.
    #
    # It also matches the repository's own rule: AGENTS.md §10 requires
    # concurrent tests to run with `--test-threads=1`.
    skip_args=()
    for t in $(excluded_test_names); do
        skip_args+=(--skip "$t")
    done
    if [ "${#skip_args[@]}" -gt 0 ]; then
        echo "  excluded (environment-dependent): $(( ${#skip_args[@]} / 2 )) test function(s)"
    fi
    set +e
    OUTPUT=$(cargo llvm-cov test -p "$crate" "${feature_args[@]}" --no-fail-fast -- --test-threads=1 "${skip_args[@]}" 2>&1)
    RC=$?
    set -e
    # `cargo llvm-cov` prints a coverage table whose TOTAL row carries FOUR
    # percentages (regions / functions / lines / branches). The previous
    # `grep -oE '[0-9]+\.[0-9]+%' | tail -1` took the last one — the BRANCHES
    # figure — and labelled it line coverage.
    #
    # The extraction rule now lives in scripts/gate/common.sh
    # (coverage_extract_total_percent) — the 10-principles R5 rule needs the
    # same rule, and one rule gets exactly one implementation.
    COVERAGE=$(echo "$OUTPUT" | coverage_extract_total_percent)
    if [ $RC -ne 0 ] || [ -z "$COVERAGE" ]; then
        # #4944 P1-01: the previous shape did `continue` here, which dropped the
        # crate from both the numerator and the denominator. A crate that fails
        # to measure therefore *raises* the reported average — the gate could
        # pass with fewer crates measured than the L1_8 set requires. Record the
        # failure and fail the gate at the end instead of averaging over a
        # silently reduced sample.
        echo "  FAIL: could not measure $crate (exit=$RC${COVERAGE:+, got $COVERAGE%})"
        FAILED_CRATES+=("$crate")
        continue
    fi

    echo "  $crate: ${COVERAGE}%"
    TOTAL=$(echo "$TOTAL + $COVERAGE" | bc)
    COUNT=$((COUNT + 1))
done

if [ ${#FAILED_CRATES[@]} -gt 0 ]; then
    echo "FAIL: ${#FAILED_CRATES[@]}/${#L1_8_CRATES[@]} crate(s) produced no coverage data: ${FAILED_CRATES[*]}" >&2
    echo "      The L1_8 average is only meaningful over the full crate set —" >&2
    echo "      a reduced sample silently inflates it." >&2
    exit 1
fi

if [ "$COUNT" -ne "${#L1_8_CRATES[@]}" ]; then
    echo "FAIL: measured $COUNT of ${#L1_8_CRATES[@]} L1_8 crates" >&2
    exit 1
fi

if [ "$COUNT" -eq 0 ]; then
    # Unreachable: the FAILED_CRATES / count-mismatch checks above already exit
    # unless all 8 crates measured. Kept as a guard against a future edit that
    # reorders the checks, since a division by COUNT=0 would otherwise be
    # silently wrong rather than loud.
    echo "FAIL: no crates produced coverage data" >&2
    exit 1
fi

AVG=$(echo "scale=2; $TOTAL / $COUNT" | bc)
echo "L1_8 average coverage: ${AVG}% (threshold: ${THRESHOLD}%)"

if (( $(echo "$AVG < $THRESHOLD" | bc -l) )); then
    echo "FAIL: L1_8 avg ${AVG}% < ${THRESHOLD}%" >&2
    exit 1
fi

exit 0
