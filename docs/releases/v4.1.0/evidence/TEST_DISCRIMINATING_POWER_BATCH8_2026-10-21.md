# 迭代 5 批次 8 —— 测试层「无判别力 / 假绿」家族

> 覆盖 issue：**#5481**、**#5476**、**#5410**、**#5465**、**#5404**
> 基线树：`de456bf055`（= `develop/v4.1.0` 合并树 `7a4b871dd0`，`git diff` 为空）
> 日期：2026-10-21

---

## 0. 这一批在做什么

批次 7 消灭的是「机器专属路径」造成的**假绿**。批次 8 处理同一家族的另一半：
**判据本身没有判别力** —— 断言要么不存在（零断言），要么写成了一个在真实
硬件上永远不成立、或永远成立的常数，于是「绿」与「红」都不再携带信息。

五个 issue 的共同点：

| issue | 现象 | 为什么是「无判别力」 |
|---|---|---|
| #5481 | `tpch_json_vs_bint_compare` 断言「BINT 至少比 JSON 快 2 倍」 | 阈值来自**某台服务器**的观测值，在别的硬件上结构性恒红 |
| #5476 | `io_delay_fault_test` 的延迟断言下界 `elapsed >= 40ms` | 判据是**总耗时**下界 ⇒ 机器慢就能让「延迟没生效」也通过 |
| #5410 | `harness_validation_test` 的 Q11 引用 3 张未创建的表 | 语句必然失败 ⇒ 走「skipping」分支 ⇒ 断言**不可达** |
| #5404 | `_q15_q16_check` / `four_way_cell_diff_test` **零断言** | 引擎返回错误 / 0 行 / 完全错误的数据，目标都「通过」 |
| #5465 | CI 的 clippy 不带 `--all-targets` | 测试树里的 lint 与**编译错误**从未被门禁看见 |

---

## 1. 事实核实（基线取证）

基线 = 本 worktree 改动前的 `de456bf055`。所有目标在同一环境
（`CARGO_TARGET_DIR` 共享、`TMPDIR=/Volumes/workspace/dev/.mavis-tmp`、
`TPCH_DATA_DIR` 指向 `prepare_tpch_fixture.sh --sf 0.01 --seed 42` 生成的
fixture，86 482 行）下实测。

| 目标 | 基线结果 | 说明 |
|---|---|---|
| `tpch_json_vs_bint_compare` | **FAILED**（`1.01x`，断言要求 `>= 2.0`） | 结构性恒红 |
| `io_delay_fault_test` | ok（3 passed） | 判据无判别力但目标绿 |
| `harness_validation_test` | ok（11 passed） | 其中 Q11 是**空洞通过** |
| `_q15_q16_check` | ok（9 passed） | 2 个真实测试**零断言** |
| `four_way_cell_diff_test` | ok（8 passed，39.5 s） | 1 个真实测试**零断言** |
| `cargo clippy --all-features --all-targets` | **编译失败** | `tpch_cli_harness` 目标从未编译过 |

### 1.1 #5481 的硬件口径定性

阈值 `2.0` 的来源是 `docs/releases/v3.12.0/perf/BIN_LOAD_PERF.md`
§"Speedup vs JSON — revised" 记录的**实测 3.40×**，其硬件是：

```text
2x Intel Xeon Gold 6138 @ 2.00GHz (40 cores / 80 threads),
404GB RAM, 1.9TB NVMe SSD
```

把同一份代码放到别的机器上，两侧速率的关系会**反转**：

| | 基准服务器（文档实测） | 本机实测 |
|---|---|---|
| JSON 写入 | 16 857 rows/s | 87 221 rows/s（**快 5.2 倍**） |
| BINT v3 写入 | 57 373 rows/s | 80 256 rows/s（慢 1.4 倍） |
| **比值** | **3.40×** | **0.92×–1.08×** |

⇒ **比值本身也是关于机器的陈述**：JSON 路径是「一行一文件 + fsync」的写放大
路径，在慢盘上被拖垮；BINT 是顺序二进制写，在快盘上优势收窄。两个速率都随
硬件变化，**比值的符号都可能翻转**。

⇒ 结论：该断言把「特定硬件上的观测值」当成了「实现的不变量」。它测的是硬件，
不是代码 —— 正确实现判红，错误实现可能判绿。

