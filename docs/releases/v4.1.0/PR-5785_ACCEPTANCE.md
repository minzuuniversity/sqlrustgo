# PR-5785 ACCEPTANCE — clippy 全仓清债（LINT-2）

> **PR**: #5785 · **Issue**: #5362 · **分支**: `opencode-macmini/clippy-clean`
> **冻结基线**: `1cb5d8889e4f1cdb2a935ee354fbe9325187f759`
> **Author**: `opencode-macmini <opencode-macmini@gaoyuanyiyao.com>`
> **Created**: 2026-11-14
> **Status**: PASS（本地冻结 worktree 归档 run；**未声明 CI 绿灯 / 阶段晋级 / 独立复核通过**）
> **配套**: `PR-5785_SPEC.md` / `PR-5785_TEST_PLAN.md` / `PR-5785_TEST_DESIGN.md` / `PR-5785_TEST_REVIEW.md`

---

## 1. 结论

| 判据 | 结果 | 证据 |
|---|---|---|
| CI 同款 clippy（冷编译，`-D warnings`） | **rc=0，`^warning`=0** | run 002 臂 2 |
| workspace clippy（`-D warnings`） | **rc=0** | run 002 臂 3 |
| `cargo fmt --check --all` | **rc=0** | run 002 臂 4 |
| test-target lint 预算 root / workspace | **PASS（0≤0 / 0≤708，compare 未改基线）** | run 002 臂 5/6 |
| anti-ignore（树内 vs registry） | **rc=0（136/136；total≤150，active 82≤92）** | run 002 臂 7 |
| gate_registry | **OK（168 行，fails=0）** | run 002 臂 8 |
| 触碰 crate/test target 回归 | **全绿**（见 §3） | run 002 臂 9–16 |

## 2. 归档 run

### 2.1 通过记录：`20261114-clippy2-002`

- `frozen_commit`: `1cb5d8889e4f1cdb2a935ee354fbe9325187f759`
- `source_agent`: `opencode-macmini`
- `evidence_hash`: `sha256:835219aec30b3db2cbc5882f4e008f143ec52980fd16b2ae1a4e0222e87510f4`
- `audit_run.py verify`：重算一致（rc=0）
- **16 臂全部 `exit_code=0`**：
  1. `clippy2-clean`
  2. `clippy2-clippy-ci-same`
  3. `clippy2-clippy-workspace`
  4. `clippy2-fmt`
  5. `clippy2-lint-budget-root`
  6. `clippy2-lint-budget-workspace`
  7. `clippy2-gate-anti-ignore`
  8. `clippy2-gate-registry`
  9. `clippy2-test-mysql-server-lib`
  10. `clippy2-test-storage-lib`
  11. `clippy2-test-storage-fault-coordination`
  12. `clippy2-test-executor`
  13. `clippy2-test-optimizer`
  14. `clippy2-test-transaction`
  15. `clippy2-test-security`
  16. `clippy2-test-admin`

### 2.1b 外部评审回应验证：`20261114-clippy3-001` + `20261114-clippy3b-001`

- `20261114-clippy3-001`（frozen `23523ff8ee`，12 臂全 rc=0，
  evidence_hash=`sha256:cb9880ca463c8dd0588fcc3229556e2d89487043e342e448954c869578a5fcaa`）
  - 记录最终 HEAD；`git diff --name-only 1cb5d8889e..HEAD -- crates/ scripts/` 只输出
    4 个预期文件（mysql-server lib、security 测试注释、vtu_guard、AFP 白名单）；
  - 清掉被改 crate 缓存后 CI 同款 clippy 仍 rc=0；
  - mysql-server lib **290 passed**（0xfd 边界 10 条）；storage lib **919 passed**
    （vtu_guard 21 条）；security 153+21+29 passed。
- `20261114-clippy3b-001`（frozen `93c8a9dbbe`，3 臂全 rc=0，
  evidence_hash=`sha256:3ce1c1d2e882eac1aad7abed9b91d6e7e4838edc14927895da5360e133be06d3`）
  - 003 的 `no_namespace_db_refusal_5266` 臂因 `#![cfg(feature = "bin_storage_default")]`
    编译为 0 测试（不承重，已披露）；
  - 本 run 用 `--features bin_storage_default` 重跑 ⇒ **5 passed**，含
    `vtu_guard_forwards_foreign_db_to_inner`；同 feature 组合下 CI 同款 clippy 仍 rc=0。

### 2.2 真红取证：`20261114-clippy2-001`

- `frozen_commit`: `e590a50ed15b8d26e8ae020e50c74c9e60d3da30`
- `evidence_hash`: `sha256:fd0c0f775eaba03146c2002fcc6240090ec92662db061fd0eb8cac66f48b0e74`
- 第 10 臂 `cargo test -p sqlrustgo-storage --lib` ⇒ **rc=101**
  - `vtu_guard::tests::test_vtu_guard_tx_state_via_set_current_tx_id`
  - panic: `assertion failed: guarded.in_transaction()`（vtu_guard.rs:536）
- 该 run 原样入仓，只作「判据能红」的取证，**不作通过证据**。

## 3. 回归实测（run 002 日志摘录）

