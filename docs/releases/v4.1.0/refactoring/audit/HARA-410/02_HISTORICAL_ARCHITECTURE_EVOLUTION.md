# HARA-410 · 历史架构演进

审计类型：源码绑定的独立评审，非发布认证。source_agent=Codex GPT-6.1 Sol；source_run=HARA-410-20261120；timestamp=2026-11-20（Asia/Shanghai）。evidence_hash=sha256:3241cd015944ed541a9dd0578ae6a95daf1da536cd7987a42f17efa0be4a26ac（`evidence/source-history.json`）。conflict_resolution=以冻结源码及本轮实跑为准，历史作者声明仅作为意图证据，属主裁决架构路线。

## 真实版本基线

所有下表对象均实际查询；tag 的对象 SHA 与 peeled commit 分开。完整本地 tag 表在 `evidence/history.json`，实时远端 tag 与 release 在 `evidence/gitea-tags.json`、`evidence/gitea-releases.json`。

| 基线 | commit | tree | 选择理由与限制 |
|---|---|---|---|
| v1.0.0 | e415ed97d1a45169b9217516e9ecf6ad88d995d4 | 见 source-history | 早期 JSON 表、独立事务管理器；源码阅读，运行 NOT-RUN |
| v3.11.0-ga | 4efaa74af21ebf755ab6b7a152d390fe8f890756 | 386e1d136265b3d2f0b067a0c6e8660abd95389a | Gitea 最近非 prerelease release 对应 tag；原产物真实性未认证 |
| v3.12.0-ga | cc5e54c8582ec6c41ee2ede6780f7b258789a1c6 | 5fd59fcc822c33472e202f33b01f5ee19370bdbf | 更晚的 GA 标记，本次 API 中没有同名 Release；不等价于已认证正式发布 |
| v4.0.0-final | 0e3cc02bd147c292a57fe47e63bd084ac1b7cddc | b0b97ae4b03d1033665d9c5464c4d58c67555bb5 | 历史代码发布点；当前治理记载 GA 已撤销 |
| v4.1.0 develop | e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0 | `evidence/baseline.json` | 本轮远端冻结基线，STAGE.yaml:201=ALPHA |

VERIFIED-CODE：当前历史有两个 root（21cb64a9、95cf91bf）；仅凭它们不能证明何时、由谁改写历史。v3.11.0-ga 的 Gitea release target_commitish=5038b154…、正文另写 83c623835，而 live tag peeled commit=4efaa74…。三个 tree 并不相同；但 83c623835→tag 对 `crates/ src/ Cargo.toml Cargo.lock` 的 diff 为空，5038→tag 的差异集中在文档、覆盖率及产物删除。故“整树不同”与“内核不同”必须区分。

该 Release 的 assets 为空，无法把任何二进制 checksum 绑定到这三个 tree；发布产物等价性 UNAVAILABLE。没有历史 tag 快照就不能把这些差异定性为本次发现的 tag 漂移。历史 SHA 出现在 tag 注释里也不是内容等价证明。

## 关键演进与契约变化

以下“首次”均限定为当前可达历史、指定路径/特征的最早检出，未经全仓所有重命名溯源，不能宣称项目绝对首次。

| 节点 | 实际变更与新增责任 | 当前影响 |
|---|---|---|
| v1.0.0 | `src/storage/file_storage.rs` 有 tables HashMap、JSON 表及独立 B+Tree 索引；`src/transaction/manager.rs` 的 begin/commit/rollback 记录事务边界并管理 active map | 没有从该两文件证明事务管理器能物理撤销表。不能假定早期内核天然自洽 |
| 2a97c9c653，03-07 | FileStorage 搬到 crates/storage | 这是路径迁移，不是事务正确性的起点 |
| e8071b730f，04-06 | `checkpoint.rs` 新增 | 引入 checkpoint 管理状态；其存在不证明数据页与日志截断安全 |
| a81f8532ca，04-06 | BinaryTableStorage | 二进制表形成独立组织；性能动机来自提交信息，性能数字未复跑 |
| 69e044a98b，04-12 | `wal_storage.rs` wrapper | WAL/数据层开始组合，新增边界转发责任 |
| ee82929207，04-20 | FileStorage 新增 insert_buffer、threshold、flush_buffer、flush_all_buffers | 在 tables 外引入可变行集合。diff 中未同步建立稳定行 ID 或跨 flush 的 undo 契约；是后续问题的前置条件，不能单独称其引入 #5911（当时尚无该 undo 实现） |
| c27aeac299 / 0e46399208，05-31 | WAL replay 委托 storage、engine restart | redo 依赖 Storage 的语义；需要保留分层思路，不能保留未经证明的成功语义 |
| 1f133d31b9，07-15 | `clustered_table.rs` | 额外的内存聚类行组织；后续又向 heap/WAL 镜像，出现多份可写状态 |
| 657fa3f562，07-18 | TableLevelStorage、ParallelWalStorage | 并行写/flush 引入新的事务转发路径 |
| 9e5848ba8d，08-23 | BinaryTableStorageV2 | 新格式与流式插入；本轮源码仍有 begin=Ok(0)、rollback=Ok(()) |
| ae03e74049，08-27 | `savepoint_wiring.rs`，SAVEPOINT UndoRecord | 引擎负责捕获逻辑前后像；按 PK/行值识别，缺稳定内部行身份 |
| 4fe7f55ed9，08-30 | FileStorage tx_undo_log、UndoOp::{UpdateRow,DeleteRow,DeleteAll,BufferedInsert} | 同一天引入存储层物理位置/行值恢复；作者明确称基础设施、尚非用户缺陷完整修复 |
| 3223c23954，08-30 | TM rollback_with_undo、顶层引擎回放；取消仅 SAVEPOINT 记录限制 | 引擎层物理恢复独立存在，回调失败记录后继续，最终 Ok；这是错误吞噬的可追踪来源 |
| 976fe008bc，09-14 | 显式 rollback 先 storage.rollback_transaction_lockfree，再 TM undo | 两套恢复职责在同一路径串联，实际数据后果由当前反例证明；不是从提交声称的测试数字推导 |
| d97a53fe1e，09-14 | `mvcc_storage.rs` wrapper | 额外 pending/committed 版本、snapshot、inner merge；这是当前 wrapper 首次出现，不能说项目至此才有 MVCC |
| 577614d2a3，10-08 | tx-control 显式传 tx_id | 正确方向，但 DML 仍经 current_tx_id/TLS，身份契约未统一 |
| 22aef6be8c / 098293ebe3，10-15 | FileStorage insert/scan 可走 buffer_pool | 源码直接提前返回，绕开 WAL/undo；默认服务器是否启用需看构造，不可把可达 API 等同默认生产配置 |

