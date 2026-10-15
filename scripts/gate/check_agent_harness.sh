#!/bin/bash
# check_agent_harness.sh — SQLRustGo Agent Harness 结构门禁
# 目标: 验证根 AGENTS.md / CLAUDE.md + docs/agent-harness/ + 关键 cross-link 完整且自洽
# 退出: 0=PASS, 1=FAIL (任意 check 缺失即非零)
#
# 检查项 (8):
#   1. 根 AGENTS.md 存在且非空
#   2. 根 CLAUDE.md 存在且非空
#   3. docs/agent-harness/INDEX.md 存在且非空
#   4. docs/agent-harness/ 下 6 个必需文件全部存在
#   5. AGENTS.md 引用 develop/v4.1.0 (当前主分支名)
#   6. AGENTS.md/CLAUDE.md 含 gitea252 主仓库 (Gitea-only clause)
#   7. Cargo.toml workspace.package.edition 是 "2021" (不是 2024)
#   8. ARCHITECTURE.md 含 stale marker (避免旧 v1.2/2026-03-05 假象)

set -uo pipefail
# 不用 set -e: 我们自己检查 ERRORS 累积

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

ERRORS=0
log_pass() { echo "[PASS] $*"; }
log_fail() { echo "[FAIL] $*" >&2; ERRORS=$((ERRORS + 1)); }

# 1. 根 AGENTS.md 存在且非空
if [ -s AGENTS.md ]; then
    AGENTS_LINES=$(wc -l <AGENTS.md | tr -d ' ')
    log_pass "AGENTS.md exists (${AGENTS_LINES} lines)"
else
    log_fail "AGENTS.md missing or empty"
fi

# 2. 根 CLAUDE.md 存在且非空
if [ -s CLAUDE.md ]; then
    CLAUDE_LINES=$(wc -l <CLAUDE.md | tr -d ' ')
    log_pass "CLAUDE.md exists (${CLAUDE_LINES} lines)"
else
    log_fail "CLAUDE.md missing or empty"
fi

# 3. docs/agent-harness/INDEX.md 存在且非空
if [ -s docs/agent-harness/INDEX.md ]; then
    log_pass "docs/agent-harness/INDEX.md exists"
else
    log_fail "docs/agent-harness/INDEX.md missing or empty"
fi

# 4. docs/agent-harness/ 下 6 个必需文件全部存在 (INDEX.md 已由 #3 验证, 这里跳过来避免重复)
REQUIRED_FILES=(
    REPOSITORY_MAP.md
    GITNEXUS_ARCHITECTURE_MAP.md
    AOCI_GUIDE.md
    DOCUMENTATION_INDEX.md
    IMPLEMENTATION_TRUTH.md
)
for f in "${REQUIRED_FILES[@]}"; do
    if [ -s "docs/agent-harness/$f" ]; then
        log_pass "docs/agent-harness/$f exists"
    else
        log_fail "docs/agent-harness/$f missing or empty"
    fi
done

# 5. AGENTS.md 引用 develop/v4.1.0
if [ -f AGENTS.md ] && grep -q "develop/v4\.1\.0" AGENTS.md; then
    log_pass "AGENTS.md references develop/v4.1.0"
else
    log_fail "AGENTS.md does not reference develop/v4.1.0 (branch name mismatch)"
fi

# 6. AGENTS.md / CLAUDE.md 含 gitea-only clause
GITEA_OK=0
if [ -f AGENTS.md ] && grep -q "gitea252" AGENTS.md; then GITEA_OK=1; fi
if [ -f CLAUDE.md ] && grep -q "gitea252" CLAUDE.md; then GITEA_OK=1; fi
if [ "$GITEA_OK" -eq 1 ]; then
    log_pass "Gitea-only clause present (gitea252 mentioned)"
else
    log_fail "Gitea-only clause missing (gitea252 not referenced)"
fi

# 7. Cargo.toml edition 是 2021 (不是 2024)
if [ -f Cargo.toml ]; then
    # 找 workspace.package.edition 行
    EDITION=$(grep -E '^edition\s*=' Cargo.toml | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
    if [ "$EDITION" = "2021" ]; then
        log_pass "Cargo.toml edition = $EDITION (matches doc claim)"
    else
        log_fail "Cargo.toml edition = '${EDITION:-MISSING}' (expected 2021)"
    fi
else
    log_fail "Cargo.toml not found"
fi

# 8. ARCHITECTURE.md 含 stale marker (避免 v1.2 误导)
if [ -f docs/architecture/ARCHITECTURE.md ]; then
    if grep -qi "历史快照\|stale\|已弃用\|deprecated" docs/architecture/ARCHITECTURE.md; then
        log_pass "ARCHITECTURE.md has stale/deprecated marker"
    else
        log_fail "ARCHITECTURE.md lacks stale marker (still claims current arch)"
    fi
else
    log_fail "docs/architecture/ARCHITECTURE.md missing"
fi

echo ""
echo "============================================="
if [ "$ERRORS" -eq 0 ]; then
    echo "  agent-harness gate: PASS (0 errors)"
    exit 0
else
    echo "  agent-harness gate: FAIL ($ERRORS errors)"
    exit 1
fi
