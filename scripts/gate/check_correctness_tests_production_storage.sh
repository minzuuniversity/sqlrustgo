#!/usr/bin/env bash
#
# check_correctness_tests_production_storage.sh — 生产配置断言（#5199 项 3 / F6）
#
# 背景（#5199 / IAIA-410 F6，案例 C）:
#   「结果正确性」集成测试若在非生产存储（内存）上全绿，则该断言在生产
#   配置上从未被验证 —— 测试是绿的，但它测的不是生产配置。#5167 的
#   aggregate_scan_consistency 曾因此被误判（storage: None 实为 file-backed
#   生产默认，而非内存；见该文件头部注释与 IAIA-410 §16）。
#
#   本门禁钉死不变量：server 生产存储选择块（start_ephemeral 内
#   `match storage.as_deref()` 的 storage 装配）的**默认路径必须 file-backed**，
#   且**不得**引入 MemoryStorage（非 file 存储）。如此，任何走默认
#   （`storage: None`）的正确性测试都在生产 file 配置上运行；若有人把
#   生产默认换成内存、或新增内存存储分支，本门禁翻红。
#
#   注：MemoryStorage 在部分单测/辅助函数中仍合法（不持久化、无需 WAL 的
#   逻辑测试），本门禁不禁止其存在，只禁止它进入生产存储选择路径。
#
# 退出码: 0 = PASS, 1 = FAIL（阻断合并）, 2 = 用法错误
#
# 依据 docs/releases/v4.1.0（#5199 提案 §2「测试必须声明它测的是哪种配置」）

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

LIB="crates/mysql-server/src/lib.rs"

PASS_COUNT=0
FAIL_COUNT=0
FAIL_ITEMS=()

add_pass() { echo "  [PASS] $1"; PASS_COUNT=$((PASS_COUNT + 1)); }
add_fail() { echo "  [FAIL] $1"; FAIL_ITEMS+=("$1"); FAIL_COUNT=$((FAIL_COUNT + 1)); }

echo "=== check_correctness_tests_production_storage.sh (#5199 item 3 / F6) ==="
echo "target : $LIB"
echo

if [ ! -f "$LIB" ]; then
    echo "  [FAIL] $LIB not found"
    echo "=== Result: PASS=${PASS_COUNT} FAIL=$((FAIL_COUNT + 1)) ==="
    exit 1
fi

# 提取生产存储选择块：从 `match storage.as_deref() {` 到其后第一条顶层 `    };`。
# 块内即 server 实际装配存储引擎的全部路径（binary / parallel / 默认）。
BLOCK="$(awk '
    /match storage\.as_deref\(\)/ { f = 1 }
    f { print }
    f && /^    \};/ { exit }
' "$LIB")"

if [ -z "$BLOCK" ]; then
    echo "  [FAIL] 未在 $LIB 找到生产存储选择块 (match storage.as_deref())"
    echo "=== Result: PASS=${PASS_COUNT} FAIL=$((FAIL_COUNT + 1)) ==="
    exit 1
fi

# R1 默认（file）路径把 FileStorage 包进 Mvcc —— 生产默认是 file-backed。
if echo "$BLOCK" | grep -q "MvccStorage::new(file_storage)"; then
    add_pass "R1 生产默认路径 file-backed (MvccStorage::new(file_storage))"
else
    add_fail "R1 生产默认路径不再是 MvccStorage<FileStorage> — 正确性测试的默认存储已偏离生产 file 配置"
fi

# R2 默认路径再包进 file-backed WalStorage（WAL 持久化）。
if echo "$BLOCK" | grep -q "WalStorage::new_with_sync_mode_and_checkpoint"; then
    add_pass "R2 生产默认路径带 WAL (WalStorage::new_with_sync_mode_and_checkpoint)"
else
    add_fail "R2 生产默认路径丢失 WAL 包装 — 生产持久化语义变更，正确性测试不再覆盖生产配置"
fi

# R3 `binary` 分支 file-backed。
if echo "$BLOCK" | grep -q "BinaryTableStorage::new_with_data"; then
    add_pass "R3 binary 分支 file-backed (BinaryTableStorage::new_with_data)"
else
    add_fail "R3 binary 分支不再是 file-backed BinaryTableStorage"
fi

# R4 `parallel` 分支 file-backed。
if echo "$BLOCK" | grep -q "FileStorage::new_with_wal"; then
    add_pass "R4 parallel 分支 file-backed (FileStorage::new_with_wal)"
else
    add_fail "R4 parallel 分支不再是 file-backed FileStorage"
fi

# R5 关键负向检查：生产存储选择块内不得出现 MemoryStorage。
if echo "$BLOCK" | grep -q "MemoryStorage"; then
    add_fail "R5 生产存储选择块引入 MemoryStorage — 存在非 file（内存）生产路径，正确性断言可能在非生产配置上通过"
else
    add_pass "R5 生产存储选择块无 MemoryStorage — 不存在非 file 生产路径"
fi

echo
# ---- 汇总 ----------------------------------------------------------------
echo "=== Result: PASS=${PASS_COUNT} FAIL=${FAIL_COUNT} ==="
if [ "$FAIL_COUNT" -gt 0 ]; then
    echo
    echo "FAILED checks:"
    for item in "${FAIL_ITEMS[@]}"; do echo "  - $item"; done
    echo
    echo "A correctness test green on a non-production storage proves nothing"
    echo "about production. The default path MUST stay file-backed."
    exit 1
fi

echo "Production storage default is file-backed; correctness tests cover the production config."
exit 0
