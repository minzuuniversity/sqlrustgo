# ADR-013: v3.10 Wired-Soak DDL + Wire Protocol 修复 RFC

> **Status**: PROPOSED (2026-06-20)
> **Deciders**: Hermes Agent (claude-macmini) + User
> **Date**: 2026-06-20
> **Branch scope**: `develop/v3.10.0` (新建; **NOT** `develop/v3.9.0`)
> **Supersedes**: None
> **Related**: [ADR-008 — Test Claim Transparency](ADR-008-test-claim-transparency.md), [`docs/SYSBENCH_WIRE_PROTOCOL_FIX_GUIDE.md`](../../SYSBENCH_WIRE_PROTOCOL_FIX_GUIDE.md), [`docs/SYSBENCH_OLTP_SUPPORT_MATRIX.md`](../../SYSBENCH_OLTP_SUPPORT_MATRIX.md), [`docs/2026-06-17-SQLRUSTGO-SYSBENCH-COMPATIBILITY-PLAN.md`](../../2026-06-17-SQLRUSTGO-SYSBENCH-COMPATIBILITY-PLAN.md), [`docs/releases/v3.9.0/SOAK_72H_LIVE_STATUS_2026-06-19.md`](../../releases/v3.9.0/SOAK_72H_LIVE_STATUS_2026-06-19.md), [`docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md`](../../audit/status/2026-06-06-test-authenticity-analysis-v390.md)

## Context

### 当前 soak 路径的两条路线

v3.9.0 RC7 在 PR #3546 (run_72h_soak.sh) → #3549 (run_72h_soak_v2.sh, in-process) → #3550 (v2 guardian) → #3559 (168h dispatch) 一连串 PR 后，已形成两条 soak 路径：

| 路径 | 协议 | 工具 | 现状 |
|---|---|---|---|
| **v1 (deprecated)** | MySQL wire | `sysbench oltp_read_write` | **失败**: sysbench prepare 调 `CREATE DATABASE sbtest`，v3.9.0 引擎**不支持 CREATE DATABASE**，握手后崩在 "Malformed packet" |
| **v2 (current)** | in-process | `sqlrustgo-mysql-server soak --qps 1.0` | **工作中**: Z6G4 上 PID 104549/104564/107680, started 2026-06-19 13:05:17 UTC, ETA 2026-06-22 13:05:17 UTC |

**两路径的核心差异**：

```
v1 (wire, sysbench)  : sysbench → MySQL wire → serve → engine.execute()
v2 (in-process, soak) : sqlrustgo-mysql-server soak --qps N → engine.execute()  [no MySQL wire]
```

### v2 路径的根本问题 — 偏离生产现实

v2 in-process 路径**无法模拟生产环境**：
1. **不验证 MySQL wire protocol** — 生产环境所有客户端通过 wire 协议连接
2. **不验证握手/认证/capability negotiation** — sysbench/mysql-cli/JDBC/ODBC 都会走这些路径
3. **不验证 SQL 解析器对 wire-层 COM_QUERY / COM_STMT_PREPARE 派发的兼容** — 真实客户端用 prepared statement，v2 不覆盖
4. **不验证 Connection pool / 多并发连接** — v2 是单进程内部循环
5. **不验证 client 端实际错误处理** — 例如超时、重连、错误码返回

`SOAK_72H_LIVE_STATUS_2026-06-19.md` 显式承认："v1 used sysbench oltp_read_write against the MySQL wire protocol. sysbench prepare uses CREATE DATABASE. sqlrustgo v3.9.0 does not support CREATE DATABASE (early DDL gap). Result: sysbench died immediately."

**当前 72h soak 通过了 "sqlrustgo 引擎能跑 180s" 的测试，但没有通过 "sqlrustgo 是 MySQL 兼容服务器" 的测试**。后者是 v3.9.0 GA 的核心承诺。

### v3.9.0 GA 阻塞关系

按 v3.9.0 治理 10 原则:
- **P5 (Governance > Features)**: 治理是 GA 阻塞
- **P4 (Architecture Drift is a Release Blocker)**: v3.9.0 已 form-only 9/9 PASS (含 D9 Shadow Mainline detection)
- 当前 Z6G4 in-process soak 是 "v3.9.0 候选 GA 证据", 不是 "v3.9.0 满足设计意图的证据"

