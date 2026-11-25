# HARA-410 · 路线比较与决策建议

source_agent=Codex GPT-6.1 Sol；source_run=HARA-410-20261120；timestamp=2026-11-20（Asia/Shanghai）；evidence_hash=sha256:be201d57c2a91d6edf7926665c27e508dfc1bffb67c3294fa811751f33bd8cdd（`evidence/probe-run6.json`）；conflict_resolution=技术建议供属主裁决，未批准接口/功能/格式变更、未签M0或阶段出口。

## 决策

推荐 **C：重设 Storage / Transaction 核心契约，分阶段替换现有行存储与事务装配**。保留SQL前端、计划与表达式框架、可复用的行存储/索引算法和测试资产。没有证据要求D式重建整个内核，也没有证据支持当前B+的“不涉及持久化协议、wrapper直接转发、只补undo”边界。

决定性依据不是文件数或原型失败次数，而是F03/F04/F07/F08：未提交snapshot重启可见、两种commit有不同durability语义、恢复状态错误、checkpoint/truncation缺durable frontier。它们不由slot+单undo自动解决。能力声明、运行期恢复与提交持久化需要一个可组合的端到端协议。

若把B+重新定义为也包含这个协议、typed TxContext、恢复状态机、持久化frontier与全部wrapper适配，它在实质上已是本文的C；争论名称不能缩小工程验收边界。

## 比较

| 维度 | A 局部补丁 | B+ 有限undo重构 | C 核心契约重设（推荐） | D 替换必要内核 |
|---|---|---|---|---|
| 回滚精确一次 | 多点协调持续脆弱 | 可解决，前提完整RowId与入口覆盖 | 统一mutation receipt与tx owner | 也需同样不变量，重写不自动正确 |
| COMMIT ACK / durable_lsn | 容易只修某分支 | 原范围未包含 | 同一个协议，单一durability出口 | 可重建，代价更高 |
| 未提交快照与redo | 补flush guard不能证明peer/checkpoint | 仅内存slot不足 | 明确NO-STEAL committed snapshot或完整loser undo；前者优先 | 同样需选择 |
| Storage API | 继续implicit current_tx_id | 增has_undo/undo_to，旧写入口仍可能绕过 | 显式TxContext、typed结果、能力组合、checkpoint证书 | 全部替换，可保留adapter |
| 代码影响 | 每次小，累计不可界定 | 也触及engine/storage/mvcc/wal及全入口 | 六个leaf+五wrapper+引擎/TM/恢复，接口边界明确 | 更广；新缺陷/资产移植成本未知 |
| 持久化格式 | 无显式变更但可能继续损坏 | 不改格式前提未经崩溃验证 | 优先保留row payload；需版本化checkpoint manifest/WAL兼容边界评审 | 更可能全面迁移，尚无必要性证据 |
| 旧测试资产 | 大量仍可跑，但部分漏生产/失败边界 | U/E/RAII可复用，覆盖不足 | 同样复用，增加恢复、能力与装配oracle | 可复用输入/oracle，接口测试需大量迁移 |
| 可解除问题 | 各表现补丁，难避免复发 | R1部分，持久化阻断未闭合 | R1核心与恢复/提交接口缺口；CI/infra独立 | 不会自动解决CI/infra/治理 |
| 主要风险 | 补丁顺序与配置组合 | 把局部用例当完整事务能力 | 接口迁移期混用两协议；必须freeze与adapter fail-closed | 新代码验证面、数据迁移、执行器重接线 |
| 停止/回退 | 容易回已知错路径，不可发布 | 旧undo只能离线 | 每阶段有契约冻结、拒绝混合协议、独立数据副本 | 需要并存新内核迁移验证 |

## 最小必要边界与影响面

必须重构：`crates/storage/src/engine.rs`（trait/Memory）；`file_storage.rs`（事务私有写集、RowId、undo/flush）；`mvcc_storage.rs`、`mvcc.rs`（行身份/commit visibility）；`wal_storage.rs`、`parallel_wal_storage.rs`（同一提交协议）；`wal/file_backed_wal_manager.rs`、`checkpoint.rs`、`recovery_engine.rs`（持久化/恢复）；`crates/transaction/src/{transaction_manager,savepoint}.rs`；`src/{execution_engine_methods,engine_dml,engine_helpers,savepoint_wiring}.rs`。

需要接口适配并重新认证：BinaryTableStorage、BinaryTableStorageV2、AppendOnlyStorage、TableLevelStorage、BoxStorageEngine、VtuGuard；`mysql-server/src/lib.rs`、load_data装配；`executor/src/trigger.rs`；`src/engine_builder.rs`。不能预先说wrapper“白拿”：当前WalStorage和Parallel本身就增添WAL、active、sync语义，Box也会遗漏新增方法。

保留结构但必须适配：ClusteredTable/B+Tree、二级索引/PK cache、Binary codecs、binlog/replication/PITR。表/索引算法不必因事务接口问题重写；副作用发布必须受TxStore约束。

可保留且无证据要求系统改写：lexer/parser、表达式求值、优化器/计划器主体、wire codec、排序/聚合/join算法。这里表示不在本次必要重构边界，**不是声称这些模块正确性已验证**；局部问题仍按自身Issue修复。

## 格式与兼容性裁决

**Storage trait必须改变语义接口。** 不能仅新增bool或返回默认成功。普通读写的裸表名/implicit current_tx_id入口需要可控适配与最终退役；新代码不得从ambient state推断调用者。

**没有证据证明必须全面改写用户表行payload，也不能批准“磁盘格式完全不改”。** 推荐C以committed-only snapshot作为低风险起点，事务内RowId可只活在进程epoch内；前提是未提交状态从不进入可恢复snapshot，重启后无可继续执行的旧undo引用。F03已经否证现状满足这一前提。

checkpoint generation、durable snapshot frontier、WAL segment 原子代切换至少需要明确的持久化协议；它可能采用新增版本化sidecar而保留旧row JSON。若要保存跨重启RowId、实现steal/loser undo，或保证重复值redo幂等，则需评审新的WAL/行ID编码。是否改变文件字节格式与是否改变持久化协议是两件事。未做迁移实跑，不给“零格式影响”结论。

不支持能力的后端应fail-closed，但将已承诺的事务功能移除属于功能范围变更，必须独立提交属主裁决；不能把reject BEGIN当全部产品验收完成。

## 能改变推荐的最少证据

若要将推荐收窄回B+，至少同时提供：F03在完整默认/parallel栈不可能发生的机制证明及强杀反例；F04两路径同一durable frontier；失败恢复状态机；所有后台flush与checkpoint只发布committed数据；重复值的redo幂等证明；全部后端在不破坏旧接口语义下的认证。单独U1–U5、E9、RAII内存补偿不足以满足。

若FileStorage行组织无法在这些契约下承载NO-STEAL/原子snapshot，或无法为现有格式给出安全迁移与恢复方案，才升级到D的**存储/恢复部分替换**；保留SQL前端。升级需要新的决定性反例与属主选择，不由当前报告自动授权。

## M-Pre 与 M0

M-Pre不能按“只剩bufferpool和checkpoint两项已读”关闭。还缺默认/parallel提交与隔离实测、持久化方案、完整能力认证、跨版本兼容及独立Reviewer签字。当前建议继续M-Pre，允许属主选择C并启动M0的契约设计评审；正式接口编码、批量cherry-pick或重构PR仍未获本报告许可。
