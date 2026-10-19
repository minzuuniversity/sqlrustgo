# PR-5379 TEST REVIEW — TPC-H fixture 自包含化 + CI 三态披露

> **PR**: PR-5379 `fix/5379-tpch-fixture-hermeticity`
> **Reviewer**: 独立测试审核者（Verifier，非本变更作者）
> **Review Date**: 2026-10-20
> **审核窗口**: 2026-10-20 06:07 – 06:20 (GMT+8)
> **基线 commit**: `d8365606d69c`（`Merge pull request 'fix(sync): ... (#5382)'`）
> **分支**: `fix/5379-tpch-fixture-hermeticity`（未提交变更，worktree `/Volumes/workspace/dev/sqlrustgo-worktrees/wt-5379-tpch`）
> **Source PR-SPEC**: `docs/releases/v4.1.0/PR-5379_SPEC.md`
> **Source PR-TEST_PLAN**: `docs/releases/v4.1.0/PR-5379_TEST_PLAN.md`
> **Source PR-TEST_DESIGN**: `docs/releases/v4.1.0/PR-5379_TEST_DESIGN.md`
> **Source PR-Code**: 未提交工作树（见 §1.3 快照指纹）

---

## 1. 审核范围与基线

### 1.1 变更文件清单

**新增（13 项，含本审核文档）**

| 路径 | 类别 |
|---|---|
| `tests/common/tpch_fixture.rs` | 新增共享模块：fixture 解析 / profile / NOT-RUN 标记 / 已知差异登记表 |
| `tests/common/sqlite_oracle.rs` | 新增共享模块：进程内 SQLite 独立预言机 + 归一化 + `adapt_sql` + `check_against_oracle` |
| `scripts/gate/prepare_tpch_fixture.sh` | 新增：运行时生成 fixture + manifest（幂等） |
| `scripts/gate/check_test_target_execution.sh` | 新增：declared vs executed 目标执行核对门禁 |
| `scripts/gate/check_tpch_known_differences.sh` | 新增：已知差异登记合规门禁 |
| `scripts/gate/mutate_tpch_oracle_tests.sh` | 新增：引擎侧变异测试 harness |
| `tests/baseline/tpch_known_differences.json` | 新增：已知差异登记表（空表） |
| `docs/governance/lessons/LESSON_5379_TPCH_FIXTURE_2026-10-20.md` | 新增 |
| `docs/releases/v4.1.0/evidence/TPCH_FIXTURE_5379_2026-10-20.md` | 新增 |
| `docs/releases/v4.1.0/PR-5379_SPEC.md` | 新增 |
| `docs/releases/v4.1.0/PR-5379_TEST_DESIGN.md` | 新增 |
| `docs/releases/v4.1.0/PR-5379_TEST_PLAN.md` | 新增 |
| `openspec/changes/fix-5379-tpch-fixture-hermeticity/{proposal,design,tasks}.md` | 新增 |

**修改（42 项）**

- `.gitea/workflows/ci.yml`（`test` job：fixture 准备 / `--no-fail-fast` + 超时 / 三态摘要 / 目标执行核对 / G1 门禁改 NOT RUN）
- `scripts/gate/generate_tpch_sf.py`（数据现实度改造 + 4 类生成时自检）
- `tests/common/tpch_wire_harness.rs`（运行期 fixture 解析 / `start_sf001`/`start_sf01` 改 `Option` / `orders` schema 补 `o_totalprice`）
- `tests/integration/oracle/`：15 个文件（其中 **13 个**新增 `check_against_oracle` 对照 + 防空转不变量，共 35 处调用）
- `tests/integration/sql/`：`eval_22_vs_sf01.rs`、`eval_22_vs_sqlite.rs`、`q13_subquery_repro.rs`、`long_run_stability_72h_test.rs`
- `tests/integration/tpch/`：21 个文件（含 `oracle_g1_tpch_sha256.rs`、`oracle_g15_tpch_sf01.rs`）

**删除（2 项，staged）**：`tests/data/tpch-sf001`、`tests/data/tpch-sf01` —— 二者原为指向 `/tmp` 的符号链接：

```
$ git show HEAD:tests/data/tpch-sf01   # → /tmp/tpch-sf01
$ git show HEAD:tests/data/tpch-sf001  # → /tmp/tpch-sf001
```

### 1.2 忽略的测试副作用

`docs/releases/v4.1.0/evidence/a04-5226/wire/*.txt` 在 06:07 的初始 `git status --short` 中有 **81 个** 呈 `M` 状态（`a04_mysql_diff_matrix_test` 跑一次会改写它们），**不属于本变更**，审核中忽略。

**实测记录**：该 81 个文件在审核期间被作者还原（06:16 复查 `git status --short | grep -c evidence` → `0`）。即最终 diff 中已不含这批副作用。审核者未运行 `a04_mysql_diff_matrix_test`，未对这批文件做任何操作。

### 1.3 快照指纹与"移动靶"警示 ⚠️

**审核期间工作树被作者持续修改**，这是本次审核最重要的过程性发现：

| 时刻 | 观察 |
|---|---|
| 06:07 | `git status --short` → 123 项变更（含 81 个 evidence 副作用文件）；`scripts/gate/mutate_tpch_oracle_tests.sh` 的 MUT-3 片段是 `if false { unreachable!() } else if …` 形式 |
| 06:09:37 | `tests/common/sqlite_oracle.rs`、`tests/common/tpch_fixture.rs` mtime 更新 |
| 06:14:09 | `scripts/gate/mutate_tpch_oracle_tests.sh` mtime 更新 —— MUT-3 被改写为"`} else if agg.distinct {` → 去掉 `HashSet` 去重"形式，并附注说明旧形式是 NO-OP |
| 06:14:50 | `git status --short` → 42 `M` + 13 `??` + 2 `D`；81 个 evidence 副作用文件已被还原；新增 `PR-5379_SPEC.md` / `PR-5379_TEST_DESIGN.md` / `PR-5379_TEST_PLAN.md` |

本报告全部结论锚定在下列指纹（06:16 采集）。**合并前必须对冻结快照重跑一次本审核。**

```
13d5f7049a861fc146a9ae9b91e0c3f6f58f42781e58e2017a6faeb1a94461fd  scripts/gate/mutate_tpch_oracle_tests.sh
484d406b86af0e6c4e95d6e5a318c265fd120bd32b05e981375e7e6e22ce673e  scripts/gate/check_test_target_execution.sh
a49348cb8dc2024481a8e84cb6a313cce0ca0d997888a2ba739feff1f1cf1809  scripts/gate/check_tpch_known_differences.sh
da35415dca0637e310fa98347b39797252fb7af43c3cfaa2551ccb9196150e99  scripts/gate/prepare_tpch_fixture.sh
a96105b03ae6bf7e3132fdab2d535a5352419544f03a518a99c4f8707391fcf2  scripts/gate/generate_tpch_sf.py
fcbf6d5513a578e821e948d9331307620025ddadaeccbe4d086b7d41972ec693  tests/common/sqlite_oracle.rs
5f52b71584049166c5be7d331dc66b34a0dba989a8ac49c1f7449ee155f863ff  tests/common/tpch_fixture.rs
aaa764ddfe5c32c87b4f3e92e432817392e4f0fa35319f7035893e22c790e087  tests/common/tpch_wire_harness.rs
c2e72c70338529f023c9b1adb0c99de1de70839b44d42151f70aa802ce1085ad  .gitea/workflows/ci.yml
4ffa24417680183ccfdb2c1864ae454c4affd397f6e5ec3b4d2e4a7c5c345ac6  tests/baseline/tpch_known_differences.json
```

审核者复核了 06:09 之后 `sqlite_oracle.rs` / `tpch_fixture.rs` 的核心函数（`check_against_oracle` / `diff_rows` / `cells_equal` / `adapt_sql` / `FixtureState::resolve`）**内容未变**，本报告对这些函数的结论仍然成立。

### 1.4 审核者实际执行的动作

| # | 命令 | 结果 |
|---|---|---|
| A1 | `python3 scripts/gate/generate_tpch_sf.py --sf 0.01 --output /tmp/review_fx --seed 42` | 4 组自检全 OK（见 §2 证据） |
| A2 | `bash scripts/gate/prepare_tpch_fixture.sh --out /tmp/review_fx2 --sf 0.01 --seed 42` | 生成 + manifest 写入成功 |
| A3 | `bash scripts/gate/check_tpch_known_differences.sh` | PASS，0 条目 |
| A4 | `bash scripts/gate/check_tpch_known_differences.sh --registry /tmp/review_kd_1.json`（1 条目 / limit 0） | **FAIL exit 1**（无声增长被拦） |
| A5 | `bash scripts/gate/check_test_target_execution.sh --cargo-toml Cargo.toml --log /tmp/review_log_full.txt`（446 目标 + ANSI 色码） | PASS，0 missing |
| A6 | 同上，`--log /tmp/review_log_trunc.txt`（只跑 64 个目标） | **FAIL exit 1**，列出 382 个 missing |
| A7 | `TPCH_DATA_DIR=/tmp/review_fx2 cargo test --test diag_q11 --test diag_q11_where --test diag_shipdate_type --test q13_subquery_repro -- --nocapture` | `test result: ok. 31 passed; 0 failed` |
| A8 | `TPCH_DATA_DIR=/tmp/review_fx2 cargo test --test eval_22_vs_sf01 -- --nocapture` | 22/22 MATCHED（**但见 §3 B-3 的空集问题**） |
| A9 | `TPCH_DATA_DIR=/tmp/review_fx2 cargo test --test oracle_g1_tpch_sha256 -- --nocapture` | profile_mismatch → NOT-RUN 披露 |
| A10 | `TPCH_DATA_DIR=/tmp/review_fx2 cargo test --test oracle_g15_tpch_sf01` | `ok. 22 passed` —— **但 22 个 query 全部 SKIP** |
| A11 | `TPCH_DATA_DIR=/tmp/review_fx2 cargo test --test tpch_value_test_v2` | **FAILED**（baseline 路径已随符号链接消失） |
| A12 | `TPCH_DATA_DIR=/tmp/review_empty_fx cargo test --test diag_shipdate_type`（空目录） | **FAILED / panic** |
| A13 | `TPCH_DATA_DIR=/tmp/review_empty_fx cargo test --test eval_22_vs_sf01`（同一空目录） | OK / NOT-RUN 披露 —— **与 A12 行为分叉** |
| A14 | `cargo fmt --check` | exit 0 |