复现历史命令：`git show <commit> -- <path>`、`git log --reverse -G '<特征>' -- <path>`。已保存原始关键 diff，复查用 `python3 evidence/audit.py show source-history`（从仓库根运行则使用完整脚本路径）。SAVEPOINT/REPLACE/ODKU、多表 DML 和 Trigger 的精确引入边界尚未逐项二分，状态 NOT-RUN/UNVERIFIED；不能把当前找到的功能节点当回归引入点。

## 跨版本运行

共享 harness 是 `evidence/historical/shared.rs`，JSON 构造 TableInfo 以适配增加的元数据字段；相同数据、不带主键、独立临时目录。每个包有独立 Cargo.lock，使用当前 Rust 与离线可用依赖，不是原发布构建环境重现。

| 版本 | begin/rollback，不 flush | 事务内 flush 后 rollback | 证据 |
|---|---|---|---|
| v3.11.0-ga | begin返回Transactions not supported | begin返回Transactions not supported，后续写/撤销未执行 | VERIFIED-RUN能力拒绝；rollback用例NOT-RUN，不算通过 |
| v3.12.0-ga | expected=0，actual=0 | expected=0，actual=1 | VERIFIED-RUN，`historical-v312.json` |
| v4.1.0 冻结基线 | expected=0，actual=0 | expected=0，actual=1 | VERIFIED-RUN，`historical-current.json` |

补做相同SQL序列：`CREATE TABLE t(id INT PRIMARY KEY); INSERT INTO t VALUES(1); BEGIN; DELETE FROM t WHERE id=1; ROLLBACK; SELECT * FROM t;`。harness逐语句打印Result，遇Err停止，绝不把未执行后续当通过。

| SQL基线 | 实际行为 | 等级/证据 |
|---|---|---|
| v3.11.0-ga | CREATE/INSERT/BEGIN/DELETE/ROLLBACK均Ok，最终空表（应有1） | VERIFIED-RUN，historical-v311-sql.json；Storage本身拒绝事务，而Engine仍接受，说明调用方吞能力错误不是新问题 |
| v3.12.0-ga | CREATE/INSERT正常，BEGIN报Transaction already in progress；DELETE/ROLLBACK未执行 | BEGIN兼容性问题VERIFIED-RUN；目标回滚场景NOT-RUN，不能把最终仍有1当通过 |
| 当前v4.1.0 | 每句Ok，最终两行1（应有1） | VERIFIED-RUN，historical-current-sql.json；失败形态由缺失恢复变为重复恢复，不是某旧版本已正确的证据 |

命令：`cargo run --offline --manifest-path docs/audits/HARA-410/evidence/historical/<current|v312|v311>/Cargo.toml --target-dir target-hara410-history -j 2`。完整恢复/并发/历史SQL矩阵、历史进程崩溃、断电、磁盘格式迁移仍NOT-RUN；当前只完成这个共同SQL与存储API反例。

结论：#5911 的形态至少在 v3.12.0-ga 已存在。缓冲、物理 undo、引擎逻辑 undo、MVCC、并行 WAL 是逐步叠加，未形成统一行身份与 durable frontier；可追踪的是抽象契约冲突，不能归结为一个“引入 undo 的坏提交”。可保留 WAL manager / recovery 分离、单 WAL handle、显式 database/tx 参数和已有反例资产；不建议回滚到某个历史版本作为可发布内核。
