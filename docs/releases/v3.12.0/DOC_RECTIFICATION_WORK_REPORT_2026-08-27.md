# 文档整改工作报告 — v3.12.0 文档统一到 RC 阶段

> **provenance:** source_agent=claude-sonnet (Claude Code), source_run=v312-doc-unify-20260827, timestamp=2026-08-27T11:30:00+08:00, evidence_hash=local-git:afc3346d6, conflict_resolution=N/A
> **依据**: `docs/governance/DOC_CHECK_CORRECTION_RULES.md` §三 步骤 5（编写工作报告）
> **配套 PR**: feat/v312-doc-unify → develop/v3.12.0
> **阶段**: RC（2026-08-26 转入；ALPHA 2026-08-12，BETA 2026-08-19）

---

## 1. 背景

v3.12.0 已于 2026-08-26 由 BETA 转入 RC 阶段（`STAGE.yaml: last_transition.to: RC`）。但仓库内部分文档仍残留 BETA / ALPHA 阶段口径，且核心交付物 `docs/releases/v3.12.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` 仍为 2026-08-14 ALPHA 草案（`50e5d121b`），与 RC 阶段实际状态不匹配。

为统一文档口径并交付 RC 阶段综合评估，按 `DOC_CHECK_CORRECTION_RULES.md` §三 7 步流程执行本次整改。

## 2. 整改范围

### 2.1 已修改文件（5 项最小化整改）

| 文件 | 整改内容 | 操作类型 |
|------|---------|---------|
| `RELEASE_NOTES.md` | "v3.12.0 (BETA, 2026-08-19)" → "v3.12.0 (RC, 2026-08-26 转入；ALPHA 2026-08-12，BETA 2026-08-19)" | Edit |
| `VERSION` | 添加注释说明 last-released tag (`v3.11.0` @ `5038b154c`) vs current dev (`v3.12.0` RC @ `afc3346d6`) | Edit |
| `docs/releases/v3.12.0/FEATURE_CHECKLIST.md` | SSOT chain BETA → RC；添加 RC stage update note（2026-08-26） | Edit |
| `docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md` | 添加 RC stage update section（引用 RC_GATE_REPORT.md 12/12 + B8 13/13 at HEAD `cbe1f53f85`） | Edit |
| `docs/releases/v3.12.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` | **覆盖式重写**（275 行 ALPHA 草案 → 414 行 RC 综合评估），结构按 v3.11 11 节 + Appendix A 模板 | Write |

### 2.2 新建文件（1 项）

| 文件 | 用途 |
|------|------|
| `docs/releases/v3.12.0/DOC_RECTIFICATION_WORK_REPORT_2026-08-27.md` | 本工作报告（DOC_CHECK_CORRECTION_RULES §三 步骤 5 强制要求） |

### 2.3 未修改文件（已确认一致，8 项）

- `README.md`（已正确显示 v3.12.0 RC）
- `CURRENT_VERSION.md`（已正确显示 RC）
- `CHANGELOG.md`（已含 v3.12.0 RC drift-fix 段）
- `docs/releases/v3.12.0/README.md`（已正确显示 RC）
- `docs/releases/v3.12.0/CHANGELOG.md`（已正确显示 RC drift-fix）
- `docs/releases/v3.12.0/RELEASE_NOTES.md`（已正确显示 RC）
- `docs/releases/v3.12.0/STAGE.yaml`（SSOT，未修改）
- `docs/releases/v3.12.0/RC_GATE_REPORT.md`（RC gate 聚合证据，未修改）

## 3. 整改原则（DOC_CHECK_CORRECTION_RULES §三 步骤 2）

| 原则 | 应用 |
|------|------|
| **最小修改（2.1）** | 仅修正事实性错误（版本号 / 日期 / 状态标记 / 重复条目）；不动 commit 日志、功能描述、架构设计、实质性技术内容 |
| **证据优先（2.2）** | 每条状态声明必须引 evidence（CHANGELOG / STAGE.yaml / Gate 报告 / PR / commit SHA） |
| **可撤销（2.3）** | 每步都有 git diff，可 `git checkout -- <file>` 回退 |
| **ADR-001 Truthfulness** | 每条 claim 带 source_agent / source_run / timestamp / evidence_hash；禁止无证据 PASS |
| **阶段限制** | RC 阶段，文档更新允许；禁止改 API / 接口 / 功能描述 |
| **ADR-014 Multi-AI** | 单 AI 操作场景，本报告 5 evidence fields 完整 |

## 4. 关键整改详解

### 4.1 RELEASE_NOTES.md

