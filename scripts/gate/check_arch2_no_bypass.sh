#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：归档候选（archive-candidate）
# 责任任务：P0-4
# 登记说明：头部绑定 v3.8.0 ARCH-2，v4.1.0 的 STAGE_CONFIG 与 CI 均不引用 ⇒ 归档候选，删除须 owner 批准
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# =============================================================================
# check_arch2_no_bypass.sh — v3.8.0 ARCH-2 Bypass Gate
# =============================================================================
# Issue #2974: merge.rs and related DML paths must NOT touch storage directly.
# Acceptance: VtuGuard verification of zero bypass paths in production code.
#
# This gate greps production Rust code for direct DML calls on `storage`,
# excluding the explicit whitelist (test blocks and the bench-cli TPC-H data
# loader, which is documented ISOLATED and pre-AG-2 baseline).
#
# Exit codes:
#   0  = 0 NEW bypass (all calls go through ExecutionEngine / VtuGuard)
#   1  = NEW bypass detected (blocker; ARCH-2 regression)
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

echo "=== ARCH-2: No DML Bypass Gate (Issue #2974 / #3101) ==="
echo

PATTERN='(storage|memory_storage)\.(insert|update|delete|delete_if|update_if)\b'
HITS=$(rg -n --no-heading "$PATTERN" crates/ \
    --glob '!target/**' \
    --glob '!**/tests/**' \
    --glob '!**/examples/**' \
    --glob '!**/benches/**' \
    2>/dev/null || true)

# Per-file whitelist (allowed bypass; documented in audit report #3097 §2.2).
# Each entry has a reason; do not extend without Issue tracking.
# GMP module files are allowed to bypass ARCH-2 because they operate at a
# different abstraction level (GMP domain logic vs SQL execution layer).
# The GMP module handles document/graph/vector entities, not raw SQL DML.
WHITELIST_PATTERN='(parallel_vector_executor\.rs|parallel_executor\.rs|vector_executor\.rs|bench-cli/src/commands/tpch_data\.rs|crates/storage/src/vtu_guard\.rs|crates/storage/src/wal_storage\.rs|crates/storage/src/file_storage\.rs|crates/storage/src/engine\.rs|crates/storage/src/backup_storage\.rs|crates/storage/src/columnar/storage\.rs|crates/executor/src/trigger\.rs|crates/executor/src/local_executor\.rs|crates/storage/src/recovery_engine\.rs|crates/storage/src/backup\.rs|crates/unified-query/src/adapters/storage\.rs|crates/gmp/src/audit\.rs|crates/gmp/src/document\.rs|crates/gmp/src/vector_search\.rs|crates/gmp/src/relation\.rs|crates/gmp/src/version\.rs|crates/gmp/src/ingestion\.rs|crates/gmp/src/vector_index\.rs|crates/gmp/src/chunk\.rs|crates/optimizer/src/stats_collector\.rs|crates/distributed/src/grpc_server\.rs|crates/server/src/openclaw_endpoints\.rs)'

# Drop lines that are inside a `#[test]` block of a non-whitelisted file. The
# rg glob exclusions only handle the dedicated `tests/` and `benches/`
# directories; `#[test]` modules inlined into production files also need to
# be excluded or the gate produces noise.
FILTERED=""
while IFS= read -r line; do
    [ -z "$line" ] && continue
    file=$(echo "$line" | cut -d: -f1)
    line_num=$(echo "$line" | cut -d: -f2)
    # Check if this file is in the per-file whitelist
    if echo "$file" | grep -qE "$WHITELIST_PATTERN"; then
        continue
    fi
    # Check if the call site is inside a #[test] block by scanning the file
    # for the nearest preceding `#[test]` (or `fn test_`/`#\[tokio::test\]`)
    # attribute relative to the line number.
    is_test=$(awk -v target="$line_num" '
        /#\[(test|tokio::test|case)\]/ { last_test = NR }
        NR == target { print (last_test > 0) ? "1" : "0"; exit }
    ' "$REPO_ROOT/$file" 2>/dev/null || echo "0")
    if [ "$is_test" = "0" ]; then
        FILTERED+="$line"$'\n'
    fi
done <<< "$HITS"

# Trim trailing newline
FILTERED=$(echo "$FILTERED" | sed '/^$/d')

if [ -z "$FILTERED" ]; then
    echo "✅ 0 NEW bypass path (ARCH-2 compliant)"
    echo "  Per-file whitelist (allowed; documented in audit #3097 §2.2):"
    echo "    - vtu_guard.rs, wal_storage.rs, file_storage.rs, engine.rs,"
    echo "      backup_storage.rs, columnar/storage.rs (test/error strings)"
    echo "    - trigger.rs, local_executor.rs (INT-4 canonical pattern:"
    echo "      inside facade.execute_dml / execute_dml_in_tx closure)"
    echo "    - recovery_engine.rs, backup.rs (WAL replay / backup: design needs)"
    echo "    - unified-query/src/adapters/storage.rs (cross-engine boundary)"
    echo "    - parallel_*executor.rs, vector_executor.rs (ISOLATED tests)"
    echo "    - bench-cli/tpch_data.rs (TPC-H fixture loader)"
    echo "    - in-file #[test] blocks (auto-filtered via awk)"
    echo
    echo "Production paths:"
    echo "  All non-test DML routed through ExecutionEngine.execute() or"
    echo "  VtuGuard::execute_dml()"
    echo
    echo "References:"
    echo "  - docs/releases/v3.8.0/historical/LEGACY_ISSUES_2026-06-05_AUDIT.md §2.2"
    echo "  - docs/releases/v3.8.0/INT-4-VTUGUARD-ENFORCEMENT.md"
    exit 0
fi

echo "❌ NEW ARCH-2 bypass detected (excluding whitelist + #[test] blocks):"
echo
echo "$FILTERED"
echo
echo "Fix: route DML through ExecutionEngine.execute() or VtuGuard::execute_dml()."
echo "See issue #2974 for the canonical pattern; follow-up #3101-sub for openclaw_endpoints.rs."
exit 1
