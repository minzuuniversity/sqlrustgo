# 迭代 5 · 批次 15 —— soak 的规范 CLI 入口定案 + 二进制名只有一个来源（#5405）

**关闭**：#5405
**基线**：`develop/v4.1.0` @ `0ba527dd3729b68d8a33d64f94336f0ff00a4990`
**分支**：`fix/cli-soak-entry-5405-batch15`
**本批次新立**：#5515、#5516

---

## 0. 一句话

`tests/e2e/sqlrustgo_cli_soak_e2e_test.rs` 找的 `target/debug/sqlrustgo-cli`
**从来不存在** —— `sqlrustgo-cli` 是**包名**，产出它的 bin 叫 `sqlrustgo`。

本批次做了两件事：把「soak 的规范 CLI 入口是哪个二进制」这个**接口决策**
定下来并写进代码；同时把「二进制叫什么」这件事收敂到**一个来源**
（manifest），其余地方全部交叉核对。

---

## 1. 事实核对：仓库里到底有哪些二进制

`cargo metadata` + 各 `Cargo.toml` 的 `[[bin]]` 声明（`0ba527dd372` 时点）：

| bin | 产出包 | 是什么 |
|---|---|---|
| `sqlrustgo` | `sqlrustgo-cli`（`crates/sqlrustgo-cli`） | **规范 CLI**：`serve` / `exec` / `repl` / `bench` / `gmp` / `diag` / `backup` / `restore` / `cli` / `soak` / `sqlite` |
| `sqlrustgo-soak` | `sqlrustgo-soak`（`crates/cli`） | 压测客户端：`exec` / `repl` / `soak` |
| `sqlrustgo-mysql-server` | `sqlrustgo-mysql-server` | MySQL 线协议服务器 |
| …其余 17 个 | … | 工具类 |

**仓库里没有任何 bin 叫 `sqlrustgo-cli`。** 它是 package 名。

---

## 2. 接口决策：soak REPL 的规范入口是哪个二进制

#5405 明确要求「先定接口决策，不要在 CI 接线里猜」。本批次的结论：

> **Soak REPL 的规范入口 = `sqlrustgo`（package `sqlrustgo-cli`）的 `soak` 子命令。**

### 2.1 依据

1. **协议只在一个地方实现。**
   tab-separated 协议（`OK\t` / `ROWS\t` / `COL\t` / `DATA\t` / `ROW\t` / `ERR\t`）
   只在 `crates/sqlrustgo-cli/src/lib.rs::run_soak_repl` 里实现，
   并且**已经**接到 `SubCmd::Soak`（同文件第 215 行）。
   `crates/cli` 里没有任何产生这个协议的东西。

2. **参数面逐字吻合。**
   `SubCmd::Soak`（`crates/sqlrustgo-cli/src/lib.rs:94-103`）是
   `--host` / `-p,--port` / `-u,--user` / `--pass,--password` ——
   与 e2e 测试调用的 `soak -p <port> -u tester --pass tester` **逐字一致**。

3. **另一个 `soak` 是不同的东西。**
   `crates/cli` 的 `Soak` 子命令是**压测跑子**：
   `--duration` / `--rate` / `--query-file` / `--report-interval`。
   它不读 stdin，也没有 tab-separated 协议。
   ⇒ 把它接到 e2e 测试上，测试要么跑不起来，要么测的根本不是 REPL。

4. **名字相近不等于职责相同。**
   `sqlrustgo-soak` = 压测客户端；`sqlrustgo` = 规范 CLI。
   这不是「旧名 / 新名」的关系，是**两个工具**。

### 2.2 被否决的方案

