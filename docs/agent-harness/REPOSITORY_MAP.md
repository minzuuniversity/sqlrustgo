# Repository Map (Phase A 审核用)

> **基线**: `git rev-parse HEAD` → 验证; 当前 fix/4597 HEAD `583050bc63` / `develop/v4.1.0` `b513f56db1`
> **原则**: 真实仓库结构, 非历史记忆

## 顶层布局

```
sqlrustgo/                     (root)
├── AGENTS.md                  (26KB; 当前 Agent 入口 — 由本次审计替换为新候选)
├── README.md                  (12 节; 含 v4.0.0 GA 撤销 + v4.1.0 DRAFT 状态)
├── CLAUDE.md                  (76 行; Claude Code 项目指令 — 本次审计产出新候选)
├── ARCHITECTURE_RULES.md / .yaml  (架构规则)
├── BRANCH_GOVERNANCE.md, CODEOWNERS, CONTRIBUTING.md, ...
├── Cargo.toml                 (45 workspace members; workspace.package.version = "3.12.0-fix-zombie"; edition = "2021")
├── Cargo.lock
├── VERSION                    (v3.11.0; 标记 last-released; 当前 dev 在 STAGE.yaml)
├── aoci.code.txt              (1.4MB; 7080 行; AOCI 索引)
├── aoci.meta.txt              (AOCI protocol v2)
├── .aoci/                     (config.json + baseline.json; 5926 文件追踪)
├── .gitnexus/                 (meta.json: lastCommit=b1e9a98ee, indexedAt=2026-10-08; lbug database)
├── .superpowers/              (SDD 目录)
├── .opencode/                 (OpenCode skills: openspec-{explore,propose,apply,archive})
├── .claude/                   (Claude Code 配置; gitnexus-injected CLAUDE.md + skills/)
├── .gitea/                    (PR template, COLLABORATION_PROMPT, workflows/)
├── .github/                   (GitHub mirror 配置)
├── .gate/, scripts/gate/      (Hard Gate 脚本, 100+)
├── .worktrees/                (并行 PR worktree 目录, 20+ 个)
├── .cov-target/               (LLVM coverage 产物, gitignored *.json)
├── crates/                     (45 workspace crates)
├── src/                       (根 crate library; engine_*.rs 分散)
├── tests/                     (unit/, integration/, e2e/, gate/, oracle/, compat/, disabled/, ...)
├── docs/                      (governance/adr, architecture, releases/, design, evidence, ...)
├── reports/                   (审计/突变体/阶段报告)
├── tools/                     (xtask, compat-runner)
└── aoci.code.txt              (AOCI cognition index)
```

## Crates (45)

```
crates/
├── parser/      v3.12.0-fix-zombie  ({lexer,parser,token,transaction}.rs)
├── planner/     v3.12.0-fix-zombie
├── optimizer/   v3.12.0-fix-zombie
├── executor/    v3.12.0-fix-zombie  (50+ .rs; execution/, expr/, join/, trigger_eval/, parallel_*)
├── storage/     v3.12.0-fix-zombie  (file_storage, buffer_pool, bplus_tree, page, wal_legacy*)
├── catalog/      v3.12.0-fix-zombie
├── types/       v3.12.0-fix-zombie
├── transaction/ v3.12.0-fix-zombie
├── server/      v3.12.0-fix-zombie
├── mysql-server/ v0.1.0
├── mysql-client/ v3.9.0
├── cli/         → sqlrustgo-cli (v3.9.0)
├── sqlrustgo-cli/ v3.9.0  (REPL 入口)
├── security/, telemetry/, network/, spill/, cache/, gmp/, admin/, shard-router/
├── vector/      v2.1.0   (向量索引)
├── rag/         v2.3.0   (RAG)
├── graph/       v3.12.0-fix-zombie
├── graph-cli/   v0.1.0
├── evidence-graph/, sql-corpus/ v3.12.0-fix-zombie
├── bench/       v0.1.0
├── sqlancer/, transaction-stress/, sqlrustgo_sqllogictest/  (测试工具)
├── test-registry/, test-runner/, test-results/, test-reporter/  v0.1.0
├── information-schema/, query-stats/
├── wal-verification/, soak-client/
├── compat-runner, xtask (tools/)
└── distributed/ v0.1.0
```

## 根 src/ (Library)

`src/lib.rs` (82 行, library-only; 无 `src/main.rs`):

```rust
pub mod {cbo_estimator, ddl_to_sql, engine_*, execution_engine*, ...};
pub use sqlrustgo_executor::{Executor, ExecutorResult, ...};
pub use sqlrustgo_storage::{FileStorage, MemoryStorage, StorageEngine};

// ↓↓↓ 关键注释 (line 49-63) — 必须读 ↓↓↓
// BufferPool, BPlusTree, Page 被标为 "Test-only capability. Not used by production FileStorage."
// PR-A.5 (#5338) + PR-B (#5339) 把它们接到 EXPLAIN pages_visited, 但生产 FileStorage::insert/scan 当前走 HashMap 路径。
// See #5304 + docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md §3.1
pub use sqlrustgo_storage::{BufferPool, BPlusTree, Page};
```

