# ADR索引

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

十份ADR均PROPOSED，approved_adr_refs=[]。Owner原则接受C、组织职责已裁决；不自动批准具体接口、隔离、格式和功能变更。批准对象是文件SHA256及关联不变量/测试出口，内容一改须重新审。

| ADR | 主题 | 状态 |
|---|---|---|
| [ADR-410-001](adrs/ADR-410-001.md) | 唯一协调器与TxStore | PROPOSED |
| [ADR-410-002](adrs/ADR-410-002.md) | 身份与命名空间 | PROPOSED |
| [ADR-410-003](adrs/ADR-410-003.md) | 语句原子性与Savepoint | PROPOSED |
| [ADR-410-004](adrs/ADR-410-004.md) | 事务状态与Typed Error | PROPOSED |
| [ADR-410-005](adrs/ADR-410-005.md) | NO-STEAL与大事务资源 | PROPOSED |
| [ADR-410-006](adrs/ADR-410-006.md) | Commit、Durable Frontier与ACK | PROPOSED |
| [ADR-410-007](adrs/ADR-410-007.md) | Checkpoint、Manifest与WAL Reclaim | PROPOSED |
| [ADR-410-008](adrs/ADR-410-008.md) | MVCC、隔离与索引发布 | PROPOSED |
| [ADR-410-009](adrs/ADR-410-009.md) | StorageCapabilities与支持矩阵 | PROPOSED |
| [ADR-410-010](adrs/ADR-410-010.md) | 格式、迁移与外部消费者 | PROPOSED |

依赖：001/002→003/004；005/006/007组成持久化约束；008依赖002/006；009组合全部；010依赖005/007并反馈002。参考实现只在契约digest批准后安排，不用ADR数量当批准完成。
