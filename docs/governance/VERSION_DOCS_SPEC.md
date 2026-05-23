# VERSION_DOCS_SPEC — 版本文档规范（GMP-Platform）

> **版本**: 2.0
> **创建日期**: 2026-05-23
> **维护人**: hermes-agent
> **适用范围**: v1.0.0+
> **SSOT**: 本文档是 GMP-Platform 版本文档的唯一权威来源，定义每个版本必须完成的文档类别、数量和质量标准。

---

## 一、目的与适用范围

本文档解决以下问题：
- 文档结构、格式、内容要求不统一
- 版本号、日期、阶段状态缺失或混乱
- 文档间引用使用绝对路径导致死链
- 发布前无法快速判断必选文档是否齐全
- AI Agent 生成文档内容质量参差不齐

**适用范围**: 所有 GMP-Platform 版本（v1.0.0+）

---

## 二、元数据要求（强制）

每个版本文档头部必须包含以下元数据：

```markdown
> **版本**: X.Y.Z
> **更新日期**: YYYY-MM-DD
> **维护人**: <GitHub/Gitea 用户名>
> **阶段状态**: Draft | Alpha | Beta | RC | GA
```

**示例**:
```markdown
> **版本**: 1.2.0
> **更新日期**: 2026-05-23
> **维护人**: hermes-agent
> **阶段状态**: Beta
```

---

## 三、21 文档强制清单（v1.0.0+ 全部版本必须达到）

每个版本在 **任意阶段门禁通过前**（A/B/R/G 任一 Gate），必须完成以下 21 个文档：

### 核心文档（9 个）

| # | 文件名 | 阶段要求 | 内容要求 |
|---|--------|----------|----------|
| 1 | `00-release-summary.md` | GA 前必须存在 | 版本概述、性能亮点、发布目标、里程碑时间线 |
| 2 | `RELEASE_NOTES.md` | RC 后必须完成 | 新功能、变更、Bug 修复、已知问题、贡献者 |
| 3 | `CHANGELOG.md` | RC 后必须完成 | 逐条变更记录（Added/Changed/Fixed/Removed） |
| 4 | `DEVELOPMENT_PLAN.md` | Alpha 后必须存在 | 开发目标、任务分解、Milestone、Issue 映射 |
| 5 | `TEST_PLAN.md` | Alpha 后必须存在 | 测试策略、测试用例设计、E2E 场景清单 |
| 6 | `RELEASE_GATE_CHECKLIST.md` | Beta 后必须存在 | 门禁逐项检查结果（含命令、输出、阈值、Pass/Fail） |
| 7 | `FEATURE_MATRIX.md` | Beta 后必须存在 | 功能点列表、实现状态、验收标准 |
| 8 | `PERFORMANCE_REPORT.md` | Beta 后必须存在 | 性能基准数据（QPS/P95/延迟）、对比基线 |
| 9 | `USER_MANUAL.md` | RC 后必须存在 | 使用说明、CLI 命令、配置项、FAQ |

### 架构与设计文档（5 个）

| # | 文件名 | 阶段要求 | 内容要求 |
|---|--------|----------|----------|
| 10 | `ARCHITECTURE_DECISIONS.md` | RC 后必须存在 | ADR 架构决策记录（每条含背景/决策/后果） |
| 11 | `ARCHITECTURE_DEVIATION_REPORT.md` | Beta 后建议存在 | 架构偏离追踪（P0-P2 偏差记录） |
| 12 | `COMPREHENSIVE_STATUS_REPORT.md` | Beta 后必须存在 | 全项目状态（组件状态、Issues、风险） |
| 13 | `DEPLOYMENT_GUIDE.md` | RC 后必须存在 | 部署步骤、配置说明、回滚方案 |
| 14 | `INCREMENTAL_UPDATE_GUIDE.md` | Beta 后必须存在 | 从上一版本升级的增量变更说明 |

### 测试与质量文档（4 个）

