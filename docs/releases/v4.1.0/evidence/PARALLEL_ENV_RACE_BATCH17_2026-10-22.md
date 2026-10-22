# 迭代 5 · 批次 17 —— 根包单元测试的进程级 env 竞态（#5423）

> 分支：`fix/parallel-env-race-5423-batch17`
> 基点：`develop/v4.1.0` = `effa1bcabd7f37e0a1f853fd45176a5e949c8d2f`
> 关闭：#5423

---

## 0. 一句话

根包 lib 测试把 `SQLRUSTGO_EXECUTOR_PARALLELISM` 当作**进程级夹具**来改写，
而同一个测试二进制里的用例是**并发**跑的 ⇒ 改写对所有兄弟用例可见 ⇒
`test_v410_atomic_parallel_degree_getter_clamps_to_one` 会间歇地读到 4 而不是默认的 1
⇒ CI `test` job 有约 1/3 的随机红灯。

本批次把并行度改成**显式注入**（新增 `ExecutionEngine::with_parallel_degree` /
`base_with_raw` / 纯函数 `resolve_parallel_degree`），
根包 `src/` 里的进程级 env 写入降到 **0 处**，
并加一个元门禁让这件事**不可复发**。

---

## 1. 现象：本批次抓到的实例

批次 15 的全量回归 R1（`cargo test --all-features --no-fail-fast`，474 个目标）
在第 1 个目标（根包 lib）上就红了：

```
---- execution_engine_tests::test_v410_atomic_parallel_degree_getter_clamps_to_one stdout ----
thread 'execution_engine_tests::test_v410_atomic_parallel_degree_getter_clamps_to_one' (51572440) panicked at src/execution_engine_tests.rs:1361:5:
assertion `left == right` failed
  left: 4
 right: 1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    execution_engine_tests::test_v410_atomic_parallel_degree_getter_clamps_to_one

test result: FAILED. 163 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.46s
```

### 1.1 这是**既存**问题，不是批次 15 引入的

| 判据 | 实测 |
|---|---|
| 批次 15 是否碰过 `src/` | `git diff --stat 0ba527dd3729 -- src/` → **空** |
| 失败用例所在的文件 | `src/execution_engine_tests.rs` —— 批次 15 未触碰 |
| 失败点 | `src/execution_engine_tests.rs:1361` —— 在批次 15 的 diff 之外 |

⇒ 归类为**基线既有红灯**，不是本 PR 引入。
（#5423 的正文早就记录了同一现象：「PR 分支与基线都会复现 ⇒ 基线既有」。）

---

## 2. 根因

### 2.1 读的是进程级状态

`ExecutionEngine::base_with`（`src/execution_engine.rs`）在**每次构造引擎时**读环境变量：

```rust
let parallel_degree = std::env::var("SQLRUSTGO_EXECUTOR_PARALLELISM").ok()
    .and_then(|s| s.parse::<usize>().ok())
    .filter(|n| *n >= 1).unwrap_or(1);
```

### 2.2 写的是进程级状态

同一个测试模块里有 5 个用例把它当夹具改写：

| 位置 | 操作 |
|---|---|
| `src/execution_engine_tests.rs:559` | `set_var(..., par)`（par 可为 4） |
| `src/execution_engine_tests.rs:561` | `remove_var` |
| `src/execution_engine_tests.rs:586` | `remove_var` |
| `src/execution_engine_tests.rs:592` | `remove_var` |
| `src/execution_engine_tests.rs:618` | `set_var(..., par)`（par 可为 4） |
| `src/execution_engine_tests.rs:620` | `remove_var` |
| `src/execution_engine_tests.rs:659` | `remove_var` |

### 2.3 两者相遇

`std::env::set_var` 改的是**整个进程**的环境表；
Rust 的 test harness 默认**多线程并行**执行同一个二进制里的用例
⇒ 在 `:559` / `:618` 把变量置成 4 的窗口里，
`:1358` 的 `test_v410_atomic_parallel_degree_getter_clamps_to_one`
新建引擎就会读到 **4** 而不是默认的 1 ⇒ 断言 `left: 4 / right: 1` 失败。

**这不是「测试写得不好」，是「拿进程级全局状态当测试夹具」。**
Rust 2024 起 `env::set_var` 已是 `unsafe`，其文档明确写着：
在多线程程序里调用它是未定义行为。

### 2.4 为什么它比确定性红灯更糟

CI 的 `test` job 跑的是 `cargo test --all-features --no-fail-fast`，
根包 lib 测试就在里面 ⇒ 这个 job 有约 **1/3 量级的随机红灯**。

- 「CI 红了未必是本 PR 的锅」⇒ 需要基线对照才能甄别（本次即如此定位）；
- 「CI 绿了也未必是绿的」⇒ 绿灯有 1/3 概率只是运气。

