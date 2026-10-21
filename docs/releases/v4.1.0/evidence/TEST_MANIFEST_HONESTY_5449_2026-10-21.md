# 测试层判别力 —— 清单里的目标、执行到的代码、真的走过的路径

**迭代**：迭代 5（v4.1.0）
**批次**：批次 6 —— 测试层（T2）
**分支**：`fix/test-target-honesty-batch6`
**基线**：`13471d24cf6e`（批次 5 合并提交）
**日期**：2026-10-21
**关闭的 issue**：#5449、#5413、#5482

---

## 0. 一句话

批次 4 修的是「判定结果怎么变成 CI 结论」，
批次 5 修的是「判定本身与被判定对象无关」；
这一批修的是更靠底的一层 —— **判据本身有没有真的执行过**：

- 清单里说有多少个测试目标，和真的编译、真的执行过的，**对不上**；
- 一个声称「验证并行路径」的测试，**从来没走进过并行路径**；
- 一个生成器在**任何小规模下必崩**，而它生成的夹具是某个回归测试的唯一输入。

---

## 1. #5449 —— 「446 个测试目标」这个数字是分裂的

### 1.1 实测：issue 描述的范围**偏小**

issue 说「4 个文件共 22 个 `#[test]` 从未被任何 CI 周期执行过」。
把 `tests/` 下 483 个 `.rs` 按「是否被声明 / 是否可达」完整分类后，
真实集合是 **7 个文件、73 个用例**：

| 类别 | 文件数 | `#[test]` 用例 | 真的执行过吗 |
|------|--------|---------------|--------------|
| 已声明 `[[test]]` | 449 | — | 是 |
| 顶层 `tests/*.rs`，靠 cargo **自动发现** | 15 | 72 | **是** —— 但清单里查不到 |
| 顶层 `tests/sequence_test.rs` | 1 | 10 | **否** —— 被同名声明**静默丢弃** |
| 子目录孤儿文件 | 6 | 34 | **否** —— 从未被编译 |

### 1.2 第 7 类：`tests/sequence_test.rs` 被 cargo **静默丢弃**

这是 issue 都没提到的一类，也是最难发现的一类。

cargo 的自动发现按**文件名**推导目标名。清单里已有一条：

```toml
[[test]]
name = "sequence_test"
path = "tests/integration/sql/sequence_test.rs"
```

名字被占 ⇒ cargo 把自动发现到的 `tests/sequence_test.rs` **静默丢弃**，
**没有任何诊断**。10 个 `#[test]`（F-30 CREATE SEQUENCE）就此永远不执行。

唯一能看见它的入口是 `cargo metadata`：

```
$ cargo metadata --no-deps --format-version 1 | ...
total test targets: 464
  sequence_test -> tests/integration/sql/sequence_test.rs      # 只有这一个
```

### 1.3 为什么既有的 `check_test_target_execution.sh` 看不见这些

它比对的是「**已声明**的目标」与「**被执行**的目标」。
一个**既没被声明、也没被执行**的文件，对它而言根本不存在。

⇒ 需要一个**静态**判据：不依赖测试日志，直接从仓库的形状出发判定。

### 1.4 修法

**（a）把 23 个目标显式写进 `Cargo.toml`**

- 15 个顶层自动发现的 → 显式声明（行为不变，但清单可见）；
- `tests/sequence_test.rs` → 目标名用 `toplevel_sequence_test`（`sequence_test`
  已被占用）；**这是它第一次真正被执行**；
- 6 个子目录孤儿 → 显式声明（**34 个用例第一次被执行**）。

**（b）新门禁 `scripts/gate/check_test_manifest_honesty.sh`**

| 判据 | 内容 |
|------|------|
| C1 | 每条已声明目标的路径都真实存在 |
| C2 | 没有重复的目标名 |
| C3 | 每个含 `#[test]` 的 `tests/**/*.rs` 要么被声明、要么能从某个已声明目标经 `#[path]` / `mod` 可达 —— **无孤儿** |
| C4 | 每个顶层 `tests/*.rs` 都有自己的 `[[test]]` 声明（不依赖自动发现） |
| C5 | 已声明的目标数 == `cargo metadata` 报告的测试目标数（cargo 不在 PATH 时记 **NOT-RUN**，不静默当作通过） |

