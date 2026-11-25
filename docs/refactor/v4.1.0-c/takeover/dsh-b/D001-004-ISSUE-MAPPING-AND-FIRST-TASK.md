# D-001~004 ISSUE 映射 · 首个重构 Task 候选 · DSH-A 接入包

- source_agent: DSH-B (opencode-go/glm-5.3-flash)
- source_run: dshb-next-phase-20261120-8
- timestamp: 2026-11-20T17:31:03+08:00 起本轮
- 上游：DSH-A `docs/test-factory/TEST_FACTORY_NEXT_PHASE_DECISION.md`（D-001~004 CONFIRMED，连续 3 次稳定复现）+ DSH-B RED 实测（本目录 D001-004-DEFECT-ACCEPTANCE-PACK.md）

## 一、四个缺陷的实际 Issue 映射（防重复建单核查已做）

重复核查：104 条 open Issue 快照（2026-11-20T06:54Z，Codex evidence）关键词扫描——**无既有单覆盖"桩静默成功"族**。#5911/#5910/#5905/#5906/#5909 为 FileStorage/引擎 undo 族，与本族（BinaryTableStorageV2 能力边界）不同缺陷。adj/owner-5601-binary-mode.out 为空文件（0 字节），5601 不在 open 快照中，标注"疑似相关待核实"。

| DSH-A 编号 | 缺陷（三连稳定复现） | 固化测试 | DSH-B RED 证据 | **建议 Issue 候选**（未创建，待属主） |
|---|---|---|---|---|
| D-001 | `scan` 恒空（insert Ok 但读 0 行） | tf_t4_01 | 4/4 FAILED, exit 101（2026-11-20 16:54） | [候选][P1][存储] BinaryTableStorageV2 scan 静默空——调用方可静默丢全部读数据 |
| D-002 | `delete` 恒零（Ok(0) 但行存在） | tf_t4_02 | 同上 | 同上合并：delete/update 静默零计数 |
| D-003 | `update` 恒零 | tf_t4_03 | 同上 | 合并同上 |
| D-004 | `begin_transaction` 未拒绝（Ok(0)） | tf_t4_04 | 同上 | [候选][P1][存储] 非事务后端必须显式拒绝（UNSUPPORTED）而非静默成功 |

建议建单形态：**一张族 Issue（覆盖 D-001~003，同一根因：桩方法返回 Ok）+ 一张 D-004（独立修复项，即本 Task 候选）**，互相关联。#5478（故障注入进程级）与本族无关但同文件域，提交时引用。

## 二、Test Factory 资产复用清单（不重复建设）

| 资产 | 位置 | 复用方式 |
|---|---|---|
| test_factory.py（七态分类+交叉校验+--test+raw 留档） | srg-test-factory-pilot/tools/（DSH-B 修复版，8/8 回归绿） | RED/GREEN/MUTATION 执行器 |
| tf_t4_stub_silent_success.rs（人工 RED 测试） | crates/storage/tests/ | D-001~004 固化判据 |
| tf_t4_ai_candidate_v1.rs（AI 产出同效测试） | 同上 | 执行器产出质量对照 |
| tf_t1_undo_invariants.rs（不变量测试，4/4） | tests/integration/test_factory/ | C-ALPHA-01 契约测试基础 |
| 75 场景复跑映射（case-mapping + rerun-summary） | evidence/ | 重构回归基线（70 PASS 冻结基线） |
| p2c0_undo_proto.rs（undo/隔离探针，基线 RED×2） | wt-p2b-undo（分支） | D-002 undo 族判据（修复合并后应转绿） |
| DSH-A handoff.py（14 字段契约） | scheduler/handoff.py | Task 提交通道（本文件 §四） |
| fault-injection 服务器二进制构建法 | `cargo build -p sqlrustgo-mysql-server --all-features` | 崩溃注入测试前置 |

## 三、首个本地重构 Task 候选：**D-004 非事务后端显式拒绝**

选择理由（按指令"验收清晰、变更范围最小"）：DSH-A 决策文档唯一标记"✅ 可交本地 AI、单函数、有测试、无契约变更"的项；RED 已三连稳定复现 + DSH-B 独立复测；GREEN 判据纯确定性。

**结构化定义（同时满足 Codex 04_TASK_DAG 规范与 DSH-A 14 字段）：**

