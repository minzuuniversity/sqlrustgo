# 测试自包含性修复证据 — #5467 / #5468 / #5469

> **文档 ID**：`TEST_SELF_CONTAINMENT_5467_5469_2026-10-21`
> **日期**：2026-10-21（UTC）
> **分支**：`fix/tests-selfcontained-5467-5469`（基于 `005b6c18f3` = `develop/v4.1.0`）
> **worktree**：`/Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-tests3`
> **关联 issue**：#5467、#5468、#5469
> **治理框架**：`docs/governance/AGENT_PROMPT_TEMPLATE.md` v4.0.0
> （**说明**：该模板文件在本分支上**不存在**，只在 `fix/4597-chaos-t8d-awk-portability` 的
> `9fbb63bfbc` 上；本批次沿用其 SPEC→RED→GREEN→GATE→PR→RECORD→LEARN 闭环结构。）

---

## 0. 一句话

三个测试层缺陷，同一主题：**测试不是自包含的** —— 它们依赖仓库外部的状态
（别的测试留下的残留、进程级环境变量、仓库里已被移动走的文件），
于是「同一个测试在第二次运行时变红」。

其中 **#5469 是一次归因更正**：它最初被判定为「100 ms 绝对耗时阈值在机器负载下随机红」，
本轮用决定性实验证明**真因是进程级环境变量在并行测试间竞争** ——
那个「负载阈值」从来就不是问题。

---

## 1. #5467 — 契约测试断言了一个已被移动走的文件

### 症状

```
$ cargo test --test tpch_sf1_gate_contract_test
running 3 tests
test tpch_sf1_test_can_use_operator_supplied_fixture_dir ... FAILED
thread 'tpch_sf1_test_can_use_operator_supplied_fixture_dir' panicked at
  tests/integration/tpch/tpch_sf1_gate_contract_test.rs:40:55:
read tpch sf1 test: No such file or directory (os error 2)
test result: FAILED. 2 passed; 1 failed; ...
```

### 根因

`tests/integration/tpch/tpch_sf1_gate_contract_test.rs:36` 指向
`tests/tpch_sf1_22_vs_3engines_test.rs`，而该文件在集成测试树重组时被移到了
`tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs`。

```rust
let test_path = repo_root().join("tests/tpch_sf1_22_vs_3engines_test.rs");
let content = std::fs::read_to_string(&test_path).expect("read tpch sf1 test");
```

`expect()` 在**断言之前**就 panic ⇒ 这个用例**根本没有测它声称要测的东西**：
它想守的契约是「SF=1 的测试必须尊重 `TPCH_SF1_DIR`」，
实际守的是「这个路径下有个文件」。

### 修复

路径更新为 `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs`。
已核实该文件确实包含 `TPCH_SF1_DIR`（`:36`、`:41`）⇒ 断言现在测的是它该测的东西。

### 验证

```
$ cargo test --test tpch_sf1_gate_contract_test --no-fail-fast
test tpch_sf1_test_can_use_operator_supplied_fixture_dir ... ok
test tpch_sf1_gate_script_exists_and_delegates_to_baseline ... ok
test tpch_sf1_baseline_uses_canonical_dbgen_lineitem_count ... ok
test result: ok. 3 passed; 0 failed; ...
```

---

## 2. #5468 — 3 个测试拉起 `serve` 子进程时不传 `--data-dir`

### 症状

迭代 5 批次 1 的两次连续本机全量回归：`l3_canonical_binary` 第一次绿、第二次红：

```
thread 'l3_canonical_binary_serve_handshake_auth_and_query' panicked at
  tests/integration/sql/l3_canonical_binary.rs:112:10:
CREATE TABLE: Error("server ERR packet (seq=1): Execution error: Table 'l3' already exists\0")
```

### 根因

`serve` 子命令的 `--data-dir` **有默认值**（`crates/mysql-server/src/main.rs:207-209`）：

```rust
/// SERVER-01: data directory (currently used for temp WAL location)
#[arg(long, default_value = "/tmp/sqlrustgo-data")]
data_dir: String,
```

而这 3 个测试**都不传**它：

| 测试 | spawn 位置 |
|------|-----------|
| `tests/integration/sql/l3_canonical_binary.rs` | `:72` |
| `tests/integration/tpch/tpch_wire_smoke.rs` | `:63` |
| `tests/integration/tpch/tpch_wire_smoke_sf.rs` | `:207` |

⇒ 它们实际共用**全局共享的 `/tmp/sqlrustgo-data`**，互相踩踏，也污染共享 /tmp。

### 修复

每个测试为自己建一个 `TempDir` 并作为 `--data-dir` 传入。
`TempDir` 由 `SubprocessHandle` 持有（新增 `_data_dir: TempDir` 字段），
因此它的生命周期**覆盖**子进程：Rust 的字段析构顺序保证 `Drop`（kill 子进程）
先执行，字段随后才被删除 ⇒ **子进程还在跑时数据目录不会被删掉**。

### RED → GREEN（决定性）

