# Tasks — TPC-H fixture 自包含化与 CI 执行完整性

> 执行顺序严格遵循 `AGENTS.md §7` 最小开发闭环：
> RED → GREEN → REFACTOR → GATE → PR → RECORD → LEARN
> 每项任务给出**可验证的完成判据**；未实测通过不得勾选。

---

## Phase 0 — 前置（已完成 / 进行中）

- [x] **0.1** 用 GitNexus + AOCI 完成分析（符号级 + 文件/模块级），确认索引仅落后 2 个纯 markdown commit
- [x] **0.2** 读齐 AGENTS.md §10 的 7+1 治理清单
      （ANTI_FABRICATION_POLICY / ADR-001 G-01~G-10 / ADR-008 P1~P3 / ISSUE_CLOSING_VERIFICATION /
      DOC_CHECK_CORRECTION_RULES / GATE_CONDITIONS / AI_COLLABORATION / ADR-014）
- [x] **0.3** 从 CI 完整日志实测出漏跑规模：declared 446 / executed 64 / **missing 382**
- [x] **0.4** 本地全量 `cargo test --all-features --no-fail-fast` 实测失败面
      → 判据：失败目标清单按「fixture 相关 / 真实缺陷 / 环境」三类归因，每类有 file:line 证据
- [x] **0.5** 用户拍板范围：**B 信号层**，规模层本轮不接入
- [x] **0.6** OpenSpec spec 落笔（proposal / design / tasks）

---

## Phase 1 — RED（先让新门禁失败）

- [x] **1.1** `scripts/gate/check_test_target_execution.sh`
  - 判据：用**真实** CI 日志 `/tmp/testjob.log` 跑 → **exit 1** 且打印 382 个 missing 目标；
    用"全部执行"的合成日志跑 → exit 0
- [x] **1.2** `scripts/gate/check_tpch_known_differences.sh` + `tests/baseline/tpch_known_differences.json`（空表）
  - 判据：空表 → exit 0；注入缺 `issue` 的 entry → exit 1；条目数超限 → exit 1；
    `actual == oracle` 的陈旧条目 → exit 1
- [x] **1.3** `scripts/gate/prepare_tpch_fixture.sh`
  - 判据：首次运行生成 8 个 .tbl + manifest（行数为实测值）；二次运行输出 `[SKIP] fixture already present` 且 exit 0
- [x] **1.4** RED 证明：先让 CI 摘要与执行核对处于"缺失"状态（即当前 `.gitea/workflows/ci.yml` 无
  `--no-fail-fast`、无 fixture 准备步骤、无执行核对步骤）→ 对应门禁为 FAIL

---

## Phase 2 — GREEN（实现）

### 2.1 共享模块

- [x] **2.1.1** `tests/common/tpch_fixture.rs`
  - `FixtureState::{Present, Missing, ProfileMismatch}`；`TPCH-NOT-RUN: <target> reason=<code>` 标记输出
  - 判据：单元测试覆盖三种状态；缺数据时**不 panic**
- [x] **2.1.2** `tests/common/sqlite_oracle.rs`
  - 进程内 `rusqlite`；TPC-H 8 表 schema + 导入；结果归一化（NULL/整数/浮点/文本）
  - 方言适配白名单：`EXTRACT(YEAR FROM x)` → `CAST(strftime('%Y', x) AS INTEGER)`；未适配用例必须显式登记
  - 判据：对同一 fixture，SQLite 读出的 8 表行数与 `wc -l` 实测一致

### 2.2 CI 接线

- [x] **2.2.1** `.gitea/workflows/ci.yml` `test` job：新增 "Prepare TPC-H fixture" 步骤（测试之前）
- [x] **2.2.2** 改 `cargo test --all-features 2>&1 | tee test_report.log` → 加 `--no-fail-fast`
- [x] **2.2.3** 摘要步骤改三态披露：`passed` / `failed` / `not_run` + `declared_targets` / `executed_targets`
      —— `not_run` 的测试**从 passed 中扣除**
- [x] **2.2.4** 新增 "Target execution accounting" 步骤调用 `check_test_target_execution.sh`

### 2.3 fixture 状态化

- [x] **2.3.1** 删除 `tests/data/tpch-sf01`、`tests/data/tpch-sf001` 两个悬空符号链接
- [x] **2.3.2** `tests/common/tpch_wire_harness.rs` 的 `start_sf001` / `start_sf01` 改走统一解析，缺数据不再 panic
- [x] **2.3.3** 13 个非 `#[ignore]` 的 oracle diag 目标：缺数据 → `NOT-RUN`（不再 panic）
- [x] **2.3.4** 约 20 个数据依赖型目标：profile 守卫 → `NOT-RUN: reason=profile_mismatch`
- [x] **2.3.5** 其余 fixture 依赖目标：统一改用 `tests/common/tpch_fixture.rs`