**审核者未执行 `scripts/gate/mutate_tpch_oracle_tests.sh` 全程** —— 它会临时改写仓库源文件 `src/engine_select.rs`。审核者改为把引擎文件复制到 `/tmp/review_mut/` 后，对副本应用三个变异片段（见 §2 证据 D-3）。

---

## 2. 逐项审核结论

按 `docs/governance/TEST_REVIEW_TEMPLATE.md` §2 的 8 大类 30 子项组织，**由审核者自行判定适用性**（N/A = 该子项对本变更不适用）。

### 2.1 覆盖设计（Coverage Design）

| # | 检查项 | 通过标准 | 结论 | 证据（命令 + 实际输出片段） |
|---|---|---|---|---|
| 2.1.1 | 覆盖矩阵完整 | 每条需求 → ≥1 测试用例 | **PASS** | `PR-5379_TEST_PLAN.md` §2 覆盖矩阵 11 行，全部落到可执行位置（fixture 幂等 / 4 类自检 / 13 文件补断言 / 10 项 oracle 单测 / 双模式实测 / 门禁脚本 / 变异测试） |
| 2.1.2 | 边界覆盖 | min/max/空/单元素/大集合/重复值 | **PASS** | 4 类生成时自检实测全 OK：`partsupp.ps_suppkey distinct suppliers: 100 (need >= 100)` / `orders.o_custkey distinct customers: 1500 (need >= 1425)`；边界覆盖含 `p_brand='Brand#23' AND p_container='LG CASE': 7 rows`（复合过滤非空） |
| 2.1.3 | 异常路径 | error 码、panic 安全、resource leak | **FAIL** | 见 §3 **B-1**（`oracle_g15_tpch_sf01` 在 fixture 存在、baseline 缺失时 panic-free 但**静默通过**）、**B-2**（`tpch_value_test_v2` panic）、**I-1**（现存空目录 → panic 而非 NOT-RUN） |
| 2.1.4 | 并发场景 | 多线程/竞态 | **N/A** | 本变更不引入并发逻辑。唯一并发面是 `TPCH_NOT_RUN_FILE` 的多进程追加（见 **S-5**，按 建议 处理） |
| 2.1.5 | 性能/规模 | SF=1 至少 1 个 query 不退化 | **DEFERRED** | SF=1 层未接入 CI（`ci.yml:170` 明确 NOT RUN，跟踪 #5387）。审核者无法在无 SF=1 fixture 的环境下验证 |

### 2.2 独立性（Independence）

| # | 检查项 | 通过标准 | 结论 | 证据 |
|---|---|---|---|---|
| 2.2.1 | 无外部依赖 | 不依赖外部 DB/网络/文件系统（除临时） | **PARTIAL** | SQLite 预言机用 `rusqlite` bundled（`Cargo.toml:252`），无外部二进制 ✔。但 `eval_22_vs_sf01.rs:258,293` 仍 `Command::new("sqlite3")`（**预先存在**，非本变更新增）—— 本变更使该测试在 CI 首次真正执行，风险被本变更放大（见 **I-5**） |
| 2.2.2 | 可重复执行 | 同一测试连跑 10 次结果一致 | **PARTIAL** | 生成器确定性已验证（`prepare_tpch_fixture.sh` 幂等 + manifest 记录 `generator_sha256`）。**未执行 10 次连跑**——标注"未验证" |
| 2.2.3 | 无顺序耦合 | 可单跑、可全跑、可乱序跑 | **PASS** | A7 四个目标一起跑 → `ok. 31 passed`；A8–A11 各自单跑均达到预期终态 |
| 2.2.4 | 资源清理 | 临时文件/连接/线程全部释放 | **PASS** | 4 个 gate 脚本均用 `trap 'rm -rf -- "${TMP_DIR}"' EXIT`（`check_test_target_execution.sh:129`、`check_tpch_known_differences.sh:106`、`prepare_tpch_fixture.sh:261`）；`mutate_tpch_oracle_tests.sh:83,132` 用 `trap restore_engine EXIT INT TERM` + `rm -f` 片段文件 |
| 2.2.5 | 无全局状态污染 | 修改全局 state 后必须恢复 | **PASS** | 审计者复制引擎文件到 `/tmp` 验证变异，未触碰仓库；`mutate_tpch_oracle_tests.sh:75-83` 的 `restore_engine()` 在 EXIT/INT/TERM 三个信号上恢复 `src/engine_select.rs` |

### 2.3 断言质量（Assertion Quality）—— 重点

| # | 检查项 | 通过标准 | 结论 | 证据 |
|---|---|---|---|---|
| 2.3.1 | 断言具体 | `assert_eq!(x, 42)` 而非 `assert!(x.is_ok())` | **PASS** | 对 13 个 oracle 文件做全量正则扫描，硬编码数字断言**全部是结构不变量**，无引擎输出回填：`assert_eq!(engine_having_none.rows.len(), 0)`（`diag_q11.rs:176`）、`assert_eq!(engine_group.columns.len(), 2)`（`diag_q11_3way.rs:269`）、`assert_eq!(engine_sample.rows.len(), 5)`（`diag_shipdate_type.rs:321`）、`assert_eq!(result.rows.len(), 1)`（`diag_q14_full.rs:187` / `diag_q6_filter.rs:135`） |
| 2.3.2 | 错误信息有用 | `assert!(x, "expected y, got {}", actual)` | **PASS** | 抽样 `diag_q11_where.rs:171-175`：`assert_eq!(partsupp_rows + supplier_rows + nation_rows, loaded as i64, "the engine must see exactly the rows that were loaded into the storage")`；`diag_shipdate_type.rs:194-197` 指明行号与实际类型 |
| 2.3.3 | 不依赖 panic 顺序 | 不依赖"先 panic 再 assert"路径 | **PASS** | 13 个文件均先 `require(TARGET)` 早退再加载数据；`oracle_rows()` 在 SQLite 不可跑时返回 `None` 而非 panic（`diag_q11_where.rs:69-87`） |
| 2.3.4 | 不依赖 print 输出 | 关键值必须断言，不能靠 println 看 | **FAIL** | 见 §3 **B-1**：`oracle_g15_tpch_sf01` 全靠 `[SKIP] …` 打印，22 个 query 一个没比，却报 `ok. 22 passed` |
| 2.3.5 | 数值精度显式 | 浮点比较用 epsilon 或精确有理数 | **PASS** | `sqlite_oracle.rs:60` `FLOAT_REL_TOLERANCE = 1e-9`；`cells_equal` 相对误差 `(x-y).abs()/max(|x|,|y|) <= 1e-9`；`format_real` 归一化整数浮点（`17` ≡ `17.0`）；自带单测 `numeric_cells_compare_across_int_and_float_forms` 通过 |

**追加专项：自证（circularity）检查 —— PASS**

对 13 个 oracle 文件逐行审查，未发现任何一条"拿本引擎自己的输出回填期望值"：

- 期望值来源全部是 **SQLite 进程内预言机**（`SqliteOracle::open(fixture, &TABLES)`，35 处 `check_against_oracle` 调用）；
- SQLite 的 `.tbl` 解析路径**独立于引擎加载路径**：`sqlite_oracle.rs:162-174` 自己 `line.split('|')`，不调用 `tpch_fixture::parse_tbl`，也不经过引擎的 `LOAD DATA` / `storage.insert`；
- 唯一共用的是 fixture **文件**本身（不可避免），不共用加载器实现；
- 引擎侧 schema 由各测试自己 `CREATE TABLE` 声明，SQLite 侧 schema 是 `sqlite_oracle.rs:68-101` 的 `TPC_H_SCHEMA` —— 两条独立 DDL。

**追加专项：防空转（anti-vacuity）—— PARTIAL**

各 oracle 文件确实系统性地加了防空转断言，抽样：

