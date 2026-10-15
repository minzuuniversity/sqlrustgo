# Documentation Index

> **基线**: HEAD `583050bc63`
> **总数**: docs/ 下 123 governance .md + 大量 architecture/release/design/evidence 文档 + 根级 ~10 个 .md

## 类别 A — 项目总览

| 路径 | 状态 | 备注 |
|---|---|---|
| `README.md` | **Verified (2026-10-01)** | 12 节; v4.0.0 GA 撤销 + v4.1.0 DRAFT 状态最权威入口 |
| `AGENTS.md` | **Verified** (26KB; 355 行) | 当前 Agent 入口 — 本次审计产出新候选 |
| `CLAUDE.md` (根) | **Verified** (76 行) | Claude Code 项目指令; 本次审计产出新候选 |
| `.claude/CLAUDE.md` | **GitNexus 注入, 动态** | 含 gitnexus symbols/relationships 数字 |
| `CONTRIBUTING.md` | Stale | 引用 #2588 (历史) |
| `VERSION` | Verified (v3.11.0 last GA, dev 在 STAGE.yaml) | 有意保留 |
| `CHANGELOG.md` | Verified | v4.0.0 GA + 撤销 + v4.1.0 DRAFT |

## 类别 B — 架构与设计

| 路径 | 状态 | 备注 |
|---|---|---|
| `docs/architecture/ARCHITECTURE.md` | **Stale** | 头标注 "Version 1.2 / Vector Engine / 2026-03-05" — 落后 v4.1.0 8+ 月 |
| `docs/architecture/ARCHITECTURE_OVERVIEW.md` | Verified | 待核 |
| `docs/architecture/EXECUTION_ARCHITECTURE.md` | Verified | |
| `docs/architecture/EXECUTION_PATH.md` | Verified | |
| `docs/architecture/MODULE_LIFECYCLE.md` | Verified | 关键: 记录 test-only 模块降级 |
| `docs/architecture/SQL_SEMANTIC_AUDIT.md` | Verified | |
| `docs/architecture/SEMANTIC_DRIVEN_DEVELOPMENT.md` | Verified | |
| `docs/architecture/CASCADES_OPTIMIZER.md` + `cascades_optimizer_design.md` | Verified | |
| `docs/architecture/DISTRIBUTED_EXECUTION.md` + `distributed_scheduler_design.md` | Verified | |
| `docs/architecture/v400_graph_storage.md` | Verified (v4.0.0) | |
| `docs/architecture/ARCHITECTURE_EVOLUTION.md` | Historical | |
| `docs/architecture/EXECUTION_PIPELINE_REFACTORING.md` | Historical | |
| `docs/architecture/DIRECTORY_STRUCTURE.md` | Stale | 旧结构 |

## 类别 C — 开发流程

| 路径 | 状态 | 备注 |
|---|---|---|
| `docs/governance/DEVELOPMENT_PROCESS.md` | Verified | |
| `docs/governance/ENGINEERING_AUTOMATION.md` | Verified | |
| `docs/governance/ENGINEERING_EVOLUTION_STANDARD.md` | Verified | |
| `docs/governance/BRANCH_GOVERNANCE.md` | Verified | |
| `docs/governance/BRANCH_PROTECTION_v4.0.0.md` | Verified | |
| `docs/governance/RELEASE_GOVERNANCE.md` | Verified | |
| `docs/governance/CI_GATE_CONTRACT.md` | Verified | |
| `docs/governance/GATE_CI_CD.md` | Verified | |
| `docs/governance/GATE_CONDITIONS.md` | Verified | |
| `docs/governance/TEST_CHECKLIST.md` | Verified | |
| `docs/governance/DOCUMENT_REVIEW_WORKFLOW.md` | Verified | |
| `docs/governance/DOC_GOVERNANCE_SKILL.md` | Verified | |
| `docs/governance/FILE_GOVERNANCE.md` | Verified | |
| `docs/governance/DIRECTORY_POLICY.md` | Verified | |
| `.gitea/pull_request_template.md` | Verified | 5 类文档强制 |