C5 只有 cargo 自己能回答 —— 它是「cargo 认为本仓库有哪些测试目标」的唯一权威来源。

**（c）4 个死文件第一次编译 ⇒ 暴露 4 类真实缺陷**

| 文件 | 第一次编译暴露的 | 修法 |
|------|----------------|------|
| `tests/compat/bustubx_differential_test.rs` | `make_db_dir()` **未定义**；引擎二进制路径硬编码为 `/Users/liying/dev/sqlrustgo/target/debug/sqlrustgo`（个人机器绝对路径 + 忽略 `CARGO_TARGET_DIR`，与 #5477 同类） | 改用**进程内** `ExecutionEngine` + **进程内** `rusqlite`（root 包依赖，`bundled`）做 oracle；SQLite 方言差异（`MOD` / `CHAR_LENGTH`）以**显式打印**的改写处理 |
| `tests/integration/storage_engine_api_test.rs` | 断言的是**从未存在过的 API**：`Statement::KillConnection{conn_id,is_query}` / `Statement::ShowProcesslist{full}`（真实的是 `Statement::Kill{connection_id,kill_query}` / `Statement::Show(ShowStatement::Processlist{full})`）；`sqlrustgo::parser` 模块不存在 | 对齐到**真实** API（真实形状由 parser 自己的单测锁定），并补上它声称要覆盖却从未断言的 view / cancel-flag 方法 |
| `tests/integration/sql/v312_90_sign_function_test.rs` | 断言依赖 `ORDER BY` 的 **NULL 排序**，而引擎的默认是 **NULLS LAST**（#4748 / #4763，与 SQLite 3.51 一致）⇒ 断言的是排序约定而不是 SIGN 语义 | 改成按**非空键**排序，断言只针对 SIGN |
| `tests/soak/v312_mixed_soak.rs` | `execute_one` **根本不碰数据库**：只 `thread::sleep` 再返回**伪造的延迟值**；`run_ga_soak.sh` / `extract_soak_report.py` **不存在** | 改成**真的跑引擎**：每个 workload class 自己的内存引擎 + 真实 SQL；SQL 错误进 `ops_failed` 并让失败率断言变红 |

最后一个是最典型的「假覆盖」：一个从不执行任何 SQL 的 harness，
配上一个只检查**文件是否存在**的门禁（`check_ga_only_v312.sh:35`）。

### 1.5 门禁与测试

- `scripts/gate/check_test_manifest_honesty.sh`（新增，C1..C5）
- `tests/integration/gate/test_target_manifest_honesty_test.rs`（新增，9 例）
  - 含一条**判别力自证**：把门禁脚本复制到一个含孤儿文件的临时仓库上跑，
    断言它**判 FAIL 且点名那个文件** —— 一个只会输出 PASS 的门禁比没有门禁更糟。
- `.gitea/workflows/ci.yml`：`test` job 新增一步 `Test manifest honesty (issue #5449)`，
  与 `check_test_target_execution.sh` 并列。

---

## 2. #5413 —— 「并行路径」的测试从来没有走进过并行路径

### 2.1 前提不成立，而且比 issue 说的更脆

issue 指出 `const ROWS = 600_000` 的注释写着 `> PARALLEL_MIN_ROWS (500K)`，
而实际阈值是 **2 000 000**（v3.10.0 #3792 从 100K 提上来的）。

但真正致命的是第二层：**CBO 默认开启**（`engine_builder.rs:35`），
而 `should_parallelize_query` 在 CBO 分支里**根本不看扫过的行数**：

```rust
pub fn should_parallelize_query(&self, table_name, where_clause, rows) -> bool {
    if !self.cbo_enabled.load(Ordering::Relaxed) {
        return rows >= PARALLEL_MIN_ROWS;          // ← 只有 CBO 关掉时才看行数
    }
    ...
    cost_model.should_parallelize(&plan)            // ← 用的是统计信息 + 选择性估算
}
```

CBO 分支走 `UnifiedCostModel::should_parallelize` ⇒ `is_compute_bound` ⇒
`output_rows = 统计行数 × 选择性 ≥ 100_000`。
统计行数来自 cost model（表没有统计时**默认 1000**）。

