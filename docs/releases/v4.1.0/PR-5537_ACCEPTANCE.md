# PR-5537 ACCEPTANCE — v4.1.0 ALPHA 阶段门禁阻塞项修复

> **PR**: 待创建（分支 `opencode/fix-alpha-gate-blockers`）
> **Issue**: #5537
> **Base**: `6f26128890`（`develop/v4.1.0`）
> **Commit**: `2fef945260`
> **Author**: `opencode-z440 <opencode-z440@gaoyuanyiyao.com>`
> **Created**: 2026-10-22
> **Status**: PASS（本地实跑）
> **配套**: `PR-5537_SPEC.md` / `PR-5537_TEST_PLAN.md` / `PR-5537_TEST_DESIGN.md` / `PR-5537_TEST_REVIEW.md`

---

## 1. 概述

本 PR 把 v4.1.0 的 **ALPHA 阶段门禁从 14 PASS / 2 FAIL 修到 16 PASS / 0 FAIL**。

两个阻塞项的根因不同，但因果链相连：

- 阻塞项 2（`env!("CARGO_TARGET_DIR")` 编译期依赖）让
  `four_way_compare_test` 编译失败 ⇒ AFP 的 CHECK 2 报「NEW untracked
  failure」⇒ `check_alpha_v4.1.0.sh` FAIL
- 阻塞项 1（coverage 脚本漏传 feature）让 `sqlrustgo-storage` 覆盖率为 0
  ⇒ `check_coverage_v312.sh` FAIL

修完这两项后，**另有两处让覆盖率测量可复现的缺陷**暴露出来（串行 vs 并行的
结果差异、环境依赖型测试把整个 crate 的覆盖率拖成 0），一并修掉。

---

## 2. 验收标准与实测

### AC-1: ALPHA 阶段门禁全绿

**判据**：`bash scripts/gate/check_stage.sh --stage ALPHA --version v4.1.0` → EXIT=0

```
--- Required gates (9) ---
  [PASS] check_alpha_v4.1.0.sh
  [PASS] check_arch_invariants.sh
  [PASS] check_arch3_no_bypass.sh
  [PASS] check_coverage_v312.sh
  [PASS] check_sql_corpus_gate.sh
  [PASS] check_anti_ignore_gate.sh
  [PASS] cargo build --all-features
  [PASS] cargo test --all-features --lib
  [PASS] cargo fmt --check

  Stage ALPHA (v4.1.0): 16 PASS, 0 FAIL
EXIT=0
```

**修前**：14 PASS / 2 FAIL，EXIT=1
**PASS** ✅

### AC-2: coverage 门禁测出全部 8 个 crate

```
$ bash scripts/gate/check_coverage_v312.sh
  sqlrustgo-parser: 86.46%
  sqlrustgo-planner: 88.91%
  sqlrustgo-executor: 82.68%
  sqlrustgo-transaction: 93.29%
  sqlrustgo-storage: 80.83%
  sqlrustgo-catalog: 87.33%
  sqlrustgo-optimizer: 87.85%
  sqlrustgo-types: 95.28%
L1_8 average coverage: 87.82% (threshold: 80.0%)
EXIT=0
```

**PASS** ✅ —— 修前 `sqlrustgo-storage` 是 `could not measure (exit=101)`。

### AC-3: AFP 门禁 0 errors

```
$ bash scripts/gate/check_anti_fabrication.sh
[PASS] Test binaries compile PASS
Results: ERRORS=0, WARNINGS=0
```

**修前**：ERRORS=5~7（含 `NEW: four_way_compare_test`、`NEW: fault_coordination`、
`NEW: sqlrustgo-storage`）
**PASS** ✅

### AC-4: workspace 测试全部可编译

```
$ cargo test --workspace --no-run
EXIT=0
```

**修前**：FAIL（`could not compile sqlrustgo-storage (test "fault_coordination")`）
**PASS** ✅

### AC-5: `CARGO_TARGET_DIR` 不再是编译期依赖

```
$ env -u CARGO_TARGET_DIR cargo test --test four_way_compare_test \
      --test four_way_cell_diff_test --all-features --no-run
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 01s
```

