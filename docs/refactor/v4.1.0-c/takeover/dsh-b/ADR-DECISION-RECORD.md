# ADR-DECISION-RECORD — 属主集中裁决绑定账本（v7.8-B r7）

- source_agent: DSH-B (opencode-go/glm-5.3-flash)
- source_run: v78-r7-decision-record
- timestamp: 2026-11-21T01:30+08:00
- 裁决来源：属主 2026-11-21 01:28 集中裁决（OWNER-DECISION 已入 Bus，correlation=OWNER-DECISION-C-ROUTE）
- **绑定关系（属主要求的正式性条件）**：

| 对象 | 绑定引用 |
|---|---|
| ADR-410-001~010 原文 | commit `45da6968ae2baaade6a31206eeee5854a8514ee2`，manifest `sha256:c722885fef98a93ec0eae8157104861fffdeaa1454bf4710bd12464c25ab76de`（DSH-B 预检 121/121 VERIFIED） |
| ADR-410-011 原文 | PR #5917 head commit `b681b74ca3`（branch codex/v410-runtime-5915，note：011 在 45da 交接包 manifest **之外**，属后续运行时证据提交——绑定以其自身文件为准，Codex 重签时应并入） |
| 裁决基础声明 | 属主自述未独立实读 11 份全文，裁决基于集中决策包 → **绑定式批准**：批准指向固定 commit 的正文，非摘要 |

## 一、摘要 ↔ 正文一致性核查（DSH-B 已实读全部 11 份关键章节）

| ADR | 属主裁决 | 正文推荐（DSH-B 实读） | 一致性 |
|---|---|---|---|
| 011 | 条件批准·收敛方案 | 选择：TxStore 唯一主账+私有 workspace+committed-only+显式协议 | ✅ 一致 |
| 001 | 批准 C | 推荐 C | ✅ |
| 002 | 批准 C | 推荐 C（含 RowId 持久化子项） | ✅ |
| 003 | 批准方向 C | 推荐 C | ✅ |
| 004 | 批准方向 C | 推荐 C | ✅ |
| 005 | 批准 B | 推荐 B | ✅ |
| 006 | 条件批准 C | 推荐 C | ✅ |
| 007 | 批准方向 B | 推荐 B | ✅ |
| 008 | 批准方向 B | 推荐 B | ✅ |
| 009 | 批准方向 C | 推荐 C | ✅ |
| 010 | 批准方向 B | 推荐 B | ✅ |

**结论：未发现摘要与正文的实质冲突，11 份均可按绑定式批准记为有效。** 若后续实读发现偏差，按属主规则提交差异裁决，不得自行解释。

## 二、裁决登记（含条件）

| ADR | 裁决 | 生效条件 |
|---|---|---|
| 011 | **条件批准** | Codex 冻结 7 项补充验收契约（durable 后不可普通回滚/durable-publish 间恢复/publish-ACK 间不重复/UNKNOWN_OUTCOME 幂等/RowId 生命周期/主数据-索引-可见性一致/Checkpoint-Recovery 边界）→ 冻结完成前不授权关联核心实现 |
| 001 | 批准 C | — |
| 002 | 批准 C | **含必决子项已决**：已提交 RowId 跨重启持久化=批准；RowId 须定义作用域/生命周期/分配持久化/复用约束；持久化本身不构成 Redo 幂等充分条件 |
| 003 | 批准方向 C | 具体契约满足阶段门禁 |
| 004 | 批准方向 C | 同上 |
| 005 | 批准 B | — |
| 006 | **条件批准 C** | 五窗口故障语义表（durable 前崩溃不得恢复为已提交/durable 后 publish 前崩溃须恢复/publish 后 ACK 前不重复/ACK 后同步不丢/UNKNOWN_OUTCOME 不得误报已回滚）冻结前不授权实现；**durable decision 成立后不得因 publish 失败判普通回滚** |
| 007 | 批准方向 B | 实施另审 |
| 008 | 批准方向 B | 同上 |
| 009 | 批准方向 C | 同上 |
| 010 | 批准方向 B | 格式迁移实施另行审批 |

## 三、预算登记（条件授权）

- 总 2,570,000（b1 400k / b2 300k / b3 1.2M / b4 670k）；云端 0；写入并发 1；整改 ≤2 轮
- **口径**：此前实际消耗（本地 ~20k）如实累计，不重置不重复计算
- **生效条件**：DSH-A 将批准范围持久化至运行态授权数据库（scheduler.db/delegations）后方可在派发时扣减；禁止批量清除 Human Gate；批次间只保留真实 DAG/预算/验收依赖

## 四、Gates 登记状态（r7 时点）

| Gate | 责任 | 状态 |
|---|---|---|
| G-OWNER-TAKEOVER | Owner | ✅ 本次裁决即属主确认（待 DSH-A 核验签发） |
| G-LOCAL-ONLY/G5 | DSH-A 实测 | ⏳ 进行中 |
| G-POLICY-BINDING | DSH-A | ⏳ |
| G-ISOLATED-CRASH-WINDOW | DSH-A/B | ⏳（DSH-B 配合故障注入） |
| M-Pre | DSH-B | ⏳ 逐条消除（本轮起为我首要任务） |
| 命名错位 | Codex | ⏳ 修订重签 |
| Reviewer 调度 | DSH-A | ⏳ 不等属主 |

**当前总状态：条件授权成立，执行仍待运行态授权持久化 + 各 Gate 逐项通过。C-ALPHA-02~05/M1-M6/合并/迁移/跃迁继续 HUMAN_GATE。**

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