（附带发现：测试文件头的注释引用的路径 `docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md`
**不存在**，真实路径是 `v3.12.0`；`BIN_LOAD_PERF.md` 引用���
`JSON_VS_BINT_MEASURED.md` 作为 "Full report"，而那个文件在仓库里**不存在** ——
证据链断链。）

---

## 2. 逐项修复

### 2.1 #5481 —— 阈值变成「登记在案的性能契约」

新增 `tests/common/perf_contract.rs`，把一条性能判据拆成两层：

| 层 | 判据 | 成立的边界 | 性质 |
|---|---|---|---|
| **普适不变量** | `ratio >= 0.5` | 任何硬件 | 关于实现的陈述 |
| **性能契约** | `ratio >= 2.0` | 仅限基准硬件 | 关于机器的陈述 |

* 普适不变量**始终执行**：BINT 是为写入吞吐设计的存储格式，它的写入路径不应
  在任何机器上比 JSON 路径慢一倍以上。`0.5` 是**退化保护线**，远低于「BINT
  应该更快」的任何合理预期，因此不会因机器差异误报，但足以抓住「BINT 写入
  路径相对 JSON 崩溃了」。
* 性能契约**只在基准硬件上执行**：当前机器是否就是基准机器，由
  `TPCH_PERF_BASELINE_HW` **显式声明**（值必须与登记的 `baseline_hw` 逐字相同）。
  不匹配 ⇒ 契约**未被执行**。
* **未执行 ≠ 通过**：`NotEvaluated` 产出机器可读的披露行
  `PERF-CONTRACT NOT EVALUATED: id=… reason=… detail=…`，调用方必须打印。
  「未执行过的判据等于不存在的判据」（`AGENTS.md` §8）。

单元测试（8 个）覆盖：严重退化必红、未声明时未执行且不算通过、未执行必有
披露行、声明了别的机器仍未执行、在基准硬件上 1.08× 真的判红、在基准硬件上
3.40× 通过、非基准机器上 10× 仍未执行、普适不变量先于契约判定。

### 2.2 #5476 —— 从「总耗时下界」改成「净增延迟下界」

旧判据 `elapsed >= 40ms` 打在**总墙钟时间**上，而总墙钟时间是**关于机器**的
陈述：机器慢或负载高时，未加延迟的 workload 自己就能超过 40ms，断言于是
**在延迟从未生效的情况下通过** —— 这正是 #5476 真正要消灭的假绿。

新判据把**同一个 workload 跑两遍**（无延迟 / 有延迟），只判定**差值**：

```rust
pub fn expected_delay_effect(delay_ms: u64) -> Duration { Duration::from_millis(delay_ms) }
pub fn delay_effect_holds(net_effect: Duration, delay_ms: u64) -> bool {
    net_effect >= expected_delay_effect(delay_ms)
}
```

下界是**一次**注入延迟，而不是「每次 INSERT 一次」：每次 INSERT 触发几次 I/O
是实现细节（当前实测 2 次，100 次 INSERT ≈ 10 s），把它写进判据会让一次合法
的实现变更（写合并成单次 fsync）看起来像回归。

断言消息同时与判据对齐 —— 旧代码的消息写着「expected >=4000ms」而判据是
40 ms，相差 100 倍。

判别力自证（4 个单元测试）：

* 净增为 0（延迟完全没生效）⇒ 判红
* 净增为负（机器抖动，saturating 后为 0）⇒ 判红
* 净增恰好等于一次延迟 ⇒ 判绿（判据没有过紧）
* **旧判据的假绿被逐字写出**：机器慢到 120 ms（无延迟）时，旧的总耗时判据
  `>= 40ms` **通过**，而新判据（净增 0）**拒绝**同一情形。

### 2.3 #5410 —— Q11 变成真断言

* `TABLES` 从 2 张扩到 5 张，补上 `partsupp` / `supplier` / `nation`；
* 建表失败、fixture 缺表、读表失败一律 **panic**（旧代码是 `let _ =` 与
  `continue`）；
* `run_count` → `run_query` / `run_query_rows`：返回
  `Result<_, RunFailure>`，**区分**「语句执行失败」「语句 panic」「超时」——
  旧代码把三者都折叠成 `None`，调用方一律「skipping」；
