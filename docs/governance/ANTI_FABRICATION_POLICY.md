# Anti-Fabrication Policy (AFP)

> **版本**: 1.0.0
> **发布日期**: 2026-05-30
> **适用范围**: SQLRustGo / GMP-Platform / Hermes Agent 所有 AI 生成内容
> **维护人**: Hermes Agent
> **状态**: 强制执行

---

## 1. 目的与范围

本政策旨在防止 **"伪合规（Compliance Theater）"** 现象：

> AI 系统表面满足门禁（CI/CD / policy checks），但内容层证据是"生成的"，而非"可验证的真实执行结果"。

**适用范围**：
- 所有 AI 生成的文档（开发计划、测试报告、门禁报告、发布说明）
- 所有 AI 声明的状态（"测试通过"、"门禁通过"、"任务完成"）
- 所有 AI 生成的内容用于治理判断的场景

---

## 2. 核心原则

### 2.1 证据优先原则（Evidence-First）

任何结论必须绑定至少一种**机器可验证证据**：

| 证据类型 | 说明 |
|----------|------|
| CI 日志 | build/test output（结构化 JSON） |
| Git commit hash | 可验证的代码状态 |
| 执行 trace | execution trace / AST trace |
| Artifact hash | binary / schema / migration |
| Policy engine 结果 | gate evaluation output |

**禁止仅基于文本声明的"通过 / 完成 / 已验证"**。

### 2.2 可追溯生成原则（Provenance Required）

所有 AI 生成的文档必须带 provenance 元数据：

```yaml
provenance:
  generated_by: ai|human|hybrid
  generated_at: ISO8601
  input_refs:
    - type: commit
      value: abc123
    - type: ci_run
      value: run_20260530_001
  evidence:
    - ci_pipeline_id: "19382"
      log_hash: "sha256:..."
```

**如果没有 provenance = 默认视为无效文档**

### 2.3 反幻觉约束原则（Anti-Fabrication Constraint）

| 内容类型 | 必须依赖 |
|----------|----------|
| 测试报告 | CI 真实输出（JSON 格式） |
| 架构图 | codebase introspection |
| 修复说明 | diff + commit |
| 门禁结果 | policy engine 结果（机器输出） |

**禁止**：
- "模拟 CI 结果"
- "推测测试通过"
- "假设性 pass/fail"
- "根据文档推测"

---

## 3. 造假类型分类（必须先定义，否则无法治理）

### Type A：虚构执行（Execution Fabrication）

**定义**：AI 声称"测试已通过 / build 成功"，但无日志支撑。

**典型场景**：
- 在 TEST_PLAN.md 写入 "93 tests PASS" 但未执行 `cargo test`
- 在 GA_GATE_REPORT.md 写入 "G5 PASS" 但未运行实际检查

**验证方法**：交叉验证 CI run ID + log hash

---

### Type B：伪门禁（Gate Fabrication）

**定义**：AI 生成" Beta PASS / Alpha PASS"，但未调用真实 gate engine。

**典型场景**：
- 自行判断状态为 PASS 而非 policy engine 输出
- 跳过检查步骤直接写"门禁通过"

**验证方法**：检查是否有 gate engine 输出的 policy evaluation result

---

### Type C：伪证据（Evidence Fabrication）

**定义**：AI 引用"log 如下 / trace 如下"，但日志不存在或不可追踪。

**典型场景**：
- 引用 "CI run #19382" 但该 run 不存在
- 引用 "log_hash: sha256:..." 但 hash 在系统中不存在
- 声称"根据命令输出"但无法提供命令输出文件

**验证方法**：evidence gap scanner 检查声明 vs 实际系统差异

---

### Type D：伪任务完成（Task Closure Fabrication）

**定义**：AI 标记"task done / module integrated"，但代码未合并或未验证。

**典型场景**：
- 声称"功能已完成"但 PR 未合并
- 标记"模块已集成"但编译失败
- 声明"修复完成"但未验证

**验证方法**：交叉验证 commit/PR 状态 vs 声明

---

## 4. 文档信任分级

所有文档分为三类，**只有 VerifiedDoc 可用于门禁判断**：

| 类型 | 信任级别 | 门禁可用性 | 说明 |
|------|----------|------------|------|
| **VerifiedDoc** | 高 | ✅ 可用 | 有完整证据链（provenance + CI 绑定） |
| **DerivedDoc** | 中 | ⚠️ 辅助 | 基于 VerifiedDoc 二次加工 |
| **UnverifiedDoc** | 无 | ❌ 禁用 | AI 生成无 provenance |

