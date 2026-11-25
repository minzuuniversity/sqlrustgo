# 迁移顺序、冻结点与安全回退

source_agent=Codex GPT-6.1 Sol；source_run=RUNTIME-5915-20261120；timestamp=2026-11-20（Asia/Shanghai）；evidence_hash=sha256:57fe60174b4642317a534de402027e42603ba976601a8ed1ff4916cb9fb7df2a；conflict_resolution=Owner待批的统一技术契约；未实施、未派发、未签产品验收。

| 顺序 | 交付边界 | 证伪出口/冻结点 | 不得跨越 |
|---|---|---|---|
| R0 证据与Owner审定 | 固定e9ff/56cfa/45da、本包hash及不变量；分开clean reopen与crash | #5915原6项及身份/故障缺口登记，Owner批准ADR digest | 不能把PR测试存在当核心已修 |
| R1 隔离参考垂直切片 | 在隔离build同时做TxContext、RowId、private workspace、唯一Undo、语句/savepoint和读边界 | Memory/File分别通过多重集+ID+peer commit/失败；语义版本冻结 | 不先删除Engine Undo再猜Storage覆盖；不混用新旧协议 |
| R2 提交/持久化参考切片 | 单WalService、两commit入口、graded ACK、CommittedPendingPublish、NO-STEAL snapshot/manifest | 真实sync及进程故障出口，格式/stream版本冻结 | 未durable不称同步ACK；不得以RAII替代crash证据 |
| R3 Recovery/MVCC/索引组合 | snapshot CP、事务frame过滤、身份级redo幂等、投影恢复 | SIGKILL/恢复失败/重复值/重放两次、publish gap/consumer floor | 不清空已截断日志之外的可信基线 |
| R4 全入口与后端适配 | REPLACE/ODKU/Trigger/多表/LOAD DATA/聚类/外部消费者 | 每承诺装配有真实客户端及第N步失败，禁止旁路 | 不因能力不足自动删承诺功能 |
| R5 数据副本迁移与认证 | 旧JSON/delta/WAL→新RowId/generation/日志协议，冻结组合态 | bag与ID映射/历史升级/备份恢复/PITR及正式CI | 无Owner许可不写生产数据/不创建tag/不晋级 |

R1的身份、private workspace、单Undo与savepoint是一个必要协议单元，可拆小PR审查但不能作为混合半状态发布。R2/R3可隔离设计，只有组合态验证满足后才能称具备相应durability。参考资产可复用，P2原型整体不可直接cherry-pick为生产实现。

## 数据迁移

1. 保留原始文件、WAL、schema和hash只读副本；停止切换中的写入，Owner批准资源窗口/RPO。当前不执行这些步骤。
2. 先证明旧基线哪些行已提交。旧文件缺事务来源、日志被截断或存在脏snapshot，标LEGACY_BASELINE_UNVERIFIED；需要可信备份、完整保留日志或业务oracle，不能猜。
3. 在新目录为每个行**出现**分配独立RowId并持久化映射。相同行值不可合并；原始重复PK/约束冲突如实隔离报告，不为迁移成功偷偷去重。
4. Legacy WAL在专用兼容重建中解析；无边界/缺身份的歧义单列，不把“无BEGIN”自动当committed。只有可证明完整时才从空临时库重建，禁止在原目录清表。
5. 新数据/身份sidecar、manifest与WAL epoch需共同校验及可靠同步；版本与格式不一致拒绝启动。验证oracle后才允许原子切换，保留旧代和可验证备份。
6. Replication/Binlog/PITR/Backup的身份和日志floor纳入同一升级；消费者未追上不得回收其所需日志。

## 回退

- 尚未作commit决定，且能证明没有有效commit record/对外发布时：可丢弃或回滚本事务workspace，committed主账不变；失败仅AbortOnly/RecoveryRequired，不报告成功。commit append/sync结果不确定，或完整commit frame已写但未sync时，不能仅因“尚未durable”就回滚并报告确定未提交；必须走OutcomeUnknown/结果查询/恢复协议。
- Commit决定已durable：禁止回滚成“未提交”；保留WAL与结果身份，恢复完成publish后查询结果，避免客户端重复提交。
- 新格式未切换：丢弃本次新目录，回到已验证旧副本（若确有可信副本）；原始材料保持可恢复。
- 新格式已写入/切换：旧程序必须拒绝打开不兼容代，需已验证逆迁移或恢复备份；仅切二进制不构成数据回退。RPO/RTO和潜在业务数据损失由Owner批准。
- 没有满足不变量的旧基线时：停止写入/隔离实例，保留所有数据和日志并人工恢复。**旧双Undo路径绝不是可发布fallback**。离线A/B对照必须使用独立二进制/数据，不能双写生产后再择一。

正式M-Pre关闭、M0接口实现、M1–M6推进、预算和阶段仍按原Owner/DSH门禁。此顺序是规划，不新增Worker任务或自动许可。
