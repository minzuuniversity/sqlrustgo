#!/usr/bin/env bash
# scripts/v312_48/check_fixture.sh — fixture readiness probe for V312-48.
set -euo pipefail
SF1="${TPCH_SF1_DIR:-/tmp/tpch-sf1}"
BIN="${TPCH_BINT_DIR:-/tmp/tpch-sf1-bin}"
EXPECT=(region:5 nation:25 supplier:10000 customer:150000 part:200000 partsupp:800000 orders:1500000 lineitem:6001215)
fail=0
for spec in "${EXPECT[@]}"; do
  tbl="${spec%:*}"; want="${spec##*:}"
  p="$SF1/$tbl.tbl"
  if [[ ! -f "$p" ]]; then echo "MISSING $p"; fail=1; continue; fi
  got=$(awk 'NF' "$p" | wc -l)
  if [[ "$got" != "$want" ]]; then
    echo "MISMATCH $tbl got=$got want=$want"; fail=1
  else
    echo "OK      $tbl=$got"
  fi
done
for tbl in region nation supplier customer part partsupp orders lineitem; do
  p="$BIN/$tbl.bin"
  if [[ ! -f "$p" ]]; then echo "BIN MISSING $p"; fail=1; continue; fi
  # Read 8 bytes: 4-byte magic + 4-byte LE version
  magic=$(head -c 4 "$p" | tr -d '\0')
  ver=$(od -An -tu4 -N4 -j4 "$p" | tr -d ' ')
  if [[ "$magic" != "BINT" ]] || [[ "$ver" != "2" ]]; then
    echo "BIN BAD  $p magic=$magic ver=$ver"; fail=1
  else
    echo "BIN OK   $tbl.bin v2"
  fi
done
exit $fail
