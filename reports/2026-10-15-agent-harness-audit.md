# SQLRustGo Agent Harness 重构 — Phase A 审计报告

> **报告日期**: 2026-10-15
> **基线**: `git rev-parse HEAD` = `583050bc6349375d01163f19f9fa32dc2fa2f0e3`
> **当前分支**: `fix/4597-chaos-t8d-awk-portability` (领先 `develop/v4.1.0` `b513f56db1` 1 个 merge commit + 3 个 fix/docs commits)
> **Phase**: A (Read-only Audit)
> **执行人**: Claude Code (MiniMax-M3)
> **状态**: 全部只读审计完成; 候选文件输出到 `/tmp/sqlrustgo-harness-audit/`; **不动正式文件**

---

## A. 仓库事实

### A.1 Commit / 分支 / Remote

```
HEAD:          583050bc6349375d01163f19f9fa32dc2fa2f0e3
Branch:        fix/4597-chaos-t8d-awk-portability
Base:          develop/v4.1.0 @ b513f56db1e08bc1ea314241ddb51a31a5724d10
Worktree:      /Volumes/workspace/dev/sqlrustgo (Mac) / /Users/liying/workspace/dev/sqlrustgo (path alias)
Status:        clean (只 .cov-target/ untracked, LLVM coverage 产物)

Remote (5):
  gitea252 → http://192.168.0.252:3000/openclaw/sqlrustgo.git (PRIMARY, 可写)
  gitea250 → http://192.168.0.250:3000/openclaw/sqlrustgo.git
  gitcode  → git@gitcode.com:BreavHeart/sqlrustgo.git
  gitee    → git@gitee.com:yinglichina/sqlrustgo.git
  github   → git@github.com:yinglichina8848/sqlrustgo-public.git (只读)
```

⚠️ 用户全局 CLAUDE.md 引用 `origin`, 实际 remote 名是 `gitea252`。

### A.2 Cargo Workspace (45 members)

| 命名空间 | 主要 crate | 版本 |
|---|---|---|
| `sqlrustgo-*` (核心) | types, common, parser, planner, optimizer, executor, storage, catalog, transaction, server, telemetry, network, gmp, vector, sql-corpus, cache, admin, spill, sqlrustgo-cli, mysql-server, security, graph, tools, soak | v3.12.0-fix-zombie (大部分) |
| 工具/测试 | test-registry, test-runner, test-results, test-reporter, test-reporter, sqlrustgo_sqllogictest, sqlancer, transaction-stress, wal-verification, bench, sqlrustgo-cli, mysql-client | v0.1.0 / v3.9.0 |
| 其他 | rag (v2.3.0), vector (v2.1.0), graph-cli, evidence-graph, shard-router, distributed, soak-client, graph-cli, information-schema, query-stats | mixed |
| 根 crate | `sqlrustgo` (library, 无 main.rs; src/lib.rs re-export) | v3.12.0-fix-zombie |

**冲突**:
- Cargo.toml line 1 注释 `# SQLRustGo v3.11.0` — 实际 workspace version = `3.12.0-fix-zombie`
- workspace.package `edition = "2021"` (项目历史改动后未升级); 旧 CLAUDE.md/ARCHITECTURE.md 写 "Rust 2024"
- Cargo.toml `name = "sqlrustgo"` 等 bin name 暗示自动 bin, 但根 crate 无 `src/bin/`, 无 `src/main.rs` — REPL 在 `crates/sqlrustgo-cli/`

### A.3 目录与文件

```
顶层 (~16 .md + Cargo.toml + VERSION + aoci.code.txt 1.4MB):
  AGENTS.md (26KB, 355 行)  ← 本次审计产出新候选
  README.md (12 节, 213-350+ 行)  ← v4.0.0 GA 撤销 + v4.1.0 DRAFT 状态入口
  CLAUDE.md (76 行)  ← 本次审计产出新候选
  ARCHITECTURE_RULES.md, ARCHITECTURE_RULES.yaml
  BRANCH_GOVERNANCE.md, CODEOWNERS, CONTRIBUTING.md
  CURRENT_VERSION.md, CHANGELOG.md, RELEASE_*.md, ROADMAP.md
  Cargo.toml, Cargo.lock, VERSION, aoci.code.txt, aoci.meta.txt
  .aoci/, .gitnexus/, .superpowers/, .opencode/, .claude/, .gitea/, .github/
  .gate/, scripts/gate/ (~100+ gate 脚本), .worktrees/ (~20+)

crates/ (45 个子目录, 各有 src/ + Cargo.toml)
src/ (根 crate library, 23 .rs + 3 子目录)
tests/ (unit/, integration/{sql,dml,optimizer,transaction,catalog,compat,fault_injection,mysql,oracle,operators}, e2e/, gate/, ...)
docs/ (governance/adr 30+, architecture, releases/{v1.0..v4.1.0}, design, evidence, audit, runbooks, ...)
```

