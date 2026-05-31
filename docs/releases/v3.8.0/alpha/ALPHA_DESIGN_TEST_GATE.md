# ALPHA 设计评审门禁规范

> **版本**: v1.0
> **日期**: 2026-06-01
> **目标**: 为 Alpha 门禁增加设计/测试文档的内容审查，确保进入 Alpha 前有完整的功能设计和测试方案
> **关联**: ALPHA_GATE_CONTRACT.md, GATE_CONDITIONS.md

---

## 1. 背景与问题

### 1.1 当前 Alpha 门禁的不足

当前 Alpha 门禁 (ALPHA_GATE_CONTRACT.md) 只检查：

| 检查项 | 当前实现 | 问题 |
|--------|----------|------|
| E1 | DEVELOPMENT_PLAN.md 存在 | 只检查文件存在，不检查内容 |
| E2 | TEST_PLAN.md 存在 | 只检查文件存在，不检查内容 |
| A1-A5 | Build/Test/Clippy/Fmt/Coverage | 数值检查，无内容审查 |

**关键缺失**：没有对功能设计和测试设计报告进行**内容质量检查**。

### 1.2 v3.8.0 的教训

v3.8.0 开发过程中发现的问题：

| 问题 | 发现时机 | 代价 |
|------|----------|------|
| UPDATE replay 未实现 | Beta 门禁 | 临时修复 PR-2715 |
| PR-870 MERGE stub 实现 | Beta 门禁 | 需要重做 |
| PR-840 UPDATE 测试只验证 COUNT | Beta 门禁 | 需要补充测试 |
| PR-2696 无测试 | Beta 门禁 | 需要补测 |

**根本原因**：进入 Alpha 时没有对设计报告和测试方案进行充分审查。

---

## 2. 新增 Alpha 门禁：A7 设计/测试文档检查

### 2.1 A7 检查项定义

在 ALPHA_GATE_CONTRACT.md 中新增 A7 检查类别：

| ID | 检查项 | 方法 | 阈值 |
|----|-------|------|------|
| A7-1 | 功能设计报告存在 | 检查 `docs/releases/v{VERSION}/*_DESIGN.md` | 每个主要功能至少 1 份 |
| A7-2 | 功能设计报告内容完整 | 内容检查（见 3.1） | 无 "TBD"、"待实现" 等占位符 |
| A7-3 | 测试设计报告存在 | 检查 `docs/releases/v{VERSION}/*_TEST_DESIGN.md` | 每个主要功能至少 1 份 |
| A7-4 | 测试设计报告内容完整 | 内容检查（见 3.2） | 无 "TBD"、"待实现" 等占位符 |
| A7-5 | 测试验收方案存在 | 检查 `docs/releases/v{VERSION}/*_ACCEPTANCE.md` | 每个主要功能至少 1 份 |
| A7-6 | 第三方 AI 评审 | 外部 AI 评审记录 | 评审记录存在 |

### 2.2 检查命令

```bash
# A7-1: 功能设计报告存在
find docs/releases/v3.8.0 -name "*_DESIGN.md" | wc -l
# 阈值: ≥ 主要功能数量

# A7-2: 功能设计报告内容完整
grep -rE "(TBD|待实现|未完成|placeholder)" docs/releases/v3.8.0/*_DESIGN.md || echo "CLEAN"
# 阈值: 无占位符

# A7-3: 测试设计报告存在
find docs/releases/v3.8.0 -name "*_TEST_DESIGN.md" -o -name "*_TEST_PLAN.md" | wc -l
# 阈值: ≥ 主要功能数量

# A7-4: 测试设计报告内容完整
grep -rE "(TBD|待实现|未完成|placeholder)" docs/releases/v3.8.0/*_TEST_DESIGN.md || echo "CLEAN"
# 阈值: 无占位符

# A7-5: 测试验收方案存在
find docs/releases/v3.8.0 -name "*_ACCEPTANCE.md" | wc -l
# 阈值: ≥ 主要功能数量

# A7-6: 第三方 AI 评审记录
ls docs/releases/v3.8.0/*_AI_REVIEW.md
# 阈值: 每个主要功能有评审记录
```

---

## 3. 内容质量检查标准

### 3.1 功能设计报告内容检查

功能设计报告必须包含：

| 章节 | 要求 | 检查方法 |
|------|------|----------|
| 1. 功能概述 | 清晰描述功能目标 | grep "功能概述" |
| 2. 接口定义 | 所有公共 API 有文档 | grep "pub fn" 或检查接口列表 |
| 3. 数据结构 | 核心数据结构有定义 | grep "struct\|enum" |
| 4. 边界条件 | 错误处理有明确说明 | grep "Error\|Result" |
| 5. 依赖关系 | 外部依赖明确列出 | grep "依赖\|dependency" |

**不合格标志**：
- 包含 "TBD"、"待实现"、"TODO"、"placeholder"
- 接口定义缺失参数说明
- 错误处理只写 "错误处理" 而无具体说明

### 3.2 测试设计报告内容检查

测试设计报告必须包含：

