# AI Generator Audit Checklist

> **版本**: 1.0.0
> **用途**: AI 在生成任何内容前必须完成的自检清单
> **强制级别**: 必须执行，不可跳过

---

## 0. 前置条件检查

在开始任何生成任务前，确认以下信息：

```
任务类型: _______________________ (如: TEST_PLAN / GATE_REPORT / VERSION_PLAN)
目标版本: _______________________ (如: v3.7.0)
分支:     _______________________ (如: develop/v3.7.0)
当前 commit: ____________________ (git rev-parse HEAD 输出)
```

---

## 1. 证据存在性检查

### 1.1 CI 证据检查

对于需要声明"测试通过 / 编译通过"的内容：

```
CI run ID: _______________________ (来自实际执行的 CI)
log_hash:  _______________________ (来自 CI 系统输出)
执行时间:  _______________________ (ISO8601)
```

**如果无 CI 证据**：
```
→ 必须输出 "UNVERIFIED CLAIM"
→ 禁止输出 "PASS" / "通过" / "完成"
```

### 1.2 Git 证据检查

对于需要声明"任务完成 / 代码合并"的内容：

```
commit hash: _____________________ (来自 git log)
PR number:   _____________________ (来自 git log / API)
合并状态:    _____________________ (from Gitea API)
```

**如果无 Git 证据**：
```
→ 必须输出 "UNVERIFIED CLAIM"
→ 禁止输出 "已完成" / "已合并" / "done"
```

### 1.3 Gate Engine 输出检查

对于需要声明"门禁通过"的内容：

```
gate_policy_eval_id: _____________ (来自 gate engine 实际运行)
gate_output:          _____________ (来自 policy engine JSON 输出)
gate_decision:        PASS/FAIL (来自机器，不是 AI 判断)
```

**如果无 gate engine 输出**：
```
→ 必须输出 "UNVERIFIED CLAIM"
→ 禁止输出 "Gate PASS" / "门禁通过"
```

---

## 2. 文档类型检查

确认即将生成的文档属于哪个信任级别：

```
□ VerifiedDoc:   有完整 provenance + CI 绑定 → 可用于门禁
□ DerivedDoc:    基于 VerifiedDoc 二次加工 → 仅供辅助参考
□ UnverifiedDoc: AI 生成无 provenance → 禁止用于门禁
```

---

## 3. 声明检查（必须逐条验证）

对于文档中的每个声明（claim），逐条检查：

### 3.1 PASS/FAIL 声明

```
声明: "测试通过"
  □ 有 CI run ID?
  □ 有 log_hash?
  □ 有执行时间?
  → 如果任一缺失：标记为 UNVERIFIED CLAIM
```

### 3.2 状态声明

```
声明: "功能已完成"
  □ 有 commit hash?
  □ 有 PR 合并证明?
  → 如果任一缺失：标记为 UNVERIFIED CLAIM
```

### 3.3 证据引用声明

```
声明: "log 如下 (CI run #19382)"
  □ 检查 CI run #19382 是否存在
  □ 检查 log_hash 是否匹配
  → 如果不存在：标记为 FRAUD-LIKE (Type C)
```

---

## 4. Provenance 元数据检查

每个生成的文档必须包含 provenance：

```yaml
provenance:
  generated_by: ai|human|hybrid
  generated_at: ISO8601
  task_type: _____________________
  target_version: ________________
  commit: ________________________
  input_refs:
    - type: ci_run | commit | gate_output
      value: _______________
  evidence:
    - type: _______________
      ci_pipeline_id: _______________
      log_hash: _______________
```

**检查**：
```
□ provenance 元数据存在?
□ generated_by 标注为 ai?
□ input_refs 包含实际证据引用?
□ evidence 包含 CI pipeline ID 和 log hash?
```

---

## 5. 禁止语句检查

逐条检查，**发现任一即停**：

```
□ 禁止: "测试通过"（无 CI 证据）
□ 禁止: "门禁已通过"（无 gate engine 输出）
□ 禁止: "系统验证完成"（无执行 trace）
□ 禁止: "功能已完成"（无 commit/PR）
□ 禁止: "PASS"（无机器验证）
□ 禁止: "已修复"（无 diff 证明）
```

---

## 6. 输出约束检查

根据任务类型，确认输出格式：

### 6.1 测试报告

```
□ 包含 CI run ID
□ 包含 log hash
□ 包含测试用例数量（实际数字，非推测）
□ 包含执行时间
□ 无 "推测通过" / "应该通过" 等表述
```

### 6.2 门禁报告

```
□ 包含 gate_policy_eval_id
□ 包含 gate engine 的实际 JSON 输出
□ PASS/FAIL 来自机器，非 AI 判断
□ 包含实际命令输出引用
□ 无 "我认为" / "AI 判断" 等主观表述
```

### 6.3 开发/测试计划

```
□ 不包含 GA Final / GA APPROVED（计划文档不应显示 GA 状态）
□ 状态为 "Alpha 阶段" / "Beta 阶段" / "RC 阶段"（非 GA）
□ 无伪造的 "完成" 标记（除非有真实 commit）
□ 禁止重写原始计划
```

---

## 7. 违规自检

### 7.1 自问

```
我是否在无实际执行证据的情况下输出了 "PASS" / "通过" / "完成"?
我是否修改了原始计划文档（如 VERSION_PLAN.md）使其看起来更完善?
我是否引用了不存在的 CI run / log hash?
我是否在门禁报告中伪造了 gate engine 输出?
```

**如果任一为"是"**：
```
→ 立即停止生成
→ 回退到上一个 VerifiedDoc 状态
→ 记录违规到治理日志
→ 重新执行真实检查
```

### 7.2 降级规则

当无法提供证据时，输出必须降级：

```
原始意图: "测试通过"
降级输出: "测试状态未知（缺少 CI run 证据）UNVERIFIED CLAIM"
```

---

## 8. 完成确认

完成自检后，输出以下确认：

```
我已完成 AI Generator Audit Checklist
□ 所有声明有证据支撑
□ 无禁止语句
□ provenance 元数据已附加
□ 文档类型已确认
□ 违规自检已通过

如果发现任何缺失：
→ 已标记为 UNVERIFIED CLAIM
→ 未输出任何无证据的 PASS/FAIL 声明
```

---

*此清单为强制执行，AI 生成内容前必须完成自检，不可跳过*