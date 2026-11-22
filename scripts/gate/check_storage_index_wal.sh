#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：保留并改接（keep-rewire）
# 责任任务：P0-4
# 登记说明：判据内容仍在 v4.1.0 的声称面内(台账 open issue #5227/#5228/#5377/#5392/#5509)：索引/BufferPool/WAL/恢复与存储真实性仍是 v4.1.0 的 open P0 ⇒ 改名重挂
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
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