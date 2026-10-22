# PR-5537 TEST_PLAN — v4.1.0 ALPHA 阶段门禁阻塞项修复

> Issue: #5537 · 分支: `opencode/fix-alpha-gate-blockers` · 基线: `6f26128890`
> 配套: `PR-5537_SPEC.md` / `PR-5537_TEST_DESIGN.md` / `PR-5537_TEST_REVIEW.md` / `PR-5537_ACCEPTANCE.md`

---

## 1. 覆盖目标

本 PR 的测试目标不是「覆盖了多少行」，而是**「每个修复在它最可能骗人的地方
会不会骗人」**。

| 失效模式 | 对应验证 |
|---|---|
| feature 清单手工维护 → 静默过期 → 覆盖率数字悄悄变错 | 断言脚本里**没有** `CRATE_FEATURES=`，且**有** `derive_crate_features` |
| 推导逻辑本身坏了（漏掉 `not(feature=)` 的反向门控）| 断言 storage 推导出 2 个 feature、types 推导出 0 个 |
| `#![cfg]` 门控写错 → 测试**永远**消失（比原来更糟）| 双向验证：无 feature 时不存在，有 feature 时 `Executable` 存在 |
| 用 `#[ignore]` 代替 cfg → 门禁在测试从未运行时也能 PASS | 断言 7 个文件里**没有** `#[ignore]` |
| `env!` 藏在注释里 → 断言误报 | 断言只检查**代码**（剥离注释），且用 `option_env!` 替换 `env!` 来做变异 |
| 并行执行 → 覆盖率随机器负载波动 | 断言脚本含 `--test-threads=1` |
| 环境依赖型测试复活 → 整个 crate 覆盖率归零 | 断言 4 个排除文件都在脚本里 |

---

## 2. 测试层次

### 2.1 层 1：Rust 回归套件（新增）

`tests/integration/gate/alpha_gate_blocker_test.rs` —— **8 个用例，0 ignored**。

| 用例 | 守护什么 |
|---|---|
| `coverage_gate_derives_features_from_the_crate_sources` | 推导存在、手写清单已删 |
| `coverage_gate_derives_the_features_storage_actually_needs` | 推导结果正确（含双向：无 feature 的 crate 推导出空）|
| `coverage_gate_does_not_derive_features_for_a_plain_crate` | 推导不过度（`sqlrustgo-types` → 空）|
| `coverage_gate_runs_the_tests_serially` | `--test-threads=1` |
| `coverage_gate_excludes_the_environment_dependent_tests` | 4 个排除文件在册 |
| `the_feature_gated_tests_compile_without_the_feature` | 7 个文件有 `#![cfg(feature = ...)]` |
| `the_feature_gate_is_not_an_ignore` | 7 个文件没有 `#[ignore]` |
| `the_cargo_target_dir_dependency_is_runtime_resolved` | `option_env!` 而非 `env!` |

### 2.2 层 2：负向路径（构造 fixture 实跑）

| 场景 | 期望 | 实测 |
|---|---|---|
| `env -u CARGO_TARGET_DIR` 下编译 four_way 测试 | 成功 | ✅ |
| 变异回 `env!` 后编译 | **失败** | ✅ |
| `cargo test --workspace --no-run` | EXIT=0 | ✅ |
| coverage 脚本完整跑 | EXIT=0，8/8 crate | ✅ |
| ALPHA 阶段门禁 | EXIT=0，16 PASS | ✅ |

### 2.3 层 3：既有门禁回归

| 门禁 | 结果 |
|---|---|
| `check_anti_fabrication.sh` | ERRORS=0, WARNINGS=0（修前 5-7 errors）|
| `check_gate_criterion_existence_test` | 23 passed; 0 failed; 0 ignored |
| `cargo fmt --check --all` | EXIT=0 |
| `cargo clippy --all-features -- -D warnings` | 0 warnings |

---

## 3. 变异测试（判别力证据）

「测试通过」本身不能证明测试有效。逐条把被测逻辑改坏，确认测试会红：

| 变异 | 期望 | 实测 |
|---|---|---|
| `option_env!` → `env!`（four_way_compare_test）| 该用例 FAILED | ✅ |
| 删掉 `derive_crate_features` | 推导用例 FAILED | ✅ |
| 删掉 `--test-threads=1` | 串行用例 FAILED | ✅ |
| 删掉某个排除文件 | 排除用例 FAILED | ✅ |
| 删掉某个文件的 `#![cfg]` | 门控用例 FAILED | ✅ |
| 给某文件加 `#[ignore]` | 非-ignore 用例 FAILED | ✅ |

---

## 4. 未覆盖范围（明确声明）

| 未覆盖 | 原因 |
|---|---|
| 全量回归 `cargo test --all-features --no-fail-fast` | 耗时超预算，**如实声明未跑** |
| CI runner（Alpine/musl）上的实际行为 | 本地是 glibc；新改动无编译期环境依赖、无新增二进制依赖 |
| `write_path_scaling_5181` 的性能债本身 | 既存债，独立 issue；本 PR 只保证它不再摧毁覆盖率测量 |
| 3 个新门禁脚本的 CI 接线 | `ci.yml` 被 5 个他人分支高频改动 |
