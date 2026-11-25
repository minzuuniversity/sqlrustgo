# C路线分阶段重构主计划

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

目标：用唯一TxCoordinator/TxStore、显式身份、私有workspace与durable frontier收敛内核。范围以HARA 07/08为依据，不整体重写SQL前端。基线e9ff21fca3；发布主线develop/v4.1.0。归档、规划、实现、认证分别建状态。

| 里程碑 | 工程边界 | TF目标（不是验收分母） | 执行授权 |
|---|---|---|---|
| C-ALPHA-01 | M-Pre补证/M0契约/现有TF适配 | 50–80核心场景 | 仅本轮架构准备；DSH启动另批 |
| C-ALPHA-02 | M1 Memory/File参考内核、RowId、单undo、savepoint | 150–200 | CANDIDATE_LOCKED |
| C-ALPHA-03 | M2/M3 WAL/恢复/checkpoint/MVCC/索引组合 | 并发+故障 | CANDIDATE_LOCKED |
| C-ALPHA-04 | M4/M5全DML、后端、格式/外部消费者 | 500–2000差分 | CANDIDATE_LOCKED |
| C-ALPHA-05 | M6可信CI、回归、SOAK、独立认证 | 分层回归 | CANDIDATE_LOCKED |

最小影响：StorageEngine/Memory、FileStorage、TM/Savepoint、MVCC、WAL/Parallel、checkpoint/recovery、engine_dml/helpers/transaction methods。适配六leaf+五wrapper、Trigger/LOAD DATA、聚类/索引、Binlog/Replication/PITR。Parser/planner/表达式/执行算法主体保留，不据此声称其正确性已验。现有#5899/#5902库身份字段、#5903的db部分、#5907门禁框架、P2 U/E/RAII可作候选资产，必须固定最新diff独立复核；不整包cherry-pick，不采用全值删除幂等启发式。

组织：Owner最高批准；Codex设计/关键评审；DSH唯一执行总控；OpenCode主要实施；OMP测试/独立审查/备用实施；KOS知识/证据索引；Hub跨项目观测；Gitea/CI提交与合并事实权威；TF确定性判定。KOS旧task_state不是第二个SQLRustGo调度器。Codex不轮询、不常规派发、不自签自己的独立合格证。

第一批18个有限工作包的native payload在交接03，执行约束单一主账为REFACTOR_TASK_DAG.json。M1–M6只有epic候选，没有可直接派发的核心编码任务。DSH LOCAL_ONLY未成立，包中的可候选普通任务仍不能入live队列。

停止点：ALPHA-01契约不可测则停编码；02参考内核无法正确则停扩大投资；03持久化不成立则停功能适配；04兼容失败重新裁决范围；05认证不足保持ALPHA。所有正式功能删减、隔离/格式/阶段变更单列human gate。