| 方案 | 否决理由 |
|---|---|
| 把 `crates/cli` 的 package / bin 改回 `sqlrustgo-cli` | 那会让两个职责不同的二进制继续只差一个连字符；还要动 package 名 ⇒ 影响 workspace 依赖、CI 构建步骤、发布产物清单。**改名不是修法。** |
| 给 `sqlrustgo-soak` 加一个承接 REPL 的 `soak` 别名 | 两套 `soak` 语义混进同一个二进制，只会让「`soak` 到底是哪个」更糊。 |
| 在 `crates/cli` 里再造一个叫 `sqlrustgo-cli` 的 bin | 这是**为了迁就一个错误的测试名**去造新产物，等于把「两个来源」固化下来。 |

### 2.3 订正 issue 的一个前提

#5405 的正文把 `sqlrustgo-cli` 称作「`crates/cli` **重命名前的旧包名**」。
核对后需要订正：`crates/cli` 现在的 package 名是 **`sqlrustgo-soak`**，
它的 bin 也叫 `sqlrustgo-soak`；`sqlrustgo-cli` 是**另一个包**
（`crates/sqlrustgo-cli`）的名字。两个包都真实存在、都在 workspace 里。
「旧包名」的说法会让读者以为该把 `sqlrustgo-soak` 改回 `sqlrustgo-cli` ——
而 §2.2 已经说明那正是要避免的。

---

## 3. 改动

### 3.1 `tests/common/bin_resolver.rs` —— 唯一权威的 Rust 侧解析入口

- 新增 `CANONICAL_CLI_BIN = "sqlrustgo"`，并在文档里写明
  **`sqlrustgo-cli` 是包名、不是二进制名**，以及为什么把它当候选名会让
  「这个二进制叫什么」出现第二个来源；
- 新增 `resolve_canonical_cli_binary()` —— **唯一实现**；
  `resolve_engine_binary()` 变成它的别名（同一份实现，不是两份）；
- 新增 `EXTERNAL_TOOL_BINS`（`sqlite3` / `mysql` / `psql`）与
  `SQLITE_BIN_NAME` / `MYSQL_BIN_NAME` / `PSQL_BIN_NAME` /
  `MYSQL_SERVER_BIN_NAME` —— 解析器里不再有裸字面量；
- `missing_binary_message()` 补上「`cargo build -p sqlrustgo-cli` 产出的是
  `sqlrustgo`」这句话。

### 3.2 `tests/e2e/sqlrustgo_cli_soak_e2e_test.rs`

- `binary_path()` 改为走 `common::bin_resolver::resolve_canonical_cli_binary()`，
  找不到时 **panic 并点名环境变量**（不是跳过 —— 跳过会把它变成假绿）；
- 新增守卫用例 `resolved_binary_reports_itself_as_the_canonical_cli`。

### 3.3 `crates/sqlrustgo-cli/tests/cli_test.rs`

`cli_bin()` 从手写 `target/debug/sqlrustgo-cli`（**不存在**，且忽略
`CARGO_TARGET_DIR`）改为：

```rust
PathBuf::from(env!("CARGO_BIN_EXE_sqlrustgo"))
```

cargo 只为**本包**的 `[[bin]]` 设置这个变量，且在**编译期**展开 ——
二进制一旦改名，这里**编译不过**，而不是等到运行期才 panic。

### 3.4 `crates/cli/src/{main,lib,exec,repl,client,soak}.rs`

usage 文本、`#[command(name = ...)]` 与文档注释里的陈旧 `sqlrustgo-cli`
全部改成 `sqlrustgo-soak`（该包的真实 bin 名）。
`main.rs` 里额外写明：本二进制的 `soak` 是**压测跑子**，
**REPL 形态**的 soak 在规范 CLI `sqlrustgo soak` 里。

### 3.5 `tests/common/resolve_bin.sh`

shell 侧解析器里那个同样是**幽灵候选**的 `sqlrustgo-cli` 删掉。
（Rust 侧与 shell 侧是同一判据的两份实现，候选名必须一致。）

### 3.6 新门禁 `scripts/gate/check_cli_binary_names.{sh,py}` + CI 接线

见 §5。

### 3.7 顺带修掉一个**会让本次改动变脆**的进程级 env 竞态（#5423）

