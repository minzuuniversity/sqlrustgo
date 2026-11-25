# HARA-410 · Storage 与 DML 覆盖矩阵

source_agent=Codex GPT-6.1 Sol；source_run=HARA-410-20261120；timestamp=2026-11-20（Asia/Shanghai）；evidence_hash=sha256:2740d73eb47e3d1f861483d61586ebb7c13387a61148101d36370230175a4f3b（枚举与 metadata）；conflict_resolution=默认方法、wrapper 和 leaf 分别确认，不从默认测试后端推广能力。运行结果见 `evidence/probe-run4.json` 及后续 run5。

## 实现者集合

检索命令在 enumeration.json：跨行 `impl … StorageEngine … for`，范围 crates/src/tests，并结合 lib.rs 模块导出检查。源码候选包括11个公开可编译实现者、1个 cfg(test) helper，以及1份未导出的 columnar 实现。不是预设“只有三个后端”。该枚举是源码候选核对，非 Rust 编译器对所有 cfg 组合的形式化穷举。

| 类型 / 位置 | begin / commit / rollback（VERIFIED-CODE） | savepoint | durable/recovery | 本轮运行能力 |
|---|---|---|---|---|
| MemoryStorage，engine.rs:3156 | snapshot/delta/TxLog，局部 undo；多库/多连接需组合验证 | 无统一 backend savepoint token | volatile；无磁盘承诺 | CountWal 的内存叶及恢复状态错误注入；全事务语义 NOT-RUN |
| FileStorage，file_storage.rs:6265 | per-tx UndoOp；lockfree/legacy 均有控制入口；行值/下标恢复 | 由引擎逻辑 undo 代办 | JSON+delta，可选自带/shared WAL；dirty flush 可含未提交行 | F01/F02/F03/F09 已复现，非事务认证 |
| BinaryTableStorage，binary_storage.rs:304 | begin/commit/rollback 管理值或空成功，无真实行撤销 | 无 | binary；服务器 binary 模式 no WAL | NOT-RUN，不能标可事务 |
| BinaryTableStorageV2，binary_storage_v2.rs:440 | begin=Ok(0)，commit/rollback=Ok | 无 | bint/streaming，独立格式 | NOT-RUN，空成功已实读 |
| AppendOnlyStorage，append_only_storage.rs:241 | begin 加计数，commit/rollback 清计数 | 无 | alog/tomb/cache；flush:649 只清 dirty | rollback 后行仍在，VERIFIED-RUN |
| TableLevelStorage，table_level_storage.rs:47 | begin=Ok(1)，commit flush_wal，rollback=Ok | 无 | per-table registry；write_wal:34 未实现 append | NOT-RUN，能力声明不可认证 |
| WalStorage<S,W>，wal_storage.rs:626 | WAL boundary+inner；不同控制入口 sync/错误传播不等价 | 无 wrapper token，依赖外层 | FBWM；safe_truncation/active_txs | Every sync 调用差异已实跑，物理断电 NOT-RUN |
| ParallelWalStorage<S,W>，parallel_wal_storage.rs:197 | 独立 commit/group 路径，部分显式 tx_id 入口依赖 trait 默认转发 | 无统一 token | shared WAL+并行 flush/group commit | 当前装配源码已追踪，实跑 NOT-RUN |
| MvccStorage<S>，mvcc_storage.rs:697 | begin bind snapshot；inner 成功后 promote/discard | 无版本级 savepoint token | rebuild_from_inner；无独立持久化版本主账 | 读可见性并发矩阵 NOT-RUN |
| BoxStorageEngine，binary_storage.rs:963 | 动态分派转发；大量方法逐项 override | 外层处理 | 能力依赖 inner，不能整体赋予事务能力 | server entry 装配已读；全转发差分 NOT-RUN |
| VtuGuard<S>，vtu_guard.rs:85 | 转发生命周期，DML 活跃检查 | 无独立实现 | guard 不增加 inner 正确性 | NOT-RUN；不能把“有事务包裹”视为 ACID |
| ScanCountingStorage，wal_storage.rs:2158 | cfg(test) 仪表 helper | 不适用 | 不适用 | 非产品后端 |
| ColumnarStorage，columnar/storage.rs:480 | 存在 impl 文本，lib.rs 未声明 pub mod columnar | UNREAD | UNREAD | 编译/产品可达性未建立，不计入已支持后端 |

StorageEngine:1739/1746/1774 的默认事务方法实际返回 Err，不是默认 Ok。风险来自具体 leaf 空成功及调用者把 Err 混同“不支持”：引擎 begin/commit fallback 丢结果，WalStorage lockfree inner commit/rollback 也丢结果。目标能力声明必须显式区分 Unsupported、IoFailure、Conflict、OutcomeUnknown。

