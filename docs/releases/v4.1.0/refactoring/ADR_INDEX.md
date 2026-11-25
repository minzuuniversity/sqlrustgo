# ADR索引

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai) · 档位落地：2026-11-23（依 ADR-DECISION-RECORD.md:35-47）
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）+ sha256:eab48506c4fc70aa402330fa02e7740d0544e0682c57e0f1842773e7173a00da（属主集中裁决账本 ADR-DECISION-RECORD.md）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文档位以状态列为准，计划交付不等于任务授权或产品认证。

11 份 ADR：001-010 + 011（条件批准，前置未满足）；011 已并入编目但不进入 `approved_adr_refs`。档位依属主 2026-11-21 集中裁决（ADR-DECISION-RECORD.md:35-47）：001/002/005 无条件批准；006/011 条件批准（前置未满足）；003/004/007/008/009/010 仅批准方向、实施另审——方向批准不等于自动批准具体接口、隔离、格式和功能变更。各 ADR 正文的 `Approvals` 记录该 ADR 在依赖图上依赖或合并的 ADR id（不含授权含义），`Approval` 记录裁决路由与绑定摘要。Owner原则接受C、组织职责已裁决；批准对象是文件SHA256及关联不变量/测试出口，内容一改须重新审。

| ADR | 主题 | 状态 |
|---|---|---|
| [ADR-410-001](adrs/ADR-410-001.md) | 唯一协调器与TxStore | APPROVED（无条件批准） |
| [ADR-410-002](adrs/ADR-410-002.md) | 身份与命名空间 | APPROVED（无条件批准） |
| [ADR-410-003](adrs/ADR-410-003.md) | 语句原子性与Savepoint | APPROVED_DIRECTION（仅批准方向、实施另审） |
| [ADR-410-004](adrs/ADR-410-004.md) | 事务状态与Typed Error | APPROVED_DIRECTION（仅批准方向、实施另审） |
| [ADR-410-005](adrs/ADR-410-005.md) | NO-STEAL与大事务资源 | APPROVED（无条件批准） |
| [ADR-410-006](adrs/ADR-410-006.md) | Commit、Durable Frontier与ACK | CONDITIONALLY_APPROVED（五窗口故障语义表冻结前不授权实现） |
| [ADR-410-007](adrs/ADR-410-007.md) | Checkpoint、Manifest与WAL Reclaim | APPROVED_DIRECTION（仅批准方向、实施另审） |
| [ADR-410-008](adrs/ADR-410-008.md) | MVCC、隔离与索引发布 | APPROVED_DIRECTION（仅批准方向、实施另审） |
| [ADR-410-009](adrs/ADR-410-009.md) | StorageCapabilities与支持矩阵 | APPROVED_DIRECTION（仅批准方向、实施另审） |
| [ADR-410-010](adrs/ADR-410-010.md) | 格式、迁移与外部消费者 | APPROVED_DIRECTION（仅批准方向、实施另审） |
| [ADR-410-011](adrs/ADR-410-011.md) | 稳定身份、唯一恢复与提交/恢复统一契约 | CONDITIONALLY_APPROVED（7 项补充验收契约冻结前不授权实现） |

依赖：001/002→003/004；005/006/007组成持久化约束；008依赖002/006；009组合全部；010依赖005/007并反馈002。参考实现只在契约digest批准后安排，不用ADR数量当批准完成。