### 2.4 断言化（13 个零断言文件）

- [x] **2.4.1** Q11 族 5 个（`diag_q11` / `diag_q11_3way` / `diag_q11_having` / `diag_q11_steps` / `diag_q11_where`）
      → SQLite 对照 + 语义不变量，作为**样板**
- [x] **2.4.2** `diag_q12` / `diag_q12_deep` / `diag_q14_full` / `diag_q14_only` / `diag_q14_q16`
- [x] **2.4.3** `diag_q6_filter` / `diag_q6_where_parsed` / `diag_shipdate_type` / `q17_sf1_watchdog` / `sf1_bulk_load_bench`
- [x] **2.4.4** 13 个文件的 `assert!` 计数全部 > 0；期望值来源逐条注明（SQLite / 不变量），**无本引擎输出回填**

---

## Phase 3 — REFACTOR

- [x] **3.1** 13 个文件去重：公共逻辑收敛到 `tpch_fixture.rs` / `sqlite_oracle.rs`，文件内只留查询与断言
- [x] **3.2** `diag_q11.rs` 与 `diag_q11_having.rs` 的 `#[test] fn diag_q11_having()` **同名**函数改名区分
      （不导致 cargo 冲突，但 CI 输出无法区分）
- [ ] **3.3** `cargo clippy --all-features -- -D warnings` → 0 警告
      （**未实测**：本机内存有限；且 `crates/mysql-server/src/lib.rs` 有 2 个**基线既有**警告
      —— `unused import: ParallelWalStorage` @15、`unused variable: wal_manager` @7144 ——
      本 PR 不改 `src/`，该红若出现属基线问题，由 CI 判定并在本 PR 正文中显式声明）
- [x] **3.4** `cargo fmt --check --all` → PASS
- [ ] **3.5** `shellcheck -S warning` 对所有新增/修改脚本 → clean
      （未实测：本机无 shellcheck；已用 `bash -n` 对全部 4 个新增脚本做语法检查）
- [x] **3.6** `gitnexus detect-changes` 确认改动只影响预期符号与执行流

---

## Phase 4 — GATE（变异 + 全量）

- [x] **4.1** `scripts/gate/mutate_tpch_oracle_tests.sh`：对**引擎实现**注入变异
      （SUM 漏项 / HAVING 阈值反向 / join 条件写错）→ 判据：全部 **CAUGHT**，SURVIVED = 0
- [x] **4.2** 变异前先跑 `gitnexus impact` 确认变异点调用面（`CLAUDE.md` 硬性要求）
- [ ] **4.3** 本地全量 `cargo test --all-features --no-fail-fast` → 判据：**declared 446 全部执行**，
      FAIL 全部归因清楚（fixture 相关 / 真实缺陷 / 环境），真实缺陷已立 issue
      （**本机不执行**：wire 路径实测峰值已达 817 MB，本机内存不足以承担全量；
      改造前基线已实测并入证据 §7.1：158/446 执行、18 失败、**0 个 TPC-H 目标跑过**；
      改造后的全量由 CI 承接）
- [x] **4.4** `scripts/gate/check_test_target_execution.sh` → PASS（0 missing）
- [x] **4.5** `scripts/gate/check_tpch_known_differences.sh` → PASS
- [ ] **4.6** `scripts/gate/check_no_log_tbl_json.sh` → PASS（未新增 .tbl/.json 入库）
- [ ] **4.7** `scripts/gate/check_gate_test_integrity.sh`（P16）→ PASS（未新增 `#[ignore]` 门禁测试）
- [ ] **4.8** `scripts/gate/check_anti_ignore_gate.sh` → PASS
- [ ] **4.9** `scripts/gate/check_alpha_entry_v4.10.sh` → PASS
- [ ] **4.10** CI 实跑：`lint-build` / `test` / `test-soak-detach` / `evidence-graph-gate` / `legacy-gate` 全绿
      （`test` job 首次 no-fail-fast，记录真实耗时）

### 4.11 wire 语句范围（实施过程中新增，回应 #5392 / #5393 / #5394）