⇒ **偶发红灯必须修，不能登记。** 这正是本批次的立项理由。

---

## 3. 修法：把并行度从「进程全局」改成「显式注入」

### 3.1 生产侧：三个接缝，一份规则

`src/execution_engine.rs`：

```rust
/// 纯函数：原始配置值 → 并行度。**不碰任何进程级状态**。
pub(crate) fn resolve_parallel_degree(raw: Option<&str>) -> usize {
    raw.and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(1)
}
```

| 接缝 | 签名 | 用途 |
|---|---|---|
| `base_with` | `(storage, cbo_enabled)` | **唯一**读环境变量的地方（生产路径） |
| `base_with_raw` | `(storage, cbo_enabled, raw: Option<&str>)` | 测试接缝：喂「原始值」 |
| `base_with_parallelism` | `(storage, cbo_enabled, degree: usize)` | 内部：喂「已解析的并行度」 |
| `with_parallel_degree` | `(storage, degree) -> Self`（**public**） | 测试接缝：喂「并行度」 |

调用链：

```
base_with ──env::var──▶ base_with_raw ──resolve_parallel_degree──▶ base_with_parallelism
                                                                          ▲
                       with_parallel_degree ───────────────────────────┘
```

**一份实现**（env 路径与测试路径共用 `resolve_parallel_degree`），
符合「一个规则只能有一份实现」。

### 3.2 生产行为**没有变**

- `ExecutionEngine::new` / `with_cbo` / `with_catalog` 的语义完全不变 ——
  它们仍然读 `SQLRUSTGO_EXECUTOR_PARALLELISM`；
- 唯一变化是「读环境变量」这一步被抽成了可注入的接缝。

### 3.3 测试侧：4 个用例全部改为注入

| 用例 | 改前 | 改后 |
|---|---|---|
| `test_parallel_n1_eq_n4_cell_match` | `set_var` / `remove_var` + `ExecutionEngine::new` | `ExecutionEngine::with_parallel_degree(storage, par)` |
| `test_parallel_default_n1_no_env_var` | `remove_var` + `ExecutionEngine::new` | `resolve_parallel_degree` 矩阵 + `ExecutionEngine::base_with_raw(storage, true, None)` |
| `test_parallel_100k_cell_match_n1_vs_n4` | `set_var` / `remove_var` + `ExecutionEngine::new` | `ExecutionEngine::with_parallel_degree(storage, par)` |
| `test_v410_atomic_parallel_degree_getter_clamps_to_one` | `ExecutionEngine::new`（读环境） | `ExecutionEngine::with_parallel_degree(storage, 1)` |

⇒ 根包 lib 测试二进制里**进程级 env 写入从 7 处降到 0 处**。

### 3.4 覆盖面没有丢

| 原来覆盖的东西 | 现在覆盖它的用例 |
|---|---|
| 「默认并行度是 1」 | `test_parallel_default_n1_no_env_var`（纯函数矩阵 + `base_with_raw(None)`） |
| 「env 里的值真的生效」 | `tests/integration/sql/parallel_main_path_test.rs::test_cli_flag_propagation_via_env`（**独立测试二进制 ⇒ 无兄弟竞态**） |
| 「setter 把 0 夹到 1」 | `test_v410_atomic_parallel_degree_getter_clamps_to_one`（构造用注入值，不再受环境影响） |
| 「N=1 与 N=4 结果逐格相同」 | `test_parallel_n1_eq_n4_cell_match` / `test_parallel_100k_cell_match_n1_vs_n4`（注入后仍是同样的两条路径） |

### 3.5 顺带补齐 `crates/mysql-server` 的半套锁

`crates/mysql-server/src/lib.rs` 里有一个共享的 `utilities_tests::ENV_LOCK`，
但**只有 3 个用例在用**，`read_executor_parallelism_honors_env` 与
`read_executor_parallelism_clamps_zero_and_invalid` **改 env 却没持锁**
⇒ 它们能反过来把持锁的 `read_executor_parallelism_defaults_to_one` 顶掉。

本批次给这 2 个用例补上同一把锁（与仓库既有的
`crates/storage/tests/a06_failpoint_reachability.rs::env_lock` 同一模式）。

---

## 4. 判别力

### 4.1 负对照：把修复撤掉，门禁必须红

门禁 `scripts/gate/check_no_process_env_mutation.sh` 对**修复前**的树扫描：

```
  scanned 31 file(s)

  C1 violations:
    src/execution_engine_tests.rs:559  [set_var] std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", par.to_string());
    src/execution_engine_tests.rs:561  [remove_var] std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
    src/execution_engine_tests.rs:586  [remove_var] std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
    src/execution_engine_tests.rs:592  [remove_var] std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
    src/execution_engine_tests.rs:618  [set_var] std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", par.to_string());
    src/execution_engine_tests.rs:620  [remove_var] std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
    src/execution_engine_tests.rs:659  [remove_var] std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");

  FAIL: C1 — 7 process-global env mutation(s) in the root package
```