**GA 路径分叉**:
- 路线 A: 维持 v2 in-process → **GA 2026-12-15 推进**，但 wired mode 留给 v3.10
- 路线 B (本文): v3.10 补齐 wired → 推迟 GA 或重定 GA 日期

**当前选择**: **路线 A** (Z6G4 in-process 跑到底，wired E2E 推迟到 v3.10)。本 ADR 是路线 B 的 RFC，明确推迟的范围与依赖。

### Z6G4 72h in-process 进度 (2026-06-20 调研)

| 进程 | PID | 启动 | 状态 |
|---|---|---|---|
| `serve` (port 13306) | 104549 | 13:05:17 UTC | 健康 |
| `soak` (in-process) | 104564 | 13:05:17 UTC | 健康 |
| `guardian` (watchdog) | 107680 | 13:09:53 UTC | 健康 |
| ETA | — | 2026-06-22 13:05:17 UTC | 还剩 ~58h |

**不可打断**。本 ADR 任何 v3.10 实施不能影响 Z6G4 上既有的 v3.9.0 binary。

## Decision

**v3.10 milestone 实施 wired-soak E2E 路径，分 4 个独立 PR，每个 PR 都有独立 gate 验证**：

| PR | 范围 | Gate 验证 | 估时 |
|---|---|---|---|
| **v3.10.0-PR1** | parser: 加 `CREATE DATABASE` / `DROP DATABASE` / `USE` AST 节点 + 语法解析 | 新增 `check_g16_ddl_syntax.sh`, 跑 ~50 解析测试 | 1 周 |
| **v3.10.0-PR2** | catalog: 引入 `Database` 抽象 + `Catalog -> Database -> Schema -> Table` 4 层重构 | `check_p14_upgrade_test.sh` + 全 storage 路径回归 | 2 周 |
| **v3.10.0-PR3** | executor: `execute_create_database` / `execute_drop_database` / `execute_use` 实现 + storage 路径 `data/<db>/<table>.tbl` 改造 | `check_arch_invariants.sh` + 194 个 fixture test 回归 | 2 周 |
| **v3.10.0-PR4** | mysql-server wire: 修 `seq = incoming_seq + 1` (PHASE 2) + capability flags (PHASE 3) + auth packet | `check_g12_sysbench.sh` 从 `#[ignore]` 摘除 + 真跑 sysbench prepare/run | 1-2 周 |

**总估时**: 6-8 周全职工程师工作量。

**v3.9.0 GA 不受本 ADR 影响** — 走 in-process 路径，2026-12-15 GA 目标不变。

**v3.10 GA 目标**: 2027-Q1 (粗估)，实际需 PR1-PR4 完成 + 1 周回归期。

## Rationale (为什么这样)

### 为什么不一次性合并 4 个 PR

| 风险 | 影响 |
|---|---|
| 单 PR 跨越 parser + catalog + executor + wire = 跨子系统级 diff | review 困难，merge 冲突 |
| 一次性大改 194 fixture test 路径 | 回归周期长，bug 难定位 |
| 触发 P4 Architecture Drift 紧急审计 | 阻塞 v3.9.0 GA 流程 |
| 失去 v3.9.0 RC7 已有 in-process soak 经验复用 | 已运行的 PID 104549 等变成 legacy |

**分 PR 的好处**:
- 每个 PR 独立可回滚
- 每个 PR 的 gate 单独 PASS 才进下一 PR
- v3.9.0 GA 期间在 `develop/v3.10.0` 上并行开发，不污染 `develop/v3.9.0`
- PR4 (wire 修复) 失败不影响 PR1-3 已合入的 DDL 基础设施

### 为什么 CREATE DATABASE 是基础设施而非可选

sysbench oltp_read_write prepare 阶段必须 `CREATE DATABASE sbtest`。**没有 CREATE DATABASE，sysbench 永远无法 connect**。这是**所有 wired E2E 路径的前置条件**，不是 nice-to-have。