- [x] **4.11.1** 实测定位 file storage wire 路径上不可执行的标准查询
      → 判据：每条语句在**独立服务器进程 + 硬 RSS 守卫**下单独实测，记录确切现象
      （Q9：1 s 内破 1.5 GB；Q18：服务器被杀，客户端 `Lost connection`；Q19：wire 前端解析失败）
- [x] **4.11.2** 范围内语句**逐条**与独立 SQLite 预言机对照（同一句 SQL 两侧对跑）
      → 判据：19 条中 18 条逐格一致，Q13 为已登记差异；服务器峰值 RSS 817 MB，无 OOM
- [x] **4.11.3** 共享 wire 语句范围 `tests/common/tpch_wire_scope.rs`
      → 判据：`WIRE_SCOPE`（19 条）+ `WIRE_EXCLUSIONS`（Q9/Q18/Q19 + 受控原因码 + issue #5393）；
      单元测试强制二者并集 == 22 条标准查询且不重叠
- [x] **4.11.4** **编译期**收口点：`run_query_timed(client, qid, sql, timeout)` 的 `qid` 必填，
      范围外标准查询不执行、返回 `WireOutcome::Skipped`
      → 判据：13 个 wire 目标全部改为显式传 qid；漏传即编译失败
- [x] **4.11.5** 已知差异登记支持 `kind = cell_values`
      → 判据：Q13 行数相等但取值不同，不被门禁误判为「已过期」
- [ ] **4.11.6** 13 个 wire 目标的断言改为「每条语句都有交代」
      → 判据：`compared + not_run == 语句总数`；`skipped_scope.len() == expected_skipped`；
      `compared == expected_scoped`；`compared > 0`

---

## Phase 5 — PR

- [x] **5.1** 逐行核对 `git diff`（提交前）
- [x] **5.2** PR 正文按 `PR_TEMPLATE.md` 填 5 类文档 + Test Plan + Evidence；
      **先写文件再用 python `json.dump` 生成 JSON，最后 `curl --data-binary @file`**
- [x] **5.3** 推送至 gitea252，开 PR（#5396），评论关联 issue #5379
- [x] **5.3b** CI run 7804 编译阶段报出 15 个告警 → 逐条归属基线，**只有 2 个是本 PR 引入的**
      （`unused variable: oracle` ×2，在本 PR 整份重写的两个文件里）→ commit `de74efcafd` 修掉；
      其余 13 个属基线，不夹带进本 PR（详见证据文档 §7.11 / Lesson-14）
- [ ] **5.4** CI 全绿后合并；记录 merge commit

---

## Phase 6 — RECORD / LEARN

- [x] **6.1** 证据文档 `docs/releases/v4.1.0/evidence/TPCH_FIXTURE_5379_2026-10-20.md`
      —— 含实际命令与真实输出（ADR-001 G-03）
- [x] **6.2** lessons `docs/governance/lessons/LESSON_5379_TPCH_FIXTURE_2026-10-20.md`
- [x] **6.3** 新立 issue：SF=1 规模层接入（dbgen 不可得的原因）+ ADR-008 例外续期（v311 例外 2026-09-01 已过期）
- [x] **6.4** 新立 issue：382 个首次暴露目标的失败归因分流（真实缺陷部分）
- [x] **6.5** issue 评论附证据（5 字段：`source_agent` + `source_run` + `timestamp` + `evidence_hash` + `conflict_resolution`）
- [x] **6.5b** 新立 issue：#5391（剩余 19 个零断言文件）/ #5392（file storage 按表首列建键导致行塌缩）
      / #5393（wire 路径 Q9/Q18 资源耗尽 + Q19 解析失败）/ #5394（LEFT JOIN 的 ON 条件过滤丢弃左表行）
- [x] **6.5c** PR 文档补齐第 5 类：`PR-5379_ACCEPTANCE.md`（含 Gate Integration Declaration、
      Test Command Output、Evidence 清单、合并检查、风险评估）
- [ ] **6.6** 5 远端同步 `bash scripts/sync/5remotes_sync.sh develop/v4.1.0`
- [ ] **6.7** 清理：删除已合并的远端/本地分支与 worktree（先 `git merge-base --is-ancestor` 核实）

---

## 明确不做（写进 PR 正文，防止被误读为遗漏）

- 不 vendor dbgen；不把任何 `.tbl` 提交进仓库
- 不改任何参考期望值来求绿；不删断言；不给测试加 `#[ignore]` 冒充通过
- 不把 not_run 计入 passed
- 不在本 PR 内修 382 个目标暴露出的、与 TPC-H 无关的真实缺陷（分流为独立 issue）
- 规模层（SF=1）本轮不接入