⇒ 精确命中 #5423 正文表格里的 **7 个位置**，与人工清单一一对应。

### 4.2 负对照：把修复撤掉，**测试**必须红

（见 §6.3 —— 这是本批次的关键判别力证据）

---

## 5. 新门禁 `scripts/gate/check_no_process_env_mutation.sh`

### 5.1 判据

> **C1**：`<repo>/src/**/*.rs` 不得调用 `env::set_var` / `env::remove_var`
> （直接调用、经 `use std::env::set_var` 导入、或任何别名写法都算）。

### 5.2 为什么只管根包

每个 Cargo 测试目标都是独立进程，env 改写只会和**同一二进制里的兄弟用例**竞态。
真正咬人的是根包的 lib 测试二进制：

- `crates/storage/tests/a06_failpoint_reachability.rs` —— 已经用本地 `env_lock()` 串行化
  （故障注入钩子是在故障点现场读 env 的，注入不了）；
- `crates/mysql-server/src/lib.rs` —— 用共享 `ENV_LOCK` 串行化。

⇒ 规则刻意收窄：**不宣称「到处都不许改 env」，只禁止「在没有锁、且已实测出 1/3 红灯率」的那棵树里改 env**。

### 5.3 逃生通道（不是「此路不通」）

根包测试如果真要观察 env 驱动的代码路径，有两条正路：

1. **用显式接缝** —— `with_parallel_degree` / `base_with_raw` / `resolve_parallel_degree`
   就是为此而存在的；
2. **把测试挪到 `tests/` 下的独立测试二进制** —— 独立进程 ⇒ 没有兄弟竞态。

### 5.4 自检 `--selftest`：7/7

```
=== Self-test: synthetic fixtures ===
  PASS: clean root src: expected PASS, got PASS
  PASS: std::env::set_var in root src: expected FAIL, got FAIL
  PASS: std::env::remove_var in root src: expected FAIL, got FAIL
  PASS: use std::env::set_var import in root src: expected FAIL, got FAIL
  PASS: env mutation under crates/ is out of scope: expected PASS, got PASS
  PASS: analyzer reports the exact violation count (1)
  PASS: env::set_var without std:: prefix in root src: expected FAIL, got FAIL

  PASS: self-test: all checks passed
```

第 5 条是关键：**`crates/` 下的 env 改写必须仍然判绿** ——
证明这条规则没有把仓库里已有的、正确的用法误伤。

### 5.5 CI 接线

`.gitea/workflows/ci.yml` 的 `lint-build` job（与批次 15/16 的两个门禁并列）：

```yaml
      - name: Process-global env mutation gate (issue #5423)
        run: bash scripts/gate/check_no_process_env_mutation.sh
```

---

## 6. 本批次的验证结果

> （待填：实测输出）

---

## 7. 边界：本批次**没有**做的事

| 没做 | 为什么 | 去哪 |
|---|---|---|
| 没有动 `crates/mysql-server` 的**生产**代码 | 只给 2 个测试补了锁；生产侧 `read_executor_parallelism` 的 env 读取是**按调用时**读的，注入不了 | — |
| 没有把 `crates/*` 纳入新门禁 | 它们是独立测试二进制，且各自已有锁纪律；纳入会误伤正确的用法 | — |
| 没有动 `tests/common/sf1_fixture.rs` 的 env 改写 | 它在**集成测试**的公共库里，每个测试目标一个进程 ⇒ 同一二进制内只有它自己在用 | — |
| 没有处理 `tests/integration/sql/parallel_main_path_test.rs` 的 env 改写 | 同上：独立二进制，单一改写者 | — |
| 没有碰批次 11 的引擎缺陷 | 那是另一条线（R2 回归待跑） | 批次 11 |

---

## 8. 引用

- `src/execution_engine.rs` —— `resolve_parallel_degree` / `base_with_raw` /
  `base_with_parallelism` / `with_parallel_degree`
- `src/execution_engine_tests.rs` —— 4 个用例改为显式注入
- `crates/mysql-server/src/lib.rs` —— 2 个用例补 `ENV_LOCK`
- `scripts/gate/check_no_process_env_mutation.sh` —— 本批次新增的元门禁
- `.gitea/workflows/ci.yml` —— 新门禁的接入点（`lint-build` job）
- #5423 —— 本批次关闭的 issue
- #5405 —— 批次 15；其 R1 回归捕获了本 issue 的实例
- `crates/storage/tests/a06_failpoint_reachability.rs` —— 仓库既有的 `env_lock` 模式
