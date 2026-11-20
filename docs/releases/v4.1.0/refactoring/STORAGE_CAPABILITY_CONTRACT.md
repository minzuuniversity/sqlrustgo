# Storage能力与语义接口草案

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

能力采用独立维度：statement_atomic、transactions、savepoints、isolation_set、durable_commit_modes、namespace、recovery_protocol_version、mutation_protocol_version。六leaf（Memory/File/Binary/BinaryV2/AppendOnly/TableLevel）和五wrapper（Wal/ParallelWal/Mvcc/Box/Vtu）均逐组合认证，不能把BinaryV2误列为wrapper。

当前declared与verified见冻结HARA05。File有undo但DEFECTIVE；Memory部分路径；Binary/BinaryV2/AppendOnly/TableLevel事务空成功；Wal/Parallel的提交协议不同；Mvcc可见性/inner合并需联合断言。没有把实验后端从产品支持中自行删掉。

目标语义：begin(requested_caps,isolation)->TxHandle；scan(ReadContext,TableId)；apply(TxHandle,MutationBatch)->Receipt；savepoint->UndoCursor；rollback_to/abort->Progress或RecoveryRequired；prepare->PreparedCommit；commit->CommitReceipt(commit_lsn,durable_lsn,outcome)；checkpoint->DurableSnapshotCertificate；reclaim(consumer_floors)；recover->VerifiedBaseline或RecoveryRequired。

显式TxContext携带db/table/tx/epoch/snapshot。Typed Error区分UnsupportedBeforeWrite、ConflictBeforeDecision、AbortOnly、IOError、OutcomeUnknown、RecoveryRequired。Unsupported不是运行时失败的统称，禁止is_ok触发fallback吞错误。

跨数据库/跨表事务建议同一TxStore协调，不在namespace切换时隐式退化；范围与隔离等级由Owner审定。RowId初期可epoch内，但重启不持有旧undo引用且NO-STEAL已证才允许；若需要跨重启stable identity或steal，则ADR-010决定格式。
