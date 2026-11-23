# PR #5915 · 统一事务/恢复契约裁决包

source_agent=Codex GPT-6.1 Sol；source_run=RUNTIME-5915-20261120；timestamp=2026-11-20（Asia/Shanghai）；evidence_hash=sha256:57fe60174b4642317a534de402027e42603ba976601a8ed1ff4916cb9fb7df2a（独立复跑元数据）。conflict_resolution=Owner批准前为PROPOSED技术裁决建议，不是实施授权或最终独立合格证。

**技术建议维持C路线：稳定RowId、TxStore唯一Undo、事务私有workspace、committed-only snapshot，以及统一WAL/提交/恢复协议必须共同成立。** “只删Engine Undo”“只加slot”“只让SELECT过滤pending”均不足以关闭这些根因。

本轮在PR head `56cfa222f1450975d3b3adc6ab9c36a4d258a5f6` 独立复跑两个目标的6项：**4 FAIL、2 PASS、0 ignored**，整体exit=101。命令见[独立复跑](evidence/independent-pr5915.json)；[原始日志字节](evidence/independent-pr5915.raw.json)以无损base64封装，解码后SHA256与元数据evidence_hash相同，[文本预览](evidence/independent-pr5915.preview.log)不作为原始字节证据。没有修改断言、核心源码或派发Worker。PR #5915仍open、未合并，不能称已集成。

**INV-07b没有真实进程崩溃证据。** 它在同一测试进程flush→正常释放对象→FileStorage重新加载，未创建/强杀子进程、未调用RecoveryEngine、未启用WAL。成立的是“未提交行进入重开可读的文件基线”，而非SIGKILL/断电或WAL恢复认证。INV-07c也只是正常清理路径对照。

| 统一裁决 | 结论 |
|---|---|
| RowId | 不依赖PK、值、下标；跨UPDATE/移动/commit稳定；建议新持久化协议保存RowId，格式方案由Owner批准 |
| Undo | 顶层rollback与savepoint共用TxStore唯一journal/cursor；TM/Engine不得再用SQL作第二次物理恢复 |
| 未提交状态 | 独立TxWorkspace（可隔离spool）物理上不进入committed RowStore、快照、索引发布或备份 |
| 提交顺序 | prepare/validate→完整事务WAL→模式要求的durable frontier→原子发布→分级ACK |
| Recovery | 只加载可信committed snapshot，重放完整已提交事务；丢弃loser workspace；不能靠“过滤redo”清除脏快照 |
| 职责 | TM统一状态/冲突/决定；TxStore唯一mutation/undo/发布；Backend做可靠IO；WalService唯一LSN/fsync |

阅读：[证据复核](01_EVIDENCE_ADJUDICATION.md) → [统一ADR](../../adrs/ADR-410-011.md) → [迁移与回退](03_MIGRATION_AND_ROLLBACK.md) → [验收矩阵](04_COUNTEREXAMPLE_ACCEPTANCE_MATRIX.json) → [Owner待批项](05_OWNER_APPROVAL_REQUEST.md) → [DSH交接](handoff/00_HANDOFF_README.md)。

原45da6968ae/C722 manifest与十份PROPOSED ADR原字节保持不变。ADR-011原为待审收敛提案；属主2026-11-21条件批准（7项补充验收契约冻结前不授权实现）后，已由版本化变更并入refactoring/adrs/ADR-410-011.md，001–010的Status/Approvals行按ADR-DECISION-RECORD.md落地，不建立并行生效的架构SSOT。本包不扩大DSH-A/B已有授权；Owner待批项不自动转执行任务。