| 章节 | 要求 | 检查方法 |
|------|------|----------|
| 1. 测试目标 | 明确测试验证什么 | grep "测试目标" |
| 2. 测试用例 | 有具体的测试用例列表 | grep "test case\|测试用例" |
| 3. 验收标准 | 有明确的通过/失败标准 | grep "验收\|acceptance" |
| 4. 覆盖率目标 | 有量化覆盖率目标 | grep "coverage\|覆盖率" |
| 5. 边界测试 | 有边界条件和异常测试 | grep "边界\|edge case" |

**不合格标志**：
- 包含 "TBD"、"待实现"、"TODO"、"placeholder"
- 测试用例只有 "测试通过" 而无具体断言
- 覆盖率目标写 "尽量高" 而无具体数值

### 3.3 测试验收方案内容检查

验收方案必须包含：

| 章节 | 要求 |
|------|------|
| 1. 功能验收点 | 每个功能点有明确的验收条件 |
| 2. 测试验收证据 | 需要提供什么测试证据（截图、日志、指标） |
| 3. 人工验收清单 | 需要人工检查的项目 |
| 4. 准入标准 | 进入下一阶段的最低要求 |

---

## 4. 第三方 AI 评审要求

### 4.1 评审范围

以下文档**必须**经过第三方 AI 评审：

1. 每个主要功能的 **功能设计报告**
2. 每个主要功能的 **测试设计报告**
3. **集成测试方案**（跨模块功能）
4. **架构决策记录**（重大设计变更）

### 4.2 评审内容

第三方 AI 评审必须验证：

1. **完整性**：文档是否完整，有无缺失章节
2. **一致性**：设计与实现是否一致
3. **可测试性**：设计是否可以被测试验证
4. **风险识别**：是否有未考虑的风险点
5. **改进建议**：是否有可优化的地方

### 4.3 评审记录格式

```markdown
# {功能名} AI 评审报告

## 评审信息
- 评审 AI: [AI 名称]
- 评审日期: YYYY-MM-DD
- 评审版本: vX.X

## 评审结果
| 维度 | 评分 | 说明 |
|------|------|------|
| 完整性 | /5 | |
| 一致性 | /5 | |
| 可测试性 | /5 | |
| 风险识别 | /5 | |

## 具体问题
1. [问题描述]
2. [问题描述]

## 改进建议
1. [建议]
2. [建议]

## 评审结论
✅ 通过 / ⚠️ 需要修改 / ❌ 不通过
```

### 4.4 推荐评审 AI

| AI | 适用场景 | 备注 |
|----|----------|------|
| ChatGPT (GPT-4) | 中英文文档 | 通用能力强 |
| Claude | 英文文档 | 分析能力强 |
| DeepSeek | 中文文档 | 中文理解好 |

**注意**：评审 AI 应选择与开发 AI 不同的模型，避免思维惯性。

---

## 5. Alpha 门禁判定规则更新

### 5.1 完整 Alpha 门禁清单

| 类别 | 检查项 | 阈值 |
|------|--------|------|
| **A1-A5: 标准检查** | Build, Test, Clippy, Format, Coverage | 全部 PASS |
| **A6: Governance** | Replay Graph, Claim Registry, Decision Registry, Freshness, ADR | 全部 PASS |
| **A7: 设计/测试文档** | A7-1~A7-6 | 全部 PASS |

### 5.2 判定规则

| 判定 | 条件 |
|------|------|
| **PASS** | A1-A5 + A6 + A7 全部 PASS |
| **CONDITIONAL PASS** | A1-A5 + A6 PASS，A7 有 1-2 项 FAIL（可 Beta 前修复） |
| **FAIL** | A1-A5 任一 FAIL，或 A7 有 3+ 项 FAIL |

### 5.3 A7 FAIL 项修复期限

A7 检查 FAIL 项必须在进入 Beta 前修复，修复期限为 2 周。

---

## 6. 执行流程

### 6.1 Alpha 门禁执行顺序

```
Step 1: 执行 A1-A5 标准检查
  ↓ (全部 PASS)
Step 2: 执行 A6 Governance 检查
  ↓ (全部 PASS)
Step 3: 执行 A7 设计/测试文档检查
  ↓ (全部 PASS)
Step 4: 生成 ALPHA_GATE_REPORT.md
  ↓
Step 5: 如有 FAIL 项，创建 Issue 追踪
```

### 6.2 A7 检查详细流程

```
A7-1: 收集功能设计报告列表
  ↓
A7-2: 对每份报告执行内容检查
  ↓
A7-3: 收集测试设计报告列表
  ↓
A7-4: 对每份报告执行内容检查
  ↓
A7-5: 收集验收方案列表
  ↓
A7-6: 检查第三方 AI 评审记录
  ↓
汇总结果，记录到 ALPHA_GATE_REPORT.md
```

---

## 7. 现有文档差距分析

基于 v3.8.0 当前状态（截至 2026-06-01）：

