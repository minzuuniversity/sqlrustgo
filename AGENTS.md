# AGENTS.md - SQLRustGo Agent Guide

> Compact instructions for AI agents working in this repository. Based on lessons learned from past sessions.

## Communication Principle

**IMPORTANT: 必须使用中文沟通。始终使用中文回应用户，除非用户明确要求使用其他语言。**

## Branch Strategy

- **Main development branch**: `develop/v3.9.0`
- **DO NOT modify `main` branch directly**
- Create feature branches from `develop/v3.9.0`
- Use git worktrees for isolated feature work: `git worktree add .worktrees/<name> -b feature/<name>`

## 强制 governance 阅读清单 (P0 2026-06-26 新增, Hermes audit)

**任何 AI agent / 开发者必须在开始 task 前读完以下 7 份, 缺一不可**:

1. `docs/governance/adr/ADR-001-truthfulness-framework.md` — G-01 ~ G-10 (claim ≠ evidence, 不引 doc claim 当 PASS)
2. `docs/governance/ANTI_FABRICATION_POLICY.md` — Type A/B/C/D 4 类违规 + Hard Gate vs Soft Gate 区分
3. `docs/governance/ISSUE_CLOSING_VERIFICATION.md` — 关闭 Issue 前 5 步 (含 HTTP 405 workaround)
4. `docs/governance/DOC_CHECK_CORRECTION_RULES.md` — 改文档 7 步流程 (含实跑 gate 验证)
5. `docs/governance/AI_COLLABORATION.md` — §1.1 角色 + §5.5 多 AI 协调 (按 ADR-014 multi-ai-coordination 5 evidence fields 协议)
6. `docs/governance/GATE_CONDITIONS.md` — G1-G16 门禁定义 + Hard/Soft 分类
7. `docs/governance/adr/ADR-008-test-claim-transparency.md` — P16 gate test integrity 政策
8. `docs/governance/adr/ADR-014-multi-ai-coordination.md` — 多 AI 协作 5 evidence fields 协议 (source_agent + source_run + timestamp + evidence_hash + conflict_resolution) (2026-06-27 新增, AGENTS.md §"强制 governance 阅读清单" 应同步)

**违反 P0 红线 (governance 落实不了 7 根因, 见 reports/STAGE3_DIFF_PLAN_2026-06-26.md)**:

- ❌ 写 "5/5 PASS" 不引 P11-P16 实跑输出 (AFP Type B)
- ❌ 写"完成"不引 git SHA + commit (AFP Type D)
- ❌ gate 失败不阻断 (CI 必须 `set -euo pipefail` + `exit 1`, 2026-06-26 已修)
- ❌ AI 协作 0 协调 (必须走 ADR-014 multi-ai-coordination)

**铁律**:
- 7 份读完才能写代码
- 改任何 1 份 governance 文档必须实跑 P11-P16 验证
- 任何 claim 必须带: source agent + source_run + timestamp + evidence_hash (AFP §5.4)

**关联**:
- Issue #3600: Multi-AI 验证请求
- reports/doc-audit-2026-06-26.md (本次审计报告)
- reports/STAGE2_DIFF_PLAN_2026-06-26.md (5 governance 整改方案)
- reports/STAGE3_DIFF_PLAN_2026-06-26.md (本批 gate+CI+AGENTS 整改方案)
- **更新日期**: 2026-07-01 (新增 ADR-014 引用为 item 8; 修复 item 5 "待 ADR-014 落地" 陈旧引用; ADR-014 已于 2026-06-27 ACCEPTED, 见 docs/governance/adr/INDEX.md)

## Essential Commands

```bash
# Build entire project
cargo build --all-features

# Run all tests
cargo test --all-features

# Run single test
cargo test <test_name> --all-features

# Lint (required before commit)
cargo clippy --all-features -- -D warnings

# Format check
cargo fmt --check --all

# Format fix
cargo fmt --all

# Run REPL
cargo run --bin sqlrustgo-mysql-server -- repl

# Run doc tests
cargo test --doc
```

## Gate/Validation Scripts

Located in `scripts/gate/`:

```bash
# Check entry doc links (fast)
bash scripts/gate/check_docs_links.sh

# Check ALL doc links (slower)
bash scripts/gate/check_docs_links.sh --all

# Coverage check
bash scripts/gate/check_coverage.sh

# Security check
bash scripts/gate/check_security.sh

# Doc consistency check (version status, links, version history)
bash scripts/gate/check_docs_consistency.sh
```