**Before**:
```
> **当前活跃版本**: **v3.12.0** (BETA, 2026-08-19 转入)
```
```
| **v3.12.0** | **BETA** | 2026-08-19 | ... |
```

**After**:
```
> **当前活跃版本**: **v3.12.0** (RC, 2026-08-26 转入；ALPHA 2026-08-12，BETA 2026-08-19)
```
```
| **v3.12.0** | **RC** | 2026-08-26 | ... |
```

**依据**: `STAGE.yaml` line 13 `current_stage: "RC"`, line 18 `last_transition.to: "RC"`, line 19 `date: "2026-08-26"`

### 4.2 VERSION

**Before**:
```
v3.11.0
```

**After**:
```
v3.11.0
# last released GA tag (tag `v3.11.0-ga` @ commit 5038b154c, 2026-08-09)
# current dev: v3.12.0 RC (develop/v3.12.0 @ afc3346d6, 2026-08-26)
# per docs/governance/RELEASE_GOVERNANCE.md — VERSION file is last-released tag only,
# current dev version lives in docs/releases/v3.12.0/STAGE.yaml (current_stage: RC)
```

**依据**: `docs/governance/RELEASE_GOVERNANCE.md` §VERSION 文件语义

### 4.3 FEATURE_CHECKLIST.md

**Before**:
```
> **Stage update (2026-08-26):** v3.12.0 has entered **RC** stage ...
> All feature rows below were originally verified at BETA promotion ...
```

**After**:
```
> **Stage update (2026-08-26):** v3.12.0 is now in **RC** stage (entered 2026-08-26;
> BETA was 2026-08-19, ALPHA 2026-08-12). All feature rows below were originally
> verified at BETA promotion; they remain PASS for RC per `RC_GATE_REPORT.md`
> (12/12 RC items satisfied + B8 13/13 thresholds_override). ...
```

**依据**: `RC_GATE_REPORT.md` line 27-29（12/12 items satisfied）

### 4.4 SCOPE_TABLE_v3.12.md

**Before**:
```
> **Current-status update (2026-08-14):** The Round-9 `16/22 FAIL`
> scope below is historical. ...
```

**After**:
```
> **Current-status update (2026-08-14):** The Round-9 `16/22 FAIL`
> scope below is historical. ...
> **Stage update (2026-08-26):** v3.12.0 has entered **RC** stage
> (per `STAGE.yaml` `current_stage: "RC"`; BETA was 2026-08-19). The scope
> snapshot above (captured at BETA smoke readiness 2026-08-14) remains the
> underlying SQLLogicTest evidence; the RC layer verdict aggregator is
> `docs/releases/v3.12.0/RC_GATE_REPORT.md` (12/12 items satisfied + B8
> 13/13 at HEAD `cbe1f53f85`, drift-fix from `dd5ab204`).
```

**依据**: `RC_GATE_REPORT.md` + `evidence/v312-59-e/thresholds_override_evidence.txt`

### 4.5 COMPREHENSIVE_ASSESSMENT_REPORT.md (覆盖式重写)

**Before**: 275 行 ALPHA 草案（commit `50e5d121b`，2026-08-14）
- 阶段: ALPHA
- 评估依据: 文档静态核查 + 部分 evidence
- TPC-H 结论: 待 close-out
- SQLLogicTest: 16/22 历史 FAIL（Round-9 partial）
- Wire / LOAD DATA: PARTIAL

**After**: 414 行 RC 综合评估（2026-08-27）
- 阶段: RC（2026-08-26 转入）
- 评估依据: 12/12 RC gate + B8 13/13 thresholds_override 实跑
- TPC-H 结论: Q22 PASS + Q17/Q20 TIMEOUT documented
- SQLLogicTest: 25/25 smoke + 16/16 historical closed
- Wire / LOAD DATA: 10 步骤全 PASS（含 SF=10）
- Crash recovery: 7+4+4 scenarios PASS

**结构对齐 v3.11 11 节 + Appendix A**:
- §0 Provenance（5 evidence fields）
- §1 总体结论（含 1.1-1.4 子节）
- §2 RC Gate 复核（12/12 + B8 13/13）
- §3 功能完成度（21 项 cap matrix）
- §4 稳定性与性能（TPC-H SF=1/SF=10/SOAK/LOAD DATA）
- §5 覆盖率与测试质量（per-crate 分层口径）
- §6 安全与合规（RC4/RC11 证据）
- §7 MySQL 5.7 替代能力（NOT CLAIMED）
- §8 主要风险清单（8 项 R1-R8）
- §9 GA 收尾建议
- §10 最终评估（9 PASS + 4 PARTIAL + 1 NOT CLAIMED）
- §11 文档整改记录
- Appendix A ALPHA 草案保留区

