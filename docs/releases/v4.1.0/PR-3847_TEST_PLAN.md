# PR-3847 TEST_PLAN — 覆盖目标与阶段

> Issue: #3847 · 分支: `ci/3847-d6b-no-reexec` · 基线: `57cef09f99`

---

## 1. 覆盖目标

| 目标 | 断言 | 验证手段 |
|---|---|---|
| O1 D6b 不再执行测试 | 脚本内 cargo 子命令 ∈ {`metadata`, `--version`} | 反回归测试（源码级白名单） |
| O2 D6b 不再被 `timeout` 包裹 | 源码中不出现 `timeout` / `TIMEOUT` | 反回归测试 |
| O3 静态核对结论正确 | 实际文件数 / 声明数 / cargo 数三方可核对 | 本机实跑 + 制品 |
| O4 退出码三态 | DEGRADED ⇒ exit 2 | 反回归测试 |
| O5 冒烟是真执行 | 脚本含 `cargo test` 且目标数 3–6 | 反回归测试 |
| O6 冒烟已接线进 CI | `ci.yml` 含 D6 Smoke 步骤且 `if: always()` | 反回归测试 |
| O7 冒烟目标本身可靠 | 4 个硬目标在 base 上全部 PASS；条件目标在二进制齐备时 PASS | 本机实跑（见 ACCEPTANCE） |
| O9 DEGRADED 非静默 PASS | 二进制缺失 ⇒ exit 2 且制品记 `DEGRADED`/`binary_not_found` | 本机实跑（二进制移开） |
| O8 不回归原有门禁 | `check_full_gate_verification.sh` 三态逻辑不变 | 源码审阅 + 脚本实跑 |

## 2. 阶段

| 阶段 | 内容 | 状态 |
|---|---|---|
| S1 本机静态核对 | `bash scripts/gate/check_test_inventory.sh` | 已完成 |
| S2 本机反回归测试 | `rustc --test tests/integration/sql/test_inventory_gate_test.rs` | 已完成（12/12） |
| S3 本机冒烟真执行 | `bash scripts/gate/check_d6_smoke.sh`（硬 4 + 条件 1） | 已完成（PASS / DEGRADED 两态均实测） |
| S4 远程 CI | postcheck 步骤 "D6 Smoke Gate (issue #3847)" | 待 PR 合并后由 CI 验证 |

## 3. 不在本次覆盖范围

- `test` job 自身的执行语义（#3844 范围）。
- v312_13 LOAD DATA 丢行缺陷（#3848）。
- 其余 474 个测试目标的通过性——由 `test` job 负责，本 PR 不重复执行。
