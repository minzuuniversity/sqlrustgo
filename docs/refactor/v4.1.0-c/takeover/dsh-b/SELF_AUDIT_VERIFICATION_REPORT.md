# SELF_AUDIT_VERIFICATION_REPORT — 降级期间执行矫正的逐项验证

- source_agent: DSH-B（MiniMax-M3.1-Flash 降级期）
- source_run: dshb-corrective-audit-20261121
- timestamp: 2026-11-21T04:45+08:00
- 指令依据：属主「模型降级期间的执行矫正指令」（SUSPECTED 登记制 / 证据判定 / 角色分离 / 16 项 RED 保护）

## 〇、判定口径（本次严格执行的规则）

- 每个结论必须附**实际命令 + 输出**；无运行证据者一律登记为 SUSPECTED / NOT_VERIFIED。
- **不以"发现更多问题"为目标**；以真实缺陷检出率、误报率、证据完整性、正确整改率为准。
- 区分三类角色：Developer（我）/ Reviewer（DSH-A 独立审核 + 本地执行器复核）/ Test Factory（确定性测试裁决）。

## 一、已确认违规（VERIFIED_DEFECT，已实施整改）

### V1 · PR 模板 5 类文档锚点缺失 → VERIFIED_DEFECT（已修复）
| PR | 整改前实测 | 整改后实测 |
|---|---|---|
| #5915 | anchors=0/5 | `✅ PASS: all 5 required sections are present` rc=0 |
| #5918 | anchors=0/5 | 同上 rc=0 |
| #5919 | anchors=0/5 | 同上 rc=0 |
| #5920 | anchors=0/5 | 同上 rc=0 |
| #5921 | anchors=0/5 | 同上 rc=0 |
（#5912 整改前即 5/5 PASS）
- 判定依据：仓库真实脚本 `scripts/gate/check_pr_template_sections.sh`（模板明载「无 = 自动 FAIL」）。
- 整改内容：5 类文档落在**契约允许路径** `docs/releases/v4.1.0/refactoring/evidence/ca01/<包>/` + PR body 锚点链接真实路径。
- 证据位置：各分支 commit（f1a90ef900 / 37b4483e48 / 3949d6c1be / bfafe98d8b …）+ PR body。

### V2 · Test Factory 零测试判 PASS（反例#1）→ VERIFIED_DEFECT（已修复，属前轮）
- 证据：8/8 回归 + 真实 cargo 冒烟（40.2s，exit 0 → `class=NOT_RUN`）。

## 二、已推翻的误判（DISPROVED —— 本轮自查推翻自己上一轮的部分结论）

### D1 · 「删除 3 个无断言测试 = AFP Type A 违规」→ **DISPROVED（定性无据）**
- 我的上一轮把它判为"AFP Type A 违规"并直接删除——**该定性没有工具依据**：
  - 实查仓库门禁全集：**不存在**任何"测试必须含断言"的自动检测机制（`grep` 全 `scripts/gate/*.sh` 无 assert-count/empty-test 判定）；
  - AFP Type A 的仓库定义指向**产品实现**空函数/默认返回值冒充实现（`docs/governance/ANTI_FABRICATION_POLICY.md`），未规定"测试函数必须含断言"；
  - 被删代码客观事实：2 个为空函数（`c01_09a`/`c01_09c`）、1 个为恒真断言（`c01_07c` 断言 `type_name::<FileStorage>()` 不含 `durable_lsn`——恒真，永不可能失败）。
- **当前定性：SUSPECTED（待 Codex 裁决）**，不作为已确认违规。
- **已实施的改动保留但登记为可逆**：删除 commit 已在远端（37b4483e48→e63d3c87d0、f1a90ef900→713eb8e880），缺口记录改写为注释（含"实施后应新增真实正向断言"）。**若 Codex 判定该删除不当，可按原 commit 恢复。**
- RED 未受影响（证据）：6 包整改后逐包实跑 RED 计数 = 2/2/1/1/2/3，与整改前完全一致。

### D2 · 「RED 测试进默认 `[[test]]` = 必然破坏 CI」→ **DISPROVED（范围性误判）**
- 实测：`.gitea/workflows/ci.yml` 与 `gate.yml` 的 `on.push.branches` 仅含 develop/main/develop/v3.x 等，**不含 `dsh-b/*`** → 功能分支上不触发 CI。
- 实测：`check_test_count_monotonic.sh` 在我的分支上 → `cargo=493(+10) active=14417(+72) ignored=135(未变)`，`✅ PASS`。
- 残余真实风险（仍成立）：若在架构修复前合并，根包 `cargo test --all-features` 会因这些 RED 变红 → 已写入各 PR ACCEPTANCE（合并时机须与实现同批）。
- 注：**门禁实跑副作用已回滚**——该脚本会自动改写 `tests/baseline/test_count.json`（基线 SSOT），两次运行后均已 `git checkout --` 复原（现值 483/14345/2026-11-07）。

## 三、尚未验证的怀疑（SUSPECTED / NOT_VERIFIED — 不作为已确认缺陷）