`src/engine_*.rs` (18 个):
- `engine_select.rs`, `engine_select_pk.rs`, `engine_dml.rs`, `engine_ddl.rs`, `engine_cte.rs`, `engine_setops.rs`, `engine_create.rs`, `engine_collation.rs`, `engine_helpers.rs`, `engine_utils.rs`
- `execution_engine.rs` (顶层), `execution_engine_methods.rs` (方法), `execution_engine_tests.rs` (#[cfg(test)])
- `planner.rs`, `cbo_estimator.rs`, `expr_utils.rs`, `json_tvf.rs`, `system_tables.rs`, `savepoint_wiring.rs`, `sequence_state.rs`, `ddl_to_sql.rs`
- `memory/` 子目录, `planner/` 子目录, `execution_engine/` 子目录

## 测试目录

```
tests/
├── unit/                    (单元测试, ~10 个 .rs)
├── integration/                (集成测试)
│   ├── sql/                (SQL 测试, 100+)
│   ├── dml/                (DML 测试)
│   ├── optimizer/          (优化器测试)
│   ├── transaction/        (事务测试)
│   ├── catalog/            (catalog 测试)
│   ├── compat/             (兼容性)
│   ├── fault_injection/    (崩溃注入)
│   ├── mysql/              (MySQL 客户端集成)
│   ├── oracle/             (oracle parity)
│   └── operators/          (执行算子)
├── e2e/                      (端到端)
├── gate/                     (Hard Gate 测试)
├── benchmark/, compat/, oracle/, anomaly/, baseline/, ci/, common/, data/, disabled/, operators/, ...
├── README.md
└── 根级 *_test.rs 文件 (blk1_*, mvcc_*, ctas_*, dml_read_*, ...)
```

## docs/ 顶层

| 子目录 | 用途 |
|---|---|
| `governance/` (123 .md) | 治理 + ADR |
| `governance/adr/` | ADR-001 ~ ADR-014 |
| `governance/wal/` | WAL 契约, 事务生命周期, 违规报告 |
| `governance/patterns/` | 历史 pattern (legacy retirement, coverage dispute, ...) |
| `governance/debt/` | debt-registry.yaml + D8 Gate 脚本入口 |
| `governance/tasks/` | TASK_REGISTRY |
| `architecture/` | ARCHITECTURE.md, EXECUTION_*.md, MODULE_LIFECYCLE.md, ... |
| `releases/v{1.0..4.1.0}/` | 各版本 README + ARCHITECTURE + STAGE.yaml + DEFECTS |
| `design/` | 设计文档 |
| `evidence/` | 测试证据 |
| `audit/`, `audits/` | 审计报告 |
| `superpowers/` | plans/, specs/, reports/ |
| `runbooks/` | 运维 |
| `roadmap/` | 路线图 |
| `ai_collaboration/`, `AI_COLLABORATION_GUIDE.md` | AI 协作 |
| `tutorials/`, `教学计划/`, `教学实践/` | 中文教学资料 |
| `closer/`, `closes/` | issue 关闭记录 |
| `contracts/` | 契约 |
| `v410/`, `v1.2.0_gate_check_spec.md`, ... | 版本相关 |
| `对话记录.md`, `教学实践/`, `教学计划/`, `文档中文化推进说明.md` | 中文文档 |

## Gate 脚本 (`scripts/gate/`, 100+)

按类别 (示例):
- **架构**: `check_arch_sem_debt.sh` (D8), `check_arch_invariants.sh`, `check_arch{2,3}_no_bypass.sh`
- **Coverage**: `check_coverage*.sh`, `check_coverage_v312.sh`
- **Anti-ignore**: `check_anti_ignore_gate.sh`, `check_anti_fabrication.sh`
- **测试/原则**: `check_5_principles_v310.sh`, `check_10_principles_v310.sh`
- **版本门禁**: `check_alpha_*.sh`, `check_alpha_entry_*.sh`, `check_alpha_quality_*.sh`
- **文档**: `check_docs_links.sh`, `check_docs_consistency.sh`
- **审计**: `audit_{development,documentation,process,testing}.sh`
- **.gate/** (单独目录): 似乎存放 gate 元数据

## CI workflows (`.gitea/workflows/`)

- `ci-test.md`
- `ci.yml`
- `gate.yml`
- `reconciliation.yml`
- `soak_168h.yml`
- `soak_probe.yml`
- `z6g4-168h-soak.yml`
- `z6g4-tpch-sf1-cell-diff.yml`

## `.worktrees/` (20+ 并行 worktree)

`5168, 5177, 5177-fix, 5193-c, 5193-decision, 5193-gate, 5209-bptree, 5209-wire, 5250, 5304-doc, 5304-pr2, 5305-pr3, 5305-pr4-prA, 5305-prA.5, 5305-prB, aoci-index, audit-mutation-tests, audit-storage-mutation, audit-tooling, b0`

— 表明项目并行 PR 数量极大, 单 worktree 不够用。

## `.opencode/skills/` (OpenCode)

- `openspec-explore`, `openspec-propose`, `openspec-apply-change`, `openspec-archive-change`

— OpenSpec 协议 (类似 OpenAI Harness 的 proposal/archive 流程)。

## `.claude/skills/`

- `gitnexus/{exploring,impact-analysis,debugging,refactoring,guide,cli}/`

— GitNexus 6 个 skill (本仓库已用)。

## `.superpowers/`

- `sdd/2026-10-14-5304-pr2-bufferpool-counter-surface/` — 当前 SDD 目录

— Superpowers 的 Spec-Driven Development 工作流。