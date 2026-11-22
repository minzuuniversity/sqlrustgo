# PR-5785 TEST_PLAN — clippy 全仓清债（LINT-2）

> Issue: #5362 · PR: #5785 · 分支: `opencode-macmini/clippy-clean`
> 冻结基线: `1cb5d8889e` · 阶段: ALPHA
> 配套: `PR-5785_SPEC.md` / `PR-5785_TEST_DESIGN.md` / `PR-5785_TEST_REVIEW.md` / `PR-5785_ACCEPTANCE.md`

---

## 1. 覆盖目标

本 PR 的判据不是「clippy 输出好看」，而是**「-D warnings 的绿必须来自真的 0 warning，
而不是缓存、口径或豁免」**。

| 失效模式 | 对应验证 |
|---|---|
| 暖 target 重放旧诊断 ⇒ `-D` 绿是假绿 | 归档 run 第 1 臂 `cargo clean`，其后 clippy 全量重编译 |
| 只看退出码、不看 warning 数 | 额外统计 `^warning` 行数；未来不兼容提示不计入 |
| 用 `allow` 掩盖真实缺陷 | 两处被掩盖的缺陷（0xfd、VtuGuard）改为真修 + 常驻测试 |
| 用 `#[ignore]` 换取绿 | anti-ignore 门禁核对树内真实 `#[ignore]` 与 registry（136/136） |
| 计数型判据永远绿 | 先跑出真红 run `20261114-clippy2-001`（storage-lib rc=101），修复后转绿 |
| workspace 面与 root 面口径不一致 | 两面分别跑 `-D warnings` |
| 测试被 lint 改写弄坏 | 逐一跑本分支触碰的 crate/test target 全套测试 |

## 2. 测试层次

### 2.1 层 1：冷编译 lint 判据

| 命令 | 期望 |
|---|---|
| `cargo clean` | rc=0（随后无缓存） |
| `cargo clippy --locked --all-features --all-targets -- -D warnings` | rc=0，`^warning`=0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | rc=0 |
| `cargo fmt --check --all` | rc=0 |

### 2.2 层 2：既有门禁

| 门禁 | 期望 |
|---|---|
| `check_test_target_lint_budget.sh --scope root` | PASS（0≤0，compare 不改基线） |
| `check_test_target_lint_budget.sh --scope workspace` | PASS（0≤708，compare） |
| `check_anti_ignore_gate.sh` | rc=0（total 136≤150，active 82≤92，树内 136 一致） |
| `python3 scripts/gate/gate_registry.py check` | OK（168 行，fails=0） |

### 2.3 层 3：行为回归

| 命令 | 覆盖点 |
|---|---|
| `cargo test -p sqlrustgo-mysql-server --lib` | 0xfd/0xfc/0xfe/small/null/truncated 8 条 |
| `cargo test -p sqlrustgo-storage --lib` | vtu_guard 18 条 + storage 单测 |
| `cargo test -p sqlrustgo-storage --test fault_coordination` | 删除未使用 builder 方法后的集成路径 |
| `cargo test -p sqlrustgo-executor --tests` | 删 helper / allow(deprecated) 过渡面 |
| `cargo test -p sqlrustgo-optimizer --lib` | rules.rs 改写 |
| `cargo test -p sqlrustgo-transaction --test coverage_v3_12` | Result 显形加固 |
| `cargo test -p sqlrustgo-security` | v400_acl_audit 文件级 allow 后仍绿 |
| `cargo test -p sqlrustgo-admin` | 删 `new_admin` 与恒真断言后仍绿 |

## 3. 阶段与边界

- 阶段：ALPHA；不声明阶段晋级、不声明 GA。
- CI 执行面 #5597 仍死 ⇒ 证据为本地冻结 worktree 归档 run，不以 CI 绿灯冒充。
- 独立 Reviewer 未回（Hub #133）⇒ 状态保持「待复核」，本 PR 不自签。
- bench examples 只做编译面覆盖（`clippy --all-targets`）；不跑性能基线。