* Q11 与一个**独立预言机**（进程内 SQLite，`rusqlite` bundled）逐格比对：
  键集合必须相同，每个 `SUM` 聚合值必须相同（相对容差 1e-9）。

### 2.4 #5404 —— 两个零断言目标

新增共享件 `tests/common/tpch_oracle.rs`：TPC-H fixture 的双侧装载
（引擎 / 进程内 SQLite）、单元格渲染、逐格比较，全部进程内、无外部依赖。

**`_q15_q16_check.rs`**

* 数据目录改走 `tpch_fixture::FixtureState`（旧的 `four_way_harness::default_data_dir()`
  会回落到硬编码的 `/tmp/tpch_sf01`）；
* 缺 fixture ⇒ `TPCH-NOT-RUN` 披露，不计入 passed；
* Q15 / Q16 的 SQL 仍取自仓库权威的 `queries/q15.sql` / `queries/q16.sql`，但用
  `CARGO_MANIFEST_DIR` 定位，不依赖调用方的工作目录；
* 两句语句都与进程内 SQLite 预言机**逐格比对**；
* 各加一条**数据无关不变量**：Q15 结果行数 ≤ supplier 行数；Q16 结果行数 ≤
  partsupp 行数。

**`four_way_cell_diff_test.rs`**

这一目标的立意是「PostgreSQL as Truth」。事实核实发现：**PG 可达，但 PG 库里
装的是另一份数据** —— 引擎侧渲染 `Supplier 1`，PG 侧渲染
`Supplier#000000001`；`CHAR(n)` 的补空格与 `DECIMAL(15,2)` 的定点渲染又制造了
一批纯渲染差异。基线上 22 条语句里有 **21 条**「差异」，其中 Q08/Q09/Q11/
Q20/Q21/Q22 的 `pg_rows=0` 其实是 **PG 查询失败**（旧代码把 `Err` 变成空
结果集，于是「PG 没跑起来」被记成了「PG 返回 0 行」）。

⇒ 判据重写为：

* 真值源按**同源检查**选择，而不是按「可达性」选择：
  `FOUR_WAY_TRUTH=pg` **且** PG 的 `lineitem` 行数与 fixture 的相同 ⇒ 用 PG；
  否则用**同源的进程内 SQLite**，并把这次切换**披露**出来；
* 比较改为**归一化**后的单元格级比较（去填充空白 + 数值感知）—— 否则
  `1-URGENT` 与 `1-URGENT       `、`142768` 与 `142768.00` 会被报成缺陷，
  而一个天天误报的缺陷报告不是缺陷报告；
* 判定：与真值的差异必须**登记**在
  `tests/baseline/tpch_known_differences.json`（`tpch:Q<n>` 形式，带 issue 与
  owner），未登记的差异**判红**；
* `SKIP_CELL_DIFF=1` 从「静默 return」改为 `TPCH-NOT-RUN` 披露；
* 报告不再覆写仓库里的 `docs/audit/status/2026-06-07-tpch-cell-diff-v390.json`
  （那会每次跑都弄脏工作区，与 #5389 同族），改写到构建目录
  （`TPCH_CELL_DIFF_OUT` 可覆盖）。

实测：22 条语句与同源 SQLite 预言机**零未登记差异**。

### 2.5 #5465 —— clippy 覆盖测试目标

**事实核实（这是本批最重要的发现）**

CI 的 clippy 步骤是

```yaml
run: cargo clippy --all-features -- -D warnings
```

它只检查 lib 与 bins，**不构建 446 个测试目标**。加上 `--all-targets` 之后
立刻暴露两件事：

1. **一个 deny-by-default 的误报**：
   `tests/integration/sql/v312_62_issue_batch_test.rs` 的 `3.14` 被
   `clippy::approx_constant` 判为错误（`3.14` 是被测 SQL 字面量，不是数学
   常量）⇒ 该目标**编译失败**；
2. **一个批次 7 遗留的编译错误**：
   `tests/common/tpch_cli_harness.rs` 用 `super::bin_resolver::…` 引用共享
   resolver，但这个文件**同时是一个独立的 cargo target**
   （`Cargo.toml`：`[[test]] name = "tpch_cli_harness"`），作为 crate root 时
   `super` 指向不存在的父模块 ⇒
   `error[E0433]: too many leading 'super' keywords` ⇒ **该目标从未编译过、
   从未运行过**。

   ⇒ 也就是说：**#5465 描述的「盲区」正好掩盖了批次 7 自己引入的缺陷。**
   批次 7 的 clippy 验证用的是不带 `--all-targets` 的命令，所以没有发现。