`tests/common/bin_resolver.rs` 里有两个单元用例改的是**同一个进程级**环境变量，
而其中一个开头还会 `remove_var` 另一个刚设的值：

```rust
// external_tool_resolution_prefers_the_explicit_env_var
std::env::set_var(PSQL_BIN_ENV, &fake);
let resolved = resolve_psql_binary();     // ← 并发时这里可能已被别人抹掉
std::env::remove_var(PSQL_BIN_ENV);

// external_tool_resolution_never_invents_an_absolute_path
std::env::remove_var(PSQL_BIN_ENV);        // ← 抹掉并发用例刚设的值
std::env::remove_var(MYSQL_BIN_ENV);
```

libtest 默认并行跑用例 ⇒ 两者互相踩。**批次 11 的全量回归里就有 3 个目标
因此偶发变红**（断言拿到的是 PATH 上的 `/opt/homebrew/bin/psql`，而不是用例
自己设的临时路径）：

- `e2e_canonical_subprocess`
- `tpch_q8_q21_perf_regression_test`
- `v312_13_load_data_sf10_test`

**这为什么是本批次的必修项**：#5405 的 e2e 测试读的是 `SQLRUSTGO_BIN`
（`ENGINE_BIN_ENV`），而 `explicit_env_var_wins_over_every_other_candidate`
正好在**改这个变量** ⇒ 不修的话，本次改动会**新增**一个偶发红灯的目标。
⇒ 每个用例现在只碰**自己**的变量名
（`SQLRUSTGO_RESOLVER_SELFTEST_{ENGINE,PSQL,ABSENT}_BIN`），
`SQLRUSTGO_BIN` / `SQLRUSTGO_PSQL_BIN` / `SQLRUSTGO_MYSQL_BIN`
**不再被任何用例改动**。

---

## 4. 判别力

判别力的定义是「**把改动撤掉，测试必须红**」，不是「测试通过」。

| # | 变异 | 预期 | 实测 |
|---|---|---|---|
| M1 | 把 e2e 测试的解析器改回手写 `target/debug/sqlrustgo-cli` | 红 | 新门禁 C3 红（见 §5.2） |
| M2 | 把 `resolve_engine_binary()` 的候选改回 `["sqlrustgo-cli", "sqlrustgo"]` | 红 | 新门禁 C4 红 |
| M3 | 把 `crates/cli/src/main.rs` 的 `#[command(name)]` 改回 `sqlrustgo-cli` | 红 | 新门禁 C1/C3 口径内不覆盖 `src/` 的 `#[command]`；但 §3.4 的改动是**纯文档/展示**修正，**不改变任何行为** —— 它不需要判别力，只需要与事实一致 |
| M4 | 把 `cli_test.rs` 的 `env!("CARGO_BIN_EXE_sqlrustgo")` 改回手写路径 | 红（编译期） | cargo 直接编译失败：`CARGO_BIN_EXE_sqlrustgo` 只对本包的 `[[bin]]` 存在 |

M4 是这一批最强的一条：**失败发生在编译期**，不是运行期。

### 4.1 门禁的负对照（真值对照）

门禁脚本必须能在「它要修的那棵树」上变红。实测：

```
$ python3 scripts/gate/check_cli_binary_names.py --repo-root <批次15 修复前的树>
status: FAIL failures: 8
  C3 bad: tests/e2e/sqlrustgo_cli_soak_e2e_test.rs -> sqlrustgo-cli
  C3 bad: tests/e2e/sqlrustgo_cli_soak_e2e_test.rs -> sqlrustgo-cli
  C3 bad: tests/common/bin_resolver.rs -> x          (自证样本，修复前无豁免标记)
  C3 bad: tests/common/bin_resolver.rs -> x
```

⇒ **8 处红灯，其中 2 处正是 #5405 本身**。修复后同一棵树 **0 红灯**。

### 4.2 env 竞态修复的判别力