## 写路径覆盖

| 路径 | 数据变化与入口 | 当前 undo / WAL / index 责任 | 本轮证据与未闭合出口 |
|---|---|---|---|
| 普通 INSERT | engine_dml.rs:140→Storage.insert_in_db | Engine record_insert_undo:826；FileStorage BufferedInsert/WAL；索引后端维护 | flush 后回滚失败已跑；阈值、触发器失败原子性 NOT-RUN |
| 普通 UPDATE | :979 / :1181 / :1371 | 引擎旧 PK/new row undo + leaf row_idx/value undo；WAL 前后像 | 简单改 PK 回滚恢复；跨库/flush/并发 identity NOT-RUN |
| 普通 DELETE | :1429 / :1625 / :1673 | Engine Delete undo + leaf DeleteRow；MVCC tombstone | 双层恢复重复行已跑 |
| REPLACE | :250-298 先删冲突，再正常插入 | 预删除不在普通 record_delete_undo 路径；依赖 leaf undo，后续 INSERT 单独记录 | run5/6：回滚后空表（应保留原行）；中途插入失败/唯一键多冲突 NOT-RUN |
| ODKU / ON CONFLICT | :669→engine_helpers::apply_odku | helper 直接更新，未复用普通 UPDATE 完整前后处理；leaf/MVCC/WAL 各自维护 | run5/6：回滚后空表（应保留原行）；并发unique/savepoint NOT-RUN |
| 多表 UPDATE | :1928 apply_multi_table_updates | 逐表 delete_in_db+insert_in_db，缺统一 Engine UndoRecord/语句 receipt | VERIFIED-CODE，运行 NOT-RUN；第N表失败必须无部分发布 |
| 多表 DELETE | :1962-2070 | 按表直接删除；没有统一 statement participation 协议 | 源码路径已定位，完整失败注入 NOT-RUN |
| TRUNCATE | execution_engine_methods.rs:804→storage.delete | 裸表名、无 implicit DML begin/commit；FK CASCADE/RESTRICT 仅记录 | run5/6本用例恢复原行；语义仍应按已承诺方言决定事务/隐式提交规则 |
| Trigger DML | executor/trigger.rs:306、:800-1007 | TriggerUndoRecorder→engine sink→savepoint_wiring；storage 仍自行 undo | 有 recorder 接口不等于每种事件覆盖；嵌套/跨库/错误回滚 NOT-RUN |
| 聚类 INSERT/UPDATE/DELETE | engine_dml.rs:2100-2395 | 先改 ClusteredTable，再 mirror storage，读取从内存树 | F09 已跑 INSERT；UPDATE/DELETE/partial failure NOT-RUN |
| buffer_pool FileStorage | file_storage.rs:5034/6607 | 提前返回，绕开正常 WAL/undo/change log；read miss 跳过 | run6：公开new_with_bufferpool构造后回滚残留1行；默认服务器是否启用未证，不泛化默认配置 |
| force_insert / recovery apply | StorageEngine::force_insert_in_db，RecoveryRowIndex | 绕 buffer，但去重按行值/当前内容；非稳定 WAL operation id | 回放丢失重复值/重复执行相关场景 NOT-RUN |
| LOAD DATA / 直写 / cross-model | mysql-server load_data；GMP/vector/graph 接口 | 多个入口依赖 Storage 或各自 model tracker | 未逐条闭合，UNREAD/NOT-RUN；不可归为已覆盖 |

## 必须保留的覆盖资产与新增矩阵

可复用的源码测试入口：mysql-server/tests/rollback_undo_test.rs、storage/tests/{file_storage_wal_5055,multi_db_crash_recovery_5192,mvcc_row_identity_lifecycle_5277,mvcc_chain_identity_regression_5281,crash_wal_index_b1}.rs、tests/clustered_wal_bypass_r1_5228.rs、tests/integration/transaction/savepoint_test.rs。rollback_undo_test本轮已实跑6项，0 failed/0 ignored（wire-rollback-full.json）；其余只确认文件/部分断言已读，**本轮没有运行**，不能转述其历史注释为当前结果。

新增回归必须按“装配栈×语句×事务边界×身份变化×失败点”选择成对覆盖，所有已承诺入口各有至少一个生产装配 oracle。完整乘积大小与执行成本应由 M0 冻结的支持矩阵决定；此时给一个精确测试总数或工期没有依据。详见08的测试家族与停止点。
