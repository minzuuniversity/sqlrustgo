# AGENTS.md — SQLRustGo Agent 入口

> **适用范围**: SQLRustGo 仓库根目录; 所有 AI 编程 Agent (Claude Code / Codex / OpenCode / OMP / Pi / Hermes 等)
> **基线 Commit**: `583050bc63` (HEAD) / `develop/v4.1.0` (`b513f56db1`)
> **当前开发线**: v4.1.0 DRAFT (NOT YET ALPHA); v4.0.0 GA 已撤销
> **沟通语言**: 中文 (除非用户明确要求其他语言)

---

## 1. 最高原则 (P0, 不可降级)

1. **Anti-Fabrication**: 不引文档 claim 当 PASS; 任何"完成/通过"必须含 `source_agent + source_run + timestamp + evidence_hash`。详见 `docs/governance/ANTI_FABRICATION_POLICY.md` + `docs/governance/adr/ADR-001-truthfulness-framework.md`。
2. **Version Truth**: 当前事实以 `docs/releases/v4.1.0/STAGE.yaml` + 仓库 git HEAD 为准; Cargo.toml `version = "3.12.0-fix-zombie"` 是有意保留的发布口径 (README §0 已说明)。
3. **Gate-First**: CI 必须 `set -euo pipefail` + 失败即 exit 1。Gate 脚本在 `scripts/gate/`。Hard Gate 列表见 `docs/governance/GATE_CONDITIONS.md`。
4. **Mod-Verification**: SQLRustGo 实现分 5 状态 — Implemented / Integrated / Tested / Verified / Release-ready。严禁把空函数或绕路实现声称"完成"。详见 `docs/agent-harness/IMPLEMENTATION_TRUTH.md`。
5. **Production Path**: 修改任何模块前须确认其生产调用链 (不要只看测试路径)。FileStorage 当前生产实现在 `crates/storage/src/file_storage.rs`, 而 `BufferPool/Page/BPlusTree` 被 `src/lib.rs:49-63` 显式标为 "Test-only capability"。

---

## 2. 启动事实核验 (Phase 0)

任何 Agent 在第一次接触本仓库时, 必须先验证 (只读):

```bash
git rev-parse HEAD                            # 当前 commit
git branch --show-current                     # 当前分支
git status --short                            # 工作目录状态
git log -1 --pretty=fuller                    # 当前 commit 身份
cat docs/releases/v4.1.0/STAGE.yaml | head -50 # 当前发布阶段
bash scripts/gate/check_arch_sem_debt.sh     # D8 Gate (硬门禁)
```

若任何工具不可用 → 降级到 `rust-analyzer / rg / cargo metadata / 源码 / docs/` (见 §6)。

---

## 3. 仓库导航

完整结构 → [`docs/agent-harness/REPOSITORY_MAP.md`](docs/agent-harness/REPOSITORY_MAP.md)
文档索引 → [`docs/agent-harness/DOCUMENTATION_INDEX.md`](docs/agent-harness/DOCUMENTATION_INDEX.md)
任务 → 源码映射 → [`docs/agent-harness/INDEX.md`](docs/agent-harness/INDEX.md)

### 3.1 一级目录

| 路径 | 用途 |
|---|---|
| `crates/` | 45 个 workspace crate (parser, planner, executor, storage, transaction, server 等) |
| `src/` | 根 crate library (执行引擎顶层; engine_*.rs 分散) |
| `tests/` | 单元 + 集成 + E2E 测试, 有 `unit/`, `integration/`, `e2e/`, `gate/`, `oracle/`, `disabled/` |
| `docs/` | 所有文档 (governance/adr, architecture, releases, superpowers, ai_collaboration) |
| `scripts/gate/` | Hard Gate 脚本 (D1-D16) |
| `.gitea/workflows/` | Gitea Actions CI 配置 |
| `tools/xtask`, `tools/compat-runner` | xtask 与兼容性测试 runner |

### 3.2 Git Remote (强制只走 Gitea)

