# D-001~004 缺陷与回归验收包（实测状态 · 2026-11-20 R3 更新）

- source_run: dshb-first-round-20261120-4 / goal-R3
- 前提：四项为候选缺陷；按 DSH-B 纪律以"最新代码实跑"逐项确认。**已 2/4 RED 实证，1 项在 proto 分支已局部修复（未合并），1 项待跑。**

## 实测状态总表（base e9ff21fca3 = develop tip）

| 缺陷 | RED 实测 | 证据 | 备注 |
|---|---|---|---|
| D-001 BinaryStorage 桩静默成功 | ✅ **RED**（4/4 FAILED，CARGO_EXIT=101） | `dsh-b/artifacts/d001_20261120T165449.log`；pilot `evidence/D001-red-run-20261120T165449.log` | insert Ok 但 scan 0 行；delete/update 返回 0/静默 |
| D-002 ODKU 回滚无效撤销 | ✅ **RED on base**（CARGO_EXIT=101）| `artifacts/d002-base-adapted-*.log` | ⚠️ p2b proto 分支已局部修复（`wt-p2b-undo` tip 同测试 ok）——修复未合并 develop，属"已修未收"状态 |
| D-002b 脏读（隔离违反 RC）| ✅ **RED on base**（CARGO_EXIT=101）| `artifacts/dirtyread-base-*.log` | 另一连接读到未提交行（uncommitted_seen=2 vs 1）；R3 新增，独立于 D-002 |
| D-003 kill-9 崩溃恢复 | ⬜ 待跑 | — | 需确定性崩溃注入脚本 |
| D-004 checkpoint 撕裂写 | ⬜ 待跑 | — | 同上 |

## RED 执行明细

- D-001：`cargo test -p sqlrustgo-storage --test tf_t4_stub_silent_success` → 4 用例全失败（insert 后 scan 0 行 / delete 返回 0 / update 返回 0 / 事务方法静默成功）。首次运行退出码误取管道 tail 的 0，已用直接重定向重跑取 101。
- D-002（基线适配）：探针文件来自 `wt-p2b-undo`（分支含未合并局部修复 d794b83a43）；基线 e9ff21fca3 缺 `undo_all` API → 仅摘除依赖分支 API 的 `p2c0_d5` 用例后可编译；`p2c1id_odku_rollback_restores_the_old_value --ignored` → **FAILED**（"ON DUPLICATE KEY UPDATE was not undone"）。
- D-002b：`p2c1iso_dirty_read_and_commit_visibility --ignored` → **FAILED**（PROBE isolation: uncommitted_seen=2 after_commit_seen=2）。

## Task D 独立审查附带发现（8021 审 252 的候选测试）

- **HIGH**：候选 update 测试只断言返回计数、未 scan 验证落盘 → 桩实现可假阴（漏检）；delete 用例有 scan 校验而 update 没有（不对称覆盖）。
- MED：事务方法 panic-vs-Err 需区分；未用 trait 导入。
- 结论：过程独立审查有效（检出真实高危缺口）；8021 与 252 同血统，仍需异血统终审（8011/属主/Codex）。

## 权限边界（不变）

C-ALPHA-01 授权面 = 准备/测试/只读 → 修复实施需属主逐项批准；D-002 的 p2b 分支修复是否合并 develop 属属主/PR 流程决策。

## 统一验收流程（属主指令 §四）

最小复现 → RED 失败测试 → 根因分析 → 最小修复方案 → GREEN 回归 → 反向验证（MUTATION_RED 独立副本）→ 独立验收（Reviewer ≠ Developer）。

## D-001? BinaryStorage 名义事务

- 现象（试点/HARA）：BinaryTableStorageV2 事务接口名义存在、语义缺失；`tf_t4_04_non_tx_backend_must_reject_or_document` 断言非事务后端必须拒绝或显式文档化。
- 计划：RED 测试 = 跨 backend 契约测试（begin/commit/rollback × BinaryStorage）→ 预期失败或文档缺失 → 根因 = 事务接口未实现路径 → 修复候选：reject-or-document 二选一（属主裁决语义承诺）。
- 状态：**NOT RUN**。

## D-002? ODKU 回滚无效撤销（undo 族）

- 现象：存储 `update` 的静默无效撤销——ODKU（ON DUPLICATE KEY UPDATE）回滚不生效；a802a0c4b5 已捕获探针测试（worktree 分支）。
- 计划：以 `wt-p2a-undo`/`wt-p2b-undo` 探针为基础固化最小复现 → RED → 根因（slot 身份定位）→ 与 #5905 修复路径合并。
- 状态：**NOT RUN（有探针 commit 证据）**。

## D-003? kill -9 崩溃恢复

- 现象：FileStorage+WAL kill-9 后恢复语义未验证（试点 T6-001 特殊场景）。
- 计划：确定性崩溃注入脚本（kill -9 于 WAL append 窗口）→ 重启 → 对账（committed 集合 == durable frontier）→ RED/GREEN。
- 状态：**NOT RUN**。

## D-004? Checkpoint 撕裂写

- 现象：checkpoint 写入中途崩溃产生撕裂页（试点 T6-002 特殊场景）。
- 计划：撕裂写注入（截断 checkpoint 文件）→ 恢复路径行为断言 → RED/GREEN。
- 状态：**NOT RUN**。

## 权限边界（属主令）

若现有权限只允许候选 Issue 和测试设计，则不进入真实代码修改。当前 C-ALPHA-01 授权面 = 准备/测试/只读 → 四项产出物上限为：最小复现脚本 + RED 测试 + 根因分析 + Issue 候选；**修复实施需属主逐项批准**（或包含在已批准 DAG 工作包内时按包执行）。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