| # | 文件名 | 阶段要求 | 内容要求 |
|---|--------|----------|----------|
| 15 | `TEST_DESIGN.md` | Beta 后必须存在 | 测试设计（单元/集成/E2E 分层策略） |
| 16 | `EVALUATION_REPORT.md` | RC 后必须存在 | 评测报告（场景通过率、检索质量） |
| 17 | `COVERAGE_REPORT.md` | RC 后必须存在 | 测试覆盖率报告（按 crate/模块） |
| 18 | `INTEGRATION_STATUS.md` | Beta 后必须存在 | 组件集成状态、依赖关系、API 兼容性 |

### 项目管理文档（3 个）

| # | 文件名 | 阶段要求 | 内容要求 |
|---|--------|----------|----------|
| 19 | `ISSUE_TRACKER.md` | Beta 后必须存在 | Issue 列表（#ID、描述、状态、里程碑） |
| 20 | `PROGRESS_REPORT.md` | Beta 后必须存在 | 开发周期总结、Milestone 完成情况 |
| 21 | `FEATURE_TRACKING_MATRIX.md` | Beta 后建议存在 | 功能追踪矩阵（功能/Priority/Owner/状态） |

---

## 四、命名规范（强制）

| 文档类型 | 文件名 | 位置 |
|----------|--------|------|
| 发布摘要 | `00-release-summary.md` | `docs/releases/vX.Y.Z/` |
| 发布说明 | `RELEASE_NOTES.md` | `docs/releases/vX.Y.Z/` |
| 变更日志 | `CHANGELOG.md` | `docs/releases/vX.Y.Z/` |
| 开发计划 | `DEVELOPMENT_PLAN.md` | `docs/releases/vX.Y.Z/` |
| 测试计划 | `TEST_PLAN.md` | `docs/releases/vX.Y.Z/` |
| 门禁检查清单 | `RELEASE_GATE_CHECKLIST.md` | `docs/releases/vX.Y.Z/` |
| 功能矩阵 | `FEATURE_MATRIX.md` | `docs/releases/vX.Y.Z/` |
| 性能报告 | `PERFORMANCE_REPORT.md` | `docs/releases/vX.Y.Z/` |
| 用户手册 | `USER_MANUAL.md` | `docs/releases/vX.Y.Z/` |
| 架构决策 | `ARCHITECTURE_DECISIONS.md` | `docs/releases/vX.Y.Z/` |
| 架构偏离报告 | `ARCHITECTURE_DEVIATION_REPORT.md` | `docs/releases/vX.Y.Z/` |
| 综合状态报告 | `COMPREHENSIVE_STATUS_REPORT.md` | `docs/releases/vX.Y.Z/` |
| 部署指南 | `DEPLOYMENT_GUIDE.md` | `docs/releases/vX.Y.Z/` |
| 增量更新指南 | `INCREMENTAL_UPDATE_GUIDE.md` | `docs/releases/vX.Y.Z/` |
| 测试设计 | `TEST_DESIGN.md` | `docs/releases/vX.Y.Z/` |
| 评测报告 | `EVALUATION_REPORT.md` | `docs/releases/vX.Y.Z/` |
| 覆盖率报告 | `COVERAGE_REPORT.md` | `docs/releases/vX.Y.Z/` |
| 集成状态 | `INTEGRATION_STATUS.md` | `docs/releases/vX.Y.Z/` |
| Issue 追踪 | `ISSUE_TRACKER.md` | `docs/releases/vX.Y.Z/` |
| 进度报告 | `PROGRESS_REPORT.md` | `docs/releases/vX.Y.Z/` |
| 功能追踪矩阵 | `FEATURE_TRACKING_MATRIX.md` | `docs/releases/vX.Y.Z/` |

---

## 五、链接规范（强制）

- **必须使用相对路径**: `../governance/GATE_SPEC.md`
- **禁止使用绝对路径**: `http://192.168.0.252:3000/...`
- **跨版本引用**: 明确标注版本号，如 `gate_spec.md (v1.2.0)`

---

## 六、内容质量标准