### 为什么不暂存 DDL (workaround) sysbench `--mysql-db`

sysbench 的 `--mysql-db` 标志只影响 runtime connect 时的 `USE sbtest`，不绕过 prepare 阶段。prepare 阶段仍会调 `CREATE DATABASE sbtest` (如果不存在) → 必崩。

唯一可绕过的方案是手工 pre-create sbtest，但需要绕过 sysbench prepare 全过程，**等同于不用 sysbench prepare**，失去 sysbench 行业标准的可重复性。

### 为什么不只补 wire 不补 DDL

| 选项 | 现状 | 真实 E2E 可达? |
|---|---|---|
| 只补 DDL 不补 wire | CREATE DATABASE 可用，但握手仍错位 | ❌ sysbench 仍崩在 "Packet sequence number wrong" |
| 只补 wire 不补 DDL | 握手修好，但 CREATE DATABASE 仍 NotImplemented | ❌ sysbench 仍崩在 CREATE DATABASE |
| **同时补 DDL + wire** | 全栈到位 | ✅ sysbench prepare 跑通 → 真 E2E |

## Consequences

### 正面

- **v3.10 GA 后真 E2E 路径** = 生产环境 wire 协议可重现
- **sysbench oltp_read_write 真接入** = 行业标准可比的 benchmark
- **JDBC/ODBC 客户端** = wire 协议修复后自动受益
- **回归基线** = 用 sysbench 固定 workload 做长期回归
- **审计可见** = 走 P4 Drift Detection + P10 PR Chain 完整流程

### 负面 / 风险

| 风险 | 严重度 | 缓解 |
|---|---|---|
| **storage 路径重构破坏 194 fixture test** | 🔴 HIGH | PR3 期间保留旧路径 compat 层 (`MIGRATION_DB=off` 时仍 `data/<table>.tbl`)，feature flag 控制 |
| **WAL per-db 路径改造可能触发 wal-verification 形式化证明重做** | 🟠 MED | 维持单 WAL 文件，db 写 entry 增加 `db_id` 字段而非拆分文件 |
| **fixture 重生成 (tests/data/tpch-sf*/) 在 PR3 之后可能全失效** | 🟠 MED | PR3 commit 一次性重生成所有 fixture + commit sha256 验证 (用 `tests/baseline/tpch_hashes_v380.json` 现成机制) |
| **Z6G4 in-process 72h soak 中途 binary 失配** | 🔴 HIGH | v3.10 工作在 `develop/v3.10.0` 隔离分支；Z6G4 上 binary 用 `develop/v3.9.0` 的 release build，**不动** |
| **wire 协议修复可能暴露新握手 bug** | 🟠 MED | 先在本地 `mysql-cli` 验证 handshake，再 sysbench，最后 24h soak |
| **sysbench binary 部署/版本** | 🟡 LOW | 使用 `brew install sysbench` (已在本机 `/opt/homebrew/bin/sysbench` 验证) + Z6G4 上 `apt install sysbench` |
| **回归期 1 周** | 🟠 MED | 安排 Z6G4 24h 短期 soak + Mac Mini 30min smoke + CI 完整 cargo test |

### 治理影响

- **触发 P4 (Architecture Drift)** — 但仅在 v3.10 范围内，不影响 v3.9.0 GA
- **触发 P8 (Negative Evidence)** — 必须保留所有 PR 失败的 evidence 在 `docs/audit/status/`
- **触发 P10 (PR Chain Completeness)** — 4 PR 须明确链 `Issue #3265` (72h soak) → 分 PR 链
- **可能影响 v3.9.0 G12 gate** — `scripts/gate/check_g12_sysbench.sh` 当前是 `#[ignore]` 状态 (baseline 缺失)，v3.10 完成后必须 unignore

### 文档同步 (与本 RFC 一并提交)