| 功能 | 功能设计报告 | 测试设计报告 | 验收方案 | AI 评审 |
|------|-------------|-------------|---------|---------|
| PR-830 WAL | ✅ WAL 设计文档存在 | ✅ RECOVERY_TEST_DESIGN.md | ✅ PR-830E_CONTRACT.md | ❌ 无 |
| PR-840 DML TX | ✅ PR-840_CONTRACT.md | ✅ RECOVERY_TEST_DESIGN.md | ✅ PR-840_CONTRACT.md | ❌ 无 |
| PR-850 mysql-server | ✅ PR-850_DESIGN.md | ✅ PR-850_TEST_DESIGN.md | ⚠️ 1 行修改 | ❌ 无 |
| PR-860 Planner | ✅ ARCHITECTURE.md | ✅ TEST_PLAN.md | ✅ PR-800_ACCEPTANCE.md | ❌ 无 |
| PR-870 VTU Merge | ✅ PR-870_DESIGN.md (STUB) | ✅ PR-870_TEST_DESIGN.md | ❌ 无 | ❌ 无 |
| PR-900 EE Split | ✅ ARCHITECTURE.md | ✅ TEST_PLAN.md | ✅ PR-800_ACCEPTANCE.md | ❌ 无 |

**差距总结**（2026-06-01 更新）：
- AI 评审：0/6 功能有评审记录 ← **需要第三方 AI 审核**
- 独立设计报告：PR-850, PR-870 已补齐 ✅
- 独立测试报告：PR-850, PR-870 已补齐 ✅
- PR-870 状态：STUB（execute_merge 未被调用）- 需要继续开发

---

## 8. 附录：检查脚本

### 8.1 A7 自动检查脚本

```bash
#!/bin/bash
# alpha_design_test_gate.sh - Alpha A7 设计/测试文档检查

VERSION=${1:-v3.8.0}
DOCS_DIR="docs/releases/${VERSION}"

echo "=== Alpha A7: 设计/测试文档检查 ==="
echo ""

# A7-1: 功能设计报告存在
echo "A7-1: 功能设计报告"
DESIGN_REPORTS=$(find "$DOCS_DIR" -name "*_DESIGN.md" -o -name "*_ARCH.md" 2>/dev/null | wc -l)
echo "  找到 $DESIGN_REPORTS 份设计报告"
if [ "$DESIGN_REPORTS" -eq 0 ]; then
    echo "  ❌ FAIL: 无功能设计报告"
else
    echo "  ✅ PASS"
fi

# A7-2: 设计报告内容完整性
echo ""
echo "A7-2: 设计报告内容完整性"
TBD_COUNT=$(grep -rE "(TBD|待实现|未完成|placeholder)" "$DOCS_DIR"/*_DESIGN.md 2>/dev/null | wc -l)
if [ "$TBD_COUNT" -eq 0 ]; then
    echo "  ✅ PASS: 无占位符"
else
    echo "  ❌ FAIL: 发现 $TBD_COUNT 处占位符"
    grep -rE "(TBD|待实现|未完成|placeholder)" "$DOCS_DIR"/*_DESIGN.md 2>/dev/null | head -5
fi

# A7-3: 测试设计报告存在
echo ""
echo "A7-3: 测试设计报告"
TEST_REPORTS=$(find "$DOCS_DIR" -name "*_TEST_DESIGN.md" -o -name "*_TEST_PLAN.md" 2>/dev/null | wc -l)
echo "  找到 $TEST_REPORTS 份测试报告"
if [ "$TEST_REPORTS" -eq 0 ]; then
    echo "  ❌ FAIL: 无测试设计报告"
else
    echo "  ✅ PASS"
fi

# A7-4: 测试报告内容完整性
echo ""
echo "A7-4: 测试报告内容完整性"
TBD_COUNT=$(grep -rE "(TBD|待实现|未完成|placeholder)" "$DOCS_DIR"/*_TEST_DESIGN.md 2>/dev/null | wc -l)
if [ "$TBD_COUNT" -eq 0 ]; then
    echo "  ✅ PASS: 无占位符"
else
    echo "  ❌ FAIL: 发现 $TBD_COUNT 处占位符"
fi

# A7-5: 验收方案存在
echo ""
echo "A7-5: 验收方案"
ACCEPTANCE_REPORTS=$(find "$DOCS_DIR" -name "*_ACCEPTANCE.md" -o -name "*_CONTRACT.md" 2>/dev/null | wc -l)
echo "  找到 $ACCEPTANCE_REPORTS 份验收文档"
if [ "$ACCEPTANCE_REPORTS" -eq 0 ]; then
    echo "  ❌ FAIL: 无验收方案"
else
    echo "  ✅ PASS"
fi

# A7-6: AI 评审记录
echo ""
echo "A7-6: 第三方 AI 评审"
AI_REVIEWS=$(find "$DOCS_DIR" -name "*_AI_REVIEW.md" 2>/dev/null | wc -l)
echo "  找到 $AI_REVIEWS 份 AI 评审记录"
if [ "$AI_REVIEWS" -eq 0 ]; then
    echo "  ❌ FAIL: 无 AI 评审记录"
else
    echo "  ✅ PASS"
fi

echo ""
echo "=== A7 检查完成 ==="
```

---

*本规范为 Alpha 门禁新增检查项，建议与 ALPHA_GATE_CONTRACT.md 合并执行*
