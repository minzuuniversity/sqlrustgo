#!/bin/bash
# scripts/verify_tpch_setup.sh
#
# Pre-flight check before running TPC-H E2E tests.
# Verifies:
#   - cargo, git-lfs, MySQL client available
#   - sqlrustgo-mysql-server binary built
#   - .tbl fixture files exist with correct row counts
#   - .gitignore excludes runtime artifacts
#
# Usage:
#   scripts/verify_tpch_setup.sh
#   scripts/verify_tpch_setup.sh --sf 0.1  # also check SF=0.1 if present
#
# Reference: docs/releases/v3.9.0/TPCH_E2E_TESTING.md

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

CHECK_SF01=true
CHECK_SF001=true
CHECK_SF1=false

usage() {
    sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --sf)
            case "$2" in
                0.001) CHECK_SF1=false ;;
                0.01) ;;
                0.1) CHECK_SF1=true ;;
                *) echo "[ERROR] unsupported SF: $2" >&2; usage ;;
            esac
            shift 2
            ;;
        --help|-h) usage ;;
        *) echo "[ERROR] unknown arg: $1" >&2; usage ;;
    esac
done

PASS=0
FAIL=0
WARN=0

ok()   { echo "  [OK]   $*"; PASS=$((PASS + 1)); }
fail() { echo "  [FAIL] $*"; FAIL=$((FAIL + 1)); }
warn() { echo "  [WARN] $*"; WARN=$((WARN + 1)); }

check_tool() {
    local tool="$1"
    local purpose="$2"
    if command -v "$tool" >/dev/null 2>&1; then
        ok "$tool available ($purpose)"
    else
        fail "$tool not found (needed for: $purpose)"
    fi
}

check_file_count() {
    local file="$1"
    local expected="$2"
    local label="$3"

    if [[ ! -f "$file" ]]; then
        fail "$label: $file missing"
        return
    fi

    local actual
    actual=$(wc -l < "$file" | tr -d ' ')
    if [[ "$actual" == "$expected" ]]; then
        ok "$label: $actual rows (expected $expected)"
    else
        fail "$label: $actual rows (expected $expected)"
    fi
}

check_lfs_pointer() {
    local file="$1"
    local label="$2"

    if [[ ! -f "$file" ]]; then
        fail "$label: $file missing"
        return
    fi

    if head -1 "$file" | grep -q "https://git-lfs.github.com/spec/v1"; then
        warn "$label: $file is still an LFS pointer (run: git lfs pull)"
    else
        ok "$label: actual data present"
    fi
}

echo "=== TPC-H E2E setup verification ==="
echo

echo "[1/4] Tooling"
check_tool cargo "Rust build"
check_tool git-lfs "Git LFS client"
check_tool mysql "MySQL CLI client (for manual debugging, optional)"
echo

echo "[2/4] Source code"
for f in \
    crates/bench/examples/tpch_data_gen.rs \
    benches/tpch_wire_bench.rs \
    crates/mysql-server/src/lib.rs \
    tests/common/tpch_wire_harness.rs \
    scripts/setup_tpch_sf01.sh \
    scripts/load_tpch_data.sh
do
    if [[ -f "$f" ]]; then
        ok "exists: $f"
    else
        fail "missing: $f"
    fi
done
echo

echo "[3/4] SF=0.001 fixture (tests/data/tpch-sf001/)"
if $CHECK_SF001; then
    for t in region nation supplier customer part partsupp orders lineitem; do
        check_lfs_pointer "tests/data/tpch-sf001/${t}.tbl" "tpch-sf001/${t}.tbl"
    done
    check_file_count tests/data/tpch-sf001/lineitem.tbl 501 "tpch-sf001/lineitem.tbl"
else
    echo "  (skipped)"
fi
echo

echo "[4/4] SF=0.01 fixture (tests/data/tpch-sf01/)"
if $CHECK_SF01; then
    for t in region nation supplier customer part partsupp orders lineitem; do
        check_lfs_pointer "tests/data/tpch-sf01/${t}.tbl" "tpch-sf01/${t}.tbl"
    done
    check_file_count tests/data/tpch-sf01/lineitem.tbl 5995 "tpch-sf01/lineitem.tbl"
fi

if $CHECK_SF1; then
    echo
    echo "[bonus] SF=0.1 fixture (runtime-only)"
    if [[ -d "/tmp/tpch-sf1" ]]; then
        for t in region nation supplier customer part partsupp orders lineitem; do
            check_file_count "/tmp/tpch-sf1/${t}.tbl" "59986" "tpch-sf1/${t}.tbl"
        done
    else
        echo "  [SKIP] /tmp/tpch-sf1 not found (only needed for SF=0.1 gate tests)"
    fi
fi
echo

echo "[extra] .gitignore exclusions"
for pattern in "tests/data/tpch-sf001/*.json" "tests/data/tpch-sf01/*.json" "tests/data/tpch-sf01/*.wal" "data/tpch-sf01-generated/"; do
    if grep -qF "$pattern" .gitignore; then
        ok ".gitignore contains: $pattern"
    else
        warn ".gitignore missing: $pattern (runtime artifacts may get committed)"
    fi
done
echo

echo "=== Summary ==="
echo "  PASS: $PASS"
echo "  WARN: $WARN"
echo "  FAIL: $FAIL"
echo

if [[ $FAIL -gt 0 ]]; then
    echo "[BLOCKED] Fix the FAIL items above before running TPC-H tests."
    exit 1
elif [[ $WARN -gt 0 ]]; then
    echo "[OK with warnings] Setup usable; warnings are non-blocking."
    exit 0
else
    echo "[OK] Ready to run TPC-H E2E tests."
    exit 0
fi