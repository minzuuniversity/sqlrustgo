#!/usr/bin/env bash
# V312-57 BustubX-EDU sqlite-style CLI gate.
# Runs the manifest-driven fixtures against the `sqlrustgo` binary and
# verifies exit codes + golden output (or stable stderr prefix).
# Usage: bash scripts/gate/check_bustubx_edu_cli_v312.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="$REPO_ROOT/tests/compat/bustubx_edu_sqlite_cli/manifest.yml"
FIXTURES="$REPO_ROOT/tests/compat/bustubx_edu_sqlite_cli"
BIN="$REPO_ROOT/target/debug/sqlrustgo"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

PASS=0
FAIL=0
FAILED_CASES=()

# parse_yaml_list: naive YAML list-of-maps parser tailored to this manifest.
# Emits "case_id|field=value" lines. Python is used for robustness.
parse_manifest() {
  python3 - "$MANIFEST" << 'PYEOF'
import sys, yaml
with open(sys.argv[1]) as f:
    data = yaml.safe_load(f)
for case in data.get("cases", []):
    print(case.get("case_id", "?"))
    for k, v in case.items():
        if k != "case_id":
            print(f"  {k}={v}")
PYEOF
}

if ! command -v python3 >/dev/null 2>&1; then
  echo "FAIL: python3 is required to parse $MANIFEST"
  exit 1
fi
if ! python3 -c "import yaml" 2>/dev/null; then
  echo "FAIL: python3 yaml module is required"
  exit 1
fi

if [ ! -x "$BIN" ]; then
  echo "FAIL: $BIN not found; run: cargo build -p sqlrustgo-cli --all-features"
  exit 1
fi

# Parse manifest into case blocks.
CASES_DIR="$WORK/cases"
mkdir -p "$CASES_DIR"
parse_manifest > "$WORK/manifest.txt"

python3 - "$WORK/manifest.txt" "$CASES_DIR" << 'PYEOF'
import sys, os
lines = open(sys.argv[1]).read().splitlines()
outdir = sys.argv[2]
cur = None
for line in lines:
    if not line.startswith("  "):
        if cur is not None:
            cur.close()
        cur = open(os.path.join(outdir, line.strip()), "w")
    else:
        cur.write(line.strip() + "\n")
if cur is not None:
    cur.close()
PYEOF

run_case() {
  local case_id="$1"
  local file="$CASES_DIR/$case_id"
  local expected_exit
  local oracle_mode
  local sql_file=""
  local stdin_file=""
  local golden_file=""
  local prefix=""
  local args=""
  local db="$WORK/$case_id.db"

  expected_exit=$(grep '^expected_exit_code=' "$file" | cut -d= -f2)
  oracle_mode=$(grep '^oracle_mode=' "$file" | cut -d= -f2)
  sql_file=$(grep '^sql_file=' "$file" | cut -d= -f2-)
  stdin_file=$(grep '^stdin_file=' "$file" | cut -d= -f2- || true)
  golden_file=$(grep '^golden_file=' "$file" | cut -d= -f2- || true)
  prefix=$(grep '^expected_stderr_prefix=' "$file" | cut -d= -f2- || true)
  depends_on=$(grep '^depends_on=' "$file" | cut -d= -f2- || true)
  if grep -qi '^use_arg_continue_on_error=true' "$file"; then
    args="--continue-on-error"
  fi

  rm -rf "$db"
  if [ -n "$depends_on" ]; then
    db="$WORK/$depends_on.db"
  fi
  local stdout_file="$WORK/$case_id.out"
  local stderr_file="$WORK/$case_id.err"
  set +e
  if [ -n "$stdin_file" ]; then
    "$BIN" "$db" $args < "$FIXTURES/$stdin_file" > "$stdout_file" 2> "$stderr_file"
  else
    "$BIN" "$db" $args < "$FIXTURES/$sql_file" > "$stdout_file" 2> "$stderr_file"
  fi
  local actual_exit=$?
  set -e

  local ok=1
  if [ "$actual_exit" != "$expected_exit" ]; then
    echo "  [exit] expected=$expected_exit actual=$actual_exit"
    ok=0
  fi

  case "$oracle_mode" in
    golden)
      if ! diff -u "$FIXTURES/$golden_file" "$stdout_file" > "$WORK/diff.out" 2>&1; then
        echo "  [golden] diff vs $golden_file:"
        sed 's/^/    /' "$WORK/diff.out"
        ok=0
      fi
      ;;
    stderr_prefix)
      if [ -n "$prefix" ] && ! grep -q "Error: $prefix" "$stderr_file"; then
        echo "  [prefix] stderr does not contain 'Error: $prefix':"
        sed 's/^/    /' "$stderr_file"
        ok=0
      fi
      ;;
    json_schema)
      if ! python3 -c "
import json, sys
data = json.load(open('$stdout_file'))
assert 'columns' in data and 'rows' in data
assert isinstance(data['columns'], list)
assert isinstance(data['rows'], list)
" 2> "$WORK/json.err"; then
        echo "  [json] invalid json_schema output:"
        sed 's/^/    /' "$WORK/json.err"
        ok=0
      fi
      ;;
    *)
      echo "  [oracle] unknown oracle_mode: $oracle_mode"
      ok=0
      ;;
  esac

  if [ "$ok" -eq 1 ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    FAILED_CASES+=("$case_id")
  fi
}

# Ordered cases (preserve manifest order).
CASES=$(grep -v '^  ' "$WORK/manifest.txt")
for case_id in $CASES; do
  echo "RUN: $case_id"
  run_case "$case_id"
done

echo
echo "V312-57 bustubx_edu_cli gate: PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -gt 0 ]; then
  echo "Failed cases: ${FAILED_CASES[*]}"
  exit 1
fi
exit 0