| 文档 | 改什么 |
|---|---|
| `docs/releases/v3.9.0/SOAK_72H_LIVE_STATUS_2026-06-19.md` | 状态行加 "Wired E2E 推迟到 v3.10, see ADR-009" |
| `docs/releases/v3.9.0/V390_COMPREHENSIVE_ASSESSMENT.md` | 加 v3.10 wired-soak milestone 表 |
| `docs/ROADMAP.md` | 加 v3.10 行 |
| `wiki SQLRustGo-Overview` (gitea) | 同 |
| `wiki Governance-System` | 提一句 "v3.10 wired 路径遵循本 ADR-009" |
| `docs/SYSBENCH_OLTP_SUPPORT_MATRIX.md` | 加 "待 v3.10 PR4 实施" 标记 |

## Implementation Plan (待 PR 启动后细化)

### PR1 — Parser CREATE/DROP DATABASE/USE

```
crates/parser/src/parser.rs:
+  Statement::CreateDatabase(CreateDatabaseStatement { name, if_not_exists })
+  Statement::DropDatabase(DropDatabaseStatement { name, if_exists })
+  Statement::Use(UseStatement { name })

crates/parser/src/lib.rs:
+  re-export new types

crates/parser/src/token.rs:
+  DATABASE, USE tokens

tests/parser_ddl_test.rs (新):
+  parse "CREATE DATABASE foo"
+  parse "CREATE DATABASE IF NOT EXISTS foo"
+  parse "DROP DATABASE foo"
+  parse "USE foo"
```

Gate: `scripts/gate/check_g16_ddl_syntax.sh`

### PR2 — Catalog Database 抽象

```
crates/catalog/src/database.rs (新):
+  pub struct Database { name, schemas: HashMap<String, Schema> }
+  pub fn add_schema, get_schema, has_schema, remove_schema, schema_names

crates/catalog/src/catalog.rs:
+  catalog.databases: HashMap<String, Database>
+  catalog.current_database: String
+  fn create_database, drop_database, use_database

crates/catalog/src/lib.rs:
+  pub mod database
+  pub use database::Database

crates/catalog/src/schema.rs:
+  schema.db_name: String  // 引用父 database
```

Gate: `scripts/gate/check_p14_upgrade_test.sh` + 全 storage 路径兼容

### PR3 — Executor DDL 路径 + storage 路径

```
crates/executor/src/lib.rs:
+  fn execute_create_database(stmt) -> Result
+  fn execute_drop_database(stmt) -> Result
+  fn execute_use(stmt) -> Result (mutate engine state)
+  current_database 上下文 (在 Engine / Session 中)

crates/storage/src/file_storage.rs:
+  save_table_path(table, db) -> PathBuf 改为 data/<db>/<table>.tbl
+  feature flag: MIGRATION_DB=off 退回旧路径

crates/storage/src/recovery_engine.rs:
+  bulk_force_insert 加 db 参数
+  WAL entry 加 db_id 字段

tests/data/tpch-sf*/: 全 fixture 重生成 + tests/baseline/tpch_hashes_v380.json 更新
```

Gate: `cargo test --all-features` 全 194 fixture test 回归 + `check_arch_invariants.sh`

### PR4 — mysql-server wire 协议修复

```
crates/network/src/lib.rs (PHASE 1):
+  debug instrumentation: 完整 handshake packet header trace

crates/network/src/lib.rs (PHASE 2):
+  seq number 状态机: seq = incoming_seq + 1, 禁止 hardcode

crates/mysql-server/src/lib.rs (PHASE 3):
+  capability flags: CLIENT_PROTOCOL_41, TRANSACTIONS, SECURE_CONNECTION
+  auth packet: mysql_native_password 支持

scripts/gate/check_g12_sysbench.sh:
+  un-#[ignore] `tests/oracle_g12_sysbench.rs`
+ 跑真 sysbench prepare + oltp_read_write 短跑
```

Gate: `tests/oracle_g12_sysbench.rs` 真 PASS (不再是 `#[ignore]` 的 trivial-pass)

## Alternatives Considered

### A. 维持 v2 in-process, 跳过 wired E2E

**拒绝原因**: 不满足"GA 是 MySQL 兼容服务器"承诺。Z6G4 72h in-process 只证明引擎能跑，不证明 wire 协议可用。

### B. 仅做 wire 修复, 跳过 DDL

