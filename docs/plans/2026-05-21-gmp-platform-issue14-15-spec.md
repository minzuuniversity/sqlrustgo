# Issue 14-15 SPEC: 测试报告 + 发布文档

## Issue 14: 391 场景测试 + 通过率报告

### 需求
- 测试集：391 个 GMP 场景（已通过 PR #25 导入）
- 生成通过率报告（Recall@K, MRR）
- 5 阶段里程碑：Alpha ≥65%, Beta ≥78%, RC#1 基线, RC#2 ≥80%, GA ≥80%

### 现状
- `evaluate` 命令已实现，可指定 JSON 测试集运行
- 测试场景文件：`data/test_scenarios.json`（由 PR #25 创建）
- 报告输出：`--output results.json`

### 需要补充
- 391 场景集文件（完整版）
- 测试报告文档（EVALUATION_REPORT.md）

### 工作量评估
- 391 场景需要实际运行 evaluate 命令
- 需要实际的 GMP 索引数据
- **当前无法在 CLI 内自动完成**，需要：测试脚本 + 报告文档

---

## Issue 15: 用户手册 + 部署指南 + 发布文档

### 需求（Issue #15）
| 文档 | 状态 | 需要更新 |
|------|------|---------|
| USER_MANUAL.md | ✅ 存在 | 检查完整性 |
| DEPLOYMENT_GUIDE.md | ✅ 存在 | 检查完整性 |
| RELEASE_NOTES.md | ✅ 存在 | 更新到 v1.0.0 |
| CHANGELOG.md | ✅ 存在 | 更新到 v1.0.0 |
| 00-release-summary.md | ⚠️ 存在 | 填写 GA 信息 |
| RELEASE_GATE_CHECKLIST.md | ⚠️ 存在 | 填写执行结果 |

### 需要完成
1. 更新 `docs/releases/v1.0.0/00-release-summary.md` — GA 版本信息
2. 更新 `docs/releases/v1.0.0/RELEASE_GATE_CHECKLIST.md` — 执行结果
3. 更新 `docs/releases/v1.0.0/CHANGELOG.md` — 合并的 PR 列表
4. 更新 `docs/releases/v1.0.0/RELEASE_NOTES.md` — v1.0.0 发布说明

---

## 行动计划

### Issue 14
1. 创建 `docs/releases/v1.0.0/EVALUATION_REPORT.md` — 报告框架
2. 创建 `scripts/run_evaluation.sh` — 测试运行脚本
3. 测试场景文件（如 data/test_scenarios_sample.json）

### Issue 15
1. 更新 00-release-summary.md
2. 更新 RELEASE_GATE_CHECKLIST.md（各 gate 通过情况）
3. 更新 CHANGELOG.md（v0.9.0 → v1.0.0 的 PR 记录）
4. 更新 RELEASE_NOTES.md