- task_id：`SRG-D004-01`（Issue 候选创建后改为 SRG-<issue>-01）
- objective（原文，本地保存）：BinaryTableStorageV2 的 `begin_transaction/commit_transaction/rollback_transaction` 三个方法当前返回 `Ok(0)/Ok(())`（binary_storage_v2.rs:771-780），对不支持事务的后端构成"静默成功"。修复为显式返回 `Err(SqlError)`，错误信息须含 "UNSUPPORTED"/"not support"（与 tf_t4_04 断言一致）。commit/rollback 在无活动事务时同样返回 Err（幂等拒绝）。
- 验收条件（RED→GREEN→反向）：
  1. **RED（已成立）**：`cargo test -p sqlrustgo-storage --test tf_t4_stub_silent_success` 当前 4 FAILED（tf_t4_04 断言 UNSUPPORTED_NOT_REJECTED）
  2. **GREEN**：修复后 tf_t4_04 **PASS** 且 tf_t4_01/02/03 仍 FAILED（能力边界之外的桩行为不在本任务范围——防止顺手扩大修复面）
  3. **回归**：`test_factory.py run --package sqlrustgo-storage`（lib 全量）无新增 FAIL；75 场景基线 70 PASS 不回退
  4. **MUTATION_RED**：在**独立副本**恢复 Ok(0) 实现 → tf_t4_04 再次 FAILED（反向验证测试有效性）
- allowed_paths：`crates/storage/src/binary_storage_v2.rs`、`crates/storage/tests/tf_t4_stub_silent_success.rs`（只读复用）
- forbidden_paths：`src/`、`crates/transaction/`、`crates/mysql-server/`、`scripts/gate/`、`docs/governance/`、`docs/releases/v4.1.0/STAGE.yaml`、`.gitea/`、`aoci*.txt`、`.aoci/`、`.env`、`secrets/`
- base_sha：`e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0`（隔离 worktree，不动现有工作树）
- risk：L1（单文件 ~3 函数、无契约变更；**"拒绝"语义本身是既定方向**——DSH-A 决策文档与 Codex TEST_FACTORY 设计一致，属主可否决）
- budget：252:8001 本地模型；≤100k tokens（DSH-A N5 口径）；runtime ≤1800s；attempts ≤2
- 独立验证：Developer=252 产出 patch；Reviewer≠Developer（8021 过程独立 + 8011 异血统终审）；测试执行=test_factory 确定性判据；Receipt 走 DSH-A
- 验收自动化（R8 新增）：`srg-test-factory-pilot/tools/verify_srg_d004_acceptance.py`——`--phase pre`（RED 成立）/ `--phase post`（GREEN+不扩大修复面+回归+MUTATION_RED 独立 worktree 反向）一键四判据，fail-closed；**pre 阶段已实测通过（tf_t4_04=FAILED，exit 0）**，post 阶段待修复落地即跑。
- 隔离 worktree 已预创建：`/tmp/srg-d004-task` @ e9ff21fca3（T4 步骤就绪）。
- human_gate：属主批准 Task 派发（授权数当前=0）

## 四、DSH-A 接入与资源申请

**接入载荷（已过 handoff.validate strict，18 包 + 本候选格式同源）：**

```json
{"task_id":"SRG-D004-01","project":"sqlrustgo","objective_digest":"<sha256(上述 objective 原文)>","acceptance_count":4,"acceptance_digest":"<sha256(验收 1-4 拼接)>","allowed_paths_glob":["crates/storage/src/binary_storage_v2.rs"],"forbidden_paths_glob":["src/","crates/transaction/","crates/mysql-server/","scripts/gate/","docs/governance/","docs/releases/v4.1.0/STAGE.yaml",".gitea/","aoci.txt","aoci.code.txt",".aoci/",".env","secrets/"],"risk":"L1","file_count":1,"line_stats":{"added":9,"removed":3},"base_sha":"e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0","execution_mode":"LOCAL_ONLY","budget_seconds":1800,"budget_attempts":2}
```

**资源申请（GMP 零占用）：** 252:8001 Strata（Developer，本地 0 成本）×1 并发；8021（Reviewer）×1 错峰；8011（终审）×1 按需；本地磁盘 <50MB（worktree+target 增量）；无 GPU 独占请求。

## 五、共同验证状态（Test Factory ↔ 公共执行器）

- ✅ 载荷层：handoff.validate strict 18/18 + 本候选格式同源通过
- ⏳ 执行层：DSH-A worker 实际跑一个 Task 的端到端演练（建议以 SRG-D004-01 为演练对象，属主批准后执行）——需 DSH-A worker 环境白名单确认能执行 `cargo test`（其 17:37 commit 已实测 OMP 真实工具执行 16.8s 隔离生效，预期可行）
- ⏳ 证据层：Run→Receipt→Gate 回路在公共执行器上首验

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
