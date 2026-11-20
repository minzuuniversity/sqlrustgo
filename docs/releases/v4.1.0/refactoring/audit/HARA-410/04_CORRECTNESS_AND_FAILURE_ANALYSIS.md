# HARA-410 · 正确性与失败模式

source_agent=Codex GPT-6.1 Sol；source_run=HARA-410-20261120；timestamp=2026-11-20（Asia/Shanghai）；evidence_hash=sha256:5956d0b377def7167f4946453fad23f4eefa9205934ed94c82da9aa349042c93（`evidence/probe-run4.json`）；conflict_resolution=以本轮 expected/actual 及明确路径为准，不将旧 Issue 或作者说明充作运行证据。

## 运行环境与解释

命令：`cargo run --offline --manifest-path docs/audits/HARA-410/evidence/probe/Cargo.toml --target-dir target-hara410 -j 2`。源码均为冻结 e9ff21fca3；独立 Cargo.lock、临时数据目录、当前主机 Rust/依赖，具体版本见 enumeration.json。probe 是观察程序，exit=0 仅表示程序执行到末尾，**不是这些数据库用例通过**。代码在 evidence/probe/src/main.rs，可重复执行。run1→run4 增加了不同观察项，不把最早编译路径错误写成测试失败。

SQL 场景通过真实 parser→ExecutionEngine→FileStorage 库路径；未经过 MySQL wire，也未覆盖默认 WalStorage<MvccStorage<FileStorage>> 的所有组合。CountWal 是观察 sync/注入错误的契约桩，不能替代物理断电测试。

## 重大问题

### HARA-F01：双层物理撤销破坏多重集

VERIFIED-CODE + VERIFIED-RUN。触发：`CREATE TABLE t(id INT PRIMARY KEY); INSERT INTO t VALUES(1); BEGIN; DELETE FROM t WHERE id=1; ROLLBACK; SELECT * FROM t;`。

不变量：回滚精确恢复 BEGIN 前的行多重集，且每条物理 mutation 只撤销一次。实际：返回两行 Integer(1)，rollback=Ok。`execution_engine_methods.rs:1943` 调存储撤销；随后 TM UndoRecord::Delete 又 insert。`FileStorage::undo_transaction_in_place:5482` 已恢复原行。

后果：重复主键/重复行，逻辑层成功与约束状态不一致。修复边界：唯一恢复所有者、统一 savepoint、索引/版本同步；不能只在插回前查“同值行已存在”（无主键重复值无法区分）。验收：有/无主键、完全相同重复行、PK UPDATE、flush/迁移、并发删除同表的多重集比对，反向变异恢复双 undo 必须判红。

### HARA-F02：flush 改变位置后 undo 静默失效

VERIFIED-CODE + VERIFIED-RUN。触发：FileStorage begin→insert(42)→StorageEngine::flush→rollback。期望0，实际1；去掉 flush 为0。`file_storage.rs:5078` 记录 BufferedInsert，`:5515` 仅找 insert_buffer；flush 把行提升到 tables，未转换恢复定位。

后果：事务返回已撤销但行保留，后续 flush 可持久化。修复边界：稳定 RowId+generation，或禁止未提交行流出 transaction workspace；持久化/移动不能改变恢复身份。验收：阈值自动 flush、显式 flush、并发其它事务 flush、重排及删除后再回滚，当前与 v3.12.0 同用例。

### HARA-F03：未提交快照重启后成为可见数据

VERIFIED-RUN（进程终止、非断电）。子进程在独立目录 FileStorage::new_with_wal→create→BEGIN→INSERT→flush，随后 process::exit(0)，不运行 Rust Drop。父进程重新加载文件并调用 StatefulRecoveryEngine。报告：entries_total=2、incomplete_txns=1、committed_txns=0，scan 却有1行。

不变量：未提交事务不能经 snapshot 被当 committed baseline。RecoveryImpl:612 只过滤/重放 committed WAL，没有撤销已经加载进 tables 的 loser 行。后果：未提交数据重启可见；“redo 过滤正确”不足以证明恢复正确。

修复边界：选择 NO-STEAL 的 committed-only snapshot，或引入具备 transaction/RowId/pageLSN 的 loser undo 协议。验收：子进程在每个写/flush/commit 边界被 SIGKILL，恢复后同 oracle 对比；文件系统断电和 torn-write 另测，不能由本轮退出复现推导。

