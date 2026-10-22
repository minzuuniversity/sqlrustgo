# PR-5537 SPEC — v4.1.0 ALPHA 阶段门禁阻塞项修复

> Issue: #5537 · 分支: `opencode/fix-alpha-gate-blockers` · 基线: `6f26128890`
> 配套: `PR-5537_TEST_PLAN.md` / `PR-5537_TEST_DESIGN.md` / `PR-5537_TEST_REVIEW.md` / `PR-5537_ACCEPTANCE.md`

---

## 1. 问题陈述

`bash scripts/gate/check_stage.sh --stage ALPHA --version v4.1.0` 实测
**14 PASS / 2 FAIL**。两个失败项不是同一个根因，但指向同一条因果链。

### 1.1 阻塞项 1：coverage 脚本漏传 feature ⇒ storage 覆盖率为 0

```
Measuring coverage for sqlrustgo-storage ...
  FAIL: could not measure sqlrustgo-storage (exit=101)
FAIL: 1/8 crate(s) produced no coverage data: sqlrustgo-storage
```

**根因链**：

1. `crates/storage/src/lib.rs:28` 把 fault-injection 钩子门控在 feature 后面：
   ```rust
   #[cfg(any(test, feature = "fault-injection"))]
   pub mod fault_injection;
   ```
2. `crates/storage/src/lib.rs:92` 把默认存储引擎门控在另一个 feature 后面：
   ```rust
   #[cfg(feature = "bin_storage_default")]
   pub use binary_storage_v2::BinaryTableStorageV2;
   ```
3. 集成测试链接库时**不带** `cfg(test)`（那只是库自身的单元测试编译单元），
   所以这些项对集成测试**根本不存在**
4. `scripts/gate/check_coverage_v312.sh` 不传 feature ⇒ 编译失败：
   ```
   error[E0432]: unresolved import `sqlrustgo_storage::fault_injection`
   error[E0433]: cannot find `fault_injection` in `sqlrustgo_storage`
   ```

**为什么这比「少一个数字」严重**：`check_coverage_v312.sh` 自己的注释记录了
——「a crate that fails to measure therefore *raises* the reported average」。
即 L1_8 平均值是在**静默缩小的样本**上算出来的。数字不只是缺失，
而是**误导性的**。

### 1.2 阻塞项 2：`env!("CARGO_TARGET_DIR")` 编译期依赖

```
$ cargo test --test four_way_compare_test --all-features --no-run
error: environment variable `CARGO_TARGET_DIR` not defined at compile time
```

`env!` 在**编译期**展开，编译时该变量必须存在 —— CI 与本地 checkout
都不保证它存在。涉及两处：
`tests/integration/sql/four_way_compare_test.rs:445`、
`tests/integration/sql/four_way_cell_diff_test.rs:542`。

### 1.3 二者的关联：AFP 门禁的 CHECK 2

`check_alpha_v4.1.0.sh` 之所以 FAIL，是因为它的 Q2_ANTI_FAB 子检查
（`check_anti_fabrication.sh`）报 5 errors，其中包含：

```
[ERROR] Test binaries compile FAILED — NEW untracked failures detected:
[ERROR]   NEW: four_way_compare_test
```

`four_way_compare_test` 正是阻塞项 2 那个编译失败 —— **修好 1.2 就消掉了
`check_alpha_v4.1.0.sh` 的失败**。

---

## 2. 范围

### 2.1 本 PR 做

| # | 修什么 | 文件 |
|---|---|---|
| 1 | coverage 脚本**从 crate 源码推导**所需 feature | `scripts/gate/check_coverage_v312.sh` |
| 2 | coverage 测量**串行执行** | 同上 |
| 3 | coverage 测量**排除环境依赖型测试** | 同上 |
| 4 | `CARGO_TARGET_DIR` 改为运行时解析 | `tests/integration/sql/four_way_{compare,cell_diff}_test.rs` |
| 5 | feature-gated 测试加 crate 级 `#![cfg(...)]` | `crates/storage/tests/` 7 个文件 |
| 6 | 新增回归套件 | `tests/integration/gate/alpha_gate_blocker_test.rs` |

