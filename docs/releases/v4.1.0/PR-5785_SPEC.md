# PR-5785 SPEC — clippy 全仓清债（LINT-2）

> Issue: #5362 · PR: #5785 · 分支: `opencode-macmini/clippy-clean`
> 冻结基线: `1cb5d8889e` · 阶段: ALPHA（`STAGE.yaml current_stage: ALPHA`）
> 配套: `PR-5785_TEST_PLAN.md` / `PR-5785_TEST_DESIGN.md` / `PR-5785_TEST_REVIEW.md` / `PR-5785_ACCEPTANCE.md`

---

## 1. 问题陈述

`AGENTS.md` 要求 `cargo clippy --all-features -- -D warnings` 必须通过，但 develop tip
上仍有两层欠账：

1. **广度欠账**：`cargo clippy --workspace --all-features --all-targets` 的存量诊断
   （看板行 LINT-1 记录批次 1–9 清完后仍余 62 条）。
2. **可信度欠账**：暖 `target/` 下 cargo 会重放旧诊断；如果不先 `cargo clean`，
   `-D warnings` 的 rc=0 可能只是「这次没有重编译」而不是「真的 0 warning」。

本 PR 同时解决这两层：先冷编译，再清零，并把判据承重证据（一次真红 run）入仓。

## 2. 范围

### 2.1 纳入

约 30 处诊断，按处置分类：

| 类别 | 处置 |
|---|---|
| 结构损坏（vtu_guard.rs 双层嵌套 test、decode_lenenc 漏 `#[test]`） | 合并/补齐为单个完整 test |
| 恒真/无义断言（`assert!(true)`、`const_is_empty` 占位契约） | 删除恒真断言；占位契约文件级 `allow` + 注释 |
| 死代码（零引用 helper / builder / 字段） | 能删则删；预留配置面 `allow` + 注释 |
| deprecated 过渡面（LocalExecutorDml 测试 mod） | `allow(deprecated)` + 说明其行为守卫身份 |
| doc/格式类（续行缩进、空行后属性、穷尽 match 兜底） | 按 clippy 建议就地修 |
| `duplicate_mod ×79`（`not_run_log.rs` 被 `#[path]` 多路径加载） | 文件头 `allow` + 单 target 精确 `allow` |

### 2.2 顺带真修（清债路上暴露，不挂账）

1. **`decode_lenenc_int` 0xfd 分支 ×256**（生产路径）
   - 原实现：`u32::from_le_bytes([0, b0, b1, b2])` —— 0 放在最低字节。
   - 影响：`crates/mysql-server/src/lib.rs:4306/4317` 用它解参数长度；3 字节长度
     会读错偏移。
   - 修复：`[b0, b1, b2, 0]`；原被 `#[ignore]` 的测试转常驻，并新增 65536 用例。
2. **`VtuGuard` 事务状态不透传**
   - `impl StorageEngine for VtuGuard` 未覆盖 `in_transaction` / `current_tx_id` /
     `set_current_tx_id` / `set_current_tx_id_shared` ⇒ 全部落 trait 默认
     （false / 0 / no-op），与「不干扰被包裹存储」的契约相反。
   - 该缺陷由一条被嵌套结构吞掉、从未运行过的测试激活后实测暴露
     （run `20261114-clippy2-001` 的 storage-lib 臂 rc=101）。

### 2.3 不做（显式登记）

- `this expression always evaluates to false` 12 处：属 SEC-1 属主裁定域。
- `use of deprecated struct LocalExecutorDml` 8 处：属退役线，不是 lint 改写。
- `physical_backup` 负载 flaky 存量：与本 PR 改动零交集。
- 1.97.0 工具链 lint 预算基线录入：`--update` 是显式可复核动作，留给后续。
- 不新增任何 `check_*`，门禁分母仍 168；不动 `STAGE.yaml` 阶段。

## 3. 接口/行为变化

| 变化 | 性质 |
|---|---|
| `decode_lenenc_int` 0xfd 返回值 | **修复**：3 字节小端语义（旧值 ×256） |
| `VtuGuard::{in_transaction,current_tx_id,set_current_tx_id,set_current_tx_id_shared}` | **修复**：改为向 inner 透传（旧值为 trait 默认） |
| `GATE_CATALOG.jsonl` 的 `swallow_hits` | **对齐**：4→5（#5782 新增的合法 `\|\| true` 过滤管道） |
| 其余 30 处 | 语义不变：删死代码/恒真断言、加 allow、排版 |

## 4. 关联

- Issue #5362
- 看板行 `LINT-2`（本 PR）、`LINT-1`（批次 10 读数 62→0）
- 归档 run：`20261114-clippy2-002`（通过）/ `20261114-clippy2-001`（真红取证）
