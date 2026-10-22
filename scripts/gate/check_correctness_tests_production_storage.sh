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
# #5400 —— 判据从「grep 旧拼写」改成「按分支判定实际装配链」:
#   原来的 R2/R4 grep 的是 `WalStorage::new_with_sync_mode_and_checkpoint`
#   与 `FileStorage::new_with_wal` —— 这两个拼写在 #5317（生产栈只开一个
#   FileBackedWalManager）里**整体移除**了，于是门禁恒红。恒红的断言携带
#   零信息：把默认分支真的换成内存存储，R2/R4 依然只会打印同样两行 FAIL。
#
#   现在：
#   * 判据按 **match arm** 分别判定（默认 arm / binary arm / parallel arm），
#     不再「整块 grep」—— 整块 grep 分不清是哪个分支变了；
#   * 判据问的是**语义**（默认路径是否仍 file-backed 且仍带 WAL 接线），
#     而不是某个构造函数的历史拼写；
#   * 每个判据是一个函数，**主流程与 `--selftest` 调用的是同一批函数**
#     （一个规则只有一份实现）—— selftest 用人造 fixture 证明这些判据
#     确实还会对「退化」变红。
#
# 退出码: 0 = PASS, 1 = FAIL（阻断合并）, 2 = 用法错误
#
# 依据 docs/releases/v4.1.0（#5199 提案 §2「测试必须声明它测的是哪种配置」）

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/gate/common.sh
source "$REPO_ROOT/scripts/gate/common.sh"

LIB="crates/mysql-server/src/lib.rs"

PASS_COUNT=0
FAIL_COUNT=0
FAIL_ITEMS=()

add_pass() { echo "  [PASS] $1"; PASS_COUNT=$((PASS_COUNT + 1)); }
add_fail() { echo "  [FAIL] $1"; FAIL_ITEMS+=("$1"); FAIL_COUNT=$((FAIL_COUNT + 1)); }

# =============================================================================
# 判据（函数化：主流程与 selftest 共用）
# =============================================================================

# R1 默认路径是 file-backed：FileStorage 被包进 MvccStorage。
r1_default_is_file_backed() {
    printf '%s\n' "$1" | grep -q "MvccStorage::new(file_storage)"
}

# R2 默认路径带 WAL 持久化：WAL manager + 走 WAL 接线的 FileStorage +
# 外层 WAL 包装层（WalStorage / ParallelWalStorage）。三者缺一即判定退化。
r2_default_has_wal() {
    local arm="$1"
    printf '%s\n' "$arm" | grep -q "FileBackedWalManager::new" &&
        printf '%s\n' "$arm" | grep -q "FileStorage::new_with_external_wal" &&
        printf '%s\n' "$arm" | grep -qE "WalStorage<|ParallelWalStorage<|new_with_shared_wal_handle|new_with_shared_handle"
}

# R3 binary 分支 file-backed。
r3_binary_is_file_backed() {
    printf '%s\n' "$1" | grep -q "BinaryTableStorage::new_with_data"
}

# R4 parallel 分支 file-backed：走 WAL 接线的 FileStorage + 外层 ParallelWalStorage。
r4_parallel_is_file_backed() {
    local arm="$1"
    printf '%s\n' "$arm" | grep -q "FileStorage::new_with_external_wal" &&
        printf '%s\n' "$arm" | grep -q "ParallelWalStorage"
}

# R5 负向：生产路径里不得出现 MemoryStorage。返回 0 = 出现了（违规）。
r5_has_memory_storage() {
    printf '%s\n' "$1" | grep -q "MemoryStorage"
}

