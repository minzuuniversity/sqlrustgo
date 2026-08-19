#!/usr/bin/env bash
# V312-57 bustubx-edu sqlite3-like CLI gate (week01-04)
#
# Verifies:
# - cargo build -p sqlrustgo-cli --all-features succeeds
# - `sqlrustgo-cli sqlite --help` succeeds
# - All week01-04 fixtures match their golden output files
#
# Exits 0 on all-pass, non-zero on any failure.
#
# Usage: SQLRUSTGO_BIN=/path/to/sqlrustgo-cli bash scripts/gate/check_bustubx_edu_cli_v312.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

OUT_DIR="${OUT_DIR:-/tmp/bustubx_edu_cli_v3.12.0}"
mkdir -p "$OUT_DIR"

PASS=0
FAIL=0

echo "=== V312-57 bustubx-edu sqlite3-like CLI gate ==="

# Step 1: build
echo "[BUILD] cargo build -p sqlrustgo-cli --all-features"
if cargo build -p sqlrustgo-cli --bin sqlrustgo-cli --all-features >/dev/null 2>&1; then
    PASS=$((PASS + 1))
else
    echo "  [FAIL] build failed"
    FAIL=$((FAIL + 1))
fi

# Step 2: --help
echo "[HELP] sqlrustgo-cli sqlite --help"
BIN="${SQLRUSTGO_BIN:-$REPO_ROOT/target/debug/sqlrustgo-cli}"
if [ ! -x "$BIN" ]; then
    echo "  [FAIL] binary not found: $BIN"
    FAIL=$((FAIL + 1))
else
    if "$BIN" sqlite --help > "$OUT_DIR/01_help.out" 2>&1; then
        if diff -q "$OUT_DIR/01_help.out" "$REPO_ROOT/tests/compat/bustubx_edu_sqlite_cli/golden/01_help.out.txt" >/dev/null; then
            PASS=$((PASS + 1))
        else
            echo "  [DIFF] 01_help stdout differs from golden"
            FAIL=$((FAIL + 1))
        fi
    else
        echo "  [FAIL] --help exit non-zero"
        FAIL=$((FAIL + 1))
    fi

    # Step 3: each week0[1-4] fixture
    FIX_DIR="$REPO_ROOT/tests/compat/bustubx_edu_sqlite_cli"
    export SQLRUSTGO_BIN="$BIN"
    for wk in 01 02 03 04; do
        for fixture in "$FIX_DIR"/week${wk}/*.sh; do
            name="$(basename "$fixture" .sh)"
            golden="$FIX_DIR/golden/${name}.out.txt"
            echo "[WEEK$wk] $name"
            if bash "$fixture" > "$OUT_DIR/$name.out" 2> "$OUT_DIR/$name.err"; then
                if [ -f "$golden" ] && diff -q "$OUT_DIR/$name.out" "$golden" >/dev/null; then
                    PASS=$((PASS + 1))
                else
                    echo "  [DIFF] $name stdout differs from golden (or no golden exists)"
                    diff "$OUT_DIR/$name.out" "$golden" 2>&1 | head -10
                    FAIL=$((FAIL + 1))
                fi
            else
                echo "  [EXIT] $name exit non-zero"
                FAIL=$((FAIL + 1))
            fi
        done
    done
fi

echo "==="
echo "PASS: $PASS    FAIL: $FAIL"
[ "$FAIL" -eq 0 ]
