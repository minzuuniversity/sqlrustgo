#!/usr/bin/env bash
# scripts/tpch_sf1_baseline.sh
#
# TPC-H SF=1.0 baseline (in-process surface).
#
# This script is a developer convenience wrapper around the
# in-process test at tests/tpch_sf1_22_vs_3engines_test.rs. It does
# NOT shell out to an external `mysql` or `sqlite3` CLI. The 22
# queries are run by the in-process `MySqlTestClient` against the
# sqlrustgo ephemeral server, and the report is written by the
# test itself.
#
# The cross-engine comparison (sqlrustgo vs MariaDB vs SQLite) is
# intentionally out of scope: it requires a working external
# mysql client, which is tracked in issue #3474. This script
# only delivers the sqlrustgo baseline.
#
# Usage:
#   bash scripts/tpch_sf1_baseline.sh            # run the baseline
#   bash scripts/tpch_sf1_baseline.sh --dry-run  # print plan only
#   bash scripts/tpch_sf1_baseline.sh --sf1-dir /path/to/sf1
#                                            # override fixture path
#
# Exit codes:
#   0  baseline captured (or --dry-run)
#   1  fixture / source / dependency missing
#   2  test failed (some query did not return >= 1 row)
#
# Reference: openspec/changes/2026-06-18-tpch-sf1-baseline (Issue #3423)
# Spec:      design D1 (in-process only), D4 (sqlrustgo-only report)

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

SF1_DIR="${SF1_DIR:-${TPCH_SF1_DIR:-/tmp/tpch-sf1}}"
REPORT_PATH="$PROJECT_ROOT/docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md"
LOADER_TIMEOUT_S="${LOADER_TIMEOUT_S:-1800}"   # 30 min per query (Q9 headroom)
TEST_BUDGET_S="${TEST_BUDGET_S:-1800}"          # 30 min total wall clock

DRY_RUN=false

usage() {
    sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)    DRY_RUN=true; shift ;;
        --sf1-dir)    SF1_DIR="$2"; shift 2 ;;
        --help|-h)    usage ;;
        *)            echo "[ERROR] unknown arg: $1" >&2; usage ;;
    esac
done

# ---------------------------------------------------------------------
# Step 1: Validate the SF=1.0 fixture is present
#
# In --dry-run mode, a missing or row-count-mismatched fixture is
# reported but does NOT cause a non-zero exit: spec 8.4 requires
# `bash scripts/tpch_sf1_baseline.sh --dry-run` to exit 0 so the
# plan is inspectable on machines that have not yet produced the
# ~1.1 GB fixture. The fixture is only fatal in step 5 (real run).
# ---------------------------------------------------------------------
echo "=== Step 1: validate fixture at $SF1_DIR ==="
EXPECTED_ROWS=(
    "region:5"
    "nation:25"
    "supplier:10000"
    "customer:150000"
    "part:200000"
    "partsupp:800000"
    "orders:1500000"
    "lineitem:6001215"
)
# Issue #4549: allow small row-count tolerance so the in-process
# tpch_data_gen (sqlrustgo-bench) is acceptable. The standard
# dbgen produces exactly 6001215 lineitem rows; the in-process
# generator produces 6000000 (off by 1215 = 0.020%) due to a
# supplier-sampling rounding difference. Q17 (single aggregated
# value) is invariant to such small row-count differences.
# Override with TPCH_SF1_TOLERANCE=0 for exact match (CI / strict).
TPCH_SF1_TOLERANCE="${TPCH_SF1_TOLERANCE:-0.001}"  # default 0.1%
FIXTURE_OK=true
for spec in "${EXPECTED_ROWS[@]}"; do
    tbl="${spec%:*}"
    expected="${spec#*:}"
    f="$SF1_DIR/$tbl.tbl"
    if [[ ! -f "$f" ]]; then
        echo "  [FAIL] $f missing"
        FIXTURE_OK=false
        continue
    fi
    actual=$(wc -l < "$f" | tr -d ' ')
    # Tolerance check (Issue #4549): accept if within TPCH_SF1_TOLERANCE
    # of expected. Use awk for floating-point comparison.
    if awk -v a="$actual" -v e="$expected" -v t="$TPCH_SF1_TOLERANCE" \
        'BEGIN { exit (a == e || (e > 0 && (a - e) / e < t && (a - e) / e > -t)) ? 0 : 1 }'; then
        printf "  [OK]   %-10s %9s rows\n" "$tbl.tbl" "$actual"
    else
        printf "  [FAIL] %-10s %9s rows (expected %s, tolerance %s)\n" "$tbl.tbl" "$actual" "$expected" "$TPCH_SF1_TOLERANCE"
        FIXTURE_OK=false
    fi
done
if ! $FIXTURE_OK; then
    echo "[WARN] fixture incomplete; --dry-run will still print the plan," >&2
    echo "       but a real run will fail until the fixture is generated:" >&2
    echo "         /home/openclaw/tpch-dbgen-master/dbgen -s 1 -f" >&2
    echo "         mkdir -p $SF1_DIR" >&2
    echo "         mv /home/openclaw/tpch-dbgen-master/*.tbl $SF1_DIR/" >&2
    echo "       (or bash scripts/generate_tpch_data.sh --sf 1 --backend dbgen)" >&2