# =============================================================================
# --selftest：用造出来的 fixture 证明这些判据还会对「退化」变红
# =============================================================================
run_selftest() {
    local pass=0 fail=0

    # 健康的生产装配（形状取自 crates/mysql-server/src/lib.rs 的当前代码）
    local healthy_default='            let fbwm = FileBackedWalManager::new(wal_path.clone())?;
            let mut file_storage = sqlrustgo_storage::FileStorage::new_with_external_wal(
                wal_data_dir.clone(), shared_wal.clone())?;
            let mvcc_inner = sqlrustgo_storage::MvccStorage::new(file_storage);
            <WalStorage<_, sqlrustgo_storage::wal::MemoryWalManager>>::new_with_shared_wal_handle('
    local healthy_binary='            let bin_storage = BinaryTableStorage::new_with_data(wal_data_dir.clone())?;'
    local healthy_parallel='            let mut file_storage = sqlrustgo_storage::FileStorage::new_with_external_wal(
                wal_data_dir.clone(), shared_wal.clone())?;
            let mut storage = <sqlrustgo_storage::ParallelWalStorage<
                sqlrustgo_storage::MvccStorage<sqlrustgo_storage::FileStorage>,
                sqlrustgo_storage::wal::DynWalManager,
            >>::new_with_shared_handle(mvcc_inner, shared_wal.clone());'

    # 退化的默认路径：WAL 包装层没了
    local degraded_default="${healthy_default%%<WalStorage<*}"
    # 退化的默认路径：换成内存存储
    local memory_default='            let mem = MemoryStorage::new();
            let mvcc_inner = sqlrustgo_storage::MvccStorage::new(file_storage);'
    # 退化的 parallel 分支：不再 file-backed
    local degraded_parallel='            let mut storage = <sqlrustgo_storage::ParallelWalStorage<
                sqlrustgo_storage::MvccStorage<sqlrustgo_storage::FileStorage>,
                sqlrustgo_storage::wal::DynWalManager,
            >>::new_with_shared_handle(mvcc_inner, shared_wal.clone());'

    if r1_default_is_file_backed "$healthy_default" &&
        r2_default_has_wal "$healthy_default" &&
        r3_binary_is_file_backed "$healthy_binary" &&
        r4_parallel_is_file_backed "$healthy_parallel" &&
        ! r5_has_memory_storage "$healthy_default$healthy_binary$healthy_parallel"; then
        echo "[PASS] selftest: 一份健康的装配满足 R1–R4 且不违反 R5"
        pass=$((pass + 1))
    else
        echo "[FAIL] selftest: 健康 fixture 竟然没有满足 R1–R4"
        fail=$((fail + 1))
    fi

    if ! r2_default_has_wal "$degraded_default"; then
        echo "[PASS] selftest: 默认路径丢掉 WAL 包装层 ⇒ R2 变红"
        pass=$((pass + 1))
    else
        echo "[FAIL] selftest: 默认路径丢掉 WAL 包装层，R2 却没有变红 —— 判据失去判别力"
        fail=$((fail + 1))
    fi

    if r5_has_memory_storage "$memory_default"; then
        echo "[PASS] selftest: 生产路径出现 MemoryStorage ⇒ R5 变红"
        pass=$((pass + 1))
    else
        echo "[FAIL] selftest: MemoryStorage 进入生产路径，R5 却没有变红"
        fail=$((fail + 1))
    fi

    if ! r4_parallel_is_file_backed "$degraded_parallel"; then
        echo "[PASS] selftest: parallel 分支不再 file-backed ⇒ R4 变红"
        pass=$((pass + 1))
    else
        echo "[FAIL] selftest: parallel 分支不再 file-backed，R4 却没有变红"
        fail=$((fail + 1))
    fi

    if ! r3_binary_is_file_backed '            let mem = MemoryStorage::new();'; then
        echo "[PASS] selftest: binary 分支换成内存存储 ⇒ R3 变红"
        pass=$((pass + 1))
    else
        echo "[FAIL] selftest: binary 分支换成内存存储，R3 却没有变红"
        fail=$((fail + 1))
    fi

    echo
    echo "--- selftest summary ---"
    echo "  passed: $pass"
    echo "  failed: $fail"
    if [ "$fail" -gt 0 ]; then
        echo
        echo "❌ selftest FAILED"
        return 1
    fi
    echo
    echo "✅ selftest PASS"
    return 0
}

