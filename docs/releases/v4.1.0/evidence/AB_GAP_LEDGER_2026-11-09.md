# Alpha→Beta 门禁 issue 缺口台账(2026-11-09,冻结 tip `c06209fd11`)

> 性质:Owner 2026-11-09 裁决(批准升级 + 约束「Alpha ISSUE 必须按证据链彻底关闭」)的执行前审计。
> 口径:逐条对照 `docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md` 的 AB-01..AB-10、
> issue 正文 Hard Gate 与 `docs/governance/ISSUE_CLOSING_VERIFICATION.md` 关闭合同。
> 本文件只登记「已有证据 / 缺口」,不做任何 PASS 声明;历史 closed 不能替代本次取证
> (ALPHA_TO_BETA_GATE_PLAN.md §1 原文)。

## 0. issue 状态快照(API 实读,2026-11-09)

| issue | 状态 | 关闭合同复核 |
|---|---|---|
| #5112 事务/会话正确性 | closed(2026-10-15) | 最后实评(10-09)仍列 88 条存活变异「closure candidate」;Hard Gate(0/300、8000 tx 对账、8 线程隔离)在 tip 无复跑工件 ⇒ 晋级证据不得引用其 closed 状态,须在冻结 commit 复证 |
| #5113 CI fail-closed | closed(2026-10-15) | Hard Gate 的 fmt(已转绿 #5685,tip 0 diff)、`clippy -D warnings`(tip 仍 62 处,属主各线)、required checks success(CI 已死 #5597)⇒ 三槽两位仍红;同样不得引用 closed 替代复证 |
| #5114 工作负载回归 | open | 见 §1 |
| #5115 SOAK 阶梯 | open | 见 §2 |
| #5116 性能门禁 | open | 见 §3 |
| #5117 总控 | open | 按终验规则:全部子包 + 冻结提交复核 + 独立审核后才可关闭 |

## 1. #5114(B 轨工作负载/隔离/不变量)——8 条全部无完整关闭证据

