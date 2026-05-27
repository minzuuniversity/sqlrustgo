# SQLRustGo Governance System

SQLRustGo v2.8.0 的治理模型是一个形式化规则系统，确保数据库引擎的"绝对可信"。

## 核心规则 (R1-R7)

| 规则 | 名称 | 描述 |
|------|------|------|
| R1 | 测试不可变性 | 已通过的测试不能改变 |
| R2 | Ignore 防止 | 不能通过 `#[ignore]` 绕过失败测试 |
| R3 | 证明真实性 | 必须提供证据，无证据 = FAIL |
| R4 | 全量执行 | 所有测试必须完整运行，不能缩减 |
| R5 | 基线验证 | v2.8.0 baseline 必须 226 PASS, 0 FAIL, 0 IGNORED |
| R6 | 测试计数单调性 | 测试数量不能减少 |
| R7 | CI 完整性 | CI 流程必须完整执行所有规则 |

## 攻击面 (Attack Surface)

AV1-AV9，按严重程度:

**CRITICAL (3)**:
- AV3: 语义正确性差距 — 测试验证行为，不验证语义
- AV4: 证明状态不一致 — UNVERIFIED 证明被忽略
- AV5: 阻塞项未解决 — blockers 中有未完成的条目

**HIGH (3)**:
- AV1: 测试覆盖盲区 — 未测试的代码路径
- AV2: 回归检测缺失 — 没有自动化回归检测
- AV7: CI 状态漂移 — CI 状态与实际测试状态不一致

**MEDIUM (2)**: AV6, AV8
**LOW (1)**: AV9

## 关键限制 L1

L1 (Semantic Correctness Gap): 测试验证行为，但不验证语义正确性。这是目前治理系统最根本的限制。路线图计划 v2.9.0 用 GBrain 语义验证解决。

## 路线图

| 版本 | 目标 |
|------|------|
| v2.9.0 | GBrain 语义验证，RBAC |
| v3.0.0 | LLM-Wiki 集成 |

## 用户工作风格 (STRICT PROOF MODE)

首席架构师要求:
1. 规则优先于编码 — "先治理规则，再动手开发"
2. 严格证明模式 — 假设一切可疑，必须提供证据
3. 形式化输出 — ∀, ∃, invariant, IF/THEN 规则
4. Release Judge 角色 — ALLOW/BLOCK 仅判定，不优化

## 相关页面

- [sqlrustgo](../wiki/sqlrustgo) — 被治理的数据库引擎项目
- [proof-system](../wiki/proof-system) — 证明注册表是治理的执行机制
- [baseline-verification](../wiki/baseline-verification) — R5 的具体实现