旧文件（`git checkout HEAD -- tests/integration/sql/l3_canonical_binary.rs`），
连跑两次（`/tmp/sqlrustgo-data` 事先清空）：

```
--- 第 1 次 ---
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.80s
--- 第 2 次 ---
CREATE TABLE: Error("server ERR packet (seq=1): Execution error: Table 'l3' already exists\0")
test result: FAILED. 28 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```

修复后连跑三次：

```
--- 第 1 次 --- test result: ok. 29 passed; 0 failed; ... finished in 0.07s
--- 第 2 次 --- test result: ok. 29 passed; 0 failed; ... finished in 0.07s
--- 第 3 次 --- test result: ok. 29 passed; 0 failed; ... finished in 0.07s
$ ls -la /tmp/sqlrustgo-data
不存在（未污染共享 /tmp）
```

---

## 3. #5469 — 归因更正：不是「机器负载」，是**进程级环境变量在并行测试间竞争**

### 3.1 原判定（已被推翻）

本 issue 最初由迭代 5 批次 1 立案，判定为：

> `io_delay_fault_test` 的 **100 ms 绝对耗时阈值**在机器负载波动下随机红 ——
> 基线对照同样红（判别力为零）

当时的证据是「基线对照同样红，耗时几乎逐字相同（1.069 s vs 1.070 s）」，
机器负载 `load averages: 7.84 9.8`。

### 3.2 决定性实验

同一台机器、同一时刻、同一份代码，**并行**（默认）连跑三次：

```
run 1: FAILED — "Without delay, 10 inserts should be fast but got 1.085164708s"
run 2: ok    — test result: ok. 3 passed; 0 failed; ... finished in 0.23s
run 3: FAILED — "Without delay, 10 inserts should be fast but got 1.09372725s"
```

**串行**（`-- --test-threads=1`）连跑三次：

```
run 1: ok — test result: ok. 3 passed; 0 failed; ... finished in 11.12s
run 2: ok — test result: ok. 3 passed; 0 failed; ... finished in 11.19s
run 3: ok — test result: ok. 3 passed; 0 failed; ... finished in 11.17s
```

⇒ **同样的代码、同样的机器、同样时刻**：
并行 2/3 红，串行 3/3 绿。**机器负载解释不了这个差异**（负载在两次实验之间没变）。

### 3.3 真因

`SQLRUSTGO_IO_DELAY_MS` 是**进程级**环境变量，存储层在**每次 I/O** 都读它
（`crates/storage/src/binary_storage.rs` 的 4 处 `crate::io_delay::maybe_delay()`）。
本文件的 3 个用例都在改它，而 Rust 的 test harness **在同一进程内并行跑**这些用例：

| 用例 | 对 env var 的操作 |
|------|------------------|
| `test_io_delay_injected_on_save` | `set_io_delay(50)` → … → `clear_io_delay()` |
| `test_io_delay_injected_on_load` | `set_io_delay(50)` → … → `clear_io_delay()` |
| `test_no_delay_when_not_configured` | `clear_io_delay()` → 测量 |

⇒ 一个用例的 `clear_io_delay()` 可以**在另一个用例测量中途**把它设成 50。

**1.08 s 正是「延迟生效」的数值**：10 次 INSERT × 2 次 I/O × 50 ms ≈ 1.0 s。
也就是说「无延迟」的那次测量，**测到的其实是有延迟的那次**。

旁证：并行时整个文件只要 **0.23 s**，串行要 **11.1 s** ——
并行时延迟根本没被应用够（`test_io_delay_injected_on_save` 的 100 次 INSERT
本该花 ~10 s，却瞬间跑完）⇒ 共享 env var 让**所有**用例都测不到真实行为。

### 3.4 修复

**(1) 串行化**：新增进程级互斥锁，三个用例各自持有它 ⇒ env var 任何时刻只有一个主人。

```rust
static IO_DELAY_LOCK: Mutex<()> = Mutex::new(());

fn io_delay_guard() -> MutexGuard<'static, ()> {
    IO_DELAY_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
```

（锁中毒取 `into_inner` 而不是传播，避免一个用例 panic 连累另外两个。）

**(2) 差分判据替代绝对阈值**：`test_no_delay_when_not_configured` 现在
**在同一个用例内**测两遍同样的工作量 —— 一次 delay=0、一次 delay=200ms ——
然后断言**差值**：

```rust
let difference = slow.saturating_sub(fast);
assert!(difference >= Duration::from_millis(500), ...);
```

**为什么是差值而不是绝对值或比值**：
* 差值是**负载无关**的 —— 机器负载对两半的贡献相同、相消，而人工延迟是**固定的加性成本**；
* 绝对阈值（「10 次 INSERT 必须 < 100 ms」）是**关于机器的陈述**，不是关于代码的陈述 ——
  它在负载下变红，而红的原因与被测代码无关；
* 比值也不可靠 —— 负载会同时放大两半，把比值压向 1。

契约也由此变得可检验：「delay 旋钮为 0 时不施加人工睡眠」，
唯一能观测它的方式就是**把旋钮拧上去看时间是否变化**。

