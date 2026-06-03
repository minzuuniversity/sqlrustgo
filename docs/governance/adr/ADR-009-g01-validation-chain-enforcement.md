# ADR-009: G-01 Validation Chain Enforcement

## Status

**Accepted** — v3.8.0 GA (2026-06-03)

## Context

ISSUE-2741 P0 阻塞：测试设计审查无强制门禁。

- ✅ `TEST_REVIEW_TEMPLATE.md` (8 维度 30 项) 已存在
- ❌ 8 维度 30 项仍靠人工填写 `PR-XXX_TEST_REVIEW.md`
- ❌ 验证链（Requirement → Test Design → Assertion → Executable → Gate）不强制

ISSUE-2741 揭示：
- E-1: RECOVERY-006 断言 `COUNT(*) == 1` 不证明 "DELETE 不恢复" 需求
- E-2: 集成 WAL 测试用间接断言（commit count vs data absent）
- E-3: GOVERNANCE.md 无测试设计审查门禁规则

## Decision

**强制执行 G-01 验证链**：在 Beta/RC Gate 阶段自动验证测试设计质量。

### Decision-1: 8 维度可自动化项强制

`scripts/gate/check_validation_chain.sh` 自动化 8 维度中的可验证项：

| 维度 | 检查 | 工具 |
|------|------|------|
| 2.2.1 | 无硬编码路径 | grep `File::create("/` |
| 2.2.4 | TempDir 配对 | 计数 `TempDir::new` |
| 2.3.1 | 断言具体 | grep `assert!(*.is_ok())` |
| 2.4.2 | `#[ignore]` 有理由 | grep `#[ignore.*=.*"..."` |
| 2.4.3 | 无 TODO 占位 | grep `TODO: add test` |
| 2.4.4 | 无注释测试 | grep `^//.*#\[test\]` |
| 2.7.2 | tempfile 使用 | grep `File::create` in tests/ |
| 2.7.3 | 无硬编码密钥 | grep `let.*password.*=.*"` |
| 2.8.1 | clippy 0 warning | `cargo clippy` |
| 2.8.2 | fmt 0 error | `cargo fmt --check` |

**2.1 / 2.5 / 2.6 维度**（覆盖设计、性能、文档）保持人工审核（`TEST_REVIEW.md`）。

### Decision-2: Gate 接入策略

**渐进式启用**：
1. **v3.8.0+1 起**：Beta Gate + RC Gate 接入 `check_validation_chain.sh`
2. **失败处理**：FAIL 阻断 PR 合并
3. **WARN 处理**：WARN 不阻断，记录到 evidence

### Decision-3: 与现有规则关系

| 现有规则 | 关系 |
|----------|------|
| ADR-001 Truthfulness | G-01 强制测试结果真实 |
| ADR-002 Claim Registry | G-01 检查测试断言对应 claim |
| G-02 Source Marker | G-01 复用 source 标记概念 |
| G-06 Freshness | G-01 不检查文档新鲜度 |
| SGL (Semantic Gate) | G-01 补充 SGL 不覆盖的 test design |

### Decision-4: Evidence 持久化

`check_validation_chain.sh` 接受 OUT_DIR 参数，输出 `validation_chain_evidence.json`：

```json
{
  "version": "v3.8.0",
  "rule": "G-01",
  "spec": "SPEC-004",
  "branch": "...",
  "commit": "...",
  "summary": { "pass": N, "fail": M, "total": K },
  "checks": [{ "id": "2.x.y", "status": "PASS|FAIL|WARN", "description": "..." }]
}
```

## Consequences

### Positive

1. **测试质量量化** — 8 维度 30 项可自动验证
2. **ISSUE-2741 关闭** — 验证链强制建立
3. **失败可追溯** — evidence.json + grep 输出
4. **与现有规则一致** — 复用 G-02 等模式

### Negative

1. **现有测试可能 FAIL** — 首次运行发现 5 FAIL（如 ISSUE-2741 揭示的问题）
2. **误报风险** — 初期需人工 review WARN 项
3. **维护成本** — 规则需随新测试模式更新

### Risks

- **R-1**：38 个 `#[ignore]` 无理由（来源：早期开发遗留）→ **分批修复**
- **R-2**：5 个 bare `.is_ok()` 断言（弱断言）→ 人工 review + 修复
- **R-3**：cargo fmt 失败（PR-2782 引入）→ 已在 transactional_facade.rs 等待修复

## Implementation Roadmap

| 任务 | 状态 |
|------|------|
| `check_validation_chain.sh` 创建 | ✅ SPEC-004 |
| Beta/RC Gate 接入 | ⏳ 后续 PR |
| 38 个无理由 `#[ignore]` 修复 | ⏳ 后续 PR |
| 5 个 weak assertion 修复 | ⏳ 后续 PR |
| cargo fmt 修复 | ⏳ transactional_facade.rs |

## Related

- **Source ISSUE**: ISSUE-2741
- **Parent issue**: #2772 (Task #2747)
- **Source PR**: PR-XXX (SPEC-004 execution)
- **Related SPEC**: SPEC-004 (`docs/releases/v3.8.0/SPEC-004-g01-validation-chain.md`)
- **Source rule**: TEST_REVIEW_TEMPLATE.md
- **Refs**:
  - `docs/audit/issues/ISSUE-2741_validation_chain_missing.md`
  - `docs/audit/V380_RECTIFICATION_PLAN_2026-06-03.md` §3.2

## Change History

| 版本 | 日期 | 作者 | 说明 |
|------|------|------|------|
| 1.0 | 2026-06-03 | claude-macmini (governance-engineer) | 初始版本：G-01 强制规则 |