⇒ **光把行数提到 2M 也不够**；而 `k < 100` 在 600k 行上的输出行数是 60 000，
连 100 000 的门槛都不到。

### 2.2 修法：两样东西，缺一不可

**（a）可注入的阈值**

`ExecutionEngine::set_parallel_min_rows_override(min_rows)`。
它在 CBO 分支**之前**短路整个判定 ⇒ 默认配置下也真正生效。
没有它，测试就没有办法在**不分配 200 万行**的前提下走进并行路径。

**（b）可观测的「真的走了」信号**

`ExecutionEngine::parallel_filter_runs()` —— 并行与串行的**结果行完全相同**，
光看输出无法区分。这与仓库里 `src/engine_select.rs` 的 `DIAG_*` 计数器同族。

### 2.3 测试现在钉住两件事：结果正确 **且** 路径确实是那条

| 用例 | 结果 | 路径 |
|------|------|------|
| `test_execute_select_parallel_degree_4_path_engaged` | 12 000 行 | `parallel_filter_runs() == 1` |
| `test_execute_select_parallel_degree_1_path_sequential` | 12 000 行 | `== 0` |
| `test_execute_select_results_match_parallel_1_vs_4` | 两侧行集相同 | 一侧 `== 0`、一侧 `== 1` |
| `test_for_update_disables_parallel_path` | 12 000 行 | `== 0`（安全门真的生效） |
| `test_tpch_q1_style_aggregate_correctness` | 三个度数 SUM 相同 | 度数 >1 时 `== 1` |
| `test_injected_threshold_is_what_engages_the_parallel_path` | 生效阈值 == 生产常量 | 前提守卫 |
| `test_without_injected_threshold_parallel_path_is_not_engaged` | 12 000 行 | `== 0`（对照实验） |

后两条是**对照实验**：同样的查询、同样的行数，唯一的差别是有没有注入阈值 ——
它们保证「注入阈值确实是那个起作用的开关」，防止这些断言悄悄退化成
「串行 == 串行」。

### 2.4 变异测试：断言确实有牙齿

在 `filter_partitions_parallel` 的结果上人为丢掉**一行**（并行结果少 1 行）：

```
test test_execute_select_parallel_degree_4_path_engaged ... FAILED
  expected 12000 filtered rows, got 11999
test test_execute_select_results_match_parallel_1_vs_4 ... FAILED
  left: 12000  right: 11999
test test_tpch_q1_style_aggregate_correctness ... FAILED
  SUM with parallel_degree=2 should equal N=1 (714594000)
  left: 714474901
```

⇒ **3 个测试转红**。在基线上，同一个变异**不会**让任何测试变红
（旧断言是 `rows >= 50_000`，且并行路径根本没被走到）。

---

## 3. #5482 —— `generate_tpch_sf.py` 在小规模下必崩

### 3.1 症状

```
$ python3 scripts/gate/generate_tpch_sf.py --sf 0.0001 --output /tmp/x
  supplier.tbl: 1 rows
  ...
Traceback (most recent call last):
  File "scripts/gate/generate_tpch_sf.py", line 282, in generate_sf
    sups = random.sample(range(1, n_supp + 1), 4)
ValueError: Sample larger than population or is negative
```

`n_supp = int(10000 × sf)` ⇒ SF=0.0001 ⇒ **1** 个供应商 ⇒ 抽 4 个必崩。

### 3.2 修法

**（a）每个 part 的供应商数改成 `min(4, n_supp)`**

TPC-H 规范是「每个 part 恰好 4 个供应商」；规模因子给不出 4 个时，
就给现有的那些。**SF ≥ 0.0004（n_supp ≥ 4）时行为逐字节不变** ——
实测 SF=0.01 的 8 个 `.tbl` 与修复前 **逐字节相同**。

**（b）规模因子下界守卫**

SF < 0.0001 时供应商人口为空，`partsupp.ps_suppkey` / `lineitem.l_suppkey`
全会变成悬挂外键 ⇒ 明确拒绝生成，而不是崩在半路。

**（c）顺带堵掉两个同源的隐患**

- 订单生成里 `while custkey in order_less_customers` 在「全部客户都被预留」时
  **死循环** ⇒ 预留数上界改成 `n_cust - 1`，重试循环改成有界；
- 客户预留数同样受上界约束。