### HARA-F04：相同 Every 契约在两条 COMMIT 路径不一致

VERIFIED-CODE + VERIFIED-RUN（契约观察）。WalStorage::new 默认 WalSyncMode::Every（wal_storage.rs:35），&mut commit 在 append Commit 后调用 sync（:1179），commit_transaction_lockfree_for（:1425-1492）没有等效调用。CountWal 记录：&mut=1，lockfree=0。

显式 SQL commit 优先选择 lockfree，所以不是仅死代码差异；FileBackedWalManager::sync:160 才调用 sync_data。后果：无法从 Every 配置推出 ACK 前 commit record durable。尚未实测断电丢数据，不将“可能丢失”写成已跑崩溃结果。

修复边界：一条 prepare/append/durable/publish 协议，明确 durable_lsn>=commit_lsn；不允许按锁优化复制语义。验收：Every/Batch/Off/GroupCommit 分别定义承诺，实际 fsync 跟踪+ACK 后强杀+恢复；运行失败必须传至客户端。

### HARA-F05：撤销失败丢弃恢复状态仍返回成功

VERIFIED-CODE + VERIFIED-RUN。TM 添加一个 UndoRecord，rollback_with_undo 的回调返回 HARA injected undo failure。stderr 有错误，Result=Ok(())。TM:262-280 在回放前把 active/undo 状态取走并删除，错误只打印。

不变量：失败必须可观察，未完成恢复不能标终态成功。修复边界：AbortOnly/Aborting/RecoveryRequired，保留未应用 undo cursor；SAVEPOINT 同样不能吞错。验收：每个撤销操作的第N步注入失败，COMMIT 被拒、后续重试精确一次或隔离实例，不得继续服务未知状态。

### HARA-F06：后端以生命周期空成功冒充事务

VERIFIED-CODE；AppendOnly VERIFIED-RUN。AppendOnly begin→insert→rollback 返回 Ok，期望0实际1；源码 rollback:661 只清 tx_id。TableLevelStorage:319 begin=Ok(1)、rollback=Ok；BinaryTableStorageV2:771 begin=Ok(0)、commit/rollback=Ok；BinaryTableStorage:872 同样缺物理撤销。

不变量：能力不足必须在写入前明确拒绝，不能先接受事务再留下数据。修复边界：显式能力 descriptor 和后端认证；可作为非事务后端保留，但哪些产品功能允许这种配置须属主决定。验收：相同 SQL/失败场景覆盖全部装配栈，拒绝必须发生于任何状态变化之前。

### HARA-F07：恢复失败后第二次变成空成功

VERIFIED-CODE + VERIFIED-RUN（错误注入）。StatefulRecoveryEngine::recover:103 在 inner 调用前设 Recovered；故注入 WAL recover I/O error 时，第一次 Err，第二次 Ok(default report)。不变量：失败恢复不能等同已完成；后果是重试接口可能掩盖失败，但默认服务器是否会重用同一个 recovery 对象尚未证明。

修复边界：只在完整成功且 skipped policy 满足时进入 Recovered；失败转 Failed/RecoveryRequired。验收：读取、解码、apply、flush 第N步失败后重复调用仍拒绝成功，或有显式安全重试 token。不是声称默认服务已因此跳过恢复。

### HARA-F08：checkpoint/truncate 缺持久化与原子代切换证明

VERIFIED-CODE，崩溃后果 HYPOTHESIS/NOT-RUN。FileStorage save_table_full:1889 直接 truncate/create base 文件，:1908 只 writer.flush；delta:1945 同样如此。WalStorage:1217 记录 checkpoint 先于 inner.flush，:1277 截断错误丢弃；safe_truncation_lsn:335 仅 checkpoint/active 下界。FBWM truncate_before:191 先清空原文件，再重写 retained。

不变量：删除旧 redo 前必须证明可恢复快照 durable，且原日志不能在代切换中丢失 retained redo。最小决定性试验：两事务（一已 commit、一 active）+ snapshot flush 故障；在 File::create、base flush、delta 删除、truncate 后首条 retained 写前强杀。修复边界：generation manifest、fsync 数据/目录、durable frontier、原子 segment 替换；验收必须使用恢复 oracle，当前未跑，不得宣称已发生 committed data loss。

### HARA-F09：聚类内存表与 Storage 镜像不能一起撤销

