# S2-DESIGN-INPUT — 独立契约测试运行域设计输入包（DSH-B 提供证据；Codex 主责设计）

- source_agent: DSH-B（MiniMax-M3.1-Flash 降级期）
- source_run: dshb-s2-design-input-20260721
- timestamp: 2026-11-21T04:20+08:00
- 依据：Owner 裁决「批准方案1——独立契约测试运行域，进入正式设计文档和实施计划阶段；本次仅批准设计，不批准代码实施/CI 修改/测试迁移/PR 合并」
- 角色边界：**Codex 主责架构设计；DSH-B 提供真实测试证据；DSH-A 核验执行与调度可行性**。本包**不含设计结论**，仅提供设计所需的事实与证据。

## 一、Owner §二 十项设计要求 → 已有证据映射表

| # | 设计要求 | 我方已备证据（可核验） | 缺口（需 Codex 设计） |
|---|---|---|---|
| 1 | 非默认 Cargo 契约测试入口 | 根 Cargo.toml 已有 `required-features` 用法（line 1773 `v313_3_profile`）；tests/integration/ 已分层（compat/dml/fault_injection/gate/int2/migration…） | 入口命名与目录约定、是否用 feature 门控（见 §三 风险 R1） |
| 2 | 常规 CI 与已知缺陷测试运行隔离 | CI 实测：`ci.yml:508` 根包 `cargo test --all-features --no-fail-fast`；`ci.yml:870` 成员包 `cargo test --workspace --exclude sqlrustgo`；触发分支不含 `dsh-b/*` | 隔离机制选择（独立 target 名 / feature / 独立 package） |
| 3 | 17 项 RED 身份级保护 | `RED-IDENTITY-RECONCILIATION.md`：六包 11 项本轮逐分支实跑（身份/源文件 sha256/失败归因/rc=101/Issue 五要素）；基线 5 项 + D-002b 1 项证据在档 | 身份级保护的技术载体（清单格式 + 校验入口） |
| 4 | 预期失败签名与实际失败归因 | 六包 11 项的实际失败断言消息已逐条提取（见 `RED-IDENTITY-RECONCILIATION.md` §二"失败归因"列，含完整断言文本片段） | 签名的规范化表示与匹配强度（子串/正则/结构化） |
| 5 | RED 转 GREEN 独立复验 | 现有流程证据：DSH-A 对 #5912/#5915 的独立复跑（MP-6 APPROVE，RED 逐条一致） | 转 GREEN 触发条件、复验者与证据格式 |
| 6 | 未执行/编译失败/超时/环境故障独立分类 | test_factory 七态已有：PASS/FAIL/KNOWN_FAIL/UNSUPPORTED/DIALECT_DIFFERENCE/NOT_RUN/INFRA_ERROR；`check_test_target_execution.sh` 判 "never executed" 为 FAIL | 四类故障在契约域的具体映射与独立状态位 |
| 7 | 测试正确性及反向变异 | 已有工具：`tools/verify_srg_d004_acceptance.py`（R4 MUTATION_RED 独立 git worktree 反向验证，实测可执行）；Task C 编译反馈循环实测（3 轮收敛） | 契约域通用变异策略（每包/每 RED 抽样比例） |
| 8 | G19 / test_count 单调基线 / 发布门禁兼容 | 实测：G19 registry total_allowed=135/limits.max=150，ACTIVE=81/active_max=92（**仅余 11**，故禁用批量 ignore）；test_count 单调基线 483/14345（develop tip 实测 487/14395） | 契约域目标是否计入 test_count（建议：独立计数口径，避免污染常规单调基线） |
| 9 | CHECK 与 UPDATE_BASELINE 职责分离 | **已确认治理风险（源码级）**：`check_test_count_monotonic.sh` 无条件重写基线（line 107 首次创建 / line 214 更新），无 `--check`/`--update` 模式；而仓库其他门禁存在 `--check` 只读约定（如 check_10_principles_v310.sh 等） | 分离方案（本次不实施） |
| 10 | 包级 ADR 绑定与 Manifest 重签关系 | 核实：DAG 18/18 包 `approved_adr_refs` 均为空（Owner 方向裁决已存在但包级绑定未闭环 ⇒ BLOCKED）；manifest 121/121 哈希复算一致（commit 45da6968ae / sha256 c722885f…） | 绑定写入格式与重签顺序（Codex 主责） |

## 二、真实测试证据（供设计直接消费）

### 2.1 六包 11 项 RED 身份记录（结构化）