### 3.3 崩溃之后露出的下一层：标准查询集的判据在小规模下**不可满足**

修掉崩溃后，生成器在 SF=0.0001 下继续跑，然后自己的**谓词覆盖自检**判它 FAIL：

```
FAIL orders.o_comment LIKE '%special%requests%': 0/150
FAIL part.p_brand = 'Brand#23': 0/20
FAIL part.p_type = 'ECONOMY ANODIZED STEEL': 0/20
FAIL part.p_type LIKE 'MEDIUM POLISHED%': 0/20
```

这不是「生成器坏了」，是**判据在这个规模下无法满足**：
20 个 part 里要求 `Brand#23` 命中 ≥1%、
`ECONOMY ANODIZED STEEL` 命中 ≥0.1%、
而 `p_type` 是**随机抽**的 —— 期望命中数不到 1。

更硬的一层在标准查询漏斗检查：Q22 要求 **20** 个「余额高于均值且没有订单」的客户，
而 SF=0.0001 一共只有 **15** 个客户 ⇒ **任何**夹具都不可能满足。

实测边界：

| SF | 结果 |
|----|------|
| 0.01 | ✅ 全绿（仓库的标准夹具规模） |
| 0.005 / 0.002 / 0.001 | ❌ 谓词覆盖 FAIL |
| 0.0001 | ❌ 崩溃（修复前） |

⇒ **修法不是把判据调松**，而是让它**在该跑的规模上跑、不该跑的规模上诚实地说没跑**：

新增 `QUERY_SET_MIN_SF = 0.01`（仓库的标准夹具规模，也是漏斗下限可满足的最小规模）。
低于它时，三个**依赖标准查询集**的检查（谓词覆盖 / 复合过滤 / 查询漏斗）
报告 **NOT-RUN** 并带上原因码：

```
=== predicate coverage self-check: NOT-RUN ===
  NOT-RUN: reason=SCALE_TOO_SMALL (SF=0.0001 < 0.01)
  this fixture is NOT verified for standard-query suitability; it is only usable for load-path / row-count checks
```

**与规模无关**的检查（键分布、nation→region 映射）在**任何**规模下都照跑、照红。

### 3.4 顺带修正一个从未被验证过的期望

`tests/integration/tpch/v312_13_load_data_sf1_test.rs` 硬编码了 8 张表的行数，
其中两个**从来不是生成器的真实输出**：

- `partsupp: 80` —— 规范公式是 `part × 4`，但生成器给每个 part `min(4, n_supp)`
  个供应商，SF=0.0001 只有 **1** 个供应商 ⇒ **20** 行；
- `lineitem: 600` —— 每单的行数是**抽签**的（1..7，均值 4，
  为了让「数量之和 > 300」的查询不是结构性空集）⇒ 实测 **630** 行。

这两个期望之所以从未被发现，是因为**生成器在 SF=0.0001 必崩**，
测试从来没走到第 2 步。

修法：期望值**从生成的文件推导**（这个测试的主题是 LOAD DATA 往返：
「文件里有多少行，表里就该有多少行」），另加一条幅值合理性检查
（±25%）防止一个空夹具蒙混过关。

同时新增一条断言：**生成器必须真的披露了 NOT-RUN**
（`stdout` 必须含 `NOT-RUN: reason=SCALE_TOO_SMALL`）——
一个悄悄跳过判据的夹具，绝不能看起来像一个已验证的夹具。

---

## 4. 回归

| 项 | 结果 |
|----|------|
| root crate lib 单测 | **164 / 164 绿** |
| 28 个目标（23 个新声明 + 2 个改写 + 3 个门禁测试） | **28 / 28 目标绿，211 passed / 0 failed / 4 ignored** |
| `cargo fmt --check --all` | 干净 |
| `cargo clippy --all-features --quiet -- -D warnings` | 干净（exit 0） |
| 6 个元门禁 + 新门禁 | 7 / 7 PASS |
| `check_test_count_monotonic` | PASS（基线已更新：cargo 452→477，active 14108→14122） |

---

## 5. 本批新增/关闭的清单

**新门禁**

- `scripts/gate/check_test_manifest_honesty.sh`

**新测试目标**

- `tests/integration/gate/test_target_manifest_honesty_test.rs`（9 例）

**第一次真正执行的测试**