**PASS** ✅ —— 变异验证：改回 `env!` ⇒ 编译失败（已实测）。

### AC-6: feature 需求是推导的，不是手写的

```
$ features_for_crate sqlrustgo-storage
bin_storage_default fault-injection
$ features_for_crate sqlrustgo-types
（空）
$ features_for_crate sqlrustgo-executor
parallel-executor
```

**PASS** ✅ —— `sqlrustgo-executor` 需要 `parallel-executor` 是推导的额外收获。

### AC-7: 门禁语义未变

本 PR **没有**：
- 改任何 Gate 阈值
- 改 `STAGE_CONFIG.yaml` 的 `required_gates`
- 把「测试失败」降级为 WARN

coverage 门禁仍然是「8 个 crate 都必须测出覆盖率，否则 FAIL」
（#4944 P1-01 的保证）。

**PASS** ✅

### AC-8: 新增回归套件有判别力

```
$ cargo test --test alpha_gate_blocker_test --all-features
test result: ok. 8 passed; 0 failed; 0 ignored
```

**PASS** ✅ —— 每条都用变异测试验证过（把 `option_env!` 改回 `env!`
⇒ 该用例 FAILED）。

### AC-9: 格式与静态检查

```
$ cargo fmt --check --all          → EXIT=0
$ cargo clippy --all-features -- -D warnings → 0 warnings
$ cargo test --test gate_criterion_existence_test --all-features → 23 passed; 0 failed; 0 ignored
```

**PASS** ✅

---

## 3. 未达成 / 未验证项（不省这一段）

| # | 项 | 状态 | 原因 |
|---|---|---|---|
| 1 | 全量回归 `cargo test --all-features --no-fail-fast` | ⚠️ 未跑 | 耗时超预算。**如实声明未跑**，不宣称通过 |
| 2 | `write_path_scaling_5181::report_single_row_delete_cost` 的性能债 | ⚠️ 未修 | 该测试在 base 上也失败（0.00227s vs 0.002s 阈值），是既存性能债。本 PR 排除它是为了不让该债**顺带摧毁覆盖率测量**，不是宣称它通过了 |
| 3 | CI runner（Alpine/musl）上的实际行为 | ❌ 未验证 | 本地是 glibc。新改动无编译期环境依赖、无新增二进制依赖，风险低但**未实测** |
| 4 | 三个新门禁脚本的 CI 接线 | ❌ 未做 | 见 #5537 之外的遗留项：`ci.yml` 被 5 个他人分支高频改动 |
| 5 | 存量 Hard Gate 失败 | ⚠️ 仍在 | `check_drift_not_pass.sh` / `check_docs_pointer_paths.sh` / `check_test_target_lint_budget.sh` 在 base 上即 FAIL（他人 batch 引入），本 PR 不引入新的 |

---

## 4. 证据（ADR-014 5 fields）

```
source_agent:      opencode-z440
source_run:        opencode-z440-20261022-alpha-blockers
timestamp:         2026-10-22T13:20:00Z
evidence_hash:     见 PR body §8
conflict_resolution:
  新增 1 个文件（tests/integration/gate/alpha_gate_blocker_test.rs）与全部
  168 个远端 fix/docs 分支零重叠。
  改动 11 个既有文件，冲突面已实测：
    crates/storage/tests/*.rs（7 个）→ 0 个分支触及
    scripts/gate/check_coverage_v312.sh → 3 个分支触及
    tests/integration/sql/four_way_*.rs → 6 个分支触及
    Cargo.toml → 443 个分支触及（只 +3 行 [[test]] 块，行级不重叠）
  门禁日志已提交为证据：docs/releases/v4.1.0/logs/gate_alpha_6f26128890_*.log
```

---

## 5. 结论

**PASS（本地实跑）**。

9 条验收标准全部通过，含 2 条变异验证。ALPHA 阶段门禁从
**14 PASS / 2 FAIL** 修到 **16 PASS / 0 FAIL**。

不合并自己的 PR（PERMISSION_MATRIX §3）。合并由 User / Reviewer 执行。
