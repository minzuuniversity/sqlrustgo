#!/usr/bin/env bash
# v3.3.0 RC Gate 检查脚本（历史占位符）
# 用途: 历史版本 v3.3.0 的 RC 门禁占位符
# 说明: v3.3.0 发布时（2025-06）历史上没有 RC 门禁脚本。
#       此脚本的存在是为了文档完整性，不代表当时执行过此检查。
#       RC 阶段的验证由开发团队手动完成。
# 执行: bash scripts/gate/check_rc_v330.sh
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PASS=0; TOTAL=0; SKIP=0; BLOCKERS=0

log_skip() { echo "[rc-v3.3.0] $1 ... ⏭️  SKIP (历史版本 — 发布时无此脚本)"; SKIP=$((SKIP+1)); TOTAL=$((TOTAL+1)); }
log_note() { echo "[rc-v3.3.0] $1 ... ℹ️  NOTE"; TOTAL=$((TOTAL+1)); }

echo "=========================================="
echo "  v3.3.0 RC Gate — 历史占位符脚本"
echo "  说明: 此版本发布时无自动化 RC gate"
echo "=========================================="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

echo "━━━ 历史说明 ━━━"
log_note "R0: 历史说明 — v3.3.0 发布时无自动化 RC gate"
log_skip "R1: cargo build --release --workspace (历史手工验证)"
log_skip "R2: cargo test --lib (历史手工验证)"
log_skip "R3: cargo clippy --all-features (历史手工验证)"
log_skip "R4: cargo fmt --all -- --check (历史手工验证)"
log_skip "R5: Coverage L1 >=85% (历史手工验证，无覆盖率数据)"
log_skip "R6: cargo audit (历史手工验证)"
log_skip "R7: TPC-H SF=1 (历史手工验证，85b458c5 有 TPC-H 报告)"
log_skip "B1-B14: Beta 门禁复核 (历史手工验证)"

echo ""
echo "=== v3.3.0 RC Gate Results (历史占位) ==="
echo "⏭️  SKIP: $SKIP / $TOTAL (历史版本，无法重构验证)"
echo ""
echo "ℹ️  说明: v3.3.0 (2025-06) 发布时采用手动门禁，无自动化脚本。"
echo "   TPC-H SF=1 22/22 验证结果见 docs/releases/v3.3.0/TPC-H_SF1_VERIFICATION.md (commit 85b458c5)。"
echo "   Alpha check_alpha_v330.sh 已手动执行。RC/GA 由开发团队手动验证。"
exit 0