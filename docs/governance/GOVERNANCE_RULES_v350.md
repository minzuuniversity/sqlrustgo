# SQLRustGo Governance Rules — v3.5.0 Update
> **生效日期**: 2026-05-28
> **适用范围**: v3.5.0 及后续版本

---

## 新增规则

### 规则 1: 门禁阈值冻结节点

```
门禁阈值冻结规则（Threshold Freeze Rule）：
  1. Alpha 阶段结束后，所有阈值冻结，禁止降低
  2. Beta 阶段结束后，所有阈值冻结，禁止降低
  3. RC 阶段开始后，任何阈值修改需 RFC + 审批
  4. 只有在 Alpha 开始前才能申请降低阈值
  5. 阈值只能提升，不能降低

例外：性能基准测试阈值可以根据硬件差异调整（需文档说明）
```

**理由**: 历史上有 4 次降低阈值而非提升测试质量的先例

---

### 规则 2: 新 Crate 合入强制测试（Testing Gate for New Crates）

```
新 crate 合入必须满足以下全部条件：

  Phase 1: 合入 PR 时
    ✅ PR 中必须包含该 crate 的测试代码
    ✅ 该 crate 在合入时 coverage ≥30%（使用 --lib profile）
    ✅ Clippy 必须零 error（使用 --all-features）

  Phase 2: Alpha 阶段结束前
    ✅ 该 crate coverage ≥40%

  Phase 3: Beta 阶段结束前
    ✅ 该 crate coverage ≥50%

  Phase 4: RC 阶段开始前
    ✅ 该 crate coverage ≥60%

  违反以上任何一条：
    → 禁止合入（Phase 1）
    → 版本门禁 FAIL（Phase 2-4）
```

**当前受影响的 crate**: storage (10%), types (20%) — 历史遗留，需专项治理

---

### 规则 3: Alpha Gate Actual Execution（门禁报告真实数据）

```
门禁报告必须包含：
  ✅ 实际执行的命令和完整输出
  ✅ 实际覆盖率数值（不是模板值）
  ✅ 实际失败的测试用例名称和行号
  ✅ 实际性能基准数值（TPC-H QPS 等）
  ✅ 执行时间、环境信息（OS, RAM, CPU）
  ✅ git commit hash

门禁报告禁止：
  ❌ 只填"通过/失败"状态
  ❌ 填入模板数值而非实际测量值
  ❌ 跳过实际未执行的检查项
  ❌ 修改阈值以"符合"报告结果

报告模板结构：
  1. 执行摘要（实际命令 + 实际结果）
  2. 详细检查项（每项：命令 → 输出 → 判断）
  3. 未通过的检查（如果有）
  4. Sign-off 签字
```

---

### 规则 4: e2e_tests 条件编译（e2e_tests Conditional Compilation）

```
所有端到端集成测试模块必须使用条件编译：

  // ✅ 正确
  #[cfg(test)]
  pub mod e2e_tests;

  // ❌ 错误（e2e_tests 不应在 lib 编译时包含）
  pub mod e2e_tests;

原因：e2e_tests 依赖大量内部 API，直接编译 lib 会导致 unused import 错误
```

---

### 规则 5: Clippy Fix 自动修复（Automated Clippy Fix）

```
在 CI/CD 的 Alpha/Beta Gate 中加入 clippy --fix 步骤：

  Phase 1: cargo clippy --all-features -- -D warnings
           → 必须零 error 才能通过
  Phase 2: cargo clippy --fix --allow-dirty --all-features
           → 自动修复所有可自动修复的 lint
  Phase 3: 重新提交修复后的代码

clippy --fix 不应替代人工审查，只做格式和简单可修复项：
  ✅ unused imports, unused variables
  ✅ redundant clones, needless borrows
  ✅ simple style corrections
  ❌ 业务逻辑错误
  ❌ 需要人工判断的复杂度问题
```

---

### 规则 6: Coverage 脚本防污染（Coverage Script Pollution Prevention）

```
coverage 脚本中，cargo llvm-cov 的输出必须重定向：

  ✅ cargo llvm-cov ... --json --output-path "$tmp" >/dev/null 2>&1
  ✅ echo "[COVERAGE] Running..." >&2

  ❌ cargo llvm-cov ... --json --output-path "$tmp" 2>/dev/null
     （stdout 仍可能包含测试输出，污染 pct=$(...) 变量）

  ❌ echo "[COVERAGE] Running..."（混在 stdout 中）
```

---

## 修订规则

### 修订: Beta Gate Coverage 豁免条件

```
旧版：Beta 阶段 storage 豁免
新版：storage 和 types 在 v3.5.0 Beta 结束前 coverage ≥40%
      v3.5.0 RC 结束前 coverage ≥60%
      v3.6.0 GA 前 coverage ≥80%
```

---

## 治理流程

```
版本门禁流程：

  Alpha ──→ Beta ──→ RC ──→ GA
    │        │        │       │
    ▼        ▼        ▼       ▼
  实际执行  完整集成  全量B-Gate  发布
  报告      测试      + 安全审计  签名
```

---

## Neo4j 记录

所有规则变更已记录在 Knowledge Graph 中：
- Node: `GovernanceRule`
- Relationships: `:AMENDS`, `:SUPERSEDES`
