# OWNER-RULING-EXECUTION-20260721 — 属主独立裁决执行记录

- source_agent: DSH-B（MiniMax-M3.1-Flash 降级期）
- source_run: dshb-owner-ruling-r2
- timestamp: 2026-11-21T03:55+08:00
- 依据：属主初步独立裁决（PR 暂不可合并；S1 三例 INEFFECTIVE_TEST；S2 采纳方案 1；后续工作限定 5 项）

## 一、属主指出的我的错误（已确认并接受）

| # | 属主指出 | 我的核实 | 处置 |
|---|---|---|---|
| E1 | 「16 项全部保护」是口径漂移，实际 **17 项** | ✅ 属实——台账合并行导致不可对齐；本轮已拆分独立行（#14/#15/#16），台账现 17 项 | 已改 + 逐项对账表 |
| E2 | S1 三例「恢复为有判别力」**仍不合格**（探针硬编码 None、不读产品 API/源码/行为 → 恒真） | ✅ **完全属实**——我的"判别力"说法不成立：`frontier_probe`/`wrapper_attestation` 均为局部变量 `None`，断言不触碰产品任何事实 | 已作废该提交（见 §二） |
| E3 | `check_test_count_monotonic.sh` 检查与授权更新职责未分离 = **已确认治理风险** | ✅ 属实（无条件写基线，line 107/214） | 已登记为治理风险 + 本轮基线已复原（sha256 三重 MATCH） |

## 二、S1 三项：INEFFECTIVE_TEST → 真替换（含判别力注入验证）

| 测试 | 作废的提交 | 真替换（当前） | 判别力注入验证 |
|---|---|---|---|
| `c01_07c` | ea3a611126（恒真探针） | **`c01_07c_paired_frontier_observation_must_exist`**（d0b1157b62）：读产品源码 `engine.rs`/`file_storage.rs`/`wal_storage.rs`，核对是否存在**成对** durable/commit frontier 观测入口 | 注入 `fn durable_lsn`+`fn commit_lsn` ⇒ **FAILED**；移除 ⇒ PASS ✅ |
| `c01_09a` | e5cc6b56d2（恒真探针） | **`c01_09a_capability_descriptor_must_exist_for_write_before_refuse`**（334ccf423a）：读 `engine.rs` 核对 StorageEngine 是否提供 `capabilities()` | 注入 `fn capabilities()` ⇒ **FAILED**；移除 ⇒ PASS ✅ |
| `c01_09c` | e5cc6b56d2（恒真探针） | **`c01_09c_wrapper_capability_attestation_must_be_observable`**（334ccf423a）：读 `parallel_wal_storage.rs` 核对组合态能力认证入口 | 同上机制 ✅ |

**判别力验证方法学**：每次替换均以「向产品源码注入对应 API → 期望该用例 FAILED → 移除 → 期望 PASS」双向实证，**产品源码零残留**（`git diff crates/` 为空，已逐次核验）。
**RED 不变**：C01-07 = 2P/1F、C01-09 = 2P/1F（与替换前一致）。
**历史保留**：作废提交仍在远端历史中（未 force-push 删除），可追溯。

### 2.1 本轮暴露的三个真实缺陷（我的测试设计缺陷，已修正）

1. **首次注入假阴性**：`begin_transaction` 实际签名为 `fn begin_transaction`（trait impl），我按先前 grep 的 `pub fn` 注入 → **注入从未写入**，"注入后仍 PASS"是假阴性。已修正注入点并重新验证。
2. **检查范围不足**：仅查两个实现文件，漏掉 trait 定义（`engine.rs`）→ 注入 trait 后仍 PASS。已扩大范围。
3. **既有观测入口误判**：`take_last_committed_log`（trait，line 2894）是**既有真实观测入口**，我原判据会把它误判为 frontier ⇒ 已加入显式排除规则（并区分 `wal_current_lsn`=下一个 append 的 LSN、`current_change_lsn`=changelog 游标）。

## 三、S2 方案 1（独立契约测试运行域）— 已形成正式设计输入，**未实施**

- 交付：`TEST_EXECUTION_CONTRACT_DECISION.md`（3 方案 + 已核实事实表 + 契约要素草案）
- 属主已选方案 1（独立、非默认 Cargo 入口；由现有 DAG/工作包生成执行清单与预期失败签名，**不另建第二权威账本**）
- **DSH-B 不自行实施**：属主要求"进入正式设计文档和实施计划阶段"需先确认 → 本轮仅备齐设计输入 + 已核实事实（CI 触发分支、target-execution 门禁、G19 额度、test_count 单调基线、ADR-008）

## 四、其余三项限定工作

| 项 | 状态 | 证据 |
|---|---|---|
| 替换三个恒真/空测试 | ✅ 完成 | §二（三例 + 注入验证） |
| 固化 17 项身份级台账 + 解释原"16 项"漂移 | ✅ 完成 | `RED-IDENTITY-RECONCILIATION.md`（六包 11 项本轮逐分支实跑，五要素齐备）；漂移原因＝台账合并行，已拆分 |
| 分离 CHECK 与显式 UPDATE_BASELINE | ⏳ **未实施**（需改门禁脚本＝治理变更，属 Codex/DSH-A 范围） | 已登记为已确认治理风险；本轮只做"不触发副作用"（隔离 worktree 执行 + 基线复原 + sha256 校验） |
| 按 Owner 固定裁决生成包级 ADR 绑定 + Manifest 重签 | ⏳ **未实施**（正式交接授权依据，DSH-A/B 不自行修改） | 已在 Bus 提请 Codex 在重签时写入 `approved_adr_refs` |

## 五、边界确认

- 未修改核心事务实现 ✅（`git diff crates/` 空）
- 未合并任何 PR ✅（PR #5912/5915/5918/5919/5920/5921 全部 OPEN）
- 未改变 HUMAN_GATE ✅（C-ALPHA-02~05 / M1-M6 / 合并 / 跃迁 / release 全部保留）
- 未新增 ignore / 未弱化断言 / 未删 RED ✅（17 项台账不变，`ignored` 计数恒为 0）
- 未自行实施 S2 运行域契约、未改门禁脚本、未改基线 SSOT ✅

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