- `diag_q11.rs:112-117`：`assert!(groups > 0, "the 3-way join with n_name = 'GERMANY' matched no rows …")`
- `diag_q11_where.rs:259-273`：`germany_rows > 0`、`filtered > 0`、`filtered < nation_only`
- `diag_shipdate_type.rs:315-319`：`eq_count > 0`（"the probe date '1994-01-01' must occur in the data"）

但**生成器侧**的防空转存在缺口，见 §3 **B-3**：4 类自检无法覆盖"相关子查询 / NOT EXISTS / HAVING"形态，导致 `eval_22_vs_sf01` 的 22/22 中至少 1 项是 0-vs-0 空集对比。

### 2.4 真实可执行（Real Executability）

| # | 检查项 | 通过标准 | 结论 | 证据 |
|---|---|---|---|---|
| 2.4.1 | 命令可跑 | `cargo test --test <name>` 100% PASS | **PARTIAL** | A7 `ok. 31 passed`、A8 22/22 MATCHED、A9/A10 `ok`。**但 A11 `tpch_value_test_v2` → `test result: FAILED. 16 passed; 1 failed`**（见 **B-2**） |
| 2.4.2 | 无 `#[ignore]` 隐藏 | 所有 `#[ignore]` 必须有理由 + tracking issue | **PASS** | `diag_q7_sf1_real_count.rs:19` `#[ignore] // Heavy: SF=1 lineitem is 6M rows` + doc 注释给出运行命令；`oracle_g1_tpch_sha256.rs:213` `#[ignore = "Run with: cargo test --test oracle_g1_tpch_sha256 -- --ignored --generate-baseline"]` |
| 2.4.3 | 无 TODO 占位 | 不允许 "TODO: add test" | **PASS** | 对变更文件全量 grep `TODO` / `FIXME`：无测试占位残留（`#![allow(dead_code)]` 仅用于共享模块，见 **I-2**） |
| 2.4.4 | 无 commented-out 测试 | 不允许 `// #[test]` | **PASS** | 13 个 oracle 文件无注释掉的测试 |
| 2.4.5 | 跑通时间合理 | 单 crate 测试 ≤5 分钟 | **PASS** | A7 四个目标合计 `finished in 1.31s`（加载 59 903 行 lineitem）；A8 `eval_22_vs_sf01` 22 查询全跑完；`oracle_g1_tpch_sha256` 22 查询共用一个 server（`oracle_g1_tpch_sha256.rs:163-165` 注释说明由"每查询重启一次"改为共享，省掉 ~20 分钟） |

### 2.5 性能与稳定性（Performance & Stability）

| # | 检查项 | 通过标准 | 结论 | 证据 |
|---|---|---|---|---|
| 2.5.1 | 内存安全 | 8GB 限制内 | **PASS** | SF=0.01 fixture 实测 `Total: 8,342,297 bytes (7.96 MB)`；A7 四个目标 31 测试 1.31s 完成 |
| 2.5.2 | 无 O(n²) 隐式 | 大输入下不退化为 O(n²) | **PASS** | `tpch_fixture.rs:443` `BATCH_SIZE = 10000` 批量 `storage.insert`；`q13_subquery_repro.rs:29-32` 明确记录"旧版逐行 INSERT 86 630 次，单目标 20 分钟"的教训已修 |
| 2.5.3 | 无锁泄漏 | 多线程测试无死锁 | **PASS** | A7 并行 4 个目标无死锁；`q13_subquery_repro.rs` 使用 `Arc<RwLock<MemoryStorage>>` 与 `load_into_storage` 批量路径（避开 `engine.execute("INSERT …")` 逐行导致的 `Arc<RwLock<Storage>>` 死锁，见该文件 21-23 行注释） |
| 2.5.4 | 无 race condition | loom / `--features race` 验证 | **N/A** | 本变更无新增共享可变状态（仅 `TPCH_NOT_RUN_FILE` 追加，见 **S-5**） |
| 2.5.5 | 退出码 0 | 失败 panic → 明确 exit code | **PASS** | `check_test_target_execution.sh` 退出码语义明确（0/1/2，脚本头 44-47 行）；实测 A5 → exit 0、A6 → exit 1；A4 → exit 1 |

### 2.6 文档与可读性（Documentation）

| # | 检查项 | 通过标准 | 结论 | 证据 |
|---|---|---|---|---|
| 2.6.1 | 测试名自解释 | 命名可读 | **PASS** | `diag_q11_having_three_way`、`diag_q11_where_only`、`test_shipdate_actual_value`、`q13_bare_subquery_row_count` |
| 2.6.2 | 关键步骤注释 | 复杂 setup 有 why 注释 | **PASS** | `tpch_wire_harness.rs:88-105` 详述 `o_totalprice` 缺失导致的**列移位**（`o_orderdate` 读到价格、`o_comment` 掉出行尾）及其为何"静默"；`tpch_fixture.rs:1-61` 模块级 why |
| 2.6.3 | 引用 SPEC 条款 | doc-comment 引用 SPEC id | **PASS** | 13 个 oracle 文件头均写明 "What changed (issue #5379)"，并说明"旧文件 0 断言"的具体旧代码 |
| 2.6.4 | README/CONTRIBUTING 更新 | 新测试模式需文档化 | **PASS** | 新增 `PR-5379_SPEC.md` / `PR-5379_TEST_DESIGN.md` / `PR-5379_TEST_PLAN.md` / `LESSON_5379_TPCH_FIXTURE_2026-10-20.md` / `evidence/TPCH_FIXTURE_5379_2026-10-20.md` |
| 2.6.5 | 错误信息可定位 | 失败信息含上下文 | **PASS** | `check_test_target_execution.sh:389` 逐条打印 `target 'X' was declared but never executed (path: Y)`；`sqlite_oracle.rs:463-467` panic 信息带 `case_id` + 差异定位 + 登记指引 |

### 2.7 安全与权限（Security）

| # | 检查项 | 通过标准 | 结论 | 证据 |
|---|---|---|---|---|
| 2.7.1 | 无 SQL 注入 | 动态 SQL 用参数化 | **PASS** | `sqlite_oracle.rs:160,172` 用 `prepare` + `params_from_iter` 参数化插入；测试侧固定 SQL 字符串 |
| 2.7.2 | 无路径穿越 | 临时文件用 tempfile crate | **PARTIAL** | `prepare_tpch_fixture.sh:260` 用 `mktemp -d` ✔；但 `eval_22_vs_sf01.rs:48` `const TMP_DIR: &str = "/tmp/tpch_22_sf01_audit";` 是**硬编码共享临时目录**（预先存在，非本变更新增）——多机/并发 CI 会互相覆盖。`mutate_tpch_oracle_tests.sh` 用 `mktemp -t` ✔ |
| 2.7.3 | 无密钥硬编码 | 测试无 hardcoded credential | **PASS** | 新增文件无任何凭据/口令字面量 |
| 2.7.4 | 无越权访问 | 最小权限 | **N/A** | 本变更不涉及权限模型 |
| 2.7.5 | 输入消毒 | fuzz/恶意输入不 panic | **PASS** | `sqlite_oracle.rs:162-170` 对 `fields.len() < ncols` 的行 `continue` 跳过；`tpch_fixture.rs:456-458` 同样跳过短行；`matching_paren`（`sqlite_oracle.rs:416-432`）对不平衡括号返回 `None` 而非 panic |

### 2.8 集成与门禁（Integration & Gate）—— 重点

| # | 检查项 | 通过标准 | 结论 | 证据 |
|---|---|---|---|---|
| 2.8.1 | clippy 0 warning | `cargo clippy -- -D warnings` PASS | **未验证** | 审核者**未运行** clippy（全量构建成本过高）。标注"未验证"，不计入结论 |
| 2.8.2 | fmt 0 error | `cargo fmt --check` PASS | **PASS** | `cargo fmt --check` → `FMT_EXIT=0`，无输出 |
| 2.8.3 | 集成测试可触发 | 集成测试在 CI 默认跑 | **PASS** | `ci.yml:101` `timeout 7200 cargo test --all-features --no-fail-fast`；`ci.yml:85` 先生成 fixture 并 `GITHUB_ENV` 导出 `TPCH_DATA_DIR` / `TPCH_NOT_RUN_FILE` |
| 2.8.4 | 门禁脚本就绪 | 对应 gate 脚本包含新测试 | **FAIL** | 4 个新 gate 脚本中**只有 2 个接入 CI**：`ci.yml:85` `prepare_tpch_fixture.sh`、`ci.yml:155` `check_test_target_execution.sh`。`grep -rn "check_tpch_known_differences\|mutate_tpch_oracle_tests" .gitea/workflows/` → **0 命中**（见 **I-3**）。另：`ci.yml:158-176` 的 G1 门禁被改成永久 NOT RUN（见 **B-4**） |
| 2.8.5 | 证据可重放 | 任意审核者可一键复现 PASS | **PARTIAL** | A1/A2/A3/A5/A7/A8 全部可一键复现，命令已写入本报告 §1.4。**但 `mutate_tpch_oracle_tests.sh` 不可在"只读审核"前提下复现**（它会临时改写仓库源文件），审核者只能做副本级验证 |

#### 2.8 追加专项 A：目标执行核对门禁有效性 —— **PASS** ✅

**声明 vs 执行的解析可靠性**：`Running` 行格式与 ANSI 色码处理经实测正确。

