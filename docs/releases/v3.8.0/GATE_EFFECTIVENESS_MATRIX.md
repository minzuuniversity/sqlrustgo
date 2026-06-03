# Gate Effectiveness Matrix — v3.8.0

> 基于 Graph Gate v4.1 vs Legacy Gate 对账报告的升级版
> 原始版本：Z440 Hermes + ChatGPT analysis
> 重新实现：Hermes (Z440 primary)
> **gate_policy_eval_id**: `run_20260601_003`

## 1. 核心观点回顾

**已有充分证据的结论**（✅）：
- Legacy Gate 已不能作为唯一门禁标准 — 它遗漏了 L3 ACID 验证、L2 执行一致性、Crash Recovery
- Graph Gate v4.1 的覆盖维度更完整

**证据不足的结论**（⚠️）：
- "Graph Gate 作为唯一权威门禁标准" — 缺少 False Positive 数据

---

## 2. Gate Effectiveness Matrix

### 2.1 定义

| 指标 | 定义 | 如何测量 |
|------|------|----------|
| TP (True Positive) | Graph 发现，Legacy 也发现 | CI 中两者都 FAIL |
| FP (False Positive) | Graph FAIL，Legacy PASS，但实际无缺陷 | 需人工验证 FAIL 案例 |
| FN (False Negative) | Legacy 发现，Graph 未发现 | Graph PASS 但 Legacy FAIL |
| TN (True Negative) | 两者都 PASS，实际无缺陷 | 人工验证 PASS 案例 |

### 2.2 当前 Matrix（基于历史数据推断）

| 维度 | Legacy TP | Legacy FP | Legacy FN | 说明 |
|------|-----------|-----------|-----------|------|
| 单元测试 (L1) | ✅ 高 | ❌ 低 | ❌ 低 | Legacy 的 cargo test 能发现大部分问题 |
| 执行一致性 (L2) | ❌ 0 | N/A | N/A | Legacy 完全缺失此维度 |
| ACID 验证 (L3) | ❌ 0 | N/A | N/A | Legacy 完全缺失此维度 |
| Crash Recovery | ❌ 0 | N/A | N/A | Legacy 完全缺失此维度 |
| 性能回归 (L5) | 🟡 部分 | 🟡 未知 | 🟡 未知 | Legacy 只测 SF=0.1，Graph 测 QPS+24h |
| 文档规范 (L6) | 🟡 部分 | 🟡 未知 | 🟡 未知 | Legacy 无文档检查 |

### 2.3 当前缺失数据

**需要收集才能填满 Matrix 的数据**：

| 待收集数据 | 来源 | 方法 |
|-----------|------|------|
| FP 案例列表 | Graph FAIL + Legacy PASS 的 CI runs | 分析 3187+ runs 的 jobs 状态 |
| FN 案例列表 | Graph PASS + Legacy FAIL 的 CI runs | 需历史数据 |
| Evidence 链缺失案例 | Graph FAIL 但无 evidence 链 | 检查 evidence-graph DB |
| MTTD (Mean Time to Detect) | 从 Commit 到 Gate FAIL 的时间差 | 分析 runs 的 created_at vs completed_at |

---

## 3. Reconciliation Gate 设计规范

### 3.1 职责

Reconciliation Gate 是独立的第三层门禁，负责：

1. **检测 disagreement**：Graph PASS / Legacy FAIL 或反之
2. **分类 drift 类型**：是 Graph FP、Legacy FN 还是 evidence 链缺失
3. **生成 drift 告警**：当 drift 率超过阈值时阻断发布
4. **审计 Graph Gate 自身**：防止 Graph 成为新的权威造假源

### 3.2 架构

```
Governance
├── Legacy Gate (v3.x Shell Scripts)
├── Graph Gate v4.1 (Evidence Graph)
│   └── Rule G-01/G-02/G-03
└── Reconciliation Gate (NEW)
    ├── Disagreement Detector
    ├── Drift Classifier
    └── Meta-Audit
```

### 3.3 触发条件

Reconciliation Gate 在以下情况触发：

| 条件 | 说明 |
|------|------|
| Graph PASS + Legacy FAIL | Legacy 发现问题，Graph 未发现 → Graph FN |
| Graph FAIL + Legacy PASS | Graph 发现问题，Legacy 未发现 → Graph TP 或 FP |
| Graph FAIL + Evidence 链不完整 | Evidence 链缺失 → 降级为 UNVERIFIED |
| Drift 率 > 20% | 过去 10 次 CI 中 disagreement 超过 2 次 |