```json
[{"test":"inv01a_rollback_restores_exact_multiset","target":"ca01_file_undo","sha256":"51e7c175d448f529","rc":101},
 {"test":"inv02_identical_rows_have_independent_rowids","target":"ca01_file_undo","sha256":"51e7c175d448f529","rc":101},
 {"test":"inv07a_uncommitted_invisible_to_other_engine","target":"ca01_snapshot_reopen","sha256":"0e18a9feb959f62c","rc":101},
 {"test":"inv07b_reopen_baseline_excludes_uncommitted","target":"ca01_snapshot_reopen","sha256":"0e18a9feb959f62c","rc":101},
 {"test":"c01_07a_commit_durability_without_explicit_flush","target":"ca01_commit_frontier","sha256":"9a2e3ebed83b5148","rc":101},
 {"test":"c01_09b_insufficient_capability_must_refuse_before_write","target":"ca01_backend_caps","sha256":"ed0f547132308922","rc":101},
 {"test":"inv05a_multi_row_update_rollback_is_all_or_nothing","target":"ca01_complex_dml","sha256":"9b42aa7f5793b232","rc":101},
 {"test":"inv05b_multi_table_dml_rollback_is_atomic","target":"ca01_complex_dml","sha256":"9b42aa7f5793b232","rc":101},
 {"test":"inv14_bp_delete_insert_undo_applied","target":"ca01_bufferpool","sha256":"68f6c11f09d7b15f","rc":101},
 {"test":"inv14_bp_adds_no_extra_transaction_semantics","target":"ca01_bufferpool","sha256":"68f6c11f09d7b15f","rc":101},
 {"test":"inv15_bp_and_plain_assembly_semantically_equal","target":"ca01_bufferpool","sha256":"68f6c11f09d7b15f","rc":101}]
```

### 2.2 五类测试语义的现状归属（Owner §一 要求区分）

| 类别 | 定义（Owner 口径） | 当前仓库实例 | 备注 |
|---|---|---|---|
| **Gap Probe** | 记录当前能力缺口 | `c01_07c_paired_frontier_observation_must_exist`、`c01_09a_capability_descriptor_must_exist_for_write_before_refuse`、`c01_09c_wrapper_capability_attestation_must_be_observable` | **Owner 裁决：这三个仍属 INEFFECTIVE_TEST**；其 PASS 仅表示"能力缺口仍存在"，**不得解释为目标契约满足**。保持可追溯，暂不由 DSH-B 再改，交 Codex 独立裁决 |
| **Contract Test** | 验证正确产品行为 | 17 项 RED 中的主体（如 `inv01a`/`inv07a`/`inv05a`/`inv14_bp_delete_insert`/`c01_07a`/`c01_09b`） | 断言方向=目标契约；当前实现不满足故 RED |
| **Regression Test** | 保护已有正确行为 | 六包中的 PASS 项（`inv01b`、`inv07c`、`inv15a`、`inv15b`、`inv14_bp_update_undo_applied`、`c01_07b`、`c01_09a/c` 的非 RED 部分） | 防止修复过程回退 |
| **Mutation Test** | 验证测试判别能力 | `tools/verify_srg_d004_acceptance.py` R4（独立 git worktree 反向）；Task C 编译反馈循环实测 | 契约域需通用化 |
| **Acceptance Test** | 验证工作包出口 | Codex DAG 的 `required_tests`（18 包，各带 negative_control） | 由 DAG 生成清单 |

## 三、风险清单（设计必须回答，实测证据支撑）

| # | 风险 | 实测证据 | 影响 |
|---|---|---|---|
| **R1** | **`--all-features` 会纳入 feature 门控目标** | `ci.yml:508` 根包跑 `--all-features`；Cargo.toml 已有 `required-features` 用法 | 若契约目标用 feature 门控，CI 仍会执行 ⇒ **隔离失效**。设计须避开此路径（独立 target 名 / 独立 package / CI 显式排除） |
| **R2** | `check_test_target_execution.sh` 判"声明但未执行"为 FAIL | 该脚本对 `required-features` **无专门处理**（grep 无命中） | 契约目标若声明在根 Cargo.toml，常规 CI 日志里不会出现 ⇒ 触发该门禁 FAIL；须设计其 scope 归属 |
| **R3** | G19 ignore 额度紧张 | ACTIVE 81/92（余 11）、total 135/150（余 15） | 禁用批量 ignore（Owner 令）⇒ 契约域不能用 ignore 表达"已知 RED" |
| **R4** | test_count 单调基线口径 | 常规 CI 目标数会因契约目标增加而抬高；门禁只要求单调不减 | 契约目标**不应**计入常规回归的 test_count 统计口径（否则掩盖真实增量） |
| **R5** | `check_test_count_monotonic.sh` 有写副作用 | 无条件重写基线（line 107/214），无 CHECK/UPDATE 模式 | 设计需假定基线可能在任何检查后被改写 ⇒ 契约域必须使用**只读**校验或先分离 CHECK/UPDATE |
| **R6** | 契约测试与实现合并时序 | 六包测试当前只在 `dsh-b/*` 分支存在；develop 上不存在 | 若实现先合、测试后合，中间期主干无缺陷固化资产；若测试先合，主干红。设计需明确合并顺序约束 |

## 四、复用约束（Owner 明令）

- 复用现有 **Test Factory**（七态分类 + 交叉校验 + raw 留档）、**DAG**（18 包 required_tests）、**RED Ledger**（17 项身份台账）——**不新建第二套权威账本**。
- 契约域的执行清单与预期失败签名应由 DAG/工作包**生成**，而非人工另建台账。
- 契约域不得成为绕过正式发布门禁的手段（Owner §三）；发布门禁仍须核对全部未关闭项。

## 五、DSH-B 承诺（边界）

- 本包**不含设计结论**——设计由 Codex 主责，可行性核验由 DSH-A 负责；
- S1 三个用例**不再自行修改**，保持可追溯，等 Codex 独立裁决；
- 本轮不改 CI、不改门禁、不改测试概念设计、不动核心事务实现、不合并 PR、不改 HUMAN_GATE；
- 其他已授权且满足门禁的任务继续自治推进。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