```
# A5：446 目标 + cargo 原生 ANSI 色码
$ bash scripts/gate/check_test_target_execution.sh --cargo-toml Cargo.toml --log /tmp/review_log_full.txt
  PASS: declared test targets: 446
  log: /tmp/review_log_full.txt (447 'Running' line(s))
  declared test targets:   446
  executed test targets:   446
  extra lib/bin binaries:  1
  missing test targets:    0
Test target execution gate PASS: all 446 declared test targets were executed.
EXIT=0
```

**ANSI 剥离确有必要**（脚本注释所称属实）—— 对同一份带色码日志分别用剥离/不剥离的 Python 正则提取：

```
RAW(no strip): matched_ok=0 bad_capture=447      ← 首个坏捕获: '\x1b[0m unittests src/lib.rs'
STRIPPED:      matched_ok=446 bad_capture=0
```

即不剥离色码会导致 446 个目标全部被判"漏跑"（误报 FAIL），剥离后 0 误报。脚本注释（`check_test_target_execution.sh:32-37`）描述准确。

**截断日志必须 FAIL**：

```
# A6：只跑 64 个目标
$ bash scripts/gate/check_test_target_execution.sh --cargo-toml Cargo.toml --log /tmp/review_log_trunc.txt
  declared test targets:   446
  executed test targets:   64
  missing test targets:    382
  FAIL: target 'incremental_backup_datasource_4938' was declared but never executed (path: crates/tools/tests/incremental_backup_datasource_4938.rs)
  ...
Test target execution gate FAIL: 382 of 446 declared test targets never ran.
EXIT=1
```

**382** 这个数字与 CI run 7613 实测的"446 声明 / 64 执行"完全吻合，说明门禁参数与真实事故同构。`[[test]]` 解析器对 `crates/tools/tests/*` 这类子 crate 路径也正确处理。

#### 2.8 追加专项 B：已知差异门禁规则 —— **PASS**（规则有效） / **I-3**（未接入 CI）

规则本身能挡住"无声增长"：

```
# A3：空表
$ bash scripts/gate/check_tpch_known_differences.sh
  total entries:  0    limit: 0    violations: 0
Known-differences registry PASS.  EXIT=0

# A4：1 条目，limit 0
$ bash scripts/gate/check_tpch_known_differences.sh --registry /tmp/review_kd_1.json
  total entries:  1    limit: 0    violations: 1
  FAIL: registry holds 1 entries but the limit is 0 (known differences may not grow silently)
Known-differences registry FAIL: 1 violation(s).  EXIT=1
```

规则覆盖：缺字段 / 类型错 / `issue` 或 `owner` 为空 / `case_id` 重复 / `since` 非 `YYYY-MM-DD` / `actual_row_count == oracle_row_count`（陈旧条目强制清理）/ 条目数超限 —— 8 条规则逐条实现于 `check_tpch_known_differences.sh:158-229`。

#### 2.8 追加专项 C：变异点是否落在引擎实现上 —— **PASS**（副本级验证）⚠️ 未跑全程

审核者把 `src/engine_select.rs` 复制到 `/tmp/review_mut/`，对副本应用脚本内嵌的三个变异片段：

```
MUT-1: rc=0 stdout='MUT-1 applied: HAVING predicate inverted'
   -                    if !crate::engine_utils::eval_predicate_with_subq(
   +                    if crate::engine_utils::eval_predicate_with_subq(
MUT-2: rc=0 stdout='MUT-2 applied: SUM drops the first value'
   -                    for v in &values {
   +                    for v in values.iter().skip(1) {
MUT-3: rc=0 stdout='MUT-3 applied: COUNT(DISTINCT x) de-duplication lost'
   -                        // COUNT(DISTINCT col) - count unique non-NULL values
   -                        use std::collections::HashSet;
   ...
   +                        // MUT-3: the de-duplication is lost — every non-NULL
   +                        // value is counted, not every distinct value.
```

三个变异点全部落在**引擎实现**（`src/engine_select.rs` 的聚合/谓词求值路径），不是测试代码；锚点在当前快照中各自**恰好命中 1 次**（`count(old) != 1` 即拒绝应用并报 ERROR，符合脚本设计）。

⚠️ **未验证**：`cargo test` 层面的 "3 个变异必须全部 CAUGHT" 未实测（需临时改写仓库源文件，超出审核者只读边界）。**该结论不能由本报告背书。**

#### 2.8 追加专项 D：CI 三态摘要算术 —— **PASS**（算术正确） / **I-4**（无告警阈值）

用合成日志模拟"3 个 not-run + 3 个 ok + 1 个 FAILED"：

```
BIN_OK=0 BIN_FAIL=1 TEST_OK=3 TEST_FAIL=1 IGN=0 NOT_RUN=3 PASSED=0
```

`PASSED = TEST_OK - NOT_RUN`（`ci.yml:138`）**确实把 not_run 从 passed 里扣掉了**，不是"扣了但没生效"：

- not-run 的测试在 libtest 里报 `ok`（测试体提前 `return`），所以 `TEST_OK` 天然含它们；
- `NOT_RUN` 从 `TPCH_NOT_RUN_FILE` 侧信道计数（`ci.yml:133`），**不依赖 stdout 捕获**——这是必要的，因为 libtest 会吞掉通过测试的 stdout；
- 下界保护 `if [ "$PASSED" -lt 0 ]; then PASSED=0; fi`（`ci.yml:139`）防止负数。

`--no-fail-fast` + `timeout 7200` 组合**安全**：`ci.yml:101` `timeout 7200 cargo test --all-features --no-fail-fast 2>&1 | tee test_report.log`，`set -o pipefail` 已开（`ci.yml:90`）→ 超时时 `timeout` 返回 124、管道非零、步骤失败；部分日志仍落盘供 `check_test_target_execution.sh` 核对漏跑目标。

### 2.9 追加专项：可维护性 —— **PARTIAL**

| 检查 | 结论 | 证据 |
|---|---|---|
| `#[path]` 重复引入导致行为分叉 | **PASS** | `sqlite_oracle.rs:57` 用 `use crate::tpch_fixture::Fixture;` —— 解析到**测试二进制 crate 根**的 `tpch_fixture` 模块。已验证 15 个引用 `sqlite_oracle` 的文件**全部**同时声明 `mod tpch_fixture`（`grep` 逐文件核对，`mod_tpch_fixture=1 mod_sqlite_oracle=1`），因此 `crate::tpch_fixture::lookup_known_difference` 不会解析到错误副本 |
| `tpch_wire_harness` 与 `tpch_cli_harness` 的 TPC-H schema 一致 | **PASS** | 程序化比对两个文件的**全部** DDL 块（cli 16 块 / wire 8 块）：8 张表**逐字相同**。本变更给 wire harness 补上 `o_totalprice` 后，两边 `orders` 均为 9 列同类型。（cli harness 另有一套 DDL 变体，`l_quantity INTEGER` vs wire 的 `REAL` —— **预先存在**，非本变更引入；加载器按内容而非声明类型定型，无实际影响） |
| 共享模块的重复实现 | **PARTIAL** | `NOT_RUN_MARKER` / `report_not_run` / 候选目录列表在 `tpch_fixture.rs` 与 `tpch_wire_harness.rs` 各存一份（后者注释自称 "the two must stay in sync"）。同一 CI 计数前缀有两处定义 → 漂移风险（见 **S-4**） |
| 两个 fixture 解析器行为一致 | **FAIL** | `FixtureState::resolve()` 只查 `is_dir()`；`resolve_fixture_dir()` 要求 `lineitem.tbl` 存在 → 同一输入两种结局（见 **I-1**） |

### 2.10 追加专项：数据现实度 —— **FAIL**（见 B-3）

`generate_tpch_sf.py` 的 4 类生成时自检**实测全部通过**：

```
=== key distribution self-check ===
  OK   partsupp.ps_suppkey distinct suppliers: 100 (need >= 100)
  OK   partsupp.ps_partkey distinct parts: 2000 (need >= 100)
  OK   supplier.s_nationkey distinct nations: 25 (need >= 25)
  OK   customer.n_nationkey distinct nations: 25 (need >= 25)
  OK   lineitem.l_suppkey distinct suppliers: 100 (need >= 100)
  OK   lineitem.l_partkey distinct parts: 2000 (need >= 100)
  OK   orders.o_custkey distinct customers: 1500 (need >= 1425)
=== nation to region mapping ===
  OK   all 25 nations sit in their canonical region
=== predicate coverage self-check ===
  OK   orders.o_comment LIKE '%special%requests%': 747/15000 (4.98%, need >= 1.00%)
  OK   orders.o_orderpriority = '1-URGENT': 2991/15000 (19.94%, need >= 5.00%)
  OK   part.p_brand = 'Brand#23': 83/2000 (4.15%, need >= 1.00%)
  OK   part.p_name LIKE '%green%': 103/2000 (5.15%, need >= 1.00%)
  OK   part.p_type = 'ECONOMY ANODIZED STEEL': 16/2000 (0.80%, need >= 0.10%)
  OK   part.p_type LIKE '%BRASS': 436/2000 (21.80%, need >= 2.00%)
  OK   part.p_type LIKE 'MEDIUM POLISHED%': 72/2000 (3.60%, need >= 2.00%)
  OK   supplier.s_comment LIKE '%Customer%Complaints%': 10/100 (10.00%, need >= 5.00%)
  OK   customer.c_phone country code in Q22's list: 428/1500 (28.53%, need >= 5.00%)
  OK   lineitem.l_returnflag = 'R': 19981/60000 (33.30%, need >= 10.00%)
  OK   lineitem.l_shipmode = 'AIR': 8569/60000 (14.28%, need >= 5.00%)
  OK   lineitem.l_discount BETWEEN 0.05 AND 0.07: 12107/60000 (20.18%, need >= 5.00%)
=== compound filter coverage self-check ===
  OK   part.p_brand = 'Brand#23' AND p_container = 'LG CASE': 7 rows
  OK   part.p_brand = 'Brand#12' AND p_container IN ('SM CASE','SM BOX','SM PACK'): 21 rows
  OK   part.p_type = 'ECONOMY ANODIZED STEEL' AND p_size = 15: 1 rows
Done.
```



