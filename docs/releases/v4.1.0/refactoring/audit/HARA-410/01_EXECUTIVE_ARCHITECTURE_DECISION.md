# SQLRustGo v4.1.0 · HARA-410 架构裁决建议

source_agent=Codex GPT-6.1 Sol；source_run=HARA-410-20261120；timestamp=2026-11-20（Asia/Shanghai）；evidence_hash=sha256:be201d57c2a91d6edf7926665c27e508dfc1bffb67c3294fa811751f33bd8cdd（独立反例run6）；conflict_resolution=基于冻结源码和本轮运行的独立建议，正式架构/功能/格式/阶段裁决归属主。所有产物hash另见evidence/MANIFEST.json；本报告非发布合格证。

## 首要结论

当前事务与存储架构不自洽，必须重构。推荐 **C：重设Storage/Transaction核心契约，保留SQL前端与可复用算法，分阶段收敛行存储、提交与恢复协议**。B+的单undo/slot/能力声明是必要组成，但不足以覆盖已实证的未提交snapshot、commit同步差异和恢复失败状态。没有证据要求D式重建全部内核。

九份报告及证据已形成可评审交付；**全范围运行验证仍INCOMPLETE**，具体NOT-RUN/UNREAD见09。不能把交付文档等同所有后端、隔离、断电和迁移已通过，也不自行签发独立验收。

## 基线与事实漂移

- 审计冻结远端develop/v4.1.0=`e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0`，tree=`4194fc975ebaf749de477f42ca5a300facef1278`；STAGE.yaml:201=ALPHA。初始用户工作树是fix/tpch-fixture-handoff-20261117@cd8cefe623，不能用作远端开发线代称。
- Gitea采集时点：open issues=104，priority/p0=37，priority/p1=51，open PR=21。06按ID集合对账，母项与产品缺陷不混算关闭收益。
- 本轮Cargo.toml:61是4.1.0，AGENTS入口仍称3.12.0-fix-zombie为保留口径；这是入口漂移，未擅自改文档。模型/索引数量等入口数据也必须实测，不能照抄。
- 最近正式Gitea Release条目为v3.11.0-ga；更晚v3.12.0-ga有tag无同名Release，v4.0 GA已撤销。v3.11 release target/body/live tag不一致，整树不同但所核内核子树无差异；无release assets，未证明二进制等价或tag何时漂移。
- 初始工作树仅.gitignore脏；审计期间另观察到AGENTS.md、.aoci/baseline.json和AGENTS备份变化，来源未归因。本轮未调用索引重建、未写这些文件；独立冻结工作树避开了这些变化。

## 最关键的五类架构问题

| 问题 | 本轮证据 | 工程含义 |
|---|---|---|
| 恢复所有者与行身份分裂 | DELETE→ROLLBACK两行；flush后INSERT undo残留；REPLACE/ODKU回滚原行消失 | 唯一TxStore、稳定RowId、统一statement/savepoint/abort，不是按值补幂等 |
| 未提交状态越过持久化边界 | 无Drop子进程退出后，recovery报告incomplete=1却读到该行 | 选择committed-only snapshot或完整loser undo，redo过滤单独不成立 |
| COMMIT/checkpoint/reclaim不是一个协议 | Every下&mut sync=1、lockfree sync=0；snapshot只flush、WAL先清空再重写 | durable_lsn与generation证书必须成为接口；断电后果尚未实跑 |
| 错误与终态脱节 | undo回调失败仍Ok；recover首次Err后第二次空Ok | AbortOnly/RecoveryRequired、保留cursor、提交结果区分OutcomeUnknown |
| 后端/写路径组合不一致 | AppendOnly假回滚、BinaryV2空成功；聚类与可选BufferPool回滚残留 | 全后端显式能力认证，聚类/缓存由统一数据主账发布 |

细节及每项“触发→不变量→后果→反例→修复边界→验证”见04。观察程序正常退出只说明执行结束，expected/actual不符就是反例，未把它计成PASS。

## 必答决策