### 3.5 修复后验证

并行连跑三次：

```
run 1: ok — test result: ok. 3 passed; 0 failed; ... finished in 15.66s
run 2: ok — test result: ok. 3 passed; 0 failed; ... finished in 15.60s
run 3: ok — test result: ok. 3 passed; 0 failed; ... finished in 15.64s
```

> **代价必须说清楚**：修复后该文件从 **0.23 s 变成 ~15.6 s**。
> 这不是回归 —— 这是**真实成本**。此前之所以只要 0.23 s，
> 是因为共享 env var 让延迟**根本没被应用**（即测试没测到它声称要测的东西）。

---

## 4. 变更清单

| 文件 | 变更 | 关联 |
|------|------|------|
| `tests/integration/tpch/tpch_sf1_gate_contract_test.rs` | 契约测试路径更新为文件迁移后的真实位置 | #5467 |
| `tests/integration/sql/l3_canonical_binary.rs` | `serve` 传独立 `--data-dir`；`SubprocessHandle` 持有 `TempDir` | #5468 |
| `tests/integration/tpch/tpch_wire_smoke.rs` | 同上 | #5468 |
| `tests/integration/tpch/tpch_wire_smoke_sf.rs` | 同上 | #5468 |
| `tests/integration/sql/io_delay_fault_test.rs` | 进程级互斥串行化；`test_no_delay_when_not_configured` 改为差分判据 | #5469 |

**未改动任何生产代码**（`src/`、`crates/` 均无改动）。

---

## 5. 门禁自检

| 项 | 结果 |
|----|------|
| `cargo fmt --all -- --check` | 干净 |
| `cargo test --test tpch_sf1_gate_contract_test` | 3/3 绿 |
| `cargo test --test io_delay_fault_test`（并行 ×3） | 3/3 绿 ×3 |
| `cargo test --test l3_canonical_binary`（连跑 ×3） | 29/29 绿 ×3 |

---

## 6. 已知限制与分流

1. **`io_delay_fault_test` 的延迟断言下界仍然很松**
   （`test_io_delay_injected_on_save` 断言 `elapsed >= 40 ms`，而实测 ~10 s）。
   下界断言本身**对负载稳健**，本批次未改动它（避免顺手加严无关断言），
   但「100 次 INSERT 竟然只要 40 ms 就算过」这一判别力缺口已记录（见 §7）。
2. **故障注入仍是进程级的**：`BinaryTableStorage` 的 4 处 `maybe_delay()` 读的是
   进程 env var，没有按实例注入的口子
   （`IoFaultInjector` 已存在但**未接进** `BinaryTableStorage`）。
   本批次用互斥锁绕开，**根治方案**是给存储层加按实例注入的口子 —— 那是生产代码改动，
   超出本批次范围。
3. **另 2 个测试拉起 `serve` 时已经传了 `--data-dir`**
   （`tpch_22_mysql_cli_wire_test.rs`、`tests/common/tpch_cli_harness.rs`、
   `tests/e2e/e2e_canonical_subprocess.rs`），本批次只修了确实没传的 3 个。
4. `l3_canonical_binary` 等测试**硬编码** `<workspace>/target/debug/<bin>` 找二进制，
   忽略 `CARGO_TARGET_DIR` ⇒ 用自定义 target dir 时会报
   「canonical binary not found」。这是另一个自包含性缺口，未在本批次处理（见 §7）。
5. **本 PR 与批次 1（PR #5470）、批次 2（PR #5475）无文件重叠**，可独立合并。

---

## 7. 新立 issue

| Issue | 标题 |
|-------|------|
| 待立 | `io_delay_fault_test` 的延迟断言下界过松（`>= 40ms` vs 实测 ~10s）⇒ 判别力缺口 |
| 待立 | 多个测试硬编码 `<workspace>/target/debug/<bin>` 定位二进制，忽略 `CARGO_TARGET_DIR` ⇒ 自定义 target dir 下必然报「binary not found」 |
| 待立 | `BinaryTableStorage` 的故障注入是进程级的（4 处 `maybe_delay()` 读进程 env var），`IoFaultInjector` 未接进存储层 ⇒ 并行测试无法独立注入 |

---

## 8. 复现命令

```bash
cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-tests3

# #5467
cargo test --test tpch_sf1_gate_contract_test --no-fail-fast

# #5469 —— 决定性实验：并行 vs 串行
cargo test --test io_delay_fault_test --no-fail-fast                          # 并行（修复前 2/3 红）
cargo test --test io_delay_fault_test --no-fail-fast -- --test-threads=1      # 串行（修复前 3/3 绿）

# #5468 —— 决定性实验：连跑两次
cargo build -p sqlrustgo-mysql-server
cargo test --test l3_canonical_binary --no-fail-fast
cargo test --test l3_canonical_binary --no-fail-fast   # 修复前第二次红：Table 'l3' already exists
```

---

*本文档由 sqlrustgo 迭代 5 批次 3 生成。所有实测输出均来自上述命令的真实执行，
未做任何人工编辑或修饰。*