**但自检的覆盖面与它自己声称的目标不符** —— 见 §3 **B-3**。另见 **I-6**（分母用理论行数）。

---

## 3. 发现清单

### 阻断（必须先解决）

---

#### B-1 `oracle_g15_tpch_sf01` 在 CI 变成"静默通过"：22 个 query 一个没比，却报 `ok. 22 passed`

- **文件:行**：`tests/integration/tpch/oracle_g15_tpch_sf01.rs:97-134`（跳过分支在 `:99-106`，断言在 `:131-134`）
- **问题**：
  `require_fixture()` 只检查 **fixture**。而 **baseline**（`tests/data/tpch-sf01/expected/Q*_sf01_baseline.json`）原本挂在被本变更删除的 `tests/data/tpch-sf01` 符号链接后面，**从未入库**。现在 CI 里 fixture 可解析了 → `require_fixture` 通过 → 循环里每个 query 命中 `if !baseline_path.exists() { continue; }` → 22 次 SKIP → `all_pass` 始终为 `true` → `assert!(all_pass)` 通过。
  这正是本变更宣称要消灭的"**会骗人**"形态：一个绿色的、什么都没比对的测试。而且它是**新可达**的——正是本变更让 fixture 在 CI 可解析才触发的。
- **复现命令**：
  ```bash
  cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-5379-tpch
  TPCH_DATA_DIR=/tmp/review_fx2 cargo test --test oracle_g15_tpch_sf01
  ```
- **实测输出**：
  ```
  [SKIP] Q1: baseline not found: tests/data/tpch-sf01/expected/Q1_sf01_baseline.json
  [SKIP] Q2: baseline not found: tests/data/tpch-sf01/expected/Q2_sf01_baseline.json
  ... （Q1..Q22 全部 SKIP）
  test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  ```
  注意 `finished in 0.00s` —— 它根本没连过数据库。
- **影响**：CI 摘要会把这个目标计入 `passed`（无 NOT-RUN 标记），形成一条"绿色但零覆盖"的假账。
- **建议改法**：
  1. 缺 baseline 时走 `report_not_run(target, "baseline_missing", …)` 而非 `continue`；或
  2. 统计**实际比对数**并断言 `compared == 22`；或
  3. 把 SF=0.01 baseline 纳入版本管理（生成后提交到 `tests/baseline/`）。
  同一模式也出现在 `tests/integration/tpch/tpch_sf01_22_vs_sqlite.rs` 与 `tpch_sf01_22_vs_sqlite_test.rs`（见 **S-8**）。

---

#### B-2 `tpch_value_test_v2` 在 CI 硬失败：fixture 现在能解析，但它读的 baseline 路径随符号链接一起消失了

- **文件:行**：`tests/integration/tpch/tpch_value_test_v2.rs:58`、`:127`、`:143`（读 baseline）；`tests/common/tpch_wire_harness.rs:10`（`SF001_DIR`）、`:260`（`read_baseline` 的 panic）
- **问题**：本变更删除了 `tests/data/tpch-sf001` 符号链接，但 `tpch_value_test_v2` 仍用常量 `SF001_DIR = "tests/data/tpch-sf001"` 去读 `expected/Q*_three_way.json`。改造前该测试在 fixture 解析处就 panic（红）；改造后 fixture 能解析了，测试**往前走**并在 baseline 读取处 panic（仍然红，但错误信息完全误导）。
  **同仓已有正确做法**：`tests/integration/sql/eval_22_vs_sqlite.rs` 已改为运行期解析 `EXPECTED_CANDIDATES`（`["tests/data/tpch-sf001/expected", "tests/data/tpch-fixture/expected", "data/tpch-sf01/expected"]` + `fixture.join("expected")`）。`tpch_value_test_v2` 未做同样改造 —— **迁移不完整**。
- **复现命令**：
  ```bash
  cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-5379-tpch
  TPCH_DATA_DIR=/tmp/review_fx2 cargo test --test tpch_value_test_v2 -- --nocapture
  ```
- **实测输出**：
  ```
  thread 'test_tpch_22_value_assertions' panicked at tests/integration/tpch/../../common/tpch_wire_harness.rs:260:58:
  read tests/data/tpch-sf001/expected/Q1_three_way.json: No such file or directory (os error 2)
  test test_tpch_22_value_assertions ... FAILED
  test result: FAILED. 16 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.69s
  ```
- **影响**：`ci.yml:128` `if [ "$TEST_FAIL" -gt 0 ]; then exit 1; fi` → **CI 红灯**。`PR-5379_TEST_PLAN.md` 的 "P2 GREEN：40 个目标在新 fixture + 新 schema 下通过" 不成立。
- **建议改法**：照搬 `eval_22_vs_sqlite.rs` 的运行期 baseline 解析 + NOT-RUN 披露；并删除 `tpch_wire_harness.rs:10-11` 的 `SF001_DIR` / `SF01_DIR` 常量（后者已无使用者）。

---

#### B-3 `eval_22_vs_sf01` 的 "22/22 MATCHED" 至少含 1 项空集对比、3 项近空集对比；生成器的 4 类自检恰好无法拦住它自己声称要拦的缺陷

- **文件:行**：`tests/integration/sql/eval_22_vs_sf01.rs:287-307`（`sqlite_count`，只比 `COUNT(*)`）、`:402-430`（分类与 `MATCHED` 判定）；根因在 `scripts/gate/generate_tpch_sf.py`（`orders.o_custkey` 均匀随机 + `:318-337` 键分布自检）
- **问题**：
  `eval_22_vs_sf01` 只比对**行数**（`SELECT COUNT(*) FROM (q) sub`），不比对行内容。而在生成的 SF=0.01 fixture 上，多个标准查询的结果集是空或近乎空：

  | 查询 | 内部实际匹配行数 | 对照结果 |
  |---|---|---|
  | Q22 | 200 个客户通过 phone/acctbal 过滤，**0** 个通过 `NOT EXISTS (orders)` | `engine=Some(0) sqlite=Some(0) MATCHED` ← **0 vs 0 空集对比** |
  | Q18 | 仅 **1** 个订单满足 `HAVING SUM(l_quantity) > 300` | `engine=Some(1) sqlite=Some(1) MATCHED` |
  | Q19 | 仅 **1** 条 lineitem 满足 5 路复合过滤 | `engine=Some(1) sqlite=Some(1) MATCHED` |
  | Q2 | 2 行 | MATCHED |
  | Q20 | 3 行 | MATCHED |

  根因：`orders.o_custkey` 在 1500 个客户上均匀随机抽 15000 个订单 → **每个客户至少 1 个订单**（实测：1500/1500 个客户有订单，0 个客户无订单）→ Q22 的"无订单客户"集合恒空。而生成器自己的键分布自检 `orders.o_custkey distinct customers: 1500 (need >= 1425)` **要求** ≥95% 客户有订单，**等于在机制上保证了 Q22 的空集**。

  生成器 4 类自检的**自称目标**（`generate_tpch_sf.py` predicate coverage 段落注释）是：
  > "the standard query set would match empty or near-empty result sets, and every comparison against the independent oracle would be vacuous"

  但覆盖清单只含**单列谓词**和 3 个**手挑的品牌×容器/尺寸**复合谓词，**不覆盖** `NOT EXISTS` 相关子查询、`HAVING` 聚合阈值、Q19 的 5 路复合过滤。**实测证明该目标未达成。**
- **复现命令**：
  ```bash
  cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-5379-tpch
  # (1) 跑对照测试，看 Q22 = 0 vs 0
  TPCH_DATA_DIR=/tmp/review_fx2 cargo test --test eval_22_vs_sf01 -- --nocapture
  # (2) 统计 Q22 过滤漏斗
  python3 - <<'PY'
  from collections import Counter
  c=Counter()
  for line in open("/tmp/review_fx2/orders.tbl"):
      if line.strip(): c[line.split("|")[1]]+=1
  print("有订单的客户数:", len(c), " 无订单客户数:", 1500-len(c))
  PY
  ```
