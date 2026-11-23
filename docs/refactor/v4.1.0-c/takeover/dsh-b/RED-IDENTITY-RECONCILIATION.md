# RED-IDENTITY-RECONCILIATION — 16 项 RED 逐项对账（2026-11-21 收敛指令 §五）

- source_agent: DSH-B（MiniMax-M3.1-Flash 降级期）
- source_run: dshb-red-reconciliation-20260721
- timestamp: 2026-11-21T03:35+08:00
- 方法：**逐分支实跑 + 源文件 sha256 复算 + 失败归因提取**（非引用历史叙述）；基线 SHA 全部为 `e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0`（develop tip，无漂移）

## 一、映射总览（关键结论）

| 分组 | 项数 | 来源 | 实跑状态 |
|---|---|---|---|
| 基线 5 项 | 5 | 试点 75 场景（tf_t4_01..04、odku） | 见 §二（部分在本轮之前已实跑，证据归档） |
| **六包 11 项** | **11** | C01-05/06/07/09/10/11 契约测试 | ✅ **本轮逐分支实跑，身份/哈希/归因/rc 全部实测** |
| 额外 1 项 | 1 | D-002b 脏读（wt-p2b 分支探针，不在六包内） | 证据归档于 `artifacts/d002base*`/`dirt-*` |
| **合计** | **17** | | 台账已同步为 17 行 |

> ⚠️ 修正：上一轮台账将 `inv15_bp_and_plain` 与 `inv14_bp_delete_insert` 合并为一行，导致「16 项」与「六包 11 项」数量不可直接对齐。**本轮已拆分为独立行**（#14/#15/#16），映射现为一一对应。**不以数量相等判定保护通过**——以下逐项给出身份与证据。

## 二、六包 11 项逐项对账（本轮实跑数据）

| # | 测试身份 | 包/目标 | 源文件 sha256(前16) | rc | 失败归因（断言消息） | Issue 映射 |
|---|---|---|---|---|---|---|
| 6 | `inv01a_rollback_restores_exact_multiset` | C01-05 / ca01_file_undo | `51e7c175d448f529` | 101 | INV-01: rollback must restore the exact pre-tx multiset | #5905/#5909 族 |
| 7 | `inv02_identical_rows_have_independent_rowids` | C01-05 / ca01_file_undo | `51e7c175d448f529` | 101 | INV-02: identical rows must be independently addressable; rollback must restore (see #5908) | **#5908** |
| 8 | `inv07a_uncommitted_invisible_to_other_engine` | C01-06 / ca01_snapshot_reopen | `0e18a9feb959f62c` | 101 | INV-07: uncommitted row invisible to engine B | p2c1iso 同源 |
| 9 | `inv07b_reopen_baseline_excludes_uncommitted` | C01-06 / ca01_snapshot_reopen | `0e18a9feb959f62c` | 101 | INV-07: recovery baseline must exclude uncommitted (2,20) | #5909 裁决材料 |
| 10 | `c01_07a_commit_durability_without_explicit_flush` | C01-07 / ca01_commit_frontier | `9a2e3ebed83b5148` | 101 | panic@ca01_commit_frontier.rs（ACK 后重开数据丢失） | ADR-006（M-Pre 证据） |
| 11 | `c01_09b_insufficient_capability_must_refuse_before_write` | C01-09 / ca01_backend_caps | `ed0f547132308922` | 101 | panic@ca01_backend_caps.rs（能力不足未写前拒绝） | **#5914**（与 #5913 同族） |
| 12 | `inv05a_multi_row_update_rollback_is_all_or_nothing` | C01-10 / ca01_complex_dml | `9b42aa7f5793b232` | 101 | INV-05: one statement affecting multiple rows must be undone atomically (all rows restored, not partially) | #5905/#5909 族 |
| 13 | `inv05b_multi_table_dml_rollback_is_atomic` | C01-10 / ca01_complex_dml | `9b42aa7f5793b232` | 101 | INV-05: table a must be fully restored | #5905/#5909 族 |
| 14 | `inv14_bp_delete_insert_undo_applied` | C01-11 / ca01_bufferpool | `68f6c11f09d7b15f` | 101 | INV-14: buffer_pool assembly must restore deleted and remove inserted rows on rollback | 业主点名问题 |
| 15 | `inv15_bp_and_plain_assembly_semantically_equal` | C01-11 / ca01_bufferpool | `68f6c11f09d7b15f` | 101 | INV-15: buffer_pool and plain assembly must expose identical visible state | ADR-009 |
| 16 | `inv14_bp_adds_no_extra_transaction_semantics` | C01-11 / ca01_bufferpool | `68f6c11f09d7b15f` | 101 | INV-14: committed value must be immediately visible through buffer_pool assembly | ADR-009 |

**逐包汇总（rc=101，test result 行原文）**
- C01-05/06 ca01_file_undo: `FAILED. 1 passed; 2 failed; 0 ignored` ｜ ca01_snapshot_reopen: `FAILED. 1 passed; 2 failed; 0 ignored`
- C01-07 ca01_commit_frontier: `FAILED. 1 passed; 1 failed; 0 ignored`
- C01-09 ca01_backend_caps: `FAILED. 0 passed; 1 failed; 0 ignored`
- C01-10 ca01_complex_dml: `FAILED. 2 passed; 2 failed; 0 ignored`
- C01-11 ca01_bufferpool: `FAILED. 1 passed; 3 failed; 0 ignored`
- **`ignored` 计数全为 0** ✅（未用 skip/ignore 掩盖）

## 三、基线 5 项与额外项（前序实跑证据，本轮未重跑）

| # | 身份 | 证据 | Issue |
|---|---|---|---|
| 1-3 | `tf_t4_01/02/03`（桩静默成功） | `artifacts/d001_20261120T165449.log`（4/4 FAILED, rc=101） | **#5913** |
| 4 | `tf_t4_04`（begin_transaction 未拒绝） | 同上 | **#5914** |
| 5 | `PILOT-T5-101` odku-rollback-noop | `artifacts/d002-base-adapted-*.log`（rc=101，基线适配后） | #5905 族 |
| 17 | D-002b 脏读（双引擎） | `artifacts/dirtyread-base-*.log`（rc=101） | #5909 族 |

## 四、保护判定（不以数量相等为准）

判定依据为**逐项身份 + 源文件哈希 + 实跑 rc + 失败归因 + Issue 映射**五要素齐备：
- 六包 11 项：五要素全部实测齐备 ✅
- 基线 5 + 额外 1：证据文件在档（前序实跑）✅
- **合计 17 项，语义与断言零改动**（本轮仅拆分行结构，未改任何断言文本）
- ignore 计数 **135 未变**（基线亦 135）→ 未以 skip/ignore 掩盖任何 RED ✅

## 五、遗留（须 Codex/属主确认）

1. 六包测试在 develop 基线上不存在（仅在 DSH-B 分支）→ 「与 75 场景基线同源」的合并后基线尚未形成（待 S2 运行域契约冻结后随实现同批合并）。
2. 5 基线项来自 pilot 工作树（`srg-test-factory-pilot`），非 develop 树 → 其 Issue 关联已建，但测试身份尚未纳入 develop 仓（属 C01-04/15 范畴）。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
