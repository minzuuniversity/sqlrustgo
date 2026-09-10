# SQLRustGo v3.12.0 GA 文档检查和纠正工作报告

## 一、基本信息

- 工作时间：2026-09-11
- 执行人：codex
- 工作范围：v3.12.0 GA 发布入口文档、根 README 当前态段落
- 依据：`STAGE.yaml`、`evidence/v312-59/ga_gate_report.json`、252 Gitea issue / PR 查询、ADR-001、Anti-Fabrication Policy、DOC_CHECK_CORRECTION_RULES、ADR-014

## 二、发现的问题

| # | 文件 | 问题 | 依据 |
|---|---|---|---|
| 1 | `RELEASE_NOTES.md` | 仍声明 RC / GA candidate，禁止声明 GA | `STAGE.yaml` 当前为 GA；GA tag 已切 |
| 2 | `RELEASE_CHECKLIST.md` | 仍是 GA 前待办，含 PENDING/BLOCKED | `ga_gate_report.json` 为 full 72/72 PASS |
| 3 | `GA_GATE_REPORT.md` | 旧 GA-2 PENDING 作为当前态保留 | `STAGE.yaml` 和 aggregate JSON 已记录 GA cut |
| 4 | `COMPREHENSIVE_ASSESSMENT_REPORT.md` | 同时出现 GA authorized 和 current_stage RC | `STAGE.yaml` 是阶段 SSOT |
| 5 | `GA_RELEASE_REPORT.md` | 仍称不是 evidence-clean GA cut | `v3.12.0` / `v3.12.0-ga` tags 已存在 |
| 6 | `PERFORMANCE_REPORT.md` | 仍把 SOAK 5691 写作最终 GA blocker | GA-2 边界已重定为 demo/scaffold，不声明 168h complete |
| 7 | `SECURITY_AUDIT.md` | 仍写 GA 前必须刷新 | GA-3 已进入 aggregate PASS，但 caveat 需保留 |
| 8 | `CHANGELOG.md` | 顶部仍为 RC / old HEAD | GA cut commit 为 `355b5a3837` |
| 9 | root `README.md` | GA gate commit 仍指向中间 `b743ea95f4` | final GA JSON commit 为 `355b5a3837` |

## 三、执行的操作

| # | 文件 | 修改内容 |
|---|---|---|
| 1 | `README.md` | 修正 GA gate commit，并增加 post-cut refresh HEAD |
| 2 | `RELEASE_NOTES.md` | 重写为正式 GA 发布说明，加入 tag、gate evidence、known limitations |
| 3 | `RELEASE_CHECKLIST.md` | 重写为 GA 发布状态 checklist |
| 4 | `GA_RELEASE_REPORT.md` | 重写为正式 GA 发布报告 |
| 5 | `PERFORMANCE_REPORT.md` | 重写为 GA 范围内性能声明边界 |
| 6 | `SECURITY_AUDIT.md` | 重写为 GA 安全 rollup，保留 recorded caveats |
| 7 | `GA_GATE_REPORT.md` | 重写为最终 GA gate 报告，记录 shell diagnostic caveat |
| 8 | `COMPREHENSIVE_ASSESSMENT_REPORT.md` | 重写为当前 GA 综合评估 |
| 9 | `CHANGELOG.md` | 增加 2026-09-11 GA publication doc refresh 条目 |
| 10 | `README.md` | 补充证据索引和 open follow-up PR 状态 |
| 11 | `GA_PUBLICATION_EVIDENCE_INDEX.md` | 新增发布证据索引 |
| 12 | 历史文档链接 | 修复 `check_docs_links.sh --all` 暴露的既有 broken links，仅修路径或改为远端 Gitea Issue URL |

## 四、复核检查结果

| 检查项 | 结果 |
|---|---|
| 当前入口文档不再把 RC / GA-candidate 当作当前阶段 | 通过；当前发布入口改为 GA |
| GA 声明均绑定 tag / commit / aggregate JSON | 通过；使用 `355b5a3837` 和 `ga_gate_report.json` |
| #4846/#4847/#4848 未被写成已修复 | 通过；均记录为 open issue / open PR caveat |
| 文档链接检查 | 通过；`bash scripts/gate/check_docs_links_v312.sh` exit 0 |
| 文档一致性检查 | 通过；`bash scripts/gate/check_docs_consistency_v312.sh` exit 0 |
| Git diff 无非预期代码修改 | 通过；仅 README / release docs / OpenSpec doc link / GA-7 evidence 报告变更 |

## 五、待提交文件状态

`git diff --stat` 显示变更集中在 README、发布文档、OpenSpec doc link 和 GA-7 evidence 报告；没有 Rust 代码修改。

## 六、发现的问题

- 252 Gitea 当前存在 open PR #4868/#4869/#4870，对应 open issues #4846/#4848/#4847。它们是 post-GA follow-up，不得写入 v3.12.0 GA tag 的已修能力。
- `ga_beta_gate_20260908_121715.log` 中存在 `kind:: command not found` 诊断行；本次整改将其作为 evidence-quality caveat 保留在 `GA_GATE_REPORT.md`。

## 七、结论

本次整改让 v3.12.0 发布入口达到正式 GA 文档质量：阶段、tag、gate JSON、known limitations、open PR 状态一致；历史 evidence 原始报告不回写，避免破坏原始审计上下文。
