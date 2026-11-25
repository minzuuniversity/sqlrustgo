# TEST_EXECUTION_CONTRACT_DECISION（请求 Codex 裁决）

- source_agent: DSH-B（MiniMax-M3.1-Flash 降级期）
- source_run: dshb-corrective-audit-20260721
- timestamp: 2026-11-21T03:50+08:00
- 依据：属主收敛指令 §二（原则采用独立、显式、持续执行的契约测试运行域；Codex 制定正式方案）
- 状态：**请求裁决**（DSH-B 不自行实施运行域契约变更）

## 一、已核实的事实（全部实跑/实查，供 Codex 决策）

| 事实 | 证据 |
|---|---|
| CI 触发分支不含 `dsh-b/*` | `.gitea/workflows/ci.yml`、`gate.yml` 的 `on.push.branches` 仅 develop/main/develop/v3.x 等 |
| 常规 CI 跑根包全量 | `ci.yml:508` `cargo test --all-features --no-fail-fast`（根包）→ 含根包 `[[test]]` 目标 |
| 成员包单独 job | `ci.yml:870` `cargo test --workspace --exclude sqlrustgo` |
| 测试目标执行门禁 | `scripts/gate/check_test_target_execution.sh`：声明的 `[[test]]` 若未出现在执行日志中 ⇒ **FAIL**（"never executed"） |
| G19 ignore 额度 | `tests/baseline/ignore_registry.json`：total_allowed=135 / limits.max=150；ACTIVE=81 / active_max=92 |
| ignore 计数规则 | 同上 `counting_rule`：仅统计真实 `#[ignore]` 属性（注释/字符串中的不算） |
| 测试计数单调基线 | `tests/baseline/test_count.json`：cargo_tests=483 / active_tests=14345（**当前 develop tip 实测 487/14395**） |
| ADR-008 | 测试声明透明度：SKIP/TIMEOUT/XFAIL 不得计 PASS（`docs/governance/adr/ADR-008-test-claim-transparency.md`） |

## 二、当前冲突（真实存在，需契约裁决）

**冲突本体**：C-ALPHA-01 契约测试（当前 6 包 11 项 RED）若作为**默认 `[[test]]` 目标**登记在根包：

1. **合并前**：功能分支不触发 CI（已核实）→ 不影响受保护分支；
2. **合并后（架构修复落地前）**：根包 `cargo test --all-features` 将因 11 项 RED 变红 ⇒ **常规回归 CI 与"已知缺陷契约测试"语义耦合**；
3. `check_test_target_execution.sh` 要求声明的 target 必须被执行 ⇒ 若把契约测试移出默认集合，须确认该门禁的 scope/声明口径不产生"声明缺失"或"永不执行"误判；
4. **不得用 `#[ignore]` 规避**：业主明令禁止批量 ignore；且 G19 ACTIVE 额度仅余 11（81/92），total 仅余 15。

## 三、请 Codex 裁决的方案要点（供选择，非我的推荐结论）

### 方案 A · 独立、显式、持续执行的契约测试运行域（属主原则倾向）
- 契约测试作为**独立 target 集合**（独立 Cargo target 名或独立 test 目录 + 显式运行入口），**不进入**常规回归的默认集合；
- 每次运行产出**机器可读 RED 报告**（test_id / 源文件 sha256 / 预期失败签名 / 实际退出码 / Issue 映射 / base_sha）；
- **预期失败签名**机制：每个 RED 登记其期望失败特征（断言消息要点 + 退出码 101）；**非预期失败不得被接受为预期 RED**（签名不匹配 ⇒ 判为 NEW_BREAKING，独立告警）；
- **RED→GREEN 自动触发独立复验**（转绿后须独立 Reviewer 复跑确认，并核验无其他 RED 恶化）；
- 正式发布门禁**不得绕过未解决缺陷**（发布前须核对 RED 清单与 Issue 状态一致）。
- 需 Codex 明确：与 `check_test_target_execution.sh`（scope=all/root/crates）和 `test_count.json` 单调基线的对账口径。

### 方案 B · 保持默认集合 + 显式"已知缺陷契约测试"标记文件
- 保留默认 `[[test]]` + 一份 SSOT 标记文件列出预期 RED 项；
- 常规 CI 读取该文件，把匹配项记为 EXPECTED_RED（不计 PASS、不掩盖非预期失败）；
- 需 Codex 明确：该标记文件与 ADR-008「SKIP/XFAIL 不计 PASS」如何共存（EXPECTED_RED 是否被视作 XFAIL 类别）。

### 方案 C · 随实现同批合并（最简，但阻塞 M0）
- 契约测试不进主干，等对应修复实现同批合入 → RED 与 GREEN 同树；
- 代价：C-ALPHA-01 无法提供"缺陷已固化"的持续证据（RED 台账依赖实时复跑）。

## 四、DSH-B 已做与不做

- ✅ 已做（低风险确定性）：RED 逐项对账（身份/哈希/归因/rc/Issue 五要素）、SSOT 副作用核查与完整哈希校验、S1 三处无效测试恢复为判别力测试、PR 锚点整改（5/5 门禁 PASS）。
- ❌ 不做：任何运行域/门禁/CI 结构变更、任何 `#[ignore]` 批量添加、任何断言弱化、任何测试删除（除经裁决的替换）、任何基线 SSOT 写入。

## 五、附：本次已产出的运行契约要素（草案，供 Codex 精化）

每个 RED 需具备（属主 §二要求）：
- 身份：测试函数全名 + 所属包/目标
- 预期失败签名：断言消息关键片段 + 退出码（101）
- Issue 映射：#5913 / #5914 / #5905 族 / #5908 / #5909 族 / ADR-006 / ADR-009 / 业主点名问题
- 基线 SHA：e9ff21fca3（当前 develop tip）
- 源文件哈希：见 RED-IDENTITY-RECONCILIATION.md（六包 sha256 已实测登记）

**实测登记见**：`takeover/dsh-b/RED-IDENTITY-RECONCILIATION.md`（六包 11 项逐项 + 源文件 sha256 + 失败归因 + rc）。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