VERIFIED-CODE + VERIFIED-RUN。聚类表初始1，BEGIN→INSERT2→ROLLBACK 后 SELECT 返回1和2。`engine_dml.rs:2189` 先 ct.insert，再 :2208 storage.insert，rollback 只作用 Storage/TM，没有同一个聚类 mutation/undo receipt。

不变量：所有读取路径必须观察同一 committed state；任一半失败不得留下另一半成功。修复边界：Storage 主账+聚类派生视图，或聚类后端成为同一个 TxStore 参与者；不能用普通 heap 用例证明聚类。验收：聚类 INSERT/UPDATE/DELETE、唯一冲突中途失败、显式 rollback、savepoint、重启及并发读同 oracle 对比。

### HARA-F10：REPLACE / ODKU 回滚后原行消失

VERIFIED-RUN，新增证据 `probe-run5.json` / `probe-run6.json`，后者sha256=be201d57c2a91d6edf7926665c27e508dfc1bffb67c3294fa811751f33bd8cdd。普通FileStorage引擎建 `t(id INT PRIMARY KEY,v INT)` 并插入(1,10)，BEGIN后分别执行 `REPLACE INTO t VALUES(1,20)` 或 `INSERT INTO t VALUES(1,20) ON DUPLICATE KEY UPDATE v=20`，再ROLLBACK。期望[(1,10)]，两者实际均空表，所有语句返回Ok。

不变量：复合DML恢复原行和值，不得把已有数据丢掉。REPLACE预删除在engine_dml:250-298，ODKU helper在engine_helpers:234-321，两条不走普通UPDATE完整undo捕获；具体失败还依赖leaf undo与物理位置。此结果只绑定本轮无wrapper/无显式flush的装配，不能用它替代他方sqlite“值停在99”场景。

修复边界：同一个mutation batch覆盖冲突查找、旧行删除/更新、新行插入、索引版本和约束。验收：pre-image相同多重集，按第N冲突/第N写失败验证statement atomicity，所有后端/复合唯一键/SAVEPOINT分别测试。

### HARA-F11：可选BufferPool写路径回滚无效

VERIFIED-CODE + VERIFIED-RUN，证据probe-run6.json。FileStorage::new_with_bufferpool:1232接收公开BufferPool::new；同一临时目录create→begin→insert(42)→rollback→scan，期望0实际1。insert_at:5034提前返回insert_via_bufferpool，未走正常undo，scan:6607也独立读取页。

不变量：允许事务的装配不能按缓存配置改变恢复语义。后果：API配置可达且回滚无效；未证明默认MySQL server启用该构造，不把它冒充默认生产故障。修复边界：将其变为TxStore派生缓存，或对事务组合在写前拒绝并由属主确认产品范围。验收：on/off成对、eviction、flush/reopen、失败恢复；不能把内存Page等同durable页。

## 已跑的负对照与未证明项

run4 中简单 PK UPDATE→rollback 恢复原1，buffered delete-all→rollback 恢复一行。它们只能说明这些具体用例没有失败；不能证明 #5906/#5910 错误或已解决，更不能改变 open Issue。named-db、重复值、UPDATE 后 flush 的组合仍需各自反例。

runtime atomicity、commit durability、crash recovery 是不同出口。没有跑全仓测试、wire 全后端、真实断电、PITR/replication 故障、长期并发及迁移。它们全部 NOT-RUN，不计入任何通过率。

## 真实客户端对照

另实跑现有MySQL测试目标全部6项，0 failed、0 ignored，见 `evidence/wire-rollback-full.json`，sha256:c2987c3a5d00252cde5066b867c18ead1bdeacded54d67562b5a0cb0e81390e8；命令为 `env CARGO_TARGET_DIR=target-hara410-wire CARGO_BUILD_JOBS=2 cargo test --offline -p sqlrustgo-mysql-server --test rollback_undo_test -- --nocapture`。它覆盖commit、自身INSERT/UPDATE撤销、空事务、单行DELETE恢复、peer commit与跨连接可见性。

这些默认装配结果正常，与本报告FileStorage直连失败同时成立。没有把两者混为同一条件；wrapper为何改变观察结果需要物理主账与MVCC链同时核对，当前不从“SELECT正常”推导底层无重复，也不宣称wire该六项有缺陷。F03/F04的独立恢复/协议反例仍需默认装配的故障注入验证。