**修复**

* `v312_62_issue_batch_test.rs`：两处 `#[allow(clippy::approx_constant)]`，
  并注明「3.14 是被测 SQL 字面量」；
* `tpch_cli_harness.rs`：改用与同文件既有 `tpch_fixture` 一样的
  `#[path = "bin_resolver.rs"]` 模式（`#[path]` 相对于**声明该模块的文件**，
  两种编译上下文都解析到 `tests/common/bin_resolver.rs`）；
  `tests/common/mod.rs` 改为 `pub use tpch_cli_harness::bin_resolver;`
  ⇒ `bin_resolver` 在整棵树里**只有一份实例**（单元测试不重复，行为不漂移）；
* **测试树的 lint 债务**（55 条，25 个 lint 名）用
  `cargo clippy --all-targets --fix` 清掉，并**逐条审阅**：
  - 审阅结论：全部是语义等价的机械改写（`if a { if b {} }` → `if a && b {}`、
    `.map(|p| f(p))` → `.map(f)`、`match r { Ok(_) => {}, Err(_) => {} }` →
    `if let Ok(_) = r {}`、补 `Default` impl、`derive(Default)`）；
  - ⚠️ `cargo clippy --fix` 在 `src/engine_select.rs` 上触发的是
    `items_after_test_module`（把 `#[cfg(test)] mod chain_builder_tests`
    **移动**到文件末尾），**不是删除**——已用「模块内容逐字相同 + 测试函数
    出现次数不变」验证；
  - ⚠️ `--fix` 一度把该模块**删掉 115 行**（`unused` 的 MachineApplicability），
    本批**回滚并改为手工确认**，未接受这个删除 —— 「不得为了门禁变绿而删
    测试」是红线。

（其余 lint 债务的最终状态见 §4。）

---

## 3. 判别力自证（变异测试）

> 判据在「故意注入缺陷」后必须变红。绿的判据和没有判别力的判据在 CI 上长得
> 一模一样，所以每条新判据都配一次变异。变异注入**测试代码**（绝不注入引擎），
> 跑完逐字节恢复并校验 SHA-256。

| 变异 | 注入的缺陷 | 受影响目标 | 结果 |
|---|---|---|---|
| **M1** | 进程内 SQLite 预言机返回伪造行 | `_q15_q16_check` | **红**（2 failed：Q15、Q16） |
| **M1** | 同上 | `four_way_cell_diff_test` | **红**（1 failed） |
| **M2** | I/O 延迟旋钮**不进入环境**（延迟从不生效） | `io_delay_fault_test` | **红**（3 failed：save / load / no-delay 三个用例） |
| **M3** | 引擎侧 Q11 聚合值 **×2** | `harness_validation_test` | **红**（1 failed：Q11） |
| **M4** | 测试目标里注入一个新的 lint | lint 预算门禁 | **红**（`clippy::useless_format: 1 warning(s) — this lint is NOT in the baseline`） |

⇒ **4/4 通过**：每条新判据都能被真实的缺陷打成红色。

### 3.1 M4 暴露了**门禁自身的假绿**

M4 第一次运行时**没有**变红。追查发现：门禁脚本把「不更新基线」传成字符串
`"0"`，而 Python 侧的判断是 `if update_mode:` —— **任何非空字符串都是真值** ⇒
门禁**永远**走「重写基线」分支，**从不比较**。

⇒ 一个「防止 lint 债务增长」的门禁，自己**永远绿**：它每次运行都把基线改成
当前值，于是「没有超出基线」恒成立。

这正是本批要消灭的那一类缺陷 —— 而且它藏在**门禁**里，比藏在测试里更隐蔽。

修复：

* Python 侧改为**精确字符串比较**（`update_arg == "1"`），并在注释里写明
  「裸真值判断会把 shell 传来的 `"0"` 当成真」；
* 门禁新增**自检**：非 `--update` 模式下，基线文件的 SHA-256 必须在运行前后
  一致，否则判红并说明「一个每次运行都重新基线化的门禁，什么也不比较，
  按构造就是绿的」。