### 2.2 本 PR 不做

- ❌ 不改任何 Gate 阈值
- ❌ 不改 `STAGE_CONFIG.yaml` 的 `required_gates`（门禁数是 SSOT 事实）
- ❌ 不把「测试失败」降级为 WARN（coverage 门禁语义不变）
- ❌ 不修 `write_path_scaling_5181` 的既存性能债（那是独立 issue）
- ❌ 不碰 `.gitea/workflows/ci.yml`

---

## 3. 关键设计决策

### 3.1 feature 需求**推导**而非手写清单

手写清单会**静默过期** —— 失效表现不是报错，而是覆盖率数字悄悄变错。

推导规则：扫 crate 的 `src/` 与 `tests/`，找
`#[cfg(... feature = "X" ...)]`，只保留**模块级公共项**
（`pub mod` / `pub use`）—— 那才是集成测试能 `use` 的东西。
只门控方法的 feature 不算（测试 import 不到它）。

实测结果：

```
$ features_for_crate sqlrustgo-storage
bin_storage_default fault-injection
$ features_for_crate sqlrustgo-types
（空）
$ features_for_crate sqlrustgo-executor
parallel-executor
```

第三行是**推导的额外收获** —— `sqlrustgo-executor` 需要
`parallel-executor`，此前无人知道。

### 3.2 `#![cfg]` 而不是 `#[ignore]`（ADR-008 Policy 2）

- `#[ignore]`：测试**留在**二进制里，只是被跳过。审阅者看到
  「N ignored」，门禁就能在测试从未运行的情况下报 PASS。
- `#![cfg]`：测试**整个移出**二进制。没有「ignored」计数可被误读，
  而开启 feature 的那次构建**确实包含**它。

双向已验证：无 feature 时 `Executable` 不存在，有 feature 时存在。

### 3.3 串行执行

插桩后代码慢 2-10 倍，默认并行时线程争抢 CPU ⇒ **同一个 checkout 两次
测量结果不同**（sqlrustgo-planner：串行 88.91% PASS，并行 FAIL）。

「随机器负载而变的覆盖率不是测量」。这同时符合 AGENTS.md §10 的
「并发测试一律 `--test-threads=1`」。

### 3.4 排除环境依赖型测试

| 类别 | 文件 | 理由 |
|---|---|---|
| 崩溃测试 | `crash_commit_fsync_b1.rs`、`crash_wal_index_b1.rs`、`fault_coordination.rs` | 需要机器特定路径上的 server 二进制 + 精确 fault-point 时序。任一失败 ⇒ **整个 crate 产不出覆盖率表** ⇒ 覆盖率归零。语义由 `check_g14_real_crash.sh` 专门负责 |
| 性能断言 | `write_path_scaling_5181.rs`、`e2e_binlog_replication_test.rs`、`b22_concurrent_flush_ab.rs` | 断言 wall-clock 上界，插桩下必然失真 |

排除清单**从源码自动提取**（测试函数名），新增崩溃测试自动被覆盖。

---

## 4. 并行隔离

| 维度 | 值 |
|---|---|
| 身份 | `opencode-z440` |
| worktree | `.worktrees/opencode-z440-20261022b` |
| 分支 | `opencode/fix-alpha-gate-blockers` |
| base | `develop/v4.1.0` @ `6f26128890` |

**文件面冲突（实测）**：

| 文件 | 触及它的远端分支 | 本 PR 改动 |
|---|---|---|
| `crates/storage/tests/*.rs`（7 个）| **0** | 各加 1 个 `#![cfg]` |
| `tests/integration/gate/alpha_gate_blocker_test.rs` | **0** | 新增 |
| `scripts/gate/check_coverage_v312.sh` | 3 | 改 feature 推导 / 串行 / 排除 |
| `tests/integration/sql/four_way_*.rs` | 6 | 各改 1 处 |
| `Cargo.toml` | 443 | +3 行 `[[test]]` 块 |