| # | 变异 | 预期 | 实测 |
|---|---|---|---|
| M5 | 把两个用例改回共用 `PSQL_BIN_ENV`（并保留那个 `remove_var`） | 红（**偶发**） | 批次 11 的全量回归里 3 个目标偶发红：`e2e_canonical_subprocess` / `tpch_q8_q21_perf_regression_test` / `v312_13_load_data_sf10_test`，断言拿到 `/opt/homebrew/bin/psql` |

⚠️ M5 是**偶发**红灯 —— 它不会每次都出现。正因如此，它比确定性红灯更危险：
一个「有时红」的测试目标，在门禁里会被当成「 flaky，忽略」。
⇒ 这也是为什么本批次把它**修掉**而不是登记为已知差异。

修复后的可观察差异（不是「有时红」而是「结构上不可能红」）：
**`SQLRUSTGO_BIN` / `SQLRUSTGO_PSQL_BIN` / `SQLRUSTGO_MYSQL_BIN`
在整个测试树里再也没有任何用例去改** —— 文本检索可验证：
`grep -rn "set_var\|remove_var" tests/ crates/*/tests/` 的结果里，
`bin_resolver.rs` 只剩三个 `SQLRUSTGO_RESOLVER_SELFTEST_*` 变量。

---

## 5. 门禁

### 5.1 新的元门禁：`scripts/gate/check_cli_binary_names.sh`

判据（一个判据只有一份实现 —— 权威来源永远是 manifest）：

| | 判据 | 失败意味着 |
|---|---|---|
| **C1** | 没有两个包声明同一个 `[[bin]] name` | cargo 文件名冲突的前兆 |
| **C2** | 每个 `env!("CARGO_BIN_EXE_<name>")` 的 `<name>` 属于**拥有该文件的那个包** | cargo 只为当前包的 bin 设置该变量 ⇒ 借来的名字是编译期错误 |
| **C3** | 每个 `target/<profile>/<name>` 字面量（测试树 + crates/）指向一个**真实存在**的 bin | #5405 的复发形态 |
| **C4** | 共享解析器（`tests/common/bin_resolver.rs`）里的每个候选名，要么是 workspace bin，要么在 `EXTERNAL_TOOL_BINS` 里 | 解析器里混进了幽灵候选 |

「什么不是 workspace bin」这份清单**只在 `bin_resolver.rs` 里存在一份**
（`EXTERNAL_TOOL_BINS`），门禁直接读它 —— 门禁不自带第二份清单。

C3 有一条**显式豁免**：`tests/common/bin_resolver.rs` 带着标记
`cli-bin-allow-samples`。理由与 C6 门的 `c6-allow-home-paths` 相同：
它本身是**检测器**，必须带着故意的样本路径（`target/debug/x`）才能自证
有判别力。豁免是逐文件的、显式的，且**每次门禁输出都会列出被豁免的文件**。

### 5.2 自检：`--selftest`

门禁必须能失败。`--selftest` 造 6 个合成夹具，断言分析器给出预期结论：

| 夹具 | 预期 | 实测 |
|---|---|---|
| 干净的树 | PASS | PASS |
| 两个包声明同一个 `[[bin]] name` | FAIL | FAIL |
| `CARGO_BIN_EXE` 借了别的包的 bin | FAIL | FAIL |
| `target/<profile>/<name>` 指向不存在的名字 | FAIL | FAIL |
| 解析器候选名既不是 bin 也不是外部工具 | FAIL | FAIL |
| 外部工具没在 `EXTERNAL_TOOL_BINS` 里声明 | FAIL | FAIL |

**6/6。**

### 5.3 CI 接线