```bash
gitea252 → http://192.168.0.252:3000/openclaw/sqlrustgo.git (主, 可写)
gitcode/gitee/github → 只读镜像, 禁止 push
```
**严禁** `gh` CLI 或 GitHub。详见 `docs/gitea-only.md` + 项目 CLAUDE.md。

---

## 4. GitNexus 使用原则

GitNexus 索引 (`.gitnexus/`) 提供 103k+ 符号、157k 关系、300 个执行流的静态分析。

- **MUST**: 编辑任何符号前先 `gitnexus_impact({target, direction: "upstream"})` 报告 blast radius
- **MUST**: commit 前 `gitnexus_detect_changes()` 验证影响范围
- **MUST**: HIGH/CRITICAL 风险必须警告用户
- **DO NOT**: 用 find/replace 重命名符号 — 用 `gitnexus_rename` (它理解调用图)
- **已知问题**: 当前索引对应 commit `b1e9a98ee6` (2026-10-08), 落后 HEAD 1 周+。FTS 索引 (database version 42 vs build 40) 失效, 需降级到符号图 + 源码。运行 `npx gitnexus analyze` 可重建。
- 工具参考: `.claude/skills/gitnexus/{exploring,impact-analysis,debugging,refactoring,guide,cli}/SKILL.md`

---

## 5. AOCI 使用原则

AOCI (`aoci.code.txt`, 1.4MB, v2 protocol, FRAS-2) 提供仓库级语义索引, 给每个托管对象 (文件/表) 分配 Entry + F/R/A/S 标签。

- **DO NOT**: 用路径/AST/正则/模板生成语义 — 必须由模型基于实际证据作者
- **DO NOT**: 写 `origin=host_model` 或伪造 receipt
- 启动流程: `aoci_rules` → `aoci_overview` (或 `check_only=true` 查 checkpoint) → 任务 → `aoci_maintain` (变更后) → `aoci_update_entry` (提交批次)
- **context_compaction 时**: 必须强制刷新 (mandatory reload rule) — 不可用历史 Memory 补
- 详见 AGENTS.md §AOCI Repository Cognition (line 253-355)

---

## 6. 任务 → 源码 → 文档 映射

见 [`docs/agent-harness/INDEX.md`](docs/agent-harness/INDEX.md)。常用入口:

| 任务 | 主入口 |
|---|---|
| SQL 解析 | `crates/parser/src/{lexer,parser,token}.rs` + `tests/integration/sql/` |
| 查询计划/优化 | `crates/{planner,optimizer}/src/` + `src/planner/` |
| 执行 | `crates/executor/src/` + `src/engine_*.rs` |
| 存储/BufferPool/B+Tree | `crates/storage/src/` (注意: BufferPool 当前是 test-only) |
| 事务/WAL | `crates/transaction/src/` + `docs/governance/wal/WAL_CONTRACT.md` |
| MySQL 协议 | `crates/{mysql-server,mysql-client}/` |
| RAG/向量 | `crates/{rag,vector}/` |
| 图/GMP | `crates/{graph,gmp}/` |
| 测试/Gate | `tests/integration/`, `scripts/gate/`, `tests/gate/` |

---

## 7. 最小开发闭环

```
1. 查 gitnexus_impact + 读最小源码
2. 写测试 (RED): cargo test --test <name> --all-features
3. 最小实现 (GREEN)
4. cargo clippy --all-features -- -D warnings  (必须 0 警告)
5. cargo fmt --check --all
6. 跑 Hard Gates: bash scripts/gate/check_arch_sem_debt.sh 等
7. cargo test --all-features --no-fail-fast
8. gitnexus_detect_changes 验证
9. commit + push (gitea252 only) + Gitea PR
10. PR body 必填 5 类文档 (PR_TEMPLATE.md) + Test Plan + Evidence
```

---

## 8. 严格禁止