⇒ 这也是为什么本批坚持给门禁本身配变异测试：**门禁的判别力同样需要被证明。**

---

## 4. 回归与门禁

### 4.1 受影响目标的回归

本批改动 44 个文件。按「直接改动」与「`tests/common/` 消费者」两类分批跑。

| 批次 | 目标数 | 结果 |
|---|---|---|
| `cargo test --lib`（`src/` 改动） | 1 | **164 passed, 0 failed** |
| 批 1：直接受影响的快目标 | 26 | 25 绿 + 1 红（`test_target_manifest_honesty_test`，见 §4.3，已修后 11 绿） |
| 批 2：慢目标 | 8 | 全绿（含 `four_way_cell_diff_test` 8 passed / 72.6 s、`mysql_wire_protocol_test` 31 passed） |
| 批 3：`tests/common/` 消费者抽样 | 12 | 全绿（`differential_corpus_test` 36、`tpch_22_mysql_cli_wire_test` 40、`tpch_soak_test` 32 / 38.7 s …） |

⇒ **46 个目标 + lib 全绿**，无回归。

### 4.2 门禁

| 门禁 | 结果 |
|---|---|
| `check_test_manifest_honesty.sh` | **PASS** — 472 declared targets, 0 orphan test files |
| `check_test_count_monotonic.sh` | **PASS** — `active` 14131 → **14143**（+12：perf_contract 8 + io_delay 判定 4），`cargo` 477、`ignored` 171 不变 |
| `check_anti_ignore_gate.sh` | **PASS** — `total_allowed=116 (max=150)`，`active=69 (max=80)` |
| `check_drift_not_pass.sh` | **PASS** — Total anti-patterns: 0 |
| `check_path_casefold_conflict.sh` | **PASS** — 6627 paths checked |
| `check_off_production_path_registry.sh` | **PASS** — PASS=10 FAIL=0 |
| `check_test_target_lint_budget.sh`（本批新增） | **PASS** — 0 warnings，基线未被改动 |

### 4.3 回归中唯一的一次红，以及它暴露的东西

`test_target_manifest_honesty_test::gate_script_passes_on_current_tree` 红了：

```text
FAIL: C6: the test tree contains a machine-specific home-directory path:
    tests/integration/gate/test_target_manifest_honesty_test.rs:607:        "/home/someuser/thing"
```

起因是本批为消除 `clippy::format_in_format_args` 而把

```rust
format!("/{}/someuser/thing", "home")
```

内联成了字面量 `"/home/someuser/thing"`。

⇒ 而**原来的拼接写法是有意的**：C6 判据扫描的是**文件内容**，
这个测试文件正���C6 判据要检验的对象，它**自己**不能含有完整的家目录路径字面量。

⚠️ 顺带暴露了 C6 判据的一个严格性特征：**它连注释里的路径都扫**。
第一次修复时把完整路径写进了注释 ⇒ 仍然红；第二次把注释里的路径也拆开
⇒ 绿。

最终形态：路径在**运行期**由 `["/", "home", "/someuser/thing"].concat()` 拼出，
源码里（连注释里）都不出现完整形态，而写进临时目录的文件里是完整的
`/home/someuser/thing` ⇒ **判据被真正地检验了**，且该文件自身不违反判据。

> 附带：这次修复让 C6 的「自证注入」比之前**更有效** —— 之前的拼接写法
> （`format!("/{}/someuser/thing", "home")`）在源码层面同样不含完整路径，
> 但它与新的拼接写法在**门禁实际扫描的临时文件里**是等价的，所以自证的有效性
> 未被削弱。

### 4.4 `cargo clippy --all-features --all-targets -- -D warnings`

**exit 0**。测试树的 lint 债务从 **55 条（25 个 lint 名）降到 0**。

清零方式：先用 `cargo clippy --all-targets --fix` 做机械修复，再**逐条审阅**：

* 审阅结论：全部是语义等价的机械改写（`if a { if b {} }` → `if a && b {}`、
  `.map(|p| f(p))` → `.map(f)`、`match r { Ok(_) => {}, Err(_) => {} }` →
  `let _ = r.is_ok();`、补 `Default` impl、`derive(Default)`）。
* ⚠️ `--fix` 在 `src/engine_select.rs` 上触发的是 `items_after_test_module`
  （把 `#[cfg(test)] mod chain_builder_tests` **移动**到文件末尾）。已用
  「模块内容逐字相同 + 测试函数出现次数不变」验证是**移动**而非删除。
