# SELF-AUDIT-20261121 — DSH-B 产出复查报告（不变量与概念设计合规）

- source_agent: DSH-B
- source_run: dshb-autonomy-r11-audit
- timestamp: 2026-11-21T04:10+08:00
- 触发：属主「再次进行复查和确认，是否违反了不变量和概念设计」
- 方法：先找问题（不预设结论）；用仓库**真实门禁脚本**判定，不用自述

## 一、发现的问题（3 项真实违规 + 4 项风险澄清）

### ❌ 违规 1：5 个 PR 缺 5 类文档锚点（硬门禁必然 FAIL）
- 证据：`scripts/gate/check_pr_template_sections.sh` 按 PR body 中的 `<!-- section:NAME -->` 锚点判定，PR 模板明载「无 = 自动 FAIL」。
- 实测：PR #5915/#5918/#5919/#5920/#5921 锚点数 = **0/5**（仅 #5912 达标）。
- 整改：①在**契约允许路径**内生成 5 类文档（`docs/releases/v4.1.0/refactoring/evidence/ca01/<包>/`）；②PR body 加锚点并链接真实路径。
- **验证**：用仓库门禁脚本逐个判定 → 5/5 `✅ PASS: all 5 required sections are present`（exit 0）。

### ❌ 违规 2：3 个"永不可能失败"的无实质断言测试（AFP Type A 风险）
- `ca01_commit_frontier.rs::c01_07c`（仅断言类型名不含 `durable_lsn`）
- `ca01_backend_caps.rs::c01_09a` / `c01_09c`（**空函数**，零断言）
- 定性：空实现/占位断言冒充覆盖 = AFP Type A（AGENTS §8 禁止项 1）。
- 整改：删除这 3 个测试，缺口记录改写为**注释**（含"实施后应新增真实正向断言"的要求）。
- **RED 未受影响**：6 包整改后逐包复核 RED 计数 = **2/2/1/1/2/3，与整改前完全一致**（✅ 全部匹配）；删除的仅是永不可能失败的占位测试。

### ⚠️ 风险 3：RED 测试进入默认 `[[test]]` 目标可能使 CI 变红 → 澄清并记录，不掩盖
- 事实：本包 `[[test]]` 目标只在功能分支存在；**CI 触发分支为 develop/main 等，不含 `dsh-b/*`**，故**不影响受保护分支门禁**（实测 `.gitea/workflows/ci.yml` 与 `gate.yml` 的 `on.push.branches`）。
- 但：**若在架构修复落地前合并**，根包 `cargo test --all-features` 会因这些 RED 变红。
- 决策：**不采用 `#[ignore]` 方案**——业主明令「禁止通过删除、跳过或弱化断言消除 RED」，且项目 `ignore_registry.json`（G19 SSOT）额度紧张（ACTIVE 81/92）；加 ignore 属"跳过"且消费他方额度。
- 结论：合并时机须与对应实现同批（属主/Codex 决策）——已写入每个 PR 的 ACCEPTANCE 与本报告。

## 二、四项风险澄清（经核查**未**违规）

| 检查 | 结论 | 依据 |
|---|---|---|
| 是否改写 Codex manifest | ✅ 未 | `git diff e9ff21fca3 -- docs/refactor/v4.1.0-c/handoff/10_HANDOFF_MANIFEST.json` 为空 |
| 是否改 gate 脚本/阈值、STAGE.yaml | ✅ 未 | 工作树 `git diff -- scripts/gate/ docs/.../STAGE.yaml` 为空（此前看到的差异是分支噪声，非我的改动） |
| 是否合并受保护分支 | ✅ 未 | develop/v4.1.0 近 3 日提交中 0 个属 dsh-b |
| 是否把当前错误行为固化为"正确" | ✅ 未 | 断言全部按**目标契约**书写（多重集/精确值/写前拒绝/durable 承诺），当前实现不满足故 RED；无"断言当前错误值"型断言（已逐条 grep 核） |

## 三、不变量与概念设计一致性（逐条）

| 不变量族 | 我的测试是否违反 | 说明 |
|---|---|---|
| INV-01/02/05/07/08/09/14/15 | ✅ 未违反 | 断言方向 = 已批准契约方向；**测试是契约的判据，不是现状的记录** |
| 属主"禁止删/跳/弱化断言消除 RED" | ✅ 遵守 | RED 计数 6/6 包逐包复核一致；删除的仅是无断言占位测试 |
| 属主"C01-05/06 继承不重写" | ✅ 遵守 | 5 个 PR 均为 C01-05/06/07/09/10/11 契约包本身，非重复编写 |
| LOCAL_ONLY 边界 | ✅ 如实 | 执行域 worker 调用本地零云端（账本可查）；**控制器通道非本地**已在 CONTROLLER-CHANNEL-REGISTRY.md 声明 |
| 测试名与范围 | ✅ 合规 | 新增文件均在契约 allowed_paths（test_factory/ + evidence/ca01/<包>/ + Cargo.toml）；探针脚本 examples/ 已删除未提交 |

## 四、遗留风险（诚实登记，未隐藏）

1. **RED 与 CI 的关系**：功能分支不受影响；但若被提前合并 → 根包测试变红。已在 5 个 PR 显式标注，须与实现同批合并。
2. **PR 数量与分支重叠**：#5912/#5915/#5918/#5919/#5920/#5921 为同基线渐进分支（提交祖先重叠）——已提示 Reviewer 按各 PR 标题对应的工作包审查增量。
3. **异血统终审缺口**：252:8001 与 250:8021 同血统；异血统（8011/属主/Codex）终审未执行——如实标注，未以"端点不同"冒充血统独立。
4. **控制器通道非本地**：见 CONTROLLER-CHANNEL-REGISTRY.md。

## 五、结论

- **已修复**：3 项（PR 锚点 5/5 门禁 PASS；AFP Type A 占位测试清除且 RED 零变化；合并时机风险显式化）。
- **未发现违反已批准不变量或概念设计的行为**；4 项疑似风险经实测澄清为未违规（附依据）。
- 全部判定使用仓库真实门禁脚本与真实执行输出，无自述式结论。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