**拒绝原因**: sysbench prepare 仍崩在 CREATE DATABASE，等于没修。

### C. 绕开 sysbench, 用 hand-written mysql client

**拒绝原因**: 失去 sysbench 行业标准可重复性。每个新客户端库都要手写 harness。

### D. 整个 v3.9.0 重写以包含 DDL + wire

**拒绝原因**: 触发 P4 阻塞 + GA 推迟 6+ 月 + 现有 4 open issues 全部受连累。

### E. 拆 4 PR, 推迟 GA

**采纳** (本 ADR Decision) — 与治理 10 原则的 P5 (Governance > Features) 一致: 不为追求"完美 wired 路径"牺牲 v3.9.0 GA 节奏。

## Validation

### 当前 (v3.9.0 RC7) 状态

- v2 in-process 72h soak 跑在 Z6G4, 预计 2026-06-22 13:05 UTC 完成
- in-process 测试显示 100% query 成功, RSS 稳定, WAL 0 MB
- **但**: 不代表 wire 协议可用

### v3.10 完成判据 (Acceptance Criteria)

- [ ] PR1-PR4 全部 merged 到 develop/v3.10.0
- [ ] `cargo test --all-features` 全绿, 0 regressions vs v3.9.0 baseline
- [ ] `tests/oracle_g12_sysbench.rs` 默认 (non-`#[ignore]`) PASS
- [ ] `scripts/gate/check_g12_sysbench.sh` 默认 PASS (调用真 sysbench binary)
- [ ] Z6G4 上 wired-soak 24h 跑出 RSS/FD/CPU/p99 数据, 与 in-process 数据可比
- [ ] `tests/baseline/g12_sysbench_baseline.json` 填入真数据 (不再是 placeholder)
- [ ] `tests/baseline/ignore_registry.json` 中 sysbench 相关 `#[ignore]` 全部移除

### Negative Evidence (P8 要求)

保留所有失败的 PR 尝试:
- `docs/audit/status/v3.10-PR{N}-failure-{date}.md` 每次失败都记
- 不允许 "fix forward" — 失败的 PR 必须先关闭才能开新的

## Open Questions

1. **v3.10 仓库命名**: 是否仍叫 sqlrustgo, 还是分叉 (sqlrustgo-server + sqlrustgo-engine)?
2. **wire 协议版本**: MySQL 5.7 还是 8.0? 当前 `mysql-server` 默认 5.7
3. **sysbench 版本**: v1.0 vs v2.0? 当前 1.0 (apt), 2.0 有 Lua 脚本
4. **DDL 与 system_tables 关系**: information_schema 是否在 v3.10 范围?
5. **USE 上下文跨连接**: connection-level 还是 session-level?

## References

- PR #3546 (run_72h_soak.sh) — v1 第一次尝试, sysbench 失败
- PR #3549 (run_72h_soak_v2.sh) — v2 in-process, 当前 GA 路径
- PR #3550 (v2 guardian) — watchdog
- PR #3559 (168h dispatch) — auto-fire 168h after 72h
- Issue #3265 — 主 issue: 72h real wall-clock soak
- Issue #3225 / #3226 / #3228 / #3229 — soak-related issues
- `docs/SYSBENCH_WIRE_PROTOCOL_FIX_GUIDE.md` — wire 协议 3 阶段修复指南
- `docs/SYSBENCH_OLTP_SUPPORT_MATRIX.md` — sysbench 兼容矩阵
- `docs/2026-06-17-SQLRUSTGO-SYSBENCH-COMPATIBILITY-PLAN.md` — 兼容性计划
- `docs/releases/v3.9.0/SOAK_72H_LIVE_STATUS_2026-06-19.md` — 当前 Z6G4 状态
- `crates/bench/src/dataset/sbtest_schema.rs` — sbtest schema 定义 (v3.10 可直接用)

---

*Authored by: Hermes Agent (claude-macmini) as ADR-009 RFC, 2026-06-20.*
*Per project policy: every claim in this ADR is grounded in observed evidence (commit SHA, file path, PID, or measured metric).*
*Status: PROPOSED — awaiting Hermes + User review before PR1 kickoff.*