已有证据(节选):SQL corpus 门禁实测 98.2%(run `20261107-alpha-gates-001` 臂 005,exit 0);
must-pass 类目断言 `crates/sql-corpus/tests/corpus_test.rs:109/131/153`;cell-diff 机制
`tests/integration/sql/four_way_cell_diff_test.rs`(PG canonical、#5404 后真断言)+
`tests/baseline/tpch_known_differences.json`;隔离类探针 5/6 类(dirty read/lost update/
non-repeatable/rollback/db isolation 均有 CAUGHT 记录);a04 五入口对拍工件
`docs/releases/v4.1.0/evidence/a04-5226/wire/`(match=33/mismatch=7/silent_wrong=0)。

缺口(逐条):
1. B 轨版本化 manifest **完全缺失**(`tests/compat/bustubx_edu_b_track/` 整个目录及其
   oracle 均不在树上;门禁 `scripts/gate/check_bustubx_b_track_v312.sh:10-20` 自陈 SKELETON
   故意 exit 1;GATE_CATALOG:37 class=dead。可沿用 `bustubx_edu_sqlite_cli/manifest.yml` schema。
2. must-pass 集合未定义(B 轨集缺失 ⇒ 100% 无从谈起);A08 禁用清单(27 wire/3 compile_fail)
   是 10-09 快照无 tip 复跑。
3. corpus 失败 1.8% **无持久化登记**(gate 把报告写 mktemp 且 trap 删除,
   `check_sql_corpus_gate.sh:7-8`);无 P0 分级;neg fixture pending(NEG_FIXTURES:36)。
4. G1 TPC-H 最后归档臂 **124 超时未取得**;哈希基线锁 SF=0.001,SF=1 判定从未 recorded;
   fixture 在 /tmp symlink;neg fixture pending(NEG_FIXTURES:24)。
5. phantom 只有 synthetic GapLockManager 自测(`gap_locking_test.rs` 注释自认)+
   snapshot 隔离代替;SET TRANSACTION ISOLATION LEVEL 存成会话字符串无行为断言。
6. 行数守恒被判**未闭合**(KANBAN ENG-10:第一列身份语义无判别探针);重启一致在 develop
   活红(KANBAN ENG-13/#5584 `crash_wal_index_b1` 1/2 红);0/300 与 8000 tx 测试文件不存在。
7. fix-PR 关联机制在但 registry **停更在 PR #5075**(#5075 后的 fix PR 无登记);「先红」只是
   note 散文无机器字段;CI 死 ⇒ 强制链不生效。
8. 三类变异:gate-ignore ✅、错误库读取 ✅(三处独立 CAUGHT)、**共享 tx id 半覆盖**
   (5099 文档 §6 自认测试不驱动引擎调用序列)。

关闭前最小工作集(升序):①corpus 失败台账持久化+P0 分级;②共享 tx id 变异夹具;
③phantom 真探针(wire harness 对生产存储);④G1 无争用复跑(纳入 dataset_hashes);
⑤两枚 neg fixture(截断语料/篡改 SF1 基线);⑥fix-pr registry 续账+`red_evidence` 机器字段;
⑦行身份判别探针+#5584 转绿;⑧B 轨 manifest+oracle 门禁实体化(最大项)。

## 2. #5115(SOAK/故障注入阶梯)——机制碎片多,端到端证据零

已有:armed 故障注入内核(`fault_injection.rs` 三 FaultPoint + `fault_coordination.rs`
SIGKILL 协调器,已在 `20261109-walgc1-001` 等归档 run 实跑);重启恢复测试群;
`run_soak_loop.sh` 24h/重启循环/tier 自测;`v400_1h_soak.sh` 有 pid/报告/退出码;
归档框架 `audit_run.py` 可直接宿主 soak run。

缺口:heartbeat 机制**全仓不存在**(STAGE.yaml `known_gaps.V400_09_soak` 就是「进程消失仍
显示 RUNNING」案底);v4.1.0 **零个 soak 归档 run**;soak 产物写仓外(`$HOME/…`、
`/Users/liying/…`)无 manifest/SHA256SUMS;`soak_test_harness.rs` 自述「不跑真查询」⇒ 不得作
1h 证据;`chaos_test.rs:49-54` disk-full 只 println = **假绿空测试**;五类 soak 门禁 neg
fixture 全 pending(NEG_FIXTURES:38/41/42/45/47);杀 runner/删 heartbeat 两探针连文本都没有
(heartbeat 不存在使探针无从执行)。

关闭 Alpha 所需最小集:①选定 canonical runner 补 heartbeat/ISO 时间戳/commit/EXIT_REASON
(从 `run_72h_soak_v2.sh:76-94` 移植);②fail-closed 状态判据(heartbeat 超窗/产物缺失 ⇒
FAIL/NOT-RUN,禁 RUNNING 字样);③两枚变异夹具(`test_soak_runner_mutation_fixtures.py`);
④每轮行数/主键集合摘要 + 删/修真 `chaos_test.rs` 空测试;⑤1h oltp_read_write 归档 run
(带 run id 才销账,ING-1 同款判据)。

## 3. #5116(性能基线自动门禁)——专属工件零命中

已有:criterion 基础设施(13 个 `[[bench]]`、`benches/` 29 文件);sysbench 四场景脚本存在;
PERF_4956 一次性 A/B(单行 autocommit vs 1000 行显式分离测量 + 6 个复现工件);
判据文本已固化(GATE_PLAN AB-08:吞吐 -10%/p99 +20% 且超噪声)。

缺口:全仓对「5116」**零命中**;criterion→JSON 转换器不存在(bench-pr.yml 自陈);
`benchmarks/baselines/sf01/` **空目录**;比较器 schema 是退役格式、缺 baseline 时 skip
(与「缺 baseline 必须 FAIL」相反);live CI 无 bench 执行点、无 schedule 触发;
group commit(默认 file 路径未接线)与谓词下推(被 B+ 树 split 丢键缺陷挡住)两项 A/B 完全缺;
「自适应 flush」命名在仓内无对应实现(最接近= #4956 commit-flush,台账须记命名落差)。

关闭前最小工作集:①比较器 fail-closed 重写(qps-10%/p99+20%/噪声三元组)+ 3 枚反向夹具;
②criterion→JSON exporter + 提交版本化基线;③perf_env.yaml 固化 + run manifest 化;
④sysbench ≥5 轮 harness + 负载后行数/重复键对账 FAIL 通道 + 移植 PERF_4956 driver;
⑤live CI quick + nightly 接线 + GATE_CATALOG 登记;⑥四项独立 A/B(group commit 须先修
WALGC-1/2 才能测;谓词下推若无「未实现 ⇒ NOT RUN 撤声称」裁定则被缺陷阻塞)。

## 4. 结构性阻塞(跨包,属主/他线)

- clippy `-D warnings`(AB-02/#5113):tip 剩 62 处全部属主各异(SEC-1 族 24、退役线 10、
  死代码/覆盖面 ~15、rustdoc 7、行为裁定 5、snake-case 1、future-incompat 报告行 1)⇒ 需各
  属主线进线,不属 lint 线(已收官)。
- CI 执行面 #5597 全 Blocked ⇒ AB-03「job 实际启动 + status success」在复活前不可满足;
  本地归档 run 只能替代「可复现」不能替代「required checks success」。
- WALGC-2(#5686)与 #5584(ENG-13 重启一致活红)、SEC-1、ING-1(168h 排期)为硬前置。