- **实测输出**：
  ```
  Q18: engine=Some(     1) sqlite=Some(     1)  MATCHED
  Q19: engine=Some(     1) sqlite=Some(     1)  MATCHED
  Q22: engine=Some(     0) sqlite=Some(     0)  MATCHED
  === Totals (86533 rows loaded) ===
  MATCHED   : 22/22
  ---
  有订单的客户数: 1500   无订单客户数: 0
  ---
  SQLite 侧过滤漏斗（审核者独立测量）：
         200  Q22 inner: customers w/ qualifying phone & acctbal (before NOT EXISTS)
           0  Q22 final: customers in the result set
           15  Q17 matching lineitems
            1  Q19 matching lineitems
          680  Q14 matching lineitems
            1  Q18 orders with SUM(l_quantity)>300
            3  Q20 final rows
            2  Q2 final rows
  ```
- **影响**：`eval_22_vs_sf01` 的 "22/22 MATCHED" 是本变更对外的核心结论之一（`PR-5379_TEST_PLAN.md` L2 层），但其中至少 1/22 是空集比较、3/22 是 1–3 行的比较。这使该结论的实际强度显著低于字面含义；"引擎与预言机一致"在**退化结果集**上并不等于"有意义"。
- **建议改法**：
  1. **新增第 5 类生成时自检**：对 22 条标准查询逐条测量**聚合前**的行数，断言 ≥ 阈值（如 Q22 ≥ 20 个无订单客户、Q18 ≥ 50 个满足 HAVING 的订单、Q19 ≥ 200 条 lineitem）。这是唯一能真正兑现"防空转"的自检形态。
  2. 让 `orders.o_custkey` 的分配**确定性地留下**约 5% 无订单客户（真实 dbgen 行为），而不是均匀随机 —— 否则 SF=1 也有同样问题（10 单/客户的比例下泊松近似 P(0 单) ≈ e^-10 ≈ 0）。
  3. `eval_22_vs_sf01` 改为**逐行 cell 级**对照（`tpch_sf01_22_vs_3engines.rs:280-305` 已有现成实现），并对"两侧同时为空"的查询显式标 `EMPTY` 而非 `MATCHED`。

---

#### B-4 CI 注释里的兜底声明不成立：G1 门禁在 CI 中变成永久空转，且全仓无第二个执行点

- **文件:行**：`.gitea/workflows/ci.yml:158-176`（步骤本体），`:167-170`（注释声明）
- **问题**：注释写明
  > "The SF=1 layer stays enforced by `scripts/gate/check_stage.sh` at release promotion and by the manual `z6g4-tpch-sf1-cell-diff.yml` workflow; issue #5387 tracks wiring it into CI."

  实测：`scripts/gate/check_stage.sh` **不含任何 TPC-H 引用**。全仓 `check_g1_tpch_baseline.sh` 的唯一调用者是 `.gitea/workflows/ci.yml` —— 而本变更把该步骤改成"没有 `/var/tmp/tpch-sf1/lineitem.tbl` 就打印 NOT RUN 并 `exit 0`"。CI runner 上永远没有该文件，因此**该门禁在 CI 中是永久 no-op**。
  `z6g4-tpch-sf1-cell-diff.yml` 也不调用 `check_g1_tpch_baseline.sh`（它不引用 `check_g1`、`tpch_hash`、`hashes_v380`）。
  另外，`openspec/changes/g1-tpch-baseline/spec.md:67` 明文要求：
  > "The repository's CI workflow MUST run `bash scripts/gate/check_g1_tpch_baseline.sh` as a step on every PR that modifies any of: `crates/**`, `tests/**`, `Cargo.toml`, `Cargo.lock`."

  本变更使该步骤成为空转，却未修订该 spec。
- **复现命令**：
  ```bash
  cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-5379-tpch
  grep -in "tpch" scripts/gate/check_stage.sh ; echo "exit=$?"     # 无输出
  grep -rn "check_g1_tpch_baseline" --include="*.sh" --include="*.yml" . | grep -v "^./docs/"
  # 实际输出：仅 ./.gitea/workflows/ci.yml:174 和 :175
  ```
- **实测输出**：
  ```
  $ grep -in "tpch" scripts/gate/check_stage.sh ; echo "exit=$?"
  exit=1                       ← 0 命中
  $ grep -rn "check_g1_tpch_baseline" --include="*.sh" --include="*.yml" . | grep -v "^./docs/"
  ./.gitea/workflows/ci.yml:174:          chmod +x scripts/gate/check_g1_tpch_baseline.sh
  ./.gitea/workflows/ci.yml:175:          bash scripts/gate/check_g1_tpch_baseline.sh 2>&1 | tee g1_report.log
  ```
- **影响**：一个此前（虽然红）但**真实存在**的门禁被换成了一个永远绿的空转步骤；同时 openspec 规范与实现产生偏差。这与本变更"让门禁会说话"的目标背道而驰——它是"会说话"的一种，但说的是"NOT RUN"而不是"门禁被执行了"。
- **建议改法**：
  1. 若接受"CI 不跑 SF=1"，**必须**修订 `openspec/changes/g1-tpch-baseline/spec.md:67` 并在 PR 中显式记录该偏差；
  2. 修正 `ci.yml:167-170` 的注释（`check_stage.sh` 不管 TPC-H）；
  3. 把 SF=1 门禁接到一个真实有 fixture 的执行点（#5387），或在 `check_stage.sh` 中显式接入并给出可验证的接线。

---

### 重要（建议合并前解决）

---

#### I-1 `FixtureState::resolve()` 只检查 `is_dir()`：现存但空的 fixture 目录导致 **panic** 而非 NOT-RUN；且与 wire harness 行为分叉

- **文件:行**：`tests/common/tpch_fixture.rs:303-317`（`:307` 是 `if !path.is_dir()`）；对照 `tests/common/tpch_wire_harness.rs:35-50`（`:39` 检查 `lineitem.tbl`）
- **问题**：三态模型里 "Missing" 只覆盖"目录根本不存在"。若 `TPCH_DATA_DIR` 指向一个**存在但空/残缺**的目录：
  - `tpch_fixture` 路径 → `Present` → `load_into_storage` 报错 → **panic**（硬失败，非披露）；
  - `tpch_wire_harness` 路径 → `None` → NOT-RUN 披露（软失败）。
  **同一份输入、同一套共享模块，两种结局。**
- **复现命令**：
  ```bash
  cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-5379-tpch
  mkdir -p /tmp/review_empty_fx
  TPCH_DATA_DIR=/tmp/review_empty_fx cargo test --test diag_shipdate_type -- --nocapture
  TPCH_DATA_DIR=/tmp/review_empty_fx cargo test --test eval_22_vs_sf01 -- --nocapture
  ```
- **实测输出**：
  ```
  # diag_shipdate_type（tpch_fixture 路径）
  thread 'test_shipdate_actual_value' panicked at tests/integration/oracle/diag_shipdate_type.rs:155:29:
  test test_shipdate_actual_value ... FAILED
  test result: FAILED. 12 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

  # eval_22_vs_sf01（tpch_wire_harness 路径，同一 env）
  TPCH-NOT-RUN: eval_22_vs_sqlite_sf01_canonical reason=fixture_missing detail=no TPC-H fixture found (set TPCH_DATA_DIR to a directory with the 8 .tbl files)
  test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  ```
- **影响**：CI 上 `TPCH_DATA_DIR` 由 `prepare_tpch_fixture.sh` 生成，理论上不会残缺；但一旦生成步骤部分失败、或 `TPCH_DATA_DIR` 被外部覆盖、部分目标会**硬红**而另一些**静绿**，正是本变更要消除的"不一致"。
- **建议改法**：让 `FixtureState::resolve()` 与 `resolve_fixture_dir()` 采用同一条判据（要求 8 个 `.tbl` 齐备，或至少 `lineitem.tbl`）；更彻底的做法是**只保留一个解析器**（`tpch_wire_harness` 委托给 `tpch_fixture::FixtureState`），彻底消除分叉（同时解决 **S-4**）。

---

#### I-2 三态模型中的 `ProfileMismatch` 实际不可达，`require_profile()` 无任何调用方

- **文件:行**：`tests/common/tpch_fixture.rs:288-296`（`ProfileMismatch` 变体）、`:365-392`（`require_profile`）
- **问题**：`FixtureState::resolve()` **只会**返回 `Present` 或 `Missing`，从不构造 `ProfileMismatch`；`require_profile()` 全仓**零调用方**（`grep -rn "require_profile" tests/` 只有定义行）。两者被 `#![allow(dead_code)]` 掩盖，没有编译告警。
  实际生效的 profile 披露只在 `tests/integration/tpch/oracle_g1_tpch_sha256.rs:102-140` 里**单独实现**了一份。
- **复现命令**：
  ```bash
  grep -rn "require_profile" tests/ --include="*.rs"
  grep -rn "FixtureState::ProfileMismatch" tests/ --include="*.rs"
  ```
- **实测输出**：
  ```
  tests/common/tpch_fixture.rs:357:    pub fn require_profile(&self, target: &str, expected_sf: f64) -> Option<&Fixture> {
  ```
  （无其它命中）
- **正向证据**（披露本身在 `oracle_g1_tpch_sha256` 上确实工作）：
  ```
  $ TPCH_DATA_DIR=/tmp/review_fx2 cargo test --test oracle_g1_tpch_sha256 -- --nocapture
  TPCH-NOT-RUN: oracle_g1_tpch_sha256 reason=profile_mismatch detail=baseline was calibrated at scale factor 0.001 but the fixture on disk is sf0.01; the stored hashes belong to a different dataset and cannot be compared against it
  test result: ok. 17 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.02s
  ```