## 类别 D — 测试与发布

| 路径 | 状态 | 备注 |
|---|---|---|
| `docs/governance/TEST_REVIEW_TEMPLATE.md` | Verified | |
| `docs/governance/TEST_CHECKLIST.md` | Verified | |
| `docs/governance/wal/{WAL_CONTRACT,TX_LIFECYCLE_SPEC,VIOLATION_REPORT,MISSING_TESTS}.md` | Verified | 4 份 WAL 治理 |
| `docs/releases/v4.1.0/STAGE.yaml` | **Verified (最新)** | 当前发布阶段 SSOT |
| `docs/releases/v4.1.0/CHANGELOG.md` | Verified | |
| `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` | **Verified (最新)** | P0×6 + P1×7 + P2×7 |
| `docs/releases/v4.0.0/GA_GATE_REPORT.md` | **REVOKED** | v4.0.0 GA 撤销 |
| `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` | **Verified (关键)** | §3.1 BufferPool/Page/BPlusTree test-only 边界 |
| `docs/releases/v3.12.0/STAGE.yaml` | Historical | |
| `docs/TPC-H-TEST-GUIDE.md` | Verified | |
| `docs/SQL92_COMPLIANCE.md` | Verified | |
| `docs/SYSBENCH_OLTP_SUPPORT_MATRIX.md` | Verified | |
| `docs/SYSBENCH_WIRE_PROTOCOL_FIX_GUIDE.md` | Verified | |
| `docs/soak-test-plan.md`, `docs/soak-test-ubuntu.md` | Verified | |
| `docs/release-process.md` 等 | Verified | |

## 类别 E — 历史与执行计划

| 路径 | 状态 | 备注 |
|---|---|---|
| `docs/ROADMAP.md` | Stale | 引用 #2842 等历史 issue |
| `docs/roadmap/` | Historical | |
| `docs/SPRINT4/` | Historical | |
| `docs/releases/v{1.0..3.11.0}/` | Historical | 保留作审计轨迹 |
| `docs/governance/debt/debt-registry.yaml` | **Verified** | D8 Gate SSOT |
| `docs/governance/tasks/TASK_REGISTRY.md` | Verified | |
| `docs/governance/patterns/*` (8 份) | Verified | |
| `reports/` | Audit | 审计/突变体/阶段报告 |
| `docs/audits/`, `docs/audit/` | Audit | |
| `docs/evidence/` | `evidence/` | 测试证据 |
| `docs/closes/` | Historical | issue 关闭 |

## 类别 G — Agent 专用知识

| 路径 | 状态 | 备注 |
|---|---|---|
| `AGENTS.md` (根) | Verified (本次审计将替换) | 26KB |
| `CLAUDE.md` (根) | Verified (本次审计将替换) | 76 行 |
| `.claude/CLAUDE.md` | **GitNexus 注入** | 动态生成, 含 stats |
| `.claude/skills/gitnexus/*` | Verified | 6 个 skill |
| `docs/governance/AI_COLLABORATION.md` | Verified | |
| `docs/governance/AI_SELF_OPTIMIZATION_PROMPTS.md` | Verified | |
| `docs/governance/AI_GENERATOR_AUDIT_CHECKLIST.md` | Verified | |
| `docs/governance/ANTI_FABRICATION_POLICY.md` | **Verified (关键)** | Type A/B/C/D |
| `docs/governance/ISSUE_CLOSING_VERIFICATION.md` | **Verified (关键)** | 关闭前 5 步 |
| `docs/governance/DOC_CHECK_CORRECTION_RULES.md` | **Verified (关键)** | 改文档 7 步 |
| `docs/AI_COLLABORATION_GUIDE.md` | Verified | |
| `docs/agentsql-*.md` | Verified | |
| `docs/ai_collaboration/` | Verified | |
| `.superpowers/sdd/2026-10-14-5304-pr2-bufferpool-counter-surface/` | **Verified (本次审计目录)** | |
| `.opencode/skills/` (4 个 openspec) | Verified | OpenSpec 协议 |
| `aoci.code.txt` + `aoci.meta.txt` | AOCI 索引 | 1.4MB |
| `.aoci/{config,baseline}.json` | AOCI 配置 | 5926 文件追踪 |