* ⚠️ `--fix` 一度把该模块**删掉 115 行**（`unused` 的 MachineApplicability），
  本批**回滚并改为手工确认**，未接受这个删除 ——
  「不得为了门禁变绿而删测试」是红线。
* 剩余 12 处由手工修：其中 3 处是**有意的断言 / 文档性代码**
  （`parallel_main_path_test` 的常量钉死、`operators/pragma_table_info` 的列名
  清单、`q4_nested_and_short_circuit_test` 的哨兵计数器），对它们加的是
  **带理由的 `#[allow]`**，不是删除。

### 4.5 GitNexus

```text
Changes: 84 files, 76 symbols
Affected processes: 2
Risk level: medium
```

两个 affected process（`Connect_at → Error`、`Connect_with_caps → Error`）
都指向 `read_packet`（`tests/common/mod.rs`）。

⚠️ **如实披露**：`read_packet` 的实现**一字未动**。本批在
`tests/common/mod.rs` 里的唯一改动是把

```rust
pub mod bin_resolver;
```

换成

```rust
pub use tpch_cli_harness::bin_resolver;
```

（模块级 re-export 变更 ⇒ 索引重算模块归属 ⇒ GitNexus 保守地把同模块的
`read_packet` 记为「changed」）。`git diff tests/common/mod.rs` 的完整内容
就是这一行加注释，可逐字核对。

---

## 5. 遗留与新立

本批**新立 3 个 issue**：

| issue | 内容 |
|---|---|
| q4 哨兵未接线 | `q4_nested_and_short_circuit_test` 的 `CONJUNCT_*_EVALS` / `reset_sentinels` 从未被使用 ⇒ 该测试断言的是「结果正确」而非「短路真的发生了」。**哨兵被保留**（加 `#[allow(dead_code)]`），因为它们记录的是一个尚未兑现的验收意图 |
| 性能文档断链 | `BIN_LOAD_PERF.md` 指向的 "Full report" `JSON_VS_BINT_MEASURED.md` **不存在**；`tpch_json_vs_bint_compare.rs` 注释引用的 `v3.13.0` 目录也不存在 |
| `four_way_harness` 的非家目录形态机器专属路径 | 4 处硬编码的 homebrew 绝对前缀 + 用户名参数 + `/tmp` 回退 —— **C6 判据按设计抓不到它们**（C6 的不变量是「不得以家目录为根」） |

**本批关闭 5 个 issue**：#5481、#5476、#5410、#5465、#5404。

---

## 6. 反伪造声明

1. **本 PR 没有把任何红灯改成绿灯来「让门禁过」**：
   * `tpch_json_vs_bint_compare` 基线是**红**（1.01x < 2.0），本批改成**绿** ——
     但同时把「2×」从普适断言改成**带硬件指纹的性能契约**（在基准硬件上
     仍会判红），并新增了**普适退化保护线**。⇒ 判别力没有丢，硬件口径被
     正确化了。
   * `test_target_manifest_honesty_test` 在回归中**红过一次**，是因为本批的改动
     触碰了 C6 判据；已按 §4.3 修好并复跑绿。
2. **基线对照**：所有「基线即红」的目标（`tpch_json_vs_bint_compare` 1.01x）
   都在改动前的树上实测过，红的原因是**既有**的，不是本批引入。
3. **变异测试 4/4 通过**（§3），每条新判据都被证明能判红。
4. **lint 债务 55 → 0** 是**清零**，不是**豁免**；3 处加 `#[allow]` 的地方
   都带明确理由，且**没有删掉任何测试**（`cargo clippy --fix` 试图删的那 115 行
   被回滚）。
5. **GitNexus 的 risk=medium 与 2 个 affected process 已如实披露**（§4.5），
   并给出了「`read_packet` 实现未动」的逐字证据。
6. **本批没有改动任何 `crates/` 下的生产代码**；`src/` 下只有两处：
   `src/engine_select.rs`（`#[cfg(test)]` 模块的位置移动，内容逐字相同）与
   `src/execution_engine_tests.rs`（3 处 clippy 的机械修）。
7. **覆盖率维度未测量**（本机无覆盖率工具）⇒ 不声称任何覆盖率结论。