- `tests/sequence_test.rs`（10 例，目标名 `toplevel_sequence_test`）
- `tests/compat/bustubx_differential_test.rs`（10 例）
- `tests/compat/differential_framework.rs`（2 例）
- `tests/compat/semantic_kernel.rs`（10 例）
- `tests/integration/sql/v312_90_sign_function_test.rs`（4 例）
- `tests/integration/storage_engine_api_test.rs`（8 例）
- `tests/soak/v312_mixed_soak.rs`（3 例，含 1 例 nightly-only）

**引擎侧改动**

- `src/execution_engine.rs`：`parallel_min_rows_override` + `parallel_filter_runs`
  + `set_parallel_min_rows_override` / `parallel_min_rows` / `parallel_filter_runs`
  / `reset_parallel_filter_runs`；`should_parallelize_query` 先看覆盖值
- `src/engine_select.rs`：并行路径计数
- `src/engine_builder.rs`：7 处构造点补两个字段

---

## 6. 归因说明（重要）

本批**没有**改动任何「被判定对象」的语义，除了三处**明确记录**的对齐：

1. `storage_engine_api_test.rs` 的 KILL 断言，从「应当报错」改成
   「**实际发布的行为**是返回 OK + 一行文本」——
   真实行为由引擎自己的单测 `test_executor_kill_v312_35` 锁定。
   「KILL 对不存在的连接报告成功」这一**语义分歧****另立 issue**，不在本 PR 改。
2. `v312_90_sign_function_test.rs` 的排序依赖，去掉（引擎的 NULLS LAST 默认
   是 #4748 / #4763 的**刻意**选择）。
3. `v312_13_load_data_sf1_test.rs` 的行数期望，从「规范公式的标称值」
   改成「生成文件里的实际值」+ 幅值合理性检查。

`generate_tpch_sf.py` 在 **SF ≥ 0.01** 的输出**逐字节不变**（实测 8 个 `.tbl` 全部 SAME）。

---

## 7. 变更影响面（GitNexus）与归因

```
$ npx gitnexus detect-changes --scope all --repo /Volumes/workspace/dev/sqlrustgo
Changes: 15 files, 102 symbols
Affected processes: 30
Risk level: critical
```

**CRITICAL 评级的来源**：30 条执行流全部标注 `changed: execute_select` ——
`execute_select` 在几乎每条数据库路径上（INSERT / UPDATE / DELETE /
递归 CTE / …），所以任何对它的改动都会点亮全部流程。

**归因**：本批对 `execute_select` 的改动只有**一行** ——
在并行分支里对一个原子计数器做自增：

```rust
self.parallel_filter_runs.fetch_add(1, Ordering::SeqCst);
```

以及 `should_parallelize_query` 里新增的短路分支：

```rust
let override_rows = self.parallel_min_rows_override.load(Ordering::Relaxed);
if override_rows > 0 {
    return rows >= override_rows;
}
```

**这个分支在生产中是惰性的**，已逐项核实：

| 核实项 | 结果 |
|--------|------|
| `set_parallel_min_rows_override` 的调用点 | 只有 `tests/integration/sql/parallel_main_path_test.rs`（grep 全仓库） |
| `ExecutionEngine` 的全部 8 个构造点的 `parallel_min_rows_override` 初值 | 全部 `AtomicUsize::new(0)` |
| 生产代码（`src/`、`crates/`）里把 override 设成非 0 的路径 | 无 |

⇒ override 恒为 0 ⇒ 短路分支永不触发 ⇒ `should_parallelize_query` 的
生产行为与基线**逐字相同**。

**佐证**：

- root crate lib 单测 **164 / 164 绿**（含引擎自己的并行单测
  `test_parallel_100k_cell_match_n1_vs_n4`）；
- 28 个目标 **211 passed / 0 failed**；
- 变异测试（§2.4）确认新断言能抓到并行结果的 1 行差异。

**注**：GitNexus 的 `detect-changes` 在**主工作区** `/Volumes/workspace/dev/sqlrustgo`
运行时会分析到那里**他人未提交的 13 个文件**（HEAD 仍是旧的 `9fbb63bfbc`）。
本批的检测在 worktree `wt-v410-batch4` 内执行，指向的是本批的改动。
主工作区**全程未触碰**。