| ❌ | 行为 | 依据 |
|---|---|---|
| 1 | 空函数/默认返回值冒充实现 | AFP Type A |
| 2 | 模块标"集成"但生产路径绕过 | AFP Type B |
| 3 | 删除断言/跳过测试/修改参考结果来 PASS | AFP Type C |
| 4 | `cargo test --no-run` 当 PASS | Hard Gate |
| 5 | SKIP/TIMEOUT/XFAIL 计 PASS | ADR-008 |
| 6 | 未跑测试报"测试通过" | ADR-001 |
| 7 | 改 Git 分支保护/Gate 阈值未授权 | CI_GATE_CONTRACT |
| 8 | 只 Mock 测试, 忽略真客户端/持久化测试 | ADR-001 |
| 9 | Agent 自我签发验收 | ADR-014 (multi-AI) |
| 10 | 无证据声"修复完成/可发布" | ADR-001 |
| 11 | 推 GitHub / 用 `gh` CLI | 项目 CLAUDE.md |
| 12 | 改 docs/ 任何文件不走 5 步流程 | DOC_CHECK_CORRECTION_RULES |
| 13 | 关闭无 PR 关联的 Issue | ISSUE_CLOSING_VERIFICATION |
| 14 | 不读 7+1 governance 就动手 | AGENTS §强制阅读清单 |

---

## 9. 证据与报告要求

任何变更/声明必须附 5 字段: `source_agent + source_run + timestamp + evidence_hash + conflict_resolution`。详见 ADR-014 + AFP §5.4。

- 任务完成 → 提交 PR + Test Plan + Evidence 文件
- 文档改动 → 5 步流程 (DOC_CHECK_CORRECTION_RULES.md)
- Issue 关闭 → 5 步验证 (ISSUE_CLOSING_VERIFICATION.md, 含 HTTP 405 workaround)
- Gate 失败 → 必须修复 + 重跑 + 留 commit 证据

---

## 10. 必读 governance 清单 (P0)

每个 Agent **动手前必须读完** (`docs/governance/` 下):

1. `ANTI_FABRICATION_POLICY.md` — Type A/B/C/D 4 类违规
2. `adr/ADR-001-truthfulness-framework.md` — G-01 ~ G-10
3. `ISSUE_CLOSING_VERIFICATION.md` — 关闭前 5 步
4. `DOC_CHECK_CORRECTION_RULES.md` — 改文档 7 步
5. `AI_COLLABORATION.md` — §1.1 角色 + §5.5 多 AI 协调
6. `GATE_CONDITIONS.md` — G1-G16 分类
7. `adr/ADR-008-test-claim-transparency.md` — P16 test integrity
8. `adr/ADR-014-multi-ai-coordination.md` — 5 evidence fields 协议

**铁律**: 7+1 份读完才能写代码; 改任何 1 份 governance 必须实跑 P11-P16 验证。

---

## 11. 详细文档入口

- Agent 导航: [`docs/agent-harness/INDEX.md`](docs/agent-harness/INDEX.md)
- 仓库结构: [`docs/agent-harness/REPOSITORY_MAP.md`](docs/agent-harness/REPOSITORY_MAP.md)
- GitNexus 架构: [`docs/agent-harness/GITNEXUS_ARCHITECTURE_MAP.md`](docs/agent-harness/GITNEXUS_ARCHITECTURE_MAP.md)
- AOCI 指南: [`docs/agent-harness/AOCI_GUIDE.md`](docs/agent-harness/AOCI_GUIDE.md)
- 文档索引: [`docs/agent-harness/DOCUMENTATION_INDEX.md`](docs/agent-harness/DOCUMENTATION_INDEX.md)
- 实现真实性: [`docs/agent-harness/IMPLEMENTATION_TRUTH.md`](docs/agent-harness/IMPLEMENTATION_TRUTH.md)
- 旧 AGENTS.md (26KB, 已含 governance 清单 + GitNexus + AOCI 详细章节): 根 `AGENTS.md` (本文件替代其主入口, 但完整 AOCI 协议章节与 governance 链接仍保留在 v3 候选 AGENTS_legacy.md)