## ADR 清单 (`docs/governance/adr/`)

```
INDEX.md                                       — ADR 索引
ADR-001-truthfulness-framework.md              — G-01 ~ G-10
ADR-002-claim-registry.md
ADR-003-decision-registry.md
ADR-004-negative-evidence.md
ADR-005-legacy-gate-retirement.md
ADR-006-tx-wal-contract-deferral.md
ADR-006-meta-governance.md
ADR-007-wal-architecture-clarification.md
ADR-007-truthfulness-recovery-sequence.md
ADR-008-exception-v311-tpch-sf1.md
ADR-008-exception-v312-f2-e2e-wire.md
ADR-008-exception-v312-58-q17-sf1.md
ADR-008-test-claim-transparency.md             — P16 test integrity
ADR-009-g01-validation-chain-enforcement.md
ADR-010-cross-version-debt-governance.md
ADR-010-ghost-pr-resolution.md
ADR-011-debt-state-machine.md                  — 7 状态机
ADR-011-v3.8.0-1-tx-wal-repair.md
ADR-012-sqlrustgo-vs-gmp-platform-scope.md
ADR-013-v310-wired-soak-ddl-and-wire-protocol-repair.md
ADR-014-multi-ai-coordination.md               — 5 evidence fields 协议
```

**注意**: ADR-006, ADR-007, ADR-008, ADR-010, ADR-011 各有多个变体, 部分重复或已 superseded — 需要 consolidation。

## 文档冲突清单 (Phase A 发现)

| # | 冲突 | 证据 |
|---|---|---|
| 1 | 主分支引用 | AGENTS.md: `develop/v3.9.0` ❌; 实际 `develop/v4.1.0` |
| 2 | Rust edition | CLAUDE.md (根): "Rust 2024 + Tokio"; Cargo.toml: `edition = "2021"` ❌ |
| 3 | 版本基线 | Cargo.toml workspace.package.version: "3.12.0-fix-zombie"; 当前 dev: v4.1.0 DRAFT |
| 4 | Bin name | Cargo.toml grep `name = "sqlrustgo"` 等; 实际 root crate 无 `src/bin/`, REPL 在 `crates/sqlrustgo-cli/` |
| 5 | ARCHITECTURE.md 时效 | "Version 1.2 / 2026-03-05" — 落后 8+ 月 |
| 6 | ADR 重复 | ADR-006/007/008/010/011 都有多个变体 |
| 7 | D8 Gate 测试 fixture | `tests/integration/migration/wal_integration_test.rs:16` 历史上缺 `database_name`, 已由 #5261 修 — 但分散叙述 |
| 8 | BufferPool 状态 | src/lib.rs:49-63 标 "Test-only"; 多份 plan/spec 描述生产路径; `docs/architecture/MODULE_LIFECYCLE.md` 是规范 |
| 9 | CHANGELOG 入口指向旧 release | docs/README 等多处仍指向 v3.12.0/v4.0.0 |
| 10 | ROOT_SYSTEM_PROMPT GitHub 禁令 | 项目 CLAUDE.md 与 `.gitea/COLLABORATION_PROMPT.md` 都没明确"绝不到 GitHub" |

## 缺失文档

- `docs/agent-harness/` (本次审计新增, 待 Phase B 实施)
- 统一 Cargo workspace member 职责索引
- "Production path vs test path" 边界表 (虽然 MODULE_LIFECYCLE.md 提到, 但没集中)
- 多 Agent 兼容性测试报告 (Claude Code / Codex / OMP / Pi / Hermes 行为差异)