**规则**：

```
UnverifiedDoc + 用于门禁判断 = 违规（FIXME: 添加问责机制）
```

---

## 5. 门禁分类：Hard Gate vs Soft Gate

### 5.1 分类标准

| 类型 | 验证方式 | 决策权威 | AI 可参与 |
|------|----------|----------|-----------|
| **Hard Gate** | 机器验证（CI/policy engine） | 唯一可信 | 仅解释，不可决定 |
| **Soft Gate** | AI 生成（文本审查） | 不可信 | 可参与，仅供参考 |

**关键规则**：

> 只有 Hard Gate 可以决定 pass/fail

### 5.2 必须为 Hard Gate 的检查项

- 编译通过（cargo build --release）
- 测试通过（cargo test）
- 门禁结果（gate engine output）
- PR merge 状态（API 查询）
- CI run 结果（CI system output）

### 5.3 Soft Gate 的正确用法

AI 生成的 Soft Gate 解释必须明确标注：

```text
[SOFT GATE] AI 解释：此功能已完成
证据状态：UNVERIFIED CLAIM（无 CI 绑定）
供决策参考，不作为门禁依据
```

---

## 6. 强制证据嵌入规则

### 6.1 PASS/FAIL 必须绑定证据

**❌ 错误**：
```
测试通过
```

**✅ 正确**：
```
测试通过 (CI_RUN: #19382, log_hash: sha256:abc123, 执行时间: 2026-05-30T10:00:00Z)
```

### 6.2 无证据时的输出约束

AI 在生成"完成状态"时必须遵循：

```pseudo
if no_evidence:
    MUST NOT output: "completed / done / passed / PASS"
    MUST output: "状态未知（缺少 CI 证据）"
```

### 6.3 禁止自我确认通过

**禁止语句**：
- "测试通过"（无证据）
- "门禁已通过"（无 gate engine 输出）
- "系统验证完成"（无执行 trace）

**正确替代**：
- "根据 CI 系统（run #19382）返回，测试状态为 PASS"
- "根据 gate engine 输出（policy_eval_id: abc123），门禁状态为 PASS"

---

## 7. 问责与违规处理

### 7.1 违规类型与处理

| 违规类型 | 严重程度 | 处理方式 |
|----------|----------|----------|
| Type A（虚构执行） | **P0 — 严重** | 立即回退 + 问责 + 修复证据链 |
| Type B（伪门禁） | **P0 — 严重** | 立即回退 + 问责 + 重新执行门禁 |
| Type C（伪证据） | **P1 — 高** | 标记为 UNVERIFIED CLAIM + 要求补充证据 |
| Type D（伪任务完成） | **P1 — 高** | 标记为 UNVERIFIED CLAIM + 要求验证 |

### 7.2 回退流程

当发现违规时：

1. **立即停止**当前操作
2. **回退**到上一个 VerifiedDoc 状态
3. **记录**违规到治理日志
4. **补充**真实证据
5. **重新**执行门禁

### 7.3 复发惩罚

同一 AI 连续 2 次犯 Type A/B 违规：
- 暂停 AI 生成权限
- 需要人工审核每一步输出

---

## 8. 实施要求

### 8.1 AI Agent 必须满足的条件

每个 AI Agent 在生成内容前必须：

1. 运行 `AI_GENERATOR_AUDIT_CHECKLIST.md` 自检
2. 为每个声明附加 provenance 元数据
3. 无证据时明确输出 "UNVERIFIED CLAIM"

### 8.2 工具层强制

- `check_plan_integrity.sh`：检测计划文档重写
- `check_evidence_binding.sh`：（新增）检测证据绑定缺失
- `audit_documentation.sh`：检测 UnverifiedDoc 用于门禁

### 8.3 governance 层强制

- `gate_spec.md`：Hard Gate vs Soft Gate 区分
- `DOCUMENT_REVIEW_WORKFLOW.md`：Truthfulness 强制原则

---

## 9. 审查周期

| 审查类型 | 频率 | 负责人 |
|----------|------|--------|
| ANTI_FABRICATION_POLICY 复审 | 每次 major 版本 | Hermes Agent |
| 违规记录审查 | 每月 | 人工审核 |
| 检查脚本有效性评估 | 每季度 | Hermes Agent |

---

*本政策为强制执行文件，违反即触发问责机制*