### A.4 现有指令文件

| 文件 | 行数 | 状态 | 备注 |
|---|---|---|---|
| `AGENTS.md` (根) | 355 | **本审计替换候选** | 含 GitNexus/AOCI 注入块 |
| `CLAUDE.md` (根) | 76 | **本审计替换候选** | 部分内容陈旧 (Rust 2024) |
| `.claude/CLAUDE.md` | 41 | **GitNexus 注入, 动态** | 不应手动修改 |
| `.gitea/COLLABORATION_PROMPT.md` | - | Verified | 含 SSH 别名, commit 身份 |
| `.gitea/pull_request_template.md` | - | Verified | 5 类文档 + Gate integration |
| `.gitea/workflows/*.yml` (8) | - | Verified | ci-test, ci, soak, gate, reconciliation |
| `.github/` | - | Verified | 只读镜像配置 |
| `.superpowers/` | - | SDD 工作流 | 当前 sdd/2026-10-14-5304-pr2-bufferpool-counter-surface/ |
| `.opencode/skills/` (4) | - | OpenSpec | explore/propose/apply/archive |
| `.claude/skills/gitnexus/` (6) | - | Verified | exploring/impact-analysis/debugging/refactoring/guide/cli |

---

## B. GitNexus 分析

### B.1 索引状态

```
.gitnexus/meta.json:
  repoPath: /Volumes/workspace/dev/sqlrustgo
  lastCommit: b1e9a98ee6bac1e863ab8826c54a41500455b71d  ← 落后 HEAD 1 周+
  indexedAt: 2026-10-08T00:45:31.423Z
  stats: 5544 files, 103833 nodes, 157185 edges, 1902 communities, 300 processes, 0 embeddings

⚠️ FTS 索引报错: "Database file version: 42, Current build storage version: 40"
   - file_fts, function_fts, class_fts, method_fts, interface_fts 均不可用
   - 符号图 (103833 nodes) 仍可用
```

### B.2 GitNexus AGENTS.md 注入块的 stats 不一致

```
.claude/CLAUDE.md line 4: "110918 symbols, 160471 relationships, 300 execution flows"
.gitnexus/meta.json:        103833 nodes,   157185 edges,        300 processes
```

→ meta.json 是真实数据; 注入块落后。

### B.3 4 条核心执行链 (按源码核实, 详见 `docs/agent-harness/GITNEXUS_ARCHITECTURE_MAP.md`)

1. **SQL 查询**: Client → mysql-server → server → SqlExecutor → parser → ExecutionEngine::execute → engine_select → planner → optimizer → executor → storage → wire
2. **事务**: Client → TxManager → MVCC → WAL append → storage fsync
3. **索引**: DDL → Catalog → IndexManager (bin_index / adaptive_hash_index) → B+Tree/AHI
4. **启动 + 恢复**: main → catalog load → storage init → WAL replay → recovery → listener.accept

### B.4 关键不变量

