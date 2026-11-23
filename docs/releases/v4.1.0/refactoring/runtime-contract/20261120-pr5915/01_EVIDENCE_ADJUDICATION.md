# 证据判定与适用范围

source_agent=Codex GPT-6.1 Sol；source_run=RUNTIME-5915-20261120；timestamp=2026-11-20（Asia/Shanghai）；evidence_hash=sha256:57fe60174b4642317a534de402027e42603ba976601a8ed1ff4916cb9fb7df2a（独立复跑元数据）。conflict_resolution=Owner批准前为PROPOSED技术裁决建议，不是实施授权或最终独立合格证。

## 冻结坐标与复跑

产品基线e9ff21fca3，PR测试head=56cfa222f1，文档包父提交45da6968ae。PR相对产品基线只增加两测试文件及8行Cargo测试登记；src/与crates/生产源码diff为空。源码快照、DSH-B日志、Issue API原件均保存于evidence/。工具链与构建参数单列，无真实模型调用或Worker派发。

复现命令：在PR head独立worktree执行 `cargo test --offline --test ca01_file_undo --test ca01_snapshot_reopen --no-fail-fast -- --test-threads=1 --nocapture`，独立target与CARGO_BUILD_JOBS=2。两目标各1通过/2失败，不能把“成功捕获缺陷”记成产品PASS。

| 项 | 本轮观察 | 证明了什么 | 未证明什么 |
|---|---|---|---|
| INV-01a | 精确多重集断言FAIL，3→6 | bare FileStorage SQL混合DML回滚不精确 | 所有默认/parallel装配同样失败 |
| INV-01b | 特定PK连续UPDATE最终值7，PASS | 此用例最终结果正确 | 每mutation实际只回放一次；缺trace/身份oracle |
| INV-02 | count断言FAIL，2→4 | 重复值行恢复数量错误 | 已直接测到RowId、或只删除了一条重复行 |
| INV-07a | 同Arc<FileStorage>双Engine，B读到2而非1 | 该共享leaf装配存在未提交可见 | 真实wire、并行线程、所有MVCC装配及完整RC/SI；第44行失败后COMMIT及第47行提交后断言未执行 |
| INV-07b | 显式flush后重开读到未提交(2,20)，FAIL | 未提交状态混入重开可读文件基线 | 进程崩溃、WAL重放、fsync持久性、断电 |
| INV-07c | rollback+flush+重开仅(1,10)，PASS | 这条正常完成撤销路径清理有效 | 任意崩溃点的原子恢复、durable commit |

## INV-07b证据等级

`ca01_snapshot_reopen.rs:52-69` 在当前测试进程内用作用域结束释放对象，然后open；open:94-97仅FileStorage::new，未构造WAL manager/RecoveryEngine。61-62显式flush_all_buffers/flush，证明文件层可读状态，但不证明介质可靠落盘；OS缓存与断电模型未覆盖。状态：VERIFIED-CODE + VERIFIED-RUN（CLEAN_REOPEN），CRASH_PROCESS_KILL=NOT_RUN，WAL_REPLAY=NOT_RUN，POWER_LOSS=NOT_RUN。

HARA旧F03另有无Drop的process::exit(0)+RecoveryEngine反例；它不是#5915 INV-07b，也不是SIGKILL/断电。两条证据保留各自命令/装配，不能相互升级等级。只计明确故障模型，RAII/unwind不证明abort/OOM/进程死亡安全。

## #5908 / #5909 / #5910归并但不等同

- #5908原始场景涉及“只删一条相同行”及按值全删再插导致2→1；#5915的DELETE WHERE id=1匹配两条，再回滚为4。它们共享身份/恢复责任根因，但不是同一复现。注释“删除一行”与SQL不符；验收必须增白盒RowId定点删除一条的用例，不弱化现有2→4断言。
- #5909的双物理恢复入口可在execution_engine_methods.rs:1947、1952实读，存储rollback先执行，TM再驱动delete/insert。新多重集反例支持精确一次未成立；一个PK例子PASS不能证明两个恢复层组合正确。
- #5910提出Storage单层快照不完整。DeleteAll构造确有只clone tables.rows的分支，但delete_at另有BufferedDelete记录，delete_collect_pks_at又有不同buffer处理；不能只看一行或用新2→4测试宣称独立证实其“净少一行”。该Issue原配置/插桩仍需按固定源码与调用路径复跑，状态UNVERIFIED-RUN。本契约通过完整RowId变更集避免tables/buffer片段快照，不提前关闭Issue。
- Issue正文中“#5905已修”“存储按下标精确”等是他方历史claim，不是当前集成/所有重排场景保证；Issue仍open，PR状态单独查询。

## 与ARR材料的取舍

AGREE：单Undo、op_id/slot/loc分离、失败可见、RAII仅运行期、旧双Undo不可发布回退。

PARTIAL：早期内存slot有局部实验价值，但只覆盖部分DELETE/未覆盖全部buffer/版本/恢复；不能从“不持久化undo”推出所有持久化协议零格式影响。

DISAGREE：只因checkpoint调用在src/没找到就称不可达（WalStorage提交路径及MySQL构造需计入）；buffer_pool没有公共构造器（FileStorage::new_with_bufferpool存在）；“无Commit即丢弃”足以清除已经加载的脏snapshot。

ARR RSC_CRASH_CONTRACT提出clear_all_tables+全WAL redo是有条件方案。engine_builder.rs:with_wal_recovery确有clear；但只清内存不证明所有装配采用该入口，也不证明已截断WAL仍能重建全部committed数据/重复值。未证明完整日志保留前，不能清空可信checkpoint基线作为通用修法。新Recovery以snapshot证书+保留redo为依据。

GitNexus索引cd8不同于冻结产品e9，仅作导航；本轮按e9源码/PR原件复核。AOCI checkpoint未对齐，不声称完整系统认知。DSH当前本地仓91da5ba有并发脏变更，交接只绑定已读handoff模块/schema字节，不把其余改动作为本轮产物或授权。
