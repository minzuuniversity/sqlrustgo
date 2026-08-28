#!/bin/bash
# scripts/generate_tpch_data.sh
#
# Generate TPC-H .tbl benchmark data for sqlrustgo E2E tests.
#
# Backends: tpch_data_gen (project's built-in, default) or dbgen (official).
# Generated files are .gitignored — committed fixtures in tests/data/tpch-sf001/
# and tpch-sf01/ are LFS-tracked snapshots; regenerate only if you need a
# different SF.
#
# Usage:
#   scripts/generate_tpch_data.sh --sf 0.001
#   scripts/generate_tpch_data.sh --sf 0.01 --output tests/data/tpch-sf01
#   scripts/generate_tpch_data.sh --sf 0.1 --output /tmp/tpch-sf1 --backend dbgen
#   scripts/generate_tpch_data.sh --sf 1 --backend dbgen
#   scripts/generate_tpch_data.sh --sf 0.001 --check
#
# --check mode: verify existing data matches expected row counts; exit non-zero
# on mismatch.
#
# Reference: docs/releases/v3.9.0/TPCH_E2E_TESTING.md

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

SF=""
OUTPUT_DIR=""
BACKEND="tpch_data_gen"
CHECK_ONLY=false

expected_rows_for() {
    local table="$1"
    local sf="$2"
    case "$table:$sf" in
        region:0.001)    echo "5" ;;
        region:0.01)     echo "5" ;;
        region:0.1)      echo "5" ;;
        region:1)        echo "5" ;;
        nation:0.001)    echo "25" ;;
        nation:0.01)     echo "25" ;;
        nation:0.1)      echo "25" ;;
        nation:1)        echo "25" ;;
        supplier:0.001)  echo "10" ;;
        supplier:0.01)   echo "100" ;;
        supplier:0.1)    echo "1000" ;;
        supplier:1)      echo "10000" ;;
        customer:0.001)  echo "15" ;;
        customer:0.01)   echo "150" ;;
        customer:0.1)    echo "15000" ;;
        customer:1)      echo "150000" ;;
        part:0.001)      echo "20" ;;
        part:0.01)       echo "200" ;;
        part:0.1)        echo "20000" ;;
        part:1)          echo "200000" ;;
        partsupp:0.001)  echo "80" ;;
        partsupp:0.01)   echo "800" ;;
        partsupp:0.1)    echo "8000" ;;
        partsupp:1)      echo "800000" ;;
        orders:0.001)    echo "150" ;;
        orders:0.01)     echo "1500" ;;
        orders:0.1)      echo "150000" ;;
        orders:1)        echo "1500000" ;;
        lineitem:0.001)  echo "501" ;;
        lineitem:0.01)   echo "5995" ;;
        lineitem:0.1)    echo "59986" ;;
        lineitem:1)      echo "6001215" ;;
        *)               echo ""; return 1 ;;
    esac
}

usage() {
    sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --sf)        SF="$2"; shift 2 ;;
        --output)    OUTPUT_DIR="$2"; shift 2 ;;
        --backend)   BACKEND="$2"; shift 2 ;;
        --check)     CHECK_ONLY=true; shift ;;
        --help|-h)   usage ;;
        *)           echo "[ERROR] unknown arg: $1" >&2; usage ;;
    esac
done

if [[ -z "$SF" ]]; then
    echo "[ERROR] --sf is required (0.001, 0.01, 0.1, 1)" >&2
    usage
fi

case "$SF" in
    .001|0.001) SF="0.001" ;;
    .01|0.01)   SF="0.01"  ;;
    .1|0.1)     SF="0.1"   ;;
    1|1.0|1.00) SF="1"     ;;
    *)          echo "[ERROR] unsupported SF: $SF (use 0.001, 0.01, 0.1, 1)" >&2; exit 1 ;;
esac

if [[ -z "$OUTPUT_DIR" ]]; then
    case "$SF" in
        0.001) OUTPUT_DIR="$PROJECT_ROOT/tests/data/tpch-sf001" ;;
        0.01)  OUTPUT_DIR="$PROJECT_ROOT/tests/data/tpch-sf01"  ;;
        0.1)   OUTPUT_DIR="/tmp/tpch-sf1"  ;;
        1)     OUTPUT_DIR="/home/openclaw/tpch_baseline/sf1"  ;;
    esac
fi