| 问题 | 建议 |
|---|---|
| 是否必须重构 | 是，不能依靠单条Issue补丁证明全事务/恢复契约 |
| B+、C、D | 选C，范围限于必要核心接口与装配；保留B+中有价值的undo/identity设计 |
| 必须重构的子系统 | TxCoordinator/StorageEngine、File/Memory事务与RowId、MVCC发布、WAL/Parallel commit、checkpoint/truncate/recovery、复合DML统一mutation |
| 只需适配/局部修复 | SQL前端/优化器/执行算法主体可保留；后端wrapper、Trigger/LOAD DATA、索引/聚类、binlog/PITR/replication需契约适配与独立认证 |
| Storage trait是否必须变 | 是：显式TxContext、能力组合、typed错误、统一commit/abort/savepoint、持久化frontier；不能仅新增bool |
| 磁盘格式是否必须变 | 未证明必须改用户表row payload；必须重定义持久化协议，可能新增版本化manifest/WAL sidecar。不得先承诺完全零格式影响 |
| 可保留的历史设计 | WAL manager与Recovery分层、单shared WAL handle、显式db/tx、行存储/索引算法、已有SQL和失败反例；不恢复任一旧版本为可发布回退 |
| 可否结束M-Pre | 否：隔离/完整装配恢复/迁移及独立回执尚未闭合 |
| 可否进入M0 | 仅建议属主先批准C的契约设计评审；当前不建议授权正式核心编码或合并原型 |
| 安全退出ALPHA | 冻结C协议与支持矩阵；逐P0反例、真实客户端/崩溃/迁移/隔离及现有阶段门禁实跑，独立Reviewer和属主按SSOT批准 |

## 运行与历史证据摘要

本轮独立构建运行当前核心probe，含真实parser/ExecutionEngine及Storage API；v3.12.0同flush反例复现，说明不是仅v4.1回归。共同SQL序列在v3.11.0-ga回滚后空表、当前重复行；v3.12.0在BEGIN报错中断，后续回滚NOT-RUN。历史版本并没有提供一个已证明正确、可直接恢复的事务内核。

当前短反例不是全量cargo test、生产认证SOAK或断电验证。真实MySQL rollback_undo_test目标本轮6项实际通过、0 failed、0 ignored，见evidence/wire-rollback-full.json（sha256:c2987c3a5d00252cde5066b867c18ead1bdeacded54d67562b5a0cb0e81390e8，timestamp=2026-11-20，source_run同本报告）。这与直连FileStorage反例是不同装配，不能将其失败直接推广成默认wire相同场景也失败；六项正常也不证明flush/物理行多重集/断电正确。未对任何正式代码作修复；未push/merge/关闭Issue/改tag/阶段/权限/SOAK。

## 交付导航与下一决定

1. [02 历史架构](02_HISTORICAL_ARCHITECTURE_EVOLUTION.md)：真实tag/tree、演进、跨版本反例。
2. [03 当前内核](03_CURRENT_KERNEL_ARCHITECTURE.md)：主账、调用链、事务与恢复。
3. [04 正确性](04_CORRECTNESS_AND_FAILURE_ANALYSIS.md)：F01–F11及精确验证边界。
4. [05 能力矩阵](05_STORAGE_AND_DML_COVERAGE_MATRIX.md)：后端、DML、未认证组合。
5. [06 Issue对账](06_ROOT_CAUSE_AND_ISSUE_MAPPING.md)：P0/P1集合、根因与关闭条件。
6. [07 路线比较](07_REFACTOR_OPTIONS_AND_DECISION.md)：选C及改变建议的证据条件。
7. [08 实施路线](08_REFACTOR_IMPLEMENTATION_ROADMAP.md)：目标接口、状态机、PR依赖、出口与冻结点。
8. [09 独立复核](09_INDEPENDENT_REVIEW_AND_EVIDENCE_GAPS.md)：与ARR分歧、未闭合项、属主决策。

需要属主作具体选择：是否接受C边界；隔离/后端功能承诺；是否允许持久化协议sidecar及迁移评审；M-Pre补证和独立Reviewer职责。报告交付后停止，不自行实施重构。
