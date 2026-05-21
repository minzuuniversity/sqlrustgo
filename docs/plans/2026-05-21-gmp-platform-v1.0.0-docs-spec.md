# SPEC: v1.0.0 文档补全

## 现状分析

| 文档 | 位置 | 状态 | 行数 |
|------|------|------|------|
| USER_MANUAL.md | `docs/user/` ❌ | 存在但位置错误 | 302 |
| ARCHITECTURE_DECISIONS.md | `docs/releases/v1.0.0/` ✅ | 存在，需审查 | 109 |
| DEVELOPMENT_PLAN.md | `docs/releases/v1.0.0/` ✅ | 存在，需审查 | 286 |
| TEST_PLAN.md | `docs/releases/v1.0.0/` ✅ | 存在，需审查 | 300 |

## 需要完成

1. **USER_MANUAL.md** — 从 `docs/user/` 移动到 `docs/releases/v1.0.0/`
2. **进展报告** — 新建 `docs/releases/v1.0.0/PROGRESS_REPORT.md`
3. **总控 ISSUE 更新** — 发布进展报告到 Gitea Issue

## 审查标准

基于 VERSION_DOCS_SPEC.md 和 GOVERNANCE_STANDARD.md：
- 元数据: 版本/日期/维护人/阶段状态
- 内容完整性: 每个章节非空
- 链接规范: 相对路径，无绝对 URL
- 版本一致性: 文档版本号 = v1.0.0
