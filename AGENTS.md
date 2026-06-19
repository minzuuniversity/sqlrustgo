# AGENTS.md - SQLRustGo Agent Guide

> Compact instructions for AI agents working in this repository. Based on lessons learned from past sessions.

## Communication Principle

**IMPORTANT: 必须使用中文沟通。始终使用中文回应用户，除非用户明确要求使用其他语言。**

## Branch Strategy

- **Main development branch**: `develop/v3.9.0`
- **DO NOT modify `main` branch directly**
- Create feature branches from `develop/v3.9.0`
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
| **LOAD DATA 性能误解** | TPC-H SF=0.01 (60K 行) LOAD DATA 慢的根因**不是 LOAD DATA 本身**，是 `start_ephemeral` 启动时跑 WAL recovery，对 60K entry 调 `storage.scan()` + `force_insert()` 每个都触发 O(N) `data.clone()` + JSON 序列化。修复看 **TPC-H LOAD DATA 性能** 章节。 |
| **测试名空格 bug** | `tests/tpch_sf01_22_vs_3engines_test.rs` (renamed from `tpch_sf01_22 vs_3engines.rs`, see commit 02f7a9a8c→rename)，cargo build 会失败 (`invalid character ' ' in crate name`)。`cargo test --tests` 会因 build 失败而整体 abort。逐个 `cargo test --test <name>` 绕过。 |
| **fixture WAL 膨胀 → EAGAIN** | `tests/data/tpch-sf*/sqlrustgo.wal` 每次 `start_ephemeral` append。多次 test run 后会涨到 1+ GB，导致下一次 start 的 recovery 超 client connect timeout (60s) → EAGAIN `os error 35`。`tests/common/tpch_wire_harness.rs::start_with_fixture` 已在 start 前 truncate WAL。如发现 EAGAIN：1) `truncate -s 0 tests/data/tpch-sf*/sqlrustgo.wal` 2) 重跑。 |
| **recovery_scenarios_test 有 2 个 pre-existing 失败** | `r3_d07_single_entry_insert_only` 和 `r3_d08_single_entry_delete_only` 在 develop/v3.9.0 改 WAL 逻辑前就失败，与 LOAD DATA 性能修复无关。改 recovery 代码后必须 `git stash` 验证非回归。 |
| **tpch_q9_audit baseline 缺失** | `tests/data/tpch-sf01/baseline/Q09_three_way.json` 从未生成。un-`#[ignore]` 此测试会导致 panic / OOM。修复：要么生成 baseline，要么保持 `#[ignore]`。 |

## Test Policy (ADR-008)

### Policy 1: Test Claim Transparency (P-Claim)

Every "PASS" claim in public-facing docs MUST be qualified:
- Number of tests **actually run** (vs trivial-PASS due to `#[ignore]`)
- Or an explicit "0 `#[ignore]`" badge if all tests ran
- Reference to the meta-gate that verified the claim

**Required phrasing**:
- `TPC-H 22/22 in-process ✅` (NOT just `TPC-H 22/22 ✅`)
- `9-Dim Gate 8/8 PASS (D9 only; G2-G10 templated)` (NOT just `9-Dim Gate 8/8 PASS`)
- `Soak 10/10 PASS (10/10 are #[ignore])` (explicitly disclose ignore)

### Policy 2: No-Ignore Gate Tests (P-Gate)

**A test file referenced by any script in `scripts/gate/*.sh` is a GATE TEST. Gate tests MUST NOT be `#[ignore]`-marked.**

- **Definition**: A test file is a gate test iff its filename appears in a `cargo test ... --test X` invocation in any `scripts/gate/*.sh` script.
- **Enforcement**: P16 meta-gate (`scripts/gate/check_gate_test_integrity.sh`) — runs in CI, fails the build if any gate test is `#[ignore]`.
- **Exception process**: A gate test may be temporarily ignored only via an ADR amendment with explicit deadline + owner + success criteria, recorded as a header comment in the gate script.
- **Why**: A `#[ignore]`'d test produces `cargo test exit 0` (= "PASS") even though the test didn't run. The P16 meta-gate catches this V8-style anti-pattern.

**Current state (2026-06-17)**: 27 gate-referenced tests, 0 `#[ignore]`. 42 total `#[ignore]`'d tests are all non-gate (perf benchmarks, long-run stability, known-broken tracked in issues). See `tests/baseline/gate_test_baseline.json`.

### Running the meta-gates

```bash
# P11: Gate Self-Verification
bash scripts/gate/check_gate_self_verification.sh

# P12: No Implicit Tolerance (counts #[ignore])
bash scripts/gate/check_ignore_count.sh

# P13: Test Count Monotonicity
bash scripts/gate/check_test_count_monotonic.sh

# P14: DRIFT != PASS
bash scripts/gate/check_drift_not_pass.sh

# P15: Oracle Required
bash scripts/gate/check_oracle_present.sh

# P16: Gate Test Integrity (NEW, ADR-008 Policy 2)
bash scripts/gate/check_gate_test_integrity.sh
```

See `docs/governance/adr/ADR-008-test-claim-transparency.md` for the full policy.

## TPC-H LOAD DATA 性能 (重要！)

**症状**: SF=0.01 (60K 行) LOAD DATA 在 `cargo test` 中**永久挂起** (>11 分钟)，导致所有 13 个 TPC-H wire 测试必须 `#[ignore]` 掉。

**根因**: 不是 LOAD DATA 本身，是 `start_ephemeral(data_dir)` 启动时跑 `RecoveryEngine::recover`。`sqlrustgo.wal` (17 MB) 包含 60K 个 INSERT entry。**对每条 entry 调 `apply_entry` → O(N) `storage.scan()` dedup + O(N) `force_insert` 触发 `data.clone()` + JSON 序列化 = O(N²) = 1.8B ops**。客户端 60s 超时 → EAGAIN。

**修复** (commit `d5c8fe384`):
1. 新增 `StorageEngine::bulk_force_insert(table, Vec<Record>)` trait 方法
2. `FileStorage` override: `data.rows.extend(records) + 1× save_table`
3. `WalStorage` override: 跳过 WAL，转发 inner (recovery 路径调用)
4. `RecoveryEngine::recover` 改为按 table 分组 + HashSet dedup + `bulk_force_insert` 一次/表

**效果**: SF=0.01 LOAD DATA 11 min → 3.04s (220×)。TPC-H G1 Gate 22/22 queries 4.7s 总。13 个 TPC-H wire 测试 un-`#[ignore]`，默认 `cargo test` 全跑 61s。

**调试步骤** (如果再次发生类似挂起):
1. 加 `eprintln!` 在 `tests/common/tpch_wire_harness.rs` 的 `start_with_fixture` 各阶段 + `load_fixture` 每表 LOAD DATA 之后
2. 用 `RUST_LOG=info` 跑，捕获 server 端日志
3. 用 `git stash` 验证是否 pre-existing 失败
4. 复现：`(cargo test --release --test _profile -- --ignored --nocapture --test-threads=1 2>&1 || true) > /tmp/prof.log & TPID=$!; sleep N; kill -9 $TPID`
5. profile 后**记得清掉 eprintln** (commit 之前)

详见 `docs/releases/v3.9.0/E2E_MIGRATION_MASTER_PLAN.md` 11.

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

This project is indexed by GitNexus as **sqlrustgo** (72572 symbols, 106276 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
