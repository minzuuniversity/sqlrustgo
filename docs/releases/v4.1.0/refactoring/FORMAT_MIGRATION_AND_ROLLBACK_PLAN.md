# 格式迁移与安全回退

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

优先保留row JSON payload，新增generation manifest/WAL sidecar是待审方案，不承诺零格式变化。ADR-005/007/010必须共同决定committed-only snapshot、fsync顺序、directory durability、segment原子切换、格式版本与consumer floor。

基线夹具用v3.11、v3.12、e9ff历史commit各自生成，隔离数据目录和toolchain/依赖锁；原HARA仅一条共同SQL与API，不能当迁移认证。必须核对JSON/base/delta/WAL checksum、相同行值多重集、重复redo/部分WAL、named DB、索引/聚类重建、Binlog/Replication/PITR与Backup/Restore。

原始数据副本只读，迁移写到新generation目录；验证完才允许原子切换manifest。新格式写入后旧程序拒绝打开，不是自动降级。回退只允许恢复已验证旧备份/兼容generation，需Owner接受RPO；旧双undo不能作为发布fallback。若没有任何满足不变量的旧版本，则停止写入并保留所有日志，而不是回退到已知错路径。

公开API不兼容、持久RowId/steal/磁盘升级、后端能力缩减、升级/回退RPO分别Owner签字；当前全部未批准。