- **影响**：三态中的"profile 不匹配"在**共享模块**层面是死代码；读者若信任 `tpch_fixture.rs` 的文档会以为 profile 保护是统一的，实际只有 1 个文件有。
- **建议改法**：要么把 `require_profile` 真正接进按 SF 标定的测试（`oracle_g15_tpch_sf01`、`tpch_sf01_*`），要么删掉 `ProfileMismatch` 分支与 `require_profile`，并把"profile 披露由各测试自行实现"写进模块文档。

---

#### I-3 `check_tpch_known_differences.sh` 与 `mutate_tpch_oracle_tests.sh` 未接入任何 CI workflow

- **文件:行**：`.gitea/workflows/ci.yml:85`（`prepare_tpch_fixture.sh`）、`:155`（`check_test_target_execution.sh`）—— **仅此两处**
- **问题**：
  ```
  $ grep -rn "check_tpch_known_differences\|mutate_tpch_oracle_tests" .gitea/workflows/
  （0 命中）
  ```
  "已知差异不得无声增长"与"新断言确实能杀缺陷"两条纪律**只靠人工执行**。
  另有一个自相矛盾：`check_tpch_known_differences.sh` 的 `--max-entries` **默认 0**，即登记表必须永远为空。而 `sqlite_oracle.rs:459-462` 的设计明确允许把真实差异登记进去从而"放行"该用例 —— 一旦有人按设计登记一条，CI（若接入）会直接红，`--max-entries` 需要一个经评审的非零值。
- **复现命令**：
  ```bash
  cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-5379-tpch
  grep -rn "check_tpch_known_differences\|mutate_tpch_oracle_tests" .gitea/workflows/ ; echo "exit=$?"
  ```
- **实测输出**：
  ```
  exit=1        ← 0 命中
  ```
- **影响**：`PR-5379_TEST_PLAN.md` 覆盖矩阵里"已知差异登记合规"和"断言真的能杀缺陷"两行标称有门禁脚本，但门禁不在 CI 路径上。
- **建议改法**：把 `check_tpch_known_differences.sh --max-entries <N>` 接入 CI（建议 `gate.yml`），把 `mutate_tpch_oracle_tests.sh` 接入一个**手动/夜间** job（它需要 3 次重编译，代价高）；同时确定 `--max-entries` 的正式取值与审批流程。

---

#### I-4 CI 摘要只披露 `not_run`、不告警：回归会让大量测试"变绿"

- **文件:行**：`.gitea/workflows/ci.yml:118-146`
- **问题**：`PASSED=$((TEST_OK - NOT_RUN))`（`:138`）算术正确，但 `NOT_RUN` **没有阈值、没有失败条件**。只要 `TEST_FAIL == 0` 且每个 `[[test]]` 目标都"跑过"，job 就绿。
  而 `check_test_target_execution.sh` **无法**发现"目标跑了但所有测试都提前 return"——那种目标在 cargo 日志里照样有 `Running <path>` 行。因此一次让 300 个测试变成 not-run 的回归，CI 会显示 `passed=100 not_run=300` 并**保持绿色**。
- **复现命令 / 推理依据**：读 `ci.yml:131-146` —— `NOT_RUN` 只被 `echo` 出来，`PASSED` 只被 `echo`，二者都不参与 `exit` 判定；`exit 1` 只在 `TEST_FAIL > 0`（`:143`）或 `BIN_OK + BIN_FAIL == 0`（`:144`）时触发。
- **影响**：本变更把"披露"做对了，但**没有把披露变成信号**。三态披露若不设阈值，实际效果与"把 SKIP 记成 PASS"只在读者不看日志时有差别。
- **建议改法**：
  1. 加一条硬阈值，例如 `if [ "$NOT_RUN" -gt "$MAX_NOT_RUN" ]; then exit 1; fi`（`MAX_NOT_RUN` 随声明目标数联动）；
  2. 更强：扩展 `check_test_target_execution.sh`，要求每个声明目标至少产生一条 `test … ok|FAILED` 行，否则判"目标空跑"。

---

#### I-5 `eval_22_vs_sf01` 依赖 CI runner 上存在 `sqlite3` CLI 二进制（本变更让它第一次在 CI 真正执行）

- **文件:行**：`tests/integration/sql/eval_22_vs_sf01.rs:258`（`.expect("sqlite3 CLI must be available")`）、`:293`（`Command::new("sqlite3")`）
- **问题**：该外部依赖**预先存在**（`git show HEAD:…` 确认 HEAD 版本已用 `Command::new("sqlite3")`），不是本变更引入。但本变更让 fixture 在 CI 可解析 → 该测试**第一次会在 CI 真正执行** → 若 hermes runner 镜像没有 `sqlite3` 二进制，测试会以"sqlite3 CLI must be available" panic。
  讽刺的是，本变更**新增**的 `tests/common/sqlite_oracle.rs` 正是为了消除这类外部依赖（用 `rusqlite` bundled，`Cargo.toml:252`），但 `eval_22_vs_sf01` 没有迁移过去。
- **复现命令**：
  ```bash
  cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-5379-tpch
  git show HEAD:tests/integration/sql/eval_22_vs_sf01.rs | grep -n "Command::new"   # 预先存在
  command -v sqlite3 && sqlite3 --version                                        # 本机可用
  ```
- **实测输出**：
  ```
  $ git show HEAD:tests/integration/sql/eval_22_vs_sf01.rs | grep -n "Command::new"
  206:    let status = Command::new("sqlite3")
  217:    let out = Command::new("sqlite3")
  $ command -v sqlite3 && sqlite3 --version
  /usr/bin/sqlite3
  3.54.0 2026-04-09 12:25:13 …
  ```
- **未验证**：CI（hermes runner）镜像是否预装 `sqlite3` —— 审核者无法从本工作树访问 runner 镜像，**未验证**。若缺，CI 将新增一个 panic。
- **建议改法**：把 `eval_22_vs_sf01` 迁到进程内 `SqliteOracle`（同一 fixture、同一份 `.tbl`，不再需要 `sqlite3` 二进制，也不再需要 `/tmp/tpch_22_sf01_audit` 硬编码目录 —— 同时解决 **2.7.2**）；或至少在 CI 的 fixture 准备步骤加 `command -v sqlite3` 前置检查。

---

#### I-6 生成器自检的分母用理论行数，与 manifest 的实测行数不一致

- **文件:行**：`scripts/gate/generate_tpch_sf.py:416`（`total = counts[table]`）
- **问题**：`counts` 是生成器的**理论**行数（lineitem 60000），而实际生成 59 903 行（`random.randint(1, 7)` 的期望值是 4，但实际略低）。自检打印 `matched/total` 时用的是理论分母：
  ```
  OK   lineitem.l_returnflag = 'R': 19981/60000 (33.30%, need >= 10.00%)
  ```
  而 manifest 里记录的是 `wc -l` 实测值 `"lineitem": 59903`。
- **复现命令**：
  ```bash
  wc -l < /tmp/review_fx2/lineitem.tbl ; grep lineitem /tmp/review_fx2/tpch_fixture_manifest.json
  ```
- **实测输出**：
  ```
  59903
      "lineitem": 59903
  ```
- **影响**：低。阈值余量都很宽（最小 0.80% vs 0.10%），不影响拦截能力。但报告出来的占比与文件实际行数对不上，会误导后续按占比调阈值的人。
- **建议改法**：自检改用 `wc -l` 实测行数（或让生成器直接返回实际计数）。

---

### 建议（非阻断）