| 臂 | 日志中的 `test result` |
|---|---|
| mysql-server --lib | `288 passed; 0 failed; 0 ignored` |
| storage --lib | `916 passed; 0 failed; 3 ignored` |
| optimizer --lib | `251 passed; 0 failed` |
| transaction coverage_v3_12 | `38 passed; 0 failed` |
| security（lib + 集成） | `21 passed / 29 passed; 0 failed; 12 ignored`（12 ignored = SEC-1 已登记的假覆盖契约，不是本 PR 新增） |
| admin | `14 / 6 / 1 passed; 0 failed` |
| executor --tests | 全部 `0 failed`（含 8 ignored，均为既有登记项） |
| storage fault_coordination | `0 passed; 0 failed` —— **该文件没有 `#[test]` 函数**（它是崩溃套件的 harness 模块，且 `#![cfg(feature = "fault-injection")]`）；本臂只证明该 target 在 lint 改写后仍能编译，**不冒充行为测试** |

## 4. 两处真修的验收

| 修复 | 验收命令 | 结果 |
|---|---|---|
| `decode_lenenc_int` 0xfd 三字节 | `cargo test -p sqlrustgo-mysql-server --lib decode_lenenc_int` | `8 passed; 0 failed`（含 65536 三字节用例） |
| `VtuGuard` 事务状态透传 | `cargo test -p sqlrustgo-storage --lib vtu_guard` | `18 passed; 0 failed` |

## 4a. CI 工具链 1.99 实测（2026-11-14）

CI 已重新开始执行（runner-02/03/04 online）。run **12569**（head `526ca80dc4`）实测：

| job | 结果 |
|---|---|
| `lint-build` | **success**（此前 red 的根因是 1.99 新 lint `clippy::result_large_err`，10 条全来自 `sqlrustgo-network` 的 tonic 生成代码；本 PR 对 `pub mod dtc` 定向 allow + 修 1 处 `unused_mut`） |
| `mutation` | success |
| `test-soak-detach` | success |
| `test` / `test-crates` / `postcheck` | 被仓库级 `concurrency` 取消（同一 ci.yml group 一次只跑一个；随后 #5786 的推送取消了本 run），**不是失败** |
| `gate.yml` | pending → cancelled（同上） |

run **12581**（head `e39bd0c3ad`，同一 ci.yml group）进一步实测：
- `lint-build` **success**；`test-soak-detach` **success**；
- `postcheck` **in_progress**（不再是 skipped；L1 build PASS 后进入 cargo test）；
- `test-crates` 在 `cargo build` 阶段被 crates.io 网络拖住
  （`spurious network error: transfer too slow ... 120s`，`cache: no cached registry`），
  属 #5451/#5533 的外部依赖面；
- 该 run 随后被下一次推送按仓库级 `concurrency` 取消（不是失败）。

本机 1.99 同款命令：`cargo +1.99.0 clippy --locked --all-features --all-targets -- -D warnings` ⇒ rc=0、0 warning。
**不声明 CI 已全绿**；需要一次不被后续推送取消的完整 run（含 postcheck success）才能改状态。

## 4b. 失败 → 修复 → 通过 → 最终 HEAD 的链路（外部评审 I2）

| 环节 | commit / run | 证据 |
|---|---|---|
| 真红 | run `20261114-clippy2-001` | storage-lib rc=101；`vtu_guard::tests::test_vtu_guard_tx_state_via_set_current_tx_id` 断言 `guarded.in_transaction()` 失败 |
| 根因修复 | `1cb5d8889e` | `impl StorageEngine for VtuGuard` 透传事务状态 |
| 通过 | run `20261114-clippy2-002` | 16 臂全 rc=0；storage lib 916 passed |
| 评审加固 | `d4ef219a84` / `23523ff8ee` / `93c8a9dbbe` | VtuGuard 事务生命周期/嵌套/feature 用例；0xfd max/截断；SEC-1 NOT-VERIFIED 注记 |
| 最终 HEAD 验证 | run `20261114-clippy3-001` / `3b-001` | 相对 `1cb5d8889e` 的 code+scripts 差异只 4 个预期文件；12+3 臂全绿 |

## 4c. 外部评审指出的行为变更（已核对，不再隐藏）

1. **0xfd 是协议行为修复，不只是 lint 修复**：`decode_lenenc_int` 的 3 字节长度从
   “旧错误值 ×256” 变为 MySQL lenenc 语义。影响面：`crates/mysql-server/src/lib.rs`
   的 4306/4317 参数长度解码。已补 0x01、65536、0xFFFFFF、truncated-3byte 用例；
   本仓库未发现依赖旧错值的调用方。若下游有外部消费者依赖旧值，需另案兼容性说明。
2. **VtuGuard 事务透传**：生产代码里 `VtuGuard::new` 命中 0（只有测试与静态 marker
   `VtuGuard::<()>::assert_path_for_dml`），因此运行时行为变化限于测试/未来包装；
   已补 begin/commit/rollback、lockfree、nested、`is_transaction_active` 用例。

## 5. 偏差与未覆盖（与 TEST_REVIEW §3 一致）

1. CI 执行面 #5597 不可用 ⇒ 本地归档 run 代替 CI，**不声明 CI 绿灯**。
2. 独立 Reviewer 未回（Hub #133）⇒ 状态「待复核」，**不自签**。
3. 未跑 `cargo test --workspace` 全量 ⇒ **不声明全仓测试绿**。
4. 未做 1.97.0 工具链 lint 预算 `--update`；未清 SEC-1 12 处
   `always-evaluates-to-false`；未清 LocalExecutorDml 退役线 8 处。
5. `physical_backup` 负载 flaky 是既有存量，本 PR 不认领。
6. `decode_lenenc_int` 的修复属协议行为变更：旧值 ×256 是错误，新值符合 MySQL
   lenenc 三字节小端语义；若下游有依赖旧错值的调用方（未发现），需另案处理。