### 6.1 禁止的内容模式

```
❌ "TBD" 出现在 RC/GA 阶段的文档中（Beta 前允许）
❌ 性能数据为具体数字但未标注来源（必须写明测试命令）
❌ Issue/PR 编号无链接或无法追溯
❌ "略" / "见附件" / "此处省略" 等占位符
❌ 文档之间内容矛盾（如 A 文档说 PASS，B 文档说 FAIL）
```

### 6.2 必须包含的内容模式

```
✅ 每项性能数据必须包含：测试命令 + 环境 + 结果 + 阈值
✅ 每个 Issue 状态变更必须包含：PR 链接或验证命令
✅ 每个门禁检查项必须包含：命令 + 输出摘要 + Pass/Fail + 证据
✅ 每个 ADR 决策必须包含：背景 + 决策 + 后果
```

---

## 七、版本一致性检查（强制）

每次 **Beta 门禁** 通过前，执行以下一致性检查：

| 检查项 | 方法 | 通过标准 |
|--------|------|----------|
| 文档版本号 = 当前版本 | `grep "版本>: X.Y.Z" docs/releases/vX.Y.Z/*.md` | 全部匹配 |
| 无遗留旧版本号 | `grep -r "v1.0.0\|v1.1.0" docs/releases/vX.Y.Z/`（当前版本之外） | 仅当前版本出现 |
| 日期格式正确 | `grep -E "20[0-9]{2}-[0-9]{2}-[0-9]{2}"` | 全部 YYYY-MM-DD |
| 阶段状态正确 | 不得出现 "Draft" 状态的 Beta+ 文档 | — |
| 文档数量 = 21 | `ls docs/releases/vX.Y.Z/*.md \| wc -l` | = 21 |

### 文档完整性自动检查脚本

```bash
#!/bin/bash
# docs/completeness_check.sh — 必须在 Beta 门禁前通过
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-}"

check_required_docs() {
    local version="$1"
    local required_docs=(
        "00-release-summary.md"
        "RELEASE_NOTES.md"
        "CHANGELOG.md"
        "DEVELOPMENT_PLAN.md"
        "TEST_PLAN.md"
        "RELEASE_GATE_CHECKLIST.md"
        "FEATURE_MATRIX.md"
        "PERFORMANCE_REPORT.md"
        "USER_MANUAL.md"
        "ARCHITECTURE_DECISIONS.md"
        "COMPREHENSIVE_STATUS_REPORT.md"
        "DEPLOYMENT_GUIDE.md"
        "INCREMENTAL_UPDATE_GUIDE.md"
        "TEST_DESIGN.md"
        "EVALUATION_REPORT.md"
        "COVERAGE_REPORT.md"
        "INTEGRATION_STATUS.md"
        "ISSUE_TRACKER.md"
        "PROGRESS_REPORT.md"
        "FEATURE_TRACKING_MATRIX.md"
        "ARCHITECTURE_DEVIATION_REPORT.md"
    )
    local doc_dir="$REPO_ROOT/docs/releases/v$version"
    local missing=0

    for doc in "${required_docs[@]}"; do
        if [ ! -f "$doc_dir/$doc" ]; then
            echo "MISSING: $doc"
            missing=$((missing+1))
        fi
    done

    echo "=== Document Completeness: $((${#required_docs[@]} - missing))/${#required_docs[@]} ==="
    return $missing
}

check_required_docs "${VERSION:-1.2.0}"
```

---

## 八、豁免机制

如特殊情况导致无法达标，必须：

1. 在 `docs/governance/GATE_EXEMPTIONS.md` 创建豁免记录
2. 包含：豁免 ID、描述、关联 Issue、审批状态
3. 豁免仅对 P2 及以下问题生效，P0/P1 问题不可豁免

---

## 九、文档版本更新历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 2.0 | 2026-05-23 | 从 12 docs 扩展到 21 docs，强制覆盖架构/测试/项目管理，增加完整性检查脚本 |
| 1.0 | 2026-05-21 | 初始版本 |