**依据**: `RC_GATE_REPORT.md` + 11 个 RC wrapper 报告 + B8 thresholds + V312-58 系列 + V312-57 系列 + V312-53/14/13 报告

## 5. 风险与边界

### 5.1 已规避的风险

| 风险 | 缓解措施 |
|------|---------|
| ALPHA 草稿覆盖导致历史丢失 | 保留 commit `50e5d121b` 在 git 历史；新报告 Appendix A 引用 git log 命令 |
| 引入虚假 PASS 声明 | 每条 claim 5 evidence fields；引用 `RC_GATE_REPORT.md` 真实 verdict |
| 阶段越权（RC 改 API） | 仅修改 markdown 文件，无 .rs / .toml 改动 |
| 过度宣传 MySQL 5.7 替代 | §7 明确判定 NOT CLAIMED；引用 `STAGE.yaml: forbidden_claims` |
| 覆盖式重写超出范围 | §11 整改记录明确列出所有改动；附录 A 保留历史 |

### 5.2 RC 阶段仍未收口项（GA blocker）

- TPC-H SF=1 SHA256 22/22 匹配（Q17/Q20 TIMEOUT documented）
- SQLLogicTest 全 SQLite 官方 corpus
- per-crate 覆盖率严格 ≥80%（#3943 收口）
- 168h SOAK 全量复跑
- GA gate D1-D9 全 PASS

## 6. Gate 验证结果

实跑日期: 2026-08-27

| Gate 脚本 | Exit Code | 结果 | 备注 |
|-----------|-----------|------|------|
| `bash scripts/gate/check_docs_links.sh` | **0** | ✅ PASS | "All markdown links are valid." |
| `bash scripts/gate/check_docs_consistency.sh` | **1** | ⚠️ 1 ERROR (pre-existing false positive) | `docs/releases/v3.12.0/CHANGELOG.md: duplicate commits: dd5ab204` — 经 `git diff develop/v3.12.0..feat/v312-doc-unify -- CHANGELOG.md` 确认本次整改**未修改** `CHANGELOG.md`；`dd5ab204` 仅在 CHANGELOG 第 10 行（current_HEAD drift-fix 标注）与第 38 行（HEAD 前移说明）中作为文本引用，非 duplicate commit 条目；属脚本误报 |
| `bash scripts/gate/check_anti_fabrication.sh` | (timeout) | ⏸️ 部分验证 | CHECK 1 (cargo check PASS) + CHECK 1.5 (V312-24 SQLancer 1000/1000 + test-runner 1/1 PASS) 通过；后续 CHECK 2+ 因耗时过长（cargo test --workspace）被中断 |

### 6.1 整改引入 vs 既有问题的区分

| 项 | 是否本次整改引入 | 验证命令 |
|----|----------------|---------|
| `check_docs_links.sh` 全绿 | ✅ 引入后保持全绿 | 见上 |
| `check_docs_consistency.sh` CHANGELOG duplicate 告警 | ❌ **非本次引入**（CHANGELOG.md 未在 diff 中） | `git diff develop/v3.12.0..feat/v312-doc-unify --stat` 显示 CHANGELOG.md 无变更 |
| `check_anti_fabrication.sh` 部分通过 | ✅ 不变（CHECK 1 + 1.5 PASS） | 脚本输出 |

## 7. 下一步

1. **Phase G**：实跑 Gate 验证脚本（`scripts/gate/check_docs_links.sh`、`check_docs_consistency.sh`、`check_anti_fabrication.sh`）
2. **Phase H**：
   - Commit（Conventional Commits 格式）
   - `git push -u origin feat/v312-doc-unify`（仅 Gitea，禁用 GitHub）
   - Gitea API 创建 PR（`http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/pulls`）
3. **后续**：
   - Reviewer 审核 → merge to `develop/v3.12.0`
   - `v3.12.0-rc1` tag cut（BETA_to_RC trigger per `STAGE_CONFIG`）
   - GA 阶段需重新编写本综合评估报告（按 `DOC_CHECK_CORRECTION_RULES.md` §三 7 步流程）

---

**附录: 5 evidence fields (ADR-014)**

| 字段 | 值 |
|------|---|
| source_agent | claude-sonnet (Claude Code) |
| source_run | v312-doc-unify-20260827 |
| timestamp | 2026-08-27T11:30:00+08:00 |
| evidence_hash | local-git:`afc3346d6` (合并自 `cbe1f53f85` drift-fix per `dd5ab204`) |
| conflict_resolution | N/A（无多 AI 冲突）；与 `minimax-m2.7` 历史 PR #4483 / RC_GATE_REPORT / STAGE.yaml 一致 |