| # | 文件:行 | 问题 | 建议 |
|---|---|---|---|
| **S-1** | `tests/common/sqlite_oracle.rs:454-457` | `check_against_oracle` 对两侧 `sort()` 后比较 → **不验证行序**。但有调用方传的是带 `ORDER BY` 的查询（如 `diag_shipdate_type.rs:244` `ORDER BY l_orderkey, l_linenumber LIMIT 5`），容易被误读为"顺序已校验"。doc 注释已诚实说明，但调用点未说明 | 增加一个 `order_sensitive` 变体；对带 `ORDER BY` 的 case 显式使用 |
| **S-2** | `tests/integration/sql/eval_22_vs_sf01.rs:292` | 只比 `COUNT(*)`，不比行内容 → "MATCHED" 强度低于字面 | 改用 cell 级对照（`tpch_sf01_22_vs_3engines.rs:280-305` 已有实现） |
| **S-3** | `tests/common/tpch_wire_harness.rs:10-11` | `SF001_DIR` / `SF01_DIR` 仍指向**已删除**的符号链接；`SF01_DIR` 已无使用者，`SF001_DIR` 仍被 `tpch_value_test_v2.rs:58,143` 使用 | 修完 **B-2** 后一并删除 |
| **S-4** | `tpch_fixture.rs:77,400` / `tpch_wire_harness.rs:18,71` | `NOT_RUN_MARKER` / `report_not_run` / 候选目录列表**各存一份**（后者注释自称"must stay in sync"）。CI 计数前缀有两处定义 → 漂移风险 | 让 wire harness `use` 共享模块的常量与函数（同时解决 **I-1**） |
| **S-5** | `tpch_fixture.rs:411` / `tpch_wire_harness.rs:82` | `writeln!(f, "{line}")` 会分成 2 次 `write` 系统调用（字符串 + 换行），在 `O_APPEND` 下理论上可能被并发测试二进制交错 | 预先拼好带 `\n` 的整行，用单次 `write_all` 写入 |
| **S-6** | `tests/common/sqlite_oracle.rs:377-387` | `adapt_sql` 的"不支持"判定是对整条 SQL 的**裸子串扫描**：`"::"` / `"TOP "` 也会命中字符串字面量或标识符 → 可能**误报**为 unsupported，使该查询悄悄退化为"只跑不变量" | 用轻量分词或至少跳过引号内内容 |
| **S-7** | `tests/integration/oracle/diag_q7_sf1_real_count.rs`、`diag_q7_subset_columns.rs` | 本次只改了 fixture 解析，**断言数仍为 0**；后者未被 `#[ignore]`，会在 CI 里"空跑通过"（`diag_q7_subset_columns.rs:102` 只 `eprintln!`）。简报说 #5391 跟踪"oracle 目录剩余零断言文件"—— 审核快照下 **仍未闭环** | 纳入 #5399 之前的 #5391 范围；或本次就补上最小断言 |
| **S-8** | `tests/integration/tpch/tpch_sf01_22_vs_sqlite.rs:41-73`、`tpch_sf01_22_vs_sqlite_test.rs` | 22 个 baseline 缺失 → 每个 query `SKIP (no baseline)` → `pass=0 fail=0 skip=22` → 测试级判定仍为绿。比 **B-1** 好（逐 query 打印了 SKIP），但目标级结论仍是"绿且零覆盖" | 同 **B-1** 的处置 |
| **S-9** | 全局 | 审核期间工作树被持续修改（见 §1.3） | 合并前对**冻结快照**重跑本审核；建议评审流程要求作者在评审期间冻结分支 |
| **S-10** | `.gitea/workflows/ci.yml:168` vs `:170` | 同一 3 行块内 issue 号自相矛盾：注释说 "issue #5387 tracks wiring it into CI"，紧邻的 `echo` 说 "tracked by issue #5385"。而 `evidence/TPCH_FIXTURE_5379_2026-10-20.md` 只引用 #5387/#5388/#5389/#5390，**不含 #5385，也不含简报提到的 #5391** | 统一为一个 issue 号；把 #5391 补进 evidence 文档的跟踪列表 |
| **S-11** | `scripts/gate/check_test_target_execution.sh:236` | `RUNNING_RE = re.compile(r'Running\s+(.*?)\s+\(target/')` 硬编码 `target/`。若 CI 设置了 `CARGO_TARGET_DIR`，cargo 打印的是 `(<custom>/debug/deps/…)`，正则会**全部失配** → 446 个目标全被判漏跑（误报 FAIL） | 把 `(target/` 放宽为 `\([^)]*/debug/deps/` 或改为捕获最后一个括号内的路径后取 `deps/` 之前的部分 |
| **S-12** | `scripts/gate/check_test_target_execution.sh:264-266` | `declared_by_path.setdefault(norm(path), …)` —— 若两个 `[[test]]` 指向同一 path，只有第一个进入比对集合，第二个**永远不会被判漏跑**。当前 Cargo.toml 无重复（审核者核对：446 声明 / 446 个不同 path），但门禁本身有这个盲点 | 按 `name` 而非 `path` 建索引，或对重复 path 显式告警 |

---

## 4. 独立审核结论

### 4.1 结论

**REQUEST_CHANGES**（不建议合并，需先解决 §3 的 4 项阻断）。

### 4.2 做对了什么（应予肯定）

1. **独立预言机的方向完全正确**：SQLite 进程内预言机 + 自建 `.tbl` 解析（不共用引擎加载器）+ 自建 schema，13 个文件 35 处对照，**未发现任何自证（circularity）**。
2. **防空转意识贯穿始终**：13 个 oracle 文件无一例外地加了"结果集非空 / 过滤真的移除了行 / 比较分支确实划分全集"这类不变量，措辞具体（"the German supplier filter must therefore match real data"）。
3. **目标执行核对门禁是本次最有价值的交付**：`check_test_target_execution.sh` 的 declared/executed 解析经 ANSI 色码、截断日志、446 目标全量三种场景实测，行为正确，**382 这个数字与 CI run 7613 的真实事故完全吻合**。
4. **CI 三态摘要的算术是对的**：`PASSED = TEST_OK - NOT_RUN` 真的扣除了 not-run，且用 `TPCH_NOT_RUN_FILE` 侧信道绕开了 libtest 吞 stdout 的陷阱 —— 这是个容易做错而此处做对了的细节。
5. **生成器改造的修复是真的**：nation→region 映射（`(i // 5) + 1`）、`Brand#23` / `%green%` / `%special%requests%` / `%Customer%Complaints%` / `c_phone` 国码、`o_custkey` 随机化（打破"每客户订单数完全相同"）、lineitem 数量改为 1..7 抽签（让 `SUM(l_quantity) > 300` 可达）—— 每一条都对应一个具体的"查询退化为空"缺陷，且实测都已生效。
6. **`orders` schema 补 `o_totalprice` 的根因分析是准确的**（`tpch_wire_harness.rs:88-105`），并给出了为什么这个 bug 长期"静默"的机制解释。
7. **审计者的边界自律**：81 个 evidence 副作用文件被正确识别并排除。

### 4.3 必须先解决项（合并前）

| # | 阻断项 | 一句话验收标准 |
|---|---|---|
| **B-1** | `oracle_g15_tpch_sf01` 静默通过 | 22 个 query 要么**真的比对了 22 个**（断言 `compared == 22`），要么走 `report_not_run` 披露；不允许 `ok` 且零比对 |
| **B-2** | `tpch_value_test_v2` 硬失败 | `TPCH_DATA_DIR=<有效fixture> cargo test --test tpch_value_test_v2` 必须 **PASS 或走 NOT-RUN 披露**，不得 panic |
| **B-3** | `eval_22_vs_sf01` 的空集/近空集对比 | 新增第 5 类生成时自检（22 查询的聚合前行数下限）；Q22 在 SF=0.01 上必须有**非空**结果（需让 `o_custkey` 分配留下确定性的无订单客户）；`eval_22_vs_sf01` 需把"两侧同时为空"标为 `EMPTY` 而非 `MATCHED` |
| **B-4** | G1 门禁在 CI 永久空转 + 注释失实 | 修正 `ci.yml:167-170` 注释（`check_stage.sh` 不含 TPC-H）；修订 `g1-tpch-baseline` spec 或把 SF=1 门禁接到真实有 fixture 的执行点 |

### 4.4 强烈建议同批解决（重要项）

- **I-1** 统一 fixture 解析器（消除 panic 与分叉）
- **I-3** 把 `check_tpch_known_differences.sh` 接入 CI，并确定 `--max-entries` 的正式取值
- **I-4** 给 `not_run` 设阈值，否则"三态披露"没有牙齿
- **I-2** 接线或删除 `require_profile` / `ProfileMismatch` 死代码
- **I-5** 把 `eval_22_vs_sf01` 迁到进程内 `SqliteOracle`（同时消除 `sqlite3` CLI 依赖与硬编码 `/tmp` 目录）

### 4.5 审核未覆盖 / 无法验证的部分（明确列出）

| 项 | 状态 | 原因 |
|---|---|---|
| `cargo clippy -- -D warnings` | **未验证** | 全量构建成本过高，审核者未运行 |
| `mutate_tpch_oracle_tests.sh` 全流程（3 个变异必须全部 CAUGHT） | **未验证** | 脚本会临时改写仓库源文件 `src/engine_select.rs`，超出审核者只读边界。仅做了副本级变异点验证（§2.8 专项 C） |
| CI runner（hermes）镜像是否预装 `sqlite3` CLI | **未验证** | 审核者无法访问 runner 镜像（见 **I-5**） |
| 同一测试连跑 10 次一致性（2.2.2） | **未验证** | 仅验证了生成器幂等性，未做 10 次连跑 |
| CI 端到端行为（`GITHUB_ENV` 传递、`--no-fail-fast` + `timeout` 在 Gitea runner 上的实际行为） | **未验证** | 审核者在本地以合成日志验证了摘要算术与门禁脚本，未在真实 CI 上验证 |
| 45 个未改动的 TPC-H 目标在 CI 上的终态 | **部分未验证** | 已确认 `tpch_22_mysql_cli_wire_test.rs:215`、`load_local_infile_eagain_regression_test.rs:81,125`、`repro_3282_orderby_desc_test.rs:117` 仍硬编码已删除的 `tests/data/tpch-sf001` 路径，**本变更未迁移它们**。这批测试在 CI 仍会 panic/失败 —— 与 **B-2** 同源，建议一并纳入本变更范围 |
| SF=1 规模层 | **DEFERRED** | 无 SF=1 fixture，跟踪 #5387 |

---

**审核者**：Verifier（独立测试审核者）
**审核日期**：2026-10-20
**审核快照指纹**：见 §1.3（合并前须对冻结快照重跑）
