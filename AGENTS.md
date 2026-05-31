# AGENTS.md - SQLRustGo Agent Guide

> Compact instructions for AI agents working in this repository. Based on lessons learned from past sessions.

## Communication Principle

**IMPORTANT: 必须使用中文沟通。始终使用中文回应用户，除非用户明确要求使用其他语言。**

## Branch Strategy

- **Main development branch**: `develop/v3.8.0`
- **DO NOT modify `main` branch directly**
- Create feature branches from `develop/v3.8.0`
- Use git worktrees for isolated feature work: `git worktree add .worktrees/<name> -b feature/<name>`

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
cargo run --bin sqlrustgo

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
git ls-remote origin refs/heads/develop/v3.8.0
```

### Gitea Web UI

- URL: http://192.168.0.252:3000/openclaw/sqlrustgo
- 用户: openclaw / details8848

## Test Execution Notes

- Integration tests are in `tests/` directory
- E2E tests in `tests/e2e/`
- Crate-specific tests in each crate's `tests/` or `src/`
- Use `--test <test_name>` to run specific test files