fi

# ---------------------------------------------------------------------
# Step 2: Validate the test source exists
# ---------------------------------------------------------------------
TEST_FILE="$PROJECT_ROOT/tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs"
if [[ ! -f "$TEST_FILE" ]]; then
    echo "[ERROR] test file not found: $TEST_FILE" >&2
    exit 1
fi
if ! grep -q 'MySqlTestClient' "$TEST_FILE"; then
    echo "[ERROR] $TEST_FILE does not use MySqlTestClient" >&2
    echo "        The in-process baseline requires the MySqlTestClient surface." >&2
    exit 1
fi
if grep -Eq '(^|[^A-Za-z_])mysql[ "]|/mysql"' "$TEST_FILE"; then
    echo "[ERROR] $TEST_FILE contains an external mysql CLI call" >&2
    echo "        Spec D1 forbids this; use the in-process MySqlTestClient." >&2
    exit 1
fi
echo "  [OK] $TEST_FILE (in-process surface, no external mysql CLI)"

# ---------------------------------------------------------------------
# Step 3: Validate the queries directory
# ---------------------------------------------------------------------
QUERIES_DIR="$PROJECT_ROOT/queries"
if [[ ! -d "$QUERIES_DIR" ]]; then
    echo "[ERROR] queries directory not found: $QUERIES_DIR" >&2
    exit 1
fi
MISSING_QS=()
for n in $(seq 1 22); do
    if [[ ! -f "$QUERIES_DIR/q$n.sql" ]]; then
        MISSING_QS+=("q$n.sql")
    fi
done
if (( ${#MISSING_QS[@]} > 0 )); then
    echo "[ERROR] missing query files: ${MISSING_QS[*]}" >&2
    exit 1
fi
echo "  [OK] $QUERIES_DIR (22/22 query files present)"

# ---------------------------------------------------------------------
# Step 4: Print plan
# ---------------------------------------------------------------------
echo
echo "=== Step 4: plan ==="
echo "  fixture:          $SF1_DIR (verified)"
echo "  test file:        $TEST_FILE (in-process)"
echo "  per-query budget: ${LOADER_TIMEOUT_S}s"
echo "  test budget:      ${TEST_BUDGET_S}s"
echo "  report:           $REPORT_PATH (written by the test)"

if $DRY_RUN; then
    echo
    echo "[DRY-RUN] no work performed."
    exit 0
fi

# ---------------------------------------------------------------------
# Step 5: Run the in-process test
#
# In real-run mode, the fixture gate from step 1 is now fatal. The
# test is `#[ignore]`d when the fixture is absent, which would make
# cargo test report 0 failures (the test is "skipped") and produce
# a confusing exit 0 with no report. Reject that explicitly here.
# ---------------------------------------------------------------------
if ! $FIXTURE_OK; then
    echo "[ERROR] fixture incomplete at $SF1_DIR; cannot run baseline." >&2
    echo "        Generate it with:" >&2
    echo "          /home/openclaw/tpch-dbgen-master/dbgen -s 1 -f" >&2
    echo "          mkdir -p $SF1_DIR" >&2
    echo "          mv /home/openclaw/tpch-dbgen-master/*.tbl $SF1_DIR/" >&2
    echo "        (or bash scripts/generate_tpch_data.sh --sf 1 --backend dbgen)" >&2
    exit 1
fi
echo
echo "=== Step 5: run cargo test --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture ==="

# ---------------------------------------------------------------------
# Step 5: Run the in-process test
# ---------------------------------------------------------------------
echo "=== Step 5: run cargo test --test tpch_sf1_22_vs_3engines_test -- --include-ignored --nocapture ==="
mkdir -p "$(dirname "$REPORT_PATH")"
export TPCH_SF1_DIR="$SF1_DIR"
# `--include-ignored` is required: the test is `#[ignore]`d when the fixture
# is absent. We have just verified the fixture is present, so we
# know the test will not be skipped. Note: libtest's `--ignored` only
# runs ignored tests in *isolation* (dropping the non-ignored ones);
# we want the opposite — run BOTH — so we use `--include-ignored`.
# `set -eo pipefail` requires the explicit rc capture.
set +e
timeout "$TEST_BUDGET_S" cargo test \
    --test tpch_sf1_22_vs_3engines_test \
    --all-features \
    -- --include-ignored --nocapture
rc=$?
set -e

if [[ $rc -ne 0 ]]; then
    echo
    echo "[FAIL] cargo test exited with rc=$rc"
    case $rc in
        124) echo "       (test budget of ${TEST_BUDGET_S}s exceeded)" ;;
    esac
    if [[ -f "$REPORT_PATH" ]]; then
        echo "       partial report: $REPORT_PATH"
    fi
    exit 2
fi

# ---------------------------------------------------------------------
# Step 6: Verify the report was written
# ---------------------------------------------------------------------
if [[ ! -f "$REPORT_PATH" ]]; then
    echo "[FAIL] test passed but report is missing at $REPORT_PATH" >&2
    exit 2
fi

echo
echo "[DONE] baseline captured. Report: $REPORT_PATH"
exit 0