- `WalEntry` 字段: tx_id, entry_type, table_id, key, data, lsn, timestamp, table_name (#5055), database_name (#5192)
- 真正写 WAL: `FileStorage::wal_row_entry` (非 `WalStorage::log_insert` 死代码)
- `BufferPool/Page/BPlusTree` 在 `src/lib.rs:49-63` 标 "Test-only capability"

### B.5 信息缺失 / 降级路径

| 缺失 | 降级 |
|---|---|
| FTS 索引 (db version mismatch) | `rg` + 源码 + rust-analyzer |
| 索引 commit 落后 ~1 周 | 配合 `git log --name-only HEAD~50..HEAD` |
| stats 在 GitNexus 注入块与 meta.json 不一致 | 直接读 meta.json |

---

## C. AOCI 分析

### C.1 索引状态

```
aoci.code.txt:  1.4MB, 7080 行 (v2 protocol FRAS-2)
aoci.meta.txt:  protocol v2
.aoci/config.json:    index_path = "aoci.txt" (但实际是 aoci.code.txt)
.aoci/baseline.json:  5926 文件追踪 (5011 index + 915 observe), updated_at 2026-10-09T17:22:16Z
```

### C.2 语义覆盖

抽样 `aoci.code.txt` 第 1-100 行:
- 30+ 顶层文件 (AGENTS.md, README.md, ARCHITECTURE_RULES.md, CHANGELOG.md, CONTRIBUTING.md, ...)
- 多数带 S-tag 警告: "Written against vX.Y; version claims age with branch" 或 "Cites issues #X whose state may have moved"
- TPC-H 系列报告全标 SB4M/SB4S, 写明 "Written against v1.6.0 / v2.1.0 / v2.5.0 / v3.11.0 — outdated"

→ **AOCI 索引自身标注大量过时风险**; 必须以 commit + 文档复核为准。

### C.3 与代码一致性

AOCI baseline 用 SHA256 追踪文件变化, 但 `updated_at` 落后 HEAD 6 天。模型在维护时必须独立 author F/R/A/S, 不能从路径推导。

### C.4 建议更新

- 不需要重建索引 (语义层仍有效, 只是 S-tag "version claims age")
- 但应该跑 `aoci_maintain` 对新 commit (HEAD 6 天以来 ~50+ commit) 做 semantic candidate 检查

详见 [`docs/agent-harness/AOCI_GUIDE.md`](docs/agent-harness/AOCI_GUIDE.md)。

---

## D. 文档体系分析

### D.1 完整分类与标注

(详见 [`docs/agent-harness/DOCUMENTATION_INDEX.md`](docs/agent-harness/DOCUMENTATION_INDEX.md))

**Verified (核心)**:
- README.md, AGENTS.md, CLAUDE.md, CHANGELOG.md, VERSION
- docs/governance/{ANTI_FABRICATION_POLICY, ISSUE_CLOSING_VERIFICATION, DOC_CHECK_CORRECTION_RULES, GATE_CONDITIONS, CI_GATE_CONTRACT, AI_COLLABORATION}.md
- docs/governance/adr/ADR-{001, 008, 011, 014}.md
- docs/governance/wal/{WAL_CONTRACT, TX_LIFECYCLE_SPEC}.md
- docs/releases/v4.1.0/{STAGE.yaml, CHANGELOG.md, DEFECTS_AND_ISSUES.md}
- docs/releases/v4.0.0/{CLAIM_DOWNGRADE_MANIFEST, GA_RELEASE_TIMELINE}.md
- docs/architecture/{EXECUTION_*, MODULE_LIFECYCLE, SQL_SEMANTIC_AUDIT}.md
- .gitea/{COLLABORATION_PROMPT, pull_request_template}.md

**Stale (过时)**:
- AGENTS.md `develop/v3.9.0` → 实际 v4.1.0
- 根 CLAUDE.md "Rust 2024" → Cargo.toml "2021"
- docs/architecture/ARCHITECTURE.md "Version 1.2 / 2026-03-05" → 落后 8+ 月
- docs/architecture/DIRECTORY_STRUCTURE.md — 旧单层结构
- CONTRIBUTING.md 引用 #2588 (历史)
- ROADMAP.md 引用 #2842 等历史
- TPC-H-* 系列报告 (大部分标 outdated)

**Duplicate (重复)**:
- ADR-006, ADR-007, ADR-008, ADR-010, ADR-011 各有多个变体 (exception 系列 vs 主 ADR)
- ARCHITECTURE.md vs ARCHITECTURE_OVERVIEW.md vs docs/releases/v4.1.0/ARCHITECTURE.md
- CURRENT_VERSION.md vs VERSION vs Cargo.toml version

**Missing (缺失)**:
- `docs/agent-harness/` (本次审计新增, 待 Phase B 落地)
- 统一 Cargo workspace member 职责索引
- 多 Agent 兼容性测试报告
- "Production path vs test path" 集中边界表

### D.2 文档冲突清单

| # | 冲突 | 证据 |
|---|---|---|
| 1 | 主分支引用 | AGENTS.md: `develop/v3.9.0` ❌; 实际 `develop/v4.1.0` |
| 2 | Rust edition | CLAUDE.md (根): "Rust 2024 + Tokio"; Cargo.toml: `edition = "2021"` |
| 3 | 版本基线 | Cargo.toml `version = "3.12.0-fix-zombie"`; dev = v4.1.0 DRAFT |
| 4 | Bin name | Cargo.toml `name = "sqlrustgo"`; REPL 在 `crates/sqlrustgo-cli/` |
| 5 | ARCHITECTURE.md 时效 | v1.2 / 2026-03-05 — 落后 8+ 月 |
| 6 | ADR 重复 | 5 个 ADR 有多个变体 |
| 7 | WalEntry fixture 缺字段 | 已由 #5261 修, 但分散叙述 |
| 8 | BufferPool 状态 | src/lib.rs:49-63 test-only vs 多 plan 描述生产路径 |
| 9 | CHANGELOG/CLAUDE 入口 | 多处仍指向 v3.12.0/v4.0.0 |
| 10 | ROOT_SYSTEM_PROMPT GitHub 禁令 | 项目 CLAUDE.md/Gitea 协作 prompt 都未明说 "绝不到 GitHub" (在用户全局 CLAUDE.md 而非项目内) |

---

## E. 新 Harness 设计

### E.1 知识分层

| 知识类型 | 推荐权威来源 | 更新机制 |
|---|---|---|
| 函数实现 | Git HEAD 源码 | 每 commit |
| 类型 / Trait 引用 | rust-analyzer / 源码 | 同上 |
| 调用与依赖图 | **GitNexus** (`.gitnexus/meta.json`) | `npx gitnexus analyze` |
| 模块职责 + 设计 | **AOCI** (`aoci.code.txt` F/R/A/S) | `aoci_maintain` + `aoci_update_entry` |
| 长期语义认知 | **AOCI** | 同上 |
| 测试是否通过 | 实际测试报告 | `cargo test` 输出 |
| 架构规范与决策 | ADR / 设计文档 | git commit |
| 当前开发状态 | Git / Issue / PR / CI | 实时 |
| **Agent 导航** | **新 `AGENTS.md` (100-150 行) + `docs/agent-harness/`** | Phase B |

### E.2 候选文件结构 (输出到 `/tmp/sqlrustgo-harness-audit/`, Phase B 才落地)

```
AGENTS.md                                    — 100-150 行根入口 (渐进式加载)
CLAUDE.md                                    — Claude Code 适配子集
docs/agent-harness/
  INDEX.md                                   — 任务 → 源码 → 文档
  REPOSITORY_MAP.md                          — 仓库结构 (已核实)
  GITNEXUS_ARCHITECTURE_MAP.md               — 4 条执行链
  AOCI_GUIDE.md                              — AOCI 工作流
  DOCUMENTATION_INDEX.md                     — 文档分类 + 冲突清单
  IMPLEMENTATION_TRUTH.md                    — 5 级实现真实性
```

### E.3 Agent 导航流程

```
1. 读根 AGENTS.md (100-150 行)
2. 启动事实核验: git rev-parse HEAD + STAGE.yaml + bash check_arch_sem_debt.sh
3. 读 docs/agent-harness/INDEX.md → 找到任务对应入口
4. 读相关 REPOSITORY_MAP / DOC_INDEX 章节
5. 用 GitNexus (impact + query + context) 探索
6. 用 AOCI (overview + maintain) 确认语义
7. 读最小必要源码 + 对应测试
8. 实施 TDD (RED → GREEN → refactor)
9. 跑 clippy + fmt + relevant gates + cargo test
10. gitnexus_detect_changes 验证
11. commit + push (gitea252 only) + Gitea PR (5 类文档 + Evidence)
```

### E.4 规则优先级

```
P0 (不动): governance (AFP / ADR-001 / GATE_CONDITIONS / ADR-008 / ADR-014 等 7+1 必读)
P0 (不动): 5 级实现真实性 (Implemented/Integrated/Tested/Verified/Release-ready)
P0 (不动): Hard Gates (scripts/gate/) 失败即 exit 1
P0 (不动): Gitea only (不推 GitHub, 不用 gh CLI)
P1: 项目级强制 (Cargo.toml `--all-features`, clippy -D warnings, fmt, 8GB 内存)
P2: PR 5 类文档 + 5 字段 evidence (ADR-014)
P3: 工具使用 (GitNexus impact / AOCI overview / 启动事实核验)
```

---

## F. 候选 AGENTS.md

详见 [`AGENTS.md`](AGENTS.md) (写到 `/tmp/sqlrustgo-harness-audit/AGENTS.md`)。

### 新旧对照

| 维度 | 旧 (355 行) | 新 (候选, 估 ~150 行) |
|---|---|---|
| 行数 | 355 | ~150 |
| 主分支引用 | ❌ `develop/v3.9.0` | ✅ `develop/v4.1.0` |
| Rust edition | 提及 2021 + Tokio | 移至 CLAUDE.md (实际 edition) |
| 必读 governance 清单 | 8 项 (P0) | 保留 8 项 (P0) — 不可降级 |
| 实现真实性 | 仅"严禁 Type A/B/C/D" | 升级到 5 级 (Implemented/Integrated/Tested/Verified/Release-ready) |
| GitNexus | 注入块 25 行 + skill 列表 | 12 行 (核心 MUST/DO NOT + 失效降级) |
| AOCI | 100+ 行完整协议 | 14 行 (核心原则 + 降级 + 链向 AOCI_GUIDE.md) |
| Gitea | 35 行 (SSH 配置 + 身份) | 6 行 (链向 .gitea/COLLABORATION_PROMPT.md) |
| 测试执行 | 5 行 | 8 行 (最小闭环) |
| 严禁行为 | 14 行 | 14 行表格化 |
| 证据要求 | 5 字段 (ADR-014) | 保留 5 字段 |
| 详细入口 | 无 | 7 个 `docs/agent-harness/*.md` |
| 仓库导航 | 单层 ASCII 图 | 3 层 (1 级目录 + Crates + Remote) |
| 任务映射 | 无 | 链向 `docs/agent-harness/INDEX.md` |

---

## G. 验证结果

### G.1 路径/链接/命令验证

```
✅ /tmp/sqlrustgo-harness-audit/AGENTS.md (5 路径引用全部链向本仓库实际文件)
✅ /tmp/sqlrustgo-harness-audit/CLAUDE.md (命令基于 Cargo.toml + REPL 在 sqlrustgo-cli)
✅ /tmp/sqlrustgo-harness-audit/docs/agent-harness/INDEX.md (17 行任务映射, 全部基于真实路径)
✅ /tmp/sqlrustgo-harness-audit/docs/agent-harness/REPOSITORY_MAP.md (45 crates; 全核实)
✅ /tmp/sqlrustgo-harness-audit/docs/agent-harness/GITNEXUS_ARCHITECTURE_MAP.md (4 执行链; 基于 src/lib.rs + parser + storage)
✅ /tmp/sqlrustgo-harness-audit/docs/agent-harness/AOCI_GUIDE.md (基于 .aoci/config.json + baseline.json)
✅ /tmp/sqlrustgo-harness-audit/docs/agent-harness/DOCUMENTATION_INDEX.md (7 类别, 50+ 路径全部存在)
✅ /tmp/sqlrustgo-harness-audit/docs/agent-harness/IMPLEMENTATION_TRUTH.md (5 级 + AFP 4 类 + 10 项严禁)
```

### G.2 Cargo workspace 验证

- 45 members, 与 Cargo.toml `[workspace] members` (line 4-49) 一致
- Bins: `name = "sqlrustgo", sql_operations, lexer_bench, parser_bench` 但根 crate 无 `src/main.rs`/`src/bin/` — 实际在 `crates/sqlrustgo-cli/` (已在新 CLAUDE.md 修正)
- workspace.package version = "3.12.0-fix-zombie", edition = "2021" (与 Cargo.toml 一致)

### G.3 GitNexus/AOCI 状态验证

- `.gitnexus/meta.json`: lastCommit = `b1e9a98ee` (2026-10-08); HEAD = `583050bc63` (2026-10-15) — 落后 7 天, 已知问题
- `.aoci/baseline.json`: updated_at = `2026-10-09T17:22:16Z`, 5926 文件追踪
- FTS 索引: database version 42 vs build 40 — 失效, 已记录降级路径

### G.4 文档清单验证

- docs/ 顶层 60+ 子目录 (用 ls + find 验证)
- governance/adr/ 21 个 ADR (已 grep 验证)
- docs/releases/v4.{0,1}.0/ 当前活跃 (其它历史版本保留)

### G.5 兼容性结论 (多 Agent)

| Agent | AGENTS.md | CLAUDE.md | 备注 |
|---|---|---|---|
| Claude Code | ✅ 读根 AGENTS.md + .claude/CLAUDE.md (自动) | ✅ 读根 CLAUDE.md | GitNexus MCP 可用 |
| Codex CLI | ✅ 读根 AGENTS.md (instructions 入口) | ➖ 无 CLAUDE.md | GitNexus 降级到 CLI |
| OpenCode | ✅ 读根 AGENTS.md (opencode/AGENTS.md 也可) | ➖ | OpenSpec skill 兼容 |
| OMP / Pi | ✅ 读根 AGENTS.md (若支持) | ➖ | 降级到基础工具 |
| Hermes Agent | ✅ 读根 AGENTS.md | ➖ | 兼容 |

**核心结论**: `AGENTS.md` (根) 是通用导航, **CLAUDE.md** 是 Claude Code 适配子集; 不需要复制多份。

### G.6 未验证部分 (透明声明)

- 🔶 4 条执行链 (SQL/事务/索引/启动) 的**具体函数名**未通过 GitNexus 查询验证 — 用源码 grep + 模块路径推导
- 🔶 "生产 FileStorage 仍是 HashMap" 引用 `src/lib.rs:49-63` 注释 — 未运行时验证
- 🔶 WalEntry 字段顺序未读完整 `wal_legacy.rs:80-160` 验证 (已读 line 83-113 确认结构)
- 🔶 实际 Gate 脚本退出码 (除 D8 外) 未运行验证
- 🔶 Cargo bin name 与 src/ 实际结构未 100% 对齐 — 已知现象

---

## H. 待批准修改清单 (Phase B)

### H.1 新增 (6 份)

| 路径 | 内容 | 大小估算 |
|---|---|---|
| `AGENTS.md` (替换根) | 100-150 行根入口 | ~6KB |
| `CLAUDE.md` (替换根) | Claude Code 适配子集 | ~3KB |
| `docs/agent-harness/INDEX.md` | 任务 → 源码/文档 | ~5KB |
| `docs/agent-harness/REPOSITORY_MAP.md` | 仓库结构核实版 | ~7KB |
| `docs/agent-harness/GITNEXUS_ARCHITECTURE_MAP.md` | 4 执行链 | ~5KB |
| `docs/agent-harness/AOCI_GUIDE.md` | AOCI 工作流 | ~3KB |
| `docs/agent-harness/DOCUMENTATION_INDEX.md` | 文档分类 + 冲突 | ~10KB |
| `docs/agent-harness/IMPLEMENTATION_TRUTH.md` | 5 级真实性 | ~3KB |

### H.2 修改 (5 份)

| 路径 | 修改 |
|---|---|
| `AGENTS.md` | **完整替换** (旧 355 行 → 新 ~150 行) |
| `CLAUDE.md` | **完整替换** (旧 76 行 → 新 ~80 行, 修正 Rust 2024 → 2021, REPL → sqlrustgo-cli) |
| `ARCHITECTURE_RULES.md` | 增加 1 行: "ARCHITECTURE 是事实快照, 不是 SSOT; SSOT 见 `docs/releases/v{当前}/STAGE.yaml`" |
| `.gitea/pull_request_template.md` | 增加 1 行: "**严禁** 推 GitHub / 用 `gh` CLI" |
| `docs/architecture/ARCHITECTURE.md` | 加 1 行: "已过时 (v1.2/2026-03-05); 见 REPOSITORY_MAP.md + INDEX.md" |

### H.3 不建议动

- 根 README.md — 是项目状态入口, 内容权威
- docs/governance/* — 不可降级
- ADR-001 ~ ADR-014 — 不可降级
- .gitnexus/* — 索引数据, 不手动改
- aoci.code.txt — AOCI 索引, 由模型维护
- Cargo.toml — 有意保留 3.12.0-fix-zombie (README §0 已说明)

### H.4 工具与 CI 检查

新增轻量级 `scripts/gate/check_agent_harness.sh`:
1. 根 AGENTS.md 行数 ≤ 150 (或 ≤ 200 with justification)
2. AGENTS.md 链向的所有文件存在
3. docs/agent-harness/ 7 文件存在
4. Cargo workspace member 数量与 REPOSITORY_MAP.md 一致
5. GitNexus `.gitnexus/meta.json` lastCommit 与 HEAD diff < 14 天 (警告级)
6. AOCI `.aoci/baseline.json` updated_at 与 HEAD diff < 14 天 (警告级)
7. 核心架构文档 (ARCHITECTURE.md) 标注 "version + date"
8. 无重复 ADR-006/007/008/010/011 主 ADR 外的 exception 文件 (warning)

**Phase B 实施前必须经用户批准**, 不修改现有 CI Gate。

---

## 结束 — Phase A 完成

✅ 全部 6 个子任务完成 (基线冻结 / AOCI / GitNexus / 文档审计 / 设计 / 产出)
✅ 候选文件输出到 `/tmp/sqlrustgo-harness-audit/` (临时目录, **不动**正式文件)
✅ 审计报告本文 (`AUDIT_REPORT.md`) 同样输出到临时目录
✅ Phase B (实际修改) **不开始**, 等用户批准