| # | 怀疑 | 当前状态 | 证据/缺口 |
|---|---|---|---|
| S1 | 3 个无断言测试是否确属 AFP 违规 | SUSPECTED | 无仓库门禁依据；定性权在 Codex |
| S2 | RED 契约测试的**运行域设计**（默认 `[[test]]` vs 独立运行域）是否需正式契约 | SUSPECTED（→ TEST_EXECUTION_CONTRACT_DECISION） | 已核实：CI 触发分支不含 dsh-b/*；G19 ignore_registry 额度 81/92 紧张且**业主禁批量 ignore**；测试清单 `test_count.json` 为单调基线（+10/+72 PASS）；ADR-008 测试声明透明度适用 |
| S3 | FileStorage buffer_pool 装配下 INSERT 撤销未生效 | **VERIFIED_DEFECT（已实证）** | c0111-final：PLAIN 对照正常、BUFPOOL 回滚后未提交行仍在；退出码 101 |
| S4 | buffer_pool 装配下 WHERE/PK 查询返回空 | **VERIFIED_DEFECT（已实证）** | 同上日志；PLAIN 正常 |
| S5 | 「committed 数据在无显式 flush 时重开丢失」 | NOT_VERIFIED（当前定为**契约差距基线**，非缺陷） | 属主已裁决 ADR-006 条件批准；durable decision/publish/ACK 补充契约冻结前不作缺陷定性 |
| S6 | INV-05 多行/跨表回滚双重撤销 | VERIFIED_DEFECT（已实证） | c0110：4→8 行、1→2 行，exit 101 |

## 四、已实施整改（逐条 + 证据）

| 整改 | 证据 |
|---|---|
| PR 锚点（5 PR）+ 5 类文档落契约允许路径 | 门禁脚本 5/5 `✅ PASS` rc=0（见 §一） |
| 3 个无断言测试删除（**定性待 Codex**，可逆） | commits e63d3c87d0 / 713eb8e880；RED 计数不变（实跑复核） |
| 基线 SSOT 回滚 | `tests/baseline/test_count.json` 复原 483/14345/ts 2026-11-07 |
| 越界文件清理 | `examples/bp_probe.rs`、`tests/bp_probe.rs` 已删未提交 |
| 5 类文档内容 | 全部来自真实执行日志（artifacts/*.log），非模板填充 |

## 五、不变量专项复查（仅列有运行证据者）

| 不变量/门禁 | 结论 | 证据 |
|---|---|---|
| 产品源码零改动 | ✅ 未违反 | 5 分支 `git diff e9ff21fca3 -- src/ crates/` 全空 |
| 断言方向=契约而非现状 | ✅ 未违反 | 提取全部 21 条断言主张逐一比对，均为"目标契约"表述（如"must restore the exact pre-tx multiset"），无"断言当前错误值" |
| 允许路径 | ✅ 未违反 | 新增仅 `tests/integration/test_factory/` + `evidence/ca01/<包>/` + Cargo.toml（`[[test]]` 声明必需） |
| 阶段/HUMAN_GATE | ✅ 未违反 | 5 包 `human_gate.required=False`（包内），但**实现修复仍属属主/Codex 决策**；未合并任何 PR |
| ADR 引用状态 | ⚠️ 需登记 | 5 包 `approved_adr_refs=[]`、`proposed_adr_refs=10 份`；属主集中裁决已给方向，但**包级 ADR 绑定尚未写入 DAG** → 见 §六 |
| ignore 额度 | ✅ 未消耗 | 实跑 G19 计数：`ignored=135` 未变（基线亦 135） |
| 16 项 RED | ✅ 全部保留 | 台账未删改；6 包实跑 RED 计数逐包复核一致 |

## 六、待 Codex 裁决事项（我不自行决定）

1. **S1 定性**：3 个无断言测试（2 空函数 + 1 恒真断言）删除是否恰当？是否属 AFP Type A？可否按原 commit 恢复？（现删除为可逆状态）
2. **S2 TEST_EXECUTION_CONTRACT_DECISION**：C-ALPHA-01 契约测试的运行域设计——是否应作为**独立运行域**（不进入默认 `[[test]]` 集合，避免与"架构修复前合并"耦合），由 Codex 出契约并授权实施？（依据已备：CI 触发分支、G19 额度、test_count 单调基线、ADR-008）
3. **包级 ADR 绑定**：属主集中裁决已给 ADR-001~011 方向，但 DAG 中 `approved_adr_refs` 仍为空——是否需要 Codex 在重签 manifest 时一并写入绑定（避免"方向已批但包内无绑定"的证据链断裂）？

## 七、角色分离声明（指令 §5）

- 本轮全部判定基于**确定性命令输出**（门禁脚本 rc、cargo 退出码、测试计数、git diff），未使用模型主观判断作结论。
- 我未对任何缺陷做"定性+修复+验收"三重角色：修复实施仅限**测试/文档层**（不触碰产品实现），产品层缺陷定性归 Codex/独立 Reviewer。
- 上一轮"删除 3 个无断言测试"是我同时扮演了定性者与实施者——本轮已按 SUSPECTED 重新登记并交 Codex 裁决，不再声称其为已确认违规。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