# =============================================================================
# 主流程
# =============================================================================

if [ "${1:-}" = "--selftest" ]; then
    run_selftest
    exit $?
fi

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
TMP_BLOCK="$(make_temp_file)" || exit 2
trap 'rm -f "$TMP_BLOCK"' EXIT
awk '
    /match storage\.as_deref\(\)/ { f = 1 }
    f { print }
    f && /^    \};/ { exit }
' "$LIB" > "$TMP_BLOCK"

if [ ! -s "$TMP_BLOCK" ]; then
    echo "  [FAIL] 未在 $LIB 找到生产存储选择块 (match storage.as_deref())"
    echo "=== Result: PASS=${PASS_COUNT} FAIL=$((FAIL_COUNT + 1)) ==="
    exit 1
fi

# 按 match arm 切开：arm 起点是 8 空格缩进的 `Some("x") => {` 或 `_ => {`。
# arm_body <arm 名> 打印该 arm 的全部正文。
arm_body() {
    awk -v want="$1" '
        $0 ~ /^        (Some\("[a-z_]+"\)|_) => \{$/ {
            cur = $0
            sub(/^ +/, "", cur)
            sub(/ => \{$/, "", cur)
            next
        }
        cur == want { print }
    ' "$TMP_BLOCK"
}

DEFAULT_ARM="$(arm_body '_')"
BINARY_ARM="$(arm_body 'Some("binary")')"
PARALLEL_ARM="$(arm_body 'Some("parallel")')"

if [ -z "$DEFAULT_ARM" ] || [ -z "$BINARY_ARM" ] || [ -z "$PARALLEL_ARM" ]; then
    echo "  [FAIL] 生产存储选择块里没能按 arm 切开（default/binary/parallel 三条路径必须都在）"
    echo "         这说明装配块的结构变了 —— 门禁的解析方式必须跟着更新，别让它静默放过。"
    echo "=== Result: PASS=${PASS_COUNT} FAIL=$((FAIL_COUNT + 1)) ==="
    exit 1
fi

# R1 默认（file）路径把 FileStorage 包进 Mvcc —— 生产默认是 file-backed。
if r1_default_is_file_backed "$DEFAULT_ARM"; then
    add_pass "R1 生产默认路径 file-backed (MvccStorage::new(file_storage))"
else
    add_fail "R1 生产默认路径不再是 MvccStorage<FileStorage> — 正确性测试的默认存储已偏离生产 file 配置"
fi

# R2 默认路径带 WAL 持久化（WAL manager + WAL 接线的 FileStorage + WAL 包装层）。
if r2_default_has_wal "$DEFAULT_ARM"; then
    add_pass "R2 生产默认路径带 WAL 持久化 (FileBackedWalManager + FileStorage::new_with_external_wal + WAL 包装层)"
else
    add_fail "R2 生产默认路径丢失 WAL 包装 — 生产持久化语义变更，正确性测试不再覆盖生产配置"
fi

# R3 `binary` 分支 file-backed。
if r3_binary_is_file_backed "$BINARY_ARM"; then
    add_pass "R3 binary 分支 file-backed (BinaryTableStorage::new_with_data)"
else
    add_fail "R3 binary 分支不再是 file-backed BinaryTableStorage"
fi

# R4 `parallel` 分支 file-backed。
if r4_parallel_is_file_backed "$PARALLEL_ARM"; then
    add_pass "R4 parallel 分支 file-backed (FileStorage::new_with_external_wal + ParallelWalStorage)"
else
    add_fail "R4 parallel 分支不再是 file-backed FileStorage"
fi

# R5 关键负向检查：生产存储选择块内不得出现 MemoryStorage。
if r5_has_memory_storage "$DEFAULT_ARM$BINARY_ARM$PARALLEL_ARM"; then
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
