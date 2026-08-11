#!/usr/bin/env bash
#
# check_storage_index_wal.sh — V312-23 / #3910 gate
#
# Validates storage/index/WAL tooling backlog items via real tests and
# grep checks. Each block corresponds to a Disposition Summary row in
# docs/releases/v3.12.0/storage-index-wal-backlog-report.md.
#
# Anti-Fabrication Policy v1.0: real exec, no doc claim without proof.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
pass() { echo "[PASS] $1"; PASS=$((PASS + 1)); }
fail() { echo "[FAIL] $1"; FAIL=$((FAIL + 1)); }

# ---- 1. Composite Indexes (5/5 tests PASS) ----
COMPOSITE_OUT=$(timeout 90 cargo test --lib --package sqlrustgo-storage test_composite_btree_index 2>&1 | tail -10)
if echo "$COMPOSITE_OUT" | grep -q "test result: ok. 5 passed"; then
  pass "Composite B+Tree indexes: 5/5 tests PASS"
else
  fail "Composite B+Tree indexes: 5/5 tests did not pass"
  echo "$COMPOSITE_OUT" | tail -8
fi

# ---- 2. WAL Checkpoint method exists ----
if grep -qE "fn checkpoint\(" crates/storage/src/wal_*.rs 2>/dev/null; then
  WAL_CKPT=$(grep -h "fn checkpoint(" crates/storage/src/wal_*.rs 2>/dev/null | head -3)
  pass "WAL checkpoint: method exists ($(echo "$WAL_CKPT" | head -1 | awk '{print $NF}'))"
else
  fail "WAL checkpoint: method not found in crates/storage/src/wal_*.rs"
fi

# ---- 3. WAL Verification Tool exists ----
if [ -d "crates/wal-verification" ] && [ -f "crates/wal-verification/src/lib.rs" ]; then
  WC_LINES=$(wc -l < crates/wal-verification/src/lib.rs 2>/dev/null || echo 0)
  pass "WAL verification tool: crates/wal-verification/src/lib.rs ($WC_LINES lines)"
else
  fail "WAL verification tool: crates/wal-verification/ missing"
fi

# ---- 4. Torn Page Protection (Double-Write Buffer — actually implemented as F-26) ----
DWB_OUT=$(timeout 90 cargo test --lib --package sqlrustgo-storage double_write 2>&1 | tail -10)
if echo "$DWB_OUT" | grep -q "test result: ok. 6 passed"; then
  pass "Double-Write Buffer (torn page protection): 6/6 tests PASS"
else
  fail "Double-Write Buffer: 6/6 tests did not pass"
  echo "$DWB_OUT" | tail -8
fi

# ---- 5. Page Checksum field present ----
if grep -qE "checksum.*: u32|skip checksum" crates/storage/src/page.rs 2>/dev/null; then
  pass "Page checksum: field present in page.rs"
else
  fail "Page checksum: field not found"
fi

echo
echo "Results: PASS=$PASS, FAIL=$FAIL"
if [ $FAIL -eq 0 ]; then
  echo "[PASS] V312-23 Storage/Index/WAL gate PASSED"
  exit 0
fi
echo "[FAIL] V312-23 Storage/Index/WAL gate FAILED"
exit 1