### 3.4 硬规则 (Rule G-01 至 G-03)

#### Rule G-01: Evidence Required for PASS

```text
任何 Gate PASS 结论必须可追溯到 Evidence Graph
否则 → UNVERIFIED（不是 PASS）
```

**实现**：在 gate CLI 中增加 `--require-evidence` 标志

#### Rule G-02: Test Result Binding

```text
任何测试结果必须绑定：
  Test Case → Commit → CI Run → Artifact
否则 → UNVERIFIED
```

**实现**：evidence-graph 库的 `ingest test-result` 命令

#### Rule G-03: No Evidence = Block

```text
无法证明 ≠ 通过
必须变成：
  无法证明 → UNVERIFIED → 阻断发布
```

**实现**：gate evaluate 输出 UNVERIFIED 而非 PASS

### 3.5 实施阶段

| 阶段 | 内容 | 目标 |
|------|------|------|
| Phase 1 | Reconciliation Gate 实现（独立 workflow） | 检测所有 disagreement |
| Phase 2 | Rule G-01/G-02/G-03 硬编码到 gate CLI | 防止 evidence 缺失时误判 PASS |
| Phase 3 | Meta-Audit 报告生成 | 每月生成 drift 统计报告 |
| Phase 4 | 条件性权威化 | 20+ 次 CI 数据支撑后，Graph → Authoritative |

---

## 4. 当前 CI 实际数据分析

### 4.1 Run #3187 (latest completed)

| Job | Status | Conclusion | 说明 |
|-----|--------|------------|------|
| evidence-graph-gate | completed | failure | Checkout GITEA_SHA 为空导致失败 |
| legacy-gate | completed | failure | 同上，两个 job 失败原因相同 |

### 4.2 Run #3188 (in_progress)

| Job | Status | Conclusion | 说明 |
|-----|--------|------------|------|
| evidence-graph-gate | in_progress | — | 正在执行，修复了 GITEA_SHA 问题 |
| legacy-gate | in_progress | — | 同上 |

### 4.3 历史问题推断

基于已完成的 runs (3182-3187 全部 failed)，目前没有 PASS 数据可供 Matrix 分析。

---

## 5. 行动项

### P0（立即执行）

| 行动 | 负责方 | 状态 |
|------|--------|------|
| 修复 gate.yml GITEA_SHA 问题 | Hermes | 进行中 |
| 收集第一批 PASS/PFAIL 数据 | CI | 待 runner 正常后 |
| Reconciliation Gate 实现 | Hermes | 待开始 |

### P1（1 周内）

| 行动 | 说明 |
|------|------|
| 补充 Rule G-01/G-02/G-03 到 gate CLI | 见 3.4 |
| 建立 Meta-Audit 报告模板 | docs/releases/v3.8.0/META_AUDIT_TEMPLATE.md |
| 统计 Run #3188+ 的 disagreement 案例 | 等待 CI 正常 |

### P2（观察期）

| 行动 | 触发条件 |
|------|----------|
| Legacy → Advisory Mode | 连续 20 次 CI 无 FN |
| Graph → Authoritative Mode | FP 率 < 10% 且 drift 率 < 20% |

---

## 6. 结论

**当前阶段判断**：

- Legacy Gate **不是**合适的唯一门禁标准（证据充分）
- Graph Gate v4.1 **还不是**已验证的权威门禁标准（证据不足）
- 两者**并行运行 + Reconciliation Gate** 是当前最优解

**下一步**：

Reconciliation Gate 实现 → Rule G-01/02/03 硬编码 → 收集 20+ CI 数据 → 重新评估权威化条件

---

## 附录：Rule G-01/02/03 实现规格

### G-01 实现

```
gate evaluate <task_id> --require-evidence

输出：
  { "result": "UNVERIFIED", "reason": "no_evidence_chain", "missing": [...] }
  而不是：
  { "result": "PASS", ... }
```

### G-02 实现

```
evidence-graph 库新增：
  ingest test-result <test_id> <commit_sha> <ci_run_id> <artifact_id> [--db <path>]

新增 NodeType::TestResult
```

### G-03 实现

```
gate evaluate 在以下情况输出 UNVERIFIED 而非 PASS：
1. Task 节点存在但无完整 evidence 链
2. Evidence 链中断（Task → Commit 缺失，或 Commit → CI 缺失）
3. CI run 是 failure 状态
```