##Architecture

```
┌─────────────────────────────────────┐
│           main.rs (REPL)             │
├─────────────────────────────────────┤
│           executor/                 │  ← Query execution
├─────────────────────────────────────┤
│           parser/                    │  ← SQL → AST
│           lexer/                    │  ← SQL → Tokens
├─────────────────────────────────────┤
│           storage/                   │  ← Page, BufferPool, B+ Tree
├─────────────────────────────────────┤
│         transaction/                 │  ← WAL, TxManager
├─────────────────────────────────────┤
│           network/                   │  ← TCP server/client
├─────────────────────────────────────┤
│           types/                     │  ← Value, SqlError
└─────────────────────────────────────┘
```

## Crates (Workspace Members)

Key crates in `crates/`:
- `parser`, `planner`, `optimizer`, `executor` - Query processing
- `storage` - Buffer pool, file storage, B+ tree, columnar storage
- `transaction` - WAL, MVCC, transaction manager
- `network` - TCP server with MySQL-style protocol
- `vector`, `graph` - Advanced storage (vector index, graph store)
- `catalog`, `types` - Schema and type system
- 40+ total workspace crates - use `-p <package>` for single crate operations

## Important Constraints

1. **Clippy must pass**: `cargo clippy --all-features -- -D warnings` (no warnings allowed)
2. **Format must pass**: `cargo fmt --check --all`
3. **Doc links must be valid**: Run `check_docs_links.sh` before committing doc changes
4. **Test memory limit**: 8GB per test (configured in Cargo.toml)
5. **Rust edition**: 2021 with Tokio async runtime

## Common Pitfalls

| Issue | Prevention |
|-------|------------|
| Broken doc links | Always run `scripts/gate/check_docs_links.sh` after modifying markdown |
| Missing workspace deps | Use `-p <package>` flag for single crate operations |
| Slow builds | Use `cargo check` for fast compilation checks |
| Missing features | Use `--all-features` to enable all feature flags |

## Existing Instruction Files

- `.claude/CLAUDE.md` - Claude Code specific guidance
- `AGENT.md` - Issue-specific guide (DiskGraphStore implementation)
- `docs/governance/AI_COLLABORATION.md` - AI 协作规范
- `docs/governance/RELEASE_LIFECYCLE.md` - Release生命周期
- `docs/governance/ISSUE_CLOSING_VERIFICATION.md` - **Issue 关闭验证流程 (强制执行)**

## Issue 关闭规则 (强制)

**禁止手动关闭没有 PR 合并的 Issue。**

关闭 Issue 前必须验证有 PR 关联。详见：`docs/governance/ISSUE_CLOSING_VERIFICATION.md`

## 文档修改规则 (强制)

**修改 `docs/` 下任何文档前，必须遵循 `docs/governance/DOC_CHECK_CORRECTION_RULES.md` 规定的 5 步流程。**

详见：`docs/governance/DOC_CHECK_CORRECTION_RULES.md`

## Gitea Remote

本项目使用自托管 Gitea，CI/CD 和代码托管均在此。

### Git Remote (SSH)

```bash
git remote set-url origin git@gitea-macmini:openclaw/sqlrustgo.git
```

SSH 别名 `gitea-macmini` 已配置在 `~/.ssh/config`（指向 `192.168.0.252:222`）。

### Git 身份

```bash
git config user.name "claude-macmini"
git config user.email "claude@macmini.dev"
```

> pre-commit hook 强制邮箱为 `openheart@gaoyuanyiyao.com`，在本地 git config 中设置好。

### 测试连通性

```bash
# SSH 认证测试
ssh -T gitea-macmini

# Git ls-remote 测试
git ls-remote origin refs/heads/develop/v3.9.0
```

### Gitea Web UI

- URL: http://192.168.0.252:3000/openclaw/sqlrustgo
- 用户: openclaw / details8848

## Test Execution Notes

- Integration tests are in `tests/` directory
- E2E tests in `tests/e2e/`
- Crate-specific tests in each crate's `tests/` or `src/`
- Use `--test <test_name>` to run specific test files

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **sqlrustgo** (60470 symbols, 101322 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/sqlrustgo/context` | Codebase overview, check index freshness |
| `gitnexus://repo/sqlrustgo/clusters` | All functional areas |
| `gitnexus://repo/sqlrustgo/processes` | All execution flows |
| `gitnexus://repo/sqlrustgo/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
