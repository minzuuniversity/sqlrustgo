# SQLRustGo v3.12.0 Draft Assessment and Alpha Gate

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=ac4c82b6f, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0
>
> **日期**: 2026-08-09
> **执行人**: Codex
> **阶段判断**: Draft 内容层面完成；Alpha promotion 仍需实跑门禁。
> **gate_policy_eval_id**: v312-alpha-draft-assessment-001
## 1. Draft 完成项

| 项 | 状态 | 证据 |
|---|---|---|
| 分支 | 完成 | `develop/v3.12.0` 已推送到 252 和 250 |
| 规划提交 | 完成 | `004056a620bec8525f66206176ff0b6be8327ce8` |
| Milestone | 完成 | 252 Gitea `v3.12.0` milestone `#38` |
| 总控 Issue | 完成 | `#3887` |
| 具体任务 Issue | 完成 | `#3888-#3911`，V312-01 至 V312-24 |
| 开发计划 | 完成 | `DEVELOPMENT_PLAN.md` |
| 测试计划 | 完成 | `TEST_PLAN.md` |
| 阶段 SSOT | 完成 | `STAGE.yaml` |
| 初始 fixture | 部分完成 | `fixtures/gmp_audit_questions.yml` |

## 2. Alpha 门禁拆分

从 2026-08-11 起，v3.12.0 Alpha 不再使用单一的“13/13 PASS”口径。Alpha 被拆成三层：

| 层级 | 脚本 | 作用 | 是否可作为功能完成证据 |
|---|---|---|---|
| Alpha Entry | `scripts/gate/check_alpha_entry_v3.12.0.sh` | 检查规划文档、文档链接、一致性、SQLLogicTest runner build/help 等入口准备 | 否 |
| Deferred Approval | `scripts/gate/check_v312_deferred_followups.sh` | 检查 SQLLogicTest exclusion 是否绑定 Gitea Issue、owner、expiry、close boundary | 否，仅证明延期可追踪 |
| Alpha Quality | `scripts/gate/check_alpha_quality_v3.12.0.sh` | 执行 SQLLogicTest、P12、P16、anti-ignore、SQL corpus、AFP 等硬门禁 | 是，且必须实跑 |

Alpha Entry 必须至少检查：

- 必需文档存在：`VERSION_PLAN.md`、`DEVELOPMENT_PLAN.md`、`TEST_PLAN.md`、`ISSUES_PLAN.md`、`ARCHITECTURE.md`、`RELEASE_NOTES.md`、`STAGE.yaml`。
- 文档链接和一致性检查通过。
- `cargo build -p sqlrustgo_sqllogictest` 可执行。
- `cargo run -p sqlrustgo_sqllogictest -- --help` 可执行。
- `scripts/gate/check_alpha_quality_v3.12.0.sh`、`scripts/gate/check_sqllogictest_v312.sh` 存在且可执行。

Alpha Quality 必须至少检查：

- `scripts/gate/check_sqllogictest_v312.sh` 实跑结果。
- `scripts/gate/check_v312_deferred_followups.sh` 确认每个 exclusion 绑定 Gitea Issue。
- `scripts/gate/check_ignore_count.sh`、`scripts/gate/check_anti_ignore_gate.sh`、`scripts/gate/check_gate_test_integrity.sh`。
- `scripts/gate/check_sql_corpus_gate.sh` 和 `scripts/gate/check_anti_fabrication.sh`。

## 3. 已知 Alpha 阻断或风险

| 风险 | 状态 | 后续任务 |
|---|---|---|
| SQLLogicTest v312 脚本此前不存在 | 已补入口脚本 | V312-11 |
| 全量 SQLite 官方 corpus 尚未集成 | 未完成 | V312-11 |
| GMP schema 尚未实现 | 未完成 | V312-02 |
| 168h mixed SOAK 尚未运行 | 未完成 | V312-10 |
| TPC-H SF=1 correctness 尚未由 v3.12 gate 收口 | 未完成 | V312-12 |
| coverage 和 disabled-test debt 尚未关闭 | 未完成 | V312-17 |
| SQLLogicTest exclusion 不能作为 PASS 证据 | 已纳入 Alpha Quality | `check_v312_deferred_followups.sh` |
| P12/P16/anti-ignore hard gate 失败时必须阻断 | 已纳入 Alpha Quality | `check_alpha_quality_v3.12.0.sh` |

## 4. 结论

Draft 阶段可以移交 Hermes/OMP 进入 Alpha 开发准备，但只能称为 Alpha Entry 准备。任何 Alpha Quality PASS、功能完成或门禁通过声明，必须等待对应命令实跑并产生 evidence hash。注册到 `exclusions.yml` 的失败项只能算“可追踪延期”，不能算“测试通过”。

## 5. 本次准备验证

| 命令 | 结果 | 边界 |
|---|---|---|
| `bash scripts/gate/check_alpha_entry_v3.12.0.sh` | 新增 | 仅代表 Alpha Entry 准备，不代表业务功能完成 |
| `bash scripts/gate/check_v312_deferred_followups.sh` | 新增 | 仅检查延期项是否绑定 Gitea Issue、owner、expiry、close boundary |
| `bash scripts/gate/check_alpha_quality_v3.12.0.sh` | 新增 | 运行硬质量门禁；当前若 SQLLogicTest/P12/P16/anti-ignore 失败，必须 BLOCKED |
| `bash scripts/gate/check_sqllogictest_v312.sh` | 已存在 | smoke baseline；不代表官方 SQLite corpus 已集成；失败项注册不是 PASS 证据 |
| `bash scripts/gate/check_alpha_v3.12.0.sh` | 改为 composite wrapper | 先跑 Entry，再跑 Quality；只有两者都通过才可输出 Alpha Gate PASS |
| `bash scripts/gate/check_stage.sh --version v3.12.0 --dry-run` | PASS，DRAFT dry-run 可解析 | 不执行 cargo build |
| `bash scripts/gate/check_stage.sh --version v3.12.0 --stage ALPHA --dry-run` | PASS，ALPHA dry-run 可解析 | 不执行 ALPHA 全量 gate |
| `bash scripts/gate/check_stage.sh --version v3.12.0` | PASS，4/4 | DRAFT stage gate 实跑通过，含 docs links 和 `cargo build --all-features` |

最新 SQLLogicTest smoke evidence：

- Report: `docs/releases/v3.12.0/evidence/sqllogictest/smoke-report.md`
- Log: `docs/releases/v3.12.0/logs/sqllogictest_<commit>_<timestamp>.log`