verify_rows() {
    local dir="$1"
    local mismatches=0

    for table in region nation supplier customer part partsupp orders lineitem; do
        local file="$dir/${table}.tbl"
        if [[ ! -f "$file" ]]; then
            echo "  [FAIL] $file: missing"
            mismatches=$((mismatches + 1))
            continue
        fi

        local actual
        actual=$(wc -l < "$file" | tr -d ' ')

        local expected
        expected=$(expected_rows_for "$table" "$SF")
        if [[ "$actual" == "$expected" ]]; then
            printf "  [OK]   %-10s %6s rows\n" "$table.tbl" "$actual"
        else
            printf "  [FAIL] %-10s %6s rows (expected %s)\n" "$table.tbl" "$actual" "$expected"
            mismatches=$((mismatches + 1))
        fi
    done

    if [[ $mismatches -gt 0 ]]; then
        echo "[FAIL] $mismatches table(s) have wrong row counts in $dir"
        return 1
    fi
    echo "[OK] all 8 tables in $dir match expected row counts"
    return 0
}

if $CHECK_ONLY; then
    echo "=== Verifying TPC-H data in $OUTPUT_DIR (SF=$SF) ==="
    verify_rows "$OUTPUT_DIR"
    exit $?
fi

echo "=== Generating TPC-H SF=$SF data ==="
echo "Backend:  $BACKEND"
echo "Output:   $OUTPUT_DIR"
echo

mkdir -p "$OUTPUT_DIR"

case "$BACKEND" in
    tpch_data_gen)
        # The `tpch_data_gen` example lives in `crates/bench/` whose
        # package name is `sqlrustgo-bench`. The workspace default
        # package does NOT contain this example, so `cargo build/example`
        # without `-p sqlrustgo-bench` fails with:
        #   "no example target named `tpch_data_gen` in default-run
        #    packages"
        # which silently aborts generation (Issue #4548). Pinning the
        # package via `-p` is required for every cargo invocation that
        # touches bench-only examples.
        echo "[1/3] Building tpch_data_gen (release)..."
        cargo build --release -p sqlrustgo-bench --example tpch_data_gen

        echo "[2/3] Running tpch_data_gen --sf $SF --output $OUTPUT_DIR"
        cargo run --release -p sqlrustgo-bench --example tpch_data_gen -- \
            --sf "$SF" \
            --output "$OUTPUT_DIR"
        ;;

    dbgen)
        DBGEN_DIR="${DBGEN_DIR:-$HOME/work/tpch-dbgen}"
        if [[ ! -x "$DBGEN_DIR/dbgen" ]]; then
            echo "[ERROR] dbgen not found at $DBGEN_DIR/dbgen" >&2
            echo "        Clone + build: git clone https://github.com/electrum/tpch-dbgen.git" >&2
            echo "        Set DBGEN_DIR env var if installed elsewhere" >&2
            exit 1
        fi

        echo "[1/3] Generating via dbgen -s $SF..."
        (cd "$DBGEN_DIR" && ./dbgen -s "$SF" -f)

        echo "[2/3] Moving .tbl files to $OUTPUT_DIR..."
        for t in region nation supplier customer part partsupp orders lineitem; do
            if [[ -f "$DBGEN_DIR/$t.tbl" ]]; then
                cp -f "$DBGEN_DIR/$t.tbl" "$OUTPUT_DIR/$t.tbl"
            else
                echo "[ERROR] dbgen did not produce $t.tbl" >&2
                exit 1
            fi
        done

        echo "[3/3] Normalizing line endings (CRLF -> LF if any)..."
        for f in "$OUTPUT_DIR"/*.tbl; do
            if file "$f" | grep -q CRLF; then
                sed -i.bak 's/\r$//' "$f" && rm -f "$f.bak"
            fi
        done
        ;;

    *)
        echo "[ERROR] unknown backend: $BACKEND (use tpch_data_gen or dbgen)" >&2
        exit 1
        ;;
esac

echo
echo "[3/3] Verifying row counts..."
verify_rows "$OUTPUT_DIR"

echo
echo "[DONE] TPC-H SF=$SF data ready at $OUTPUT_DIR"
echo "       To use in tests, ensure the path matches tests/common/tpch_wire_harness.rs constants:"
echo "         SF001_DIR = \"tests/data/tpch-sf001\""
echo "         SF01_DIR  = \"tests/data/tpch-sf01\""
echo "         SF1_DIR   = \"/home/openclaw/tpch_baseline/sf1\""