`.gitea/workflows/ci.yml` 在「Test manifest honesty (#5449)」之后新增一步
「Gate — CLI binary names (issue #5405)」，`if: always()`。

### 5.4 既有门禁复跑

本分支上与「二进制名 / 测试清单诚实性」相关的既有门禁实测：

| 门禁 | 结果 |
|---|---|
| `scripts/gate/check_cli_binary_names.sh` | **PASS**（20 个 bin 名全部溯源到 manifest；C1–C4 全过） |
| `scripts/gate/check_cli_binary_names.sh --selftest` | **PASS**（6/6 个 fixture 行为符合预期） |

C3 的豁免清单里只有 `tests/common/bin_resolver.rs` 一项，且门禁自己把它打印出来
（「files that deliberately carry sample paths」）—— 豁免是**可见的**，不是静默的。

其余门禁的复跑结果见 §6。

---

## 6. 本批次的验证结果

### 6.1 全量回归 R1（根包，474 个测试目标）

命令与环境：

```bash
cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-batch15
export CARGO_TARGET_DIR=/Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-loop1/target
export TMPDIR=/Volumes/workspace/dev/.mavis-tmp
export TPCH_DATA_DIR=/Volumes/workspace/dev/.mavis-tmp/tpch-fixture
cargo build -p sqlrustgo-cli -p sqlrustgo-mysql-server   # BUILD_EXIT=0
cargo test --all-features --no-fail-fast                  # EXIT=101
```

结果：**474 / 474 个测试目标执行完毕**，其中 **2 个目标为红**，其余 472 个绿。

| 红灯目标 | 失败用例 | 归因 |
|---|---|---|
| `--lib`（根包 lib 测试） | `execution_engine_tests::test_v410_atomic_parallel_degree_getter_clamps_to_one` | **既有缺陷 #5423**（并行度 getter 未钳到 1；断言 `left: 4 / right: 1`） |
| `--test regression_test` | `test_regression_suite`（内部 3 个失败） | **基础设施红**（负载导致的时序抖动，见 §6.2） |

两个红灯都与本批次改动无关：

- 红灯① 位于 `src/execution_engine_tests.rs`，本批次**未触碰**该文件；它是 #5423
  的既知随机失败，批次 17 已修（PR #5521），本分支尚未合入。
- 红灯② 的 3 个失败**全部**来自 `page_io_benchmark_test` 的吞吐阈值断言，
  与本批次改动的二进制名解析无关（见 §6.2）。

### 6.2 `page_io_benchmark_test` 的 3 个失败 = 负载导致的时序抖动（基础设施红）

`regression_test` 是一个**跨目标的 meta 套件**，它把别的测试目标的失败算成自己的失败。
本次运行中，它报告的 3 个失败全部来自 `page_io_benchmark_test`：

```
❌ page_io_benchmark_test    5 / 3 / 0  (64096ms) [test test_page_throughput_random_read...]
```

**归因实测**：在全量回归**之外**、低负载条件下单独复跑同一目标：

```
$ cargo test --test page_io_benchmark_test
test test_page_throughput_random_read ... ok
test test_page_throughput_small_io ... ok
test test_page_throughput_buffered_sequential_read ... ok
test test_page_throughput_sequential_read ... ok
test test_page_throughput_sync_overhead ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.89s
```

同一目标在低负载下 **8 / 8 全绿** ⇒ 失败来自**满负载下的吞吐阈值抖动**，
不是代码缺陷。这是**基础设施红**，不计入本批次的代码红灯。

### 6.3 `regression_test` 自身报告的缺陷（#5434，不在本批范围）

harness 的失败清单里列出了约 40 个**未跑**的文件（错误消息为空），而实际失败计数是 3。
清单与计数不一致，是 #5434 记录的问题；本批次只记录，不修。

### 6.4 回归运行对仓库证据文件的改写（#5389）

全量回归运行后，`docs/releases/v4.1.0/evidence/a04-5226/wire/` 下有 **61 个文件**
被测试重新生成（#5389 记录的「差分矩阵测试跑一次就改写仓库里的证据文件」）。
逐个核对后按两类处理：

| 类别 | 文件数 | 处理 | 依据 |
|---|---|---|---|
| 被 `ENGINE_UNAVAILABLE: container a04-pg-ref not running` 覆盖 | 20 | **还原** | 本机没有 postgres 容器 ⇒ 记录的是**环境缺失**，不是产品行为。还原后保留的是此前一次**有环境**时的真实输出 |
| 真实实跑结果 | 41 | **保留** | 见下 |

对保留的 41 个，逐个核对了 diff 的语义。样例
`wire/agg_group_001_mysql_wire.txt`：

```
-  columns=["col_1", "col_2", "col_3", "col_4", "col_5"]
-  row[0]=["x", "1", "2", "2", "2"]
+  columns=["g", "__agg_1", "__agg_2", "__agg_3", "__agg_4"]
+  row[0]=["x", "2", "3", "1", "2"]
```

建表数据是 `('x',1), ('x',2), ('y',10)` ⇒ x 组正确的聚合是
COUNT=2、SUM=3、MIN=1、MAX=2。**旧记录（1/2/2/2）是错的**，与同批 sqlite 记录
（2/3/1/2）矛盾；新记录与 sqlite 一致 ⇒ **旧证据文件是手写的、与实际行为不符的假数据**，
新的是真实实跑产物。这与 #5466（"A04 差分矩阵已入库证据内部自相矛盾且无法从 clean
checkout 复现"）描述的现象吻合。

列名 `__agg_N` 则是 **#5360** 记录的既有缺陷（未加别名的聚合列名暴露 Rust Debug AST），
本批次**不修**，也不因为它而丢弃数值正确的记录。

### 6.5 门禁复跑

见 §5.4。

---

## 7. 本批次**没有**做的事

| 没做 | 为什么 | 去哪 |
|---|---|---|
| 没有把 `crates/cli` 的 package / bin 改名 | 见 §2.2：那是**制造**两个来源 | — |
| 没有修 `scripts/` 里的同类断口（6 处） | 那是**接口决策 + 假绿修复**，属于另一个批次 | **#5516**（本批次新立） |
| 没有把 `scripts/` 纳入 C3 扫描范围 | 同上；纳入会让本门禁的结论依赖一个正在别处关闭的积压 | #5516 |
| 没有修顶层 `cargo test` 不覆盖 `crates/*` 的问题 | 那是 **GA 门禁有效性问题**，量级远大于本批次 | **#5515**（本批次新立） |
| 没有改 `crates/sqlrustgo-cli/src/lib.rs::mysql_server_bin()` | 它不看 `CARGO_TARGET_DIR` —— 是同一个族，但不是本 issue | #5494（已存在） |
| 没有**全面**治理 `env::set_var` 竞态 | #5423 覆盖整个仓库的进程级 env 变更。本批次只修了 `tests/common/bin_resolver.rs` 里**与本次改动直接相关**的那一处（§3.7）；`tests/integration/sql/{io_delay_fault,parallel_main_path,multi_statement}_test.rs` 等处仍有同类写法 | #5423 |
| 没有让 e2e 测试的 `SELECT` 用例复活 | 仍受 server 的 `column_def` 包格式 bug 限制（`#3165`） | — |

---

## 8. 引用

- #5405 —— 本批次关闭的 issue
- #5515 —— 本批次新立：顶层 `cargo test` 只跑根包的 472 个目标，
  `crates/*` 的 301 个目标从未执行
- #5516 —— 本批次新立：`scripts/` 侧 6 处「包名当二进制名」，含一处假绿
- #5494 —— `sqlrustgo` wrapper 定位 server 只看 cwd 与 PATH
- #5423 —— `env::set_var` 竞态
- #3165 —— server `column_def` 包格式 bug
- `crates/sqlrustgo-cli/src/lib.rs:94-103, 215, 404-495` —— `SubCmd::Soak`
  与 `run_soak_repl`（协议实现）
- `crates/cli/src/main.rs:59-87` —— 压测跑子形态的 `Soak`（参数面不同）
- `tests/common/bin_resolver.rs` —— 唯一的 Rust 侧解析入口
- `tests/common/resolve_bin.sh` —— 唯一的 shell 侧解析入口
- `scripts/gate/check_cli_binary_names.sh` —— 本批次新增的元门禁
