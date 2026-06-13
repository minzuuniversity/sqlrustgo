# TPC-H 13 个 In-Process 测试 → Wire Protocol E2E 改造设计

> **Date**: 2026-06-13
> **Author**: Sisyphus (Sonnet 4.5)
> **Branch**: `feature/tpch-e2e-migration` (从 `develop/v3.9.0` fork)
> **Worktree**: `.worktrees/tpch-e2e-migration`
> **Trigger**: 用户反馈 "继续，采用 mysql-server 以 e2e 方式来完成所有的 tpch 测试，不要写独立的测试程序来进行集成测试、性能测试和稳定性测试"
> **依据**: `docs/governance/DOC_CHECK_CORRECTION_RULES.md` v1.0.0 (5-step)
> **关联文档**: `docs/releases/v3.9.0/E2E_MIGRATION_MASTER_PLAN.md` (已存在的 P0-P3 计划)

---

## 1. 背景与目标

### 1.1 问题陈述

仓库当前有 **14 个** `tests/tpch_*.rs` 文件，其中 **13 个仍是 in-process**（直接调用 `ExecutionEngine::new(MemoryStorage)` 或 `bulk_insert_records`），**1 个混合模式**（`tpch_wire_smoke.rs`）。

**违反 `E2E_MIGRATION_MASTER_PLAN.md` 原则**：
> 集成测试、性能测试、稳定性测试 **必须** 使用 `sqlrustgo-mysql-server` 作为后端（wire protocol）。**禁止** 使用单独编译的测试程序直接调用 `ExecutionEngine` 或 `MemoryStorage`。

### 1.2 目标

将 13 个 in-process 测试改造为基于 `MySqlTestClient` 的 wire protocol 测试。**零新增 test entry 文件**（每个 `.rs` 仍是 `cargo test --test <name>` 的入口）。

### 1.3 约束（来自用户决策）

| 决策 | 值 | 影响 |
|------|-----|------|
| SF 覆盖 | SF=0.001 + SF=0.1 双规模 | 沿用现有 `tests/data/tpch-sf001/` 和 `tests/data/tpch-sf01/` fixture |
| 22 查询覆盖 | 22/22 全部 E2E（含已知不通过） | 已知不通过 query 加 `[KNOWN-ISSUE]` 标签，不 panic |
| 集成/性能/稳定性 | 同 test entry 多个 `#[test]` 函数 | 不新增 test 文件 |
| perf/stability 输出 | 默认全跑 + 超时 | `#[ignore]` 不用；用 `set_timeouts(60s)` 内部控制 |
| 遗留文件处理 | 保留 + 新增 E2E | 现有 5 个 wire test (`tpch_22_queries_wire_test.rs` 等) 不动；只改造 13 个 in-process |

### 1.4 不在范围

- ❌ 不删除任何现有 in-process 测试（保留为 5 个 wire test 的补充 smoke）
- ❌ 不修改 baseline JSON 文件 (`Q*_three_way.json`, `Q*_sf01_baseline.json` 等)
- ❌ 不修改 22 个 TPC-H SQL 查询（`queries/q*.sql`）
- ❌ 不修改 schema DDL（沿用各测试文件已定义的 DDL）
- ❌ 不修改 wire test 已有 5 个文件
- ❌ 不修改 `tests/diag_*.rs` 和 `tests/operators/*.rs`（这些是组件级诊断/单元测试，in-process 合理）

---

## 2. 13 个 in-process 测试清单

按用途分 4 组：

### A 组 (5 个)：功能/集成测试

| # | 文件 | 行数 | SF | 关键功能 |
|---|------|------|-----|----------|
| 1 | `tests/tpch_gate_test.rs` | 391 | 0.1 | G1 gate 22 query 跑通 |
| 2 | `tests/tpch_full_22_test.rs` | 391 | 0.01 | 22 query 全跑通 |
| 3 | `tests/tpch_sf01_inprocess_test.rs` | 217 | 0.1 | SF=0.1 sanity |
| 4 | `tests/tpch_bug_regression_test.rs` | ? | 0.001 | 3 bugs 回归标记 |
| 5 | `tests/tpch_value_correctness_test.rs` | ? | 0.001 | 数值正确性 gate |

### B 组 (3 个)：值对比测试

| # | 文件 | 行数 | SF | 对比目标 |
|---|------|------|-----|----------|
| 6 | `tests/tpch_value_test_v2.rs` | ? | 0.001 | SQLite `Q*_three_way.json` |
| 7 | `tests/tpch_sf01_22_vs_sqlite.rs` | ? | 0.1 | SQLite `Q*_sf01_baseline.json` |
| 8 | `tests/tpch_sf01_22_vs_3engines.rs` | 491 | 0.1 | MariaDB + PostgreSQL + SQLite (cell-level) |

### C 组 (4 个)：性能测试

| # | 文件 | 行数 | SF | 输出 |
|---|------|------|-----|------|
| 9 | `tests/tpch_sf01_perf_baseline_test.rs` | ? | 0.1 | `tests/data/tpch-sf01/perf_v390_baseline.json` |
| 10 | `tests/tpch_q8_q21_perf_regression_test.rs` | ? | 0.1 | perf regression budget |
| 11 | `tests/tpch_per_query_timeout_test.rs` | ? | 0.001 | per-query timeout (default 30s) |
| 12 | `tests/tpch_hash_test.rs` | ? | 0.001 | SHA-256 hash 对比 |

### D 组 (1 个)：Q9 audit

| # | 文件 | 行数 | SF | 对比目标 |
|---|------|------|-----|----------|
| 13 | `tests/tpch_q9_audit.rs` | ? | 0.01 | SQLite ground-truth `/tmp/tpch_sf01_audit.db` |

---

## 3. 架构设计

### 3.1 新增共享 helper 模块

**`tests/common/tpch_wire_harness.rs`** (新增, ~200 行):

```rust
//! TPC-H wire-protocol test harness
//!
//! 共享给 13 个 in-process → wire 改造的 tests/tpch_*.rs。
//! 不导出独立 test entry；只是 helper functions。

use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, EphemeralHandle};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const SF001_DIR: &str = "tests/data/tpch-sf001";
pub const SF01_DIR: &str = "tests/data/tpch-sf01";

pub const SCHEMA_DDL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity REAL NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
];

pub const TABLES: &[&str] = &[
    "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
];

/// 启动 ephemeral server, 加载 SF=0.001 fixture, 返回 MySqlTestClient
pub fn start_sf001() -> MySqlTestClient;

/// 启动 ephemeral server, 加载 SF=0.1 fixture, 返回 MySqlTestClient
pub fn start_sf01() -> MySqlTestClient;

/// 通过 LOAD DATA LOCAL INFILE 加载指定 SF 的所有 8 个 .tbl
pub fn load_fixture(client: &mut MySqlTestClient, fixture_dir: &str);

/// 跑单个 query, 计时, 返回 (rows, elapsed)
pub fn run_query_timed(
    client: &mut MySqlTestClient,
    sql: &str,
    timeout_s: u64,
) -> (Result<Vec<Vec<String>>, String>, Duration);

/// 读 SQLite/MariaDB/PG baseline JSON
pub fn read_baseline(path: &Path) -> serde_json::Value;

/// cell-level 对比 (支持 float tolerance)
pub fn compare_cells(
    actual: &[Vec<String>],
    baseline: &serde_json::Value,
    float_tol: f64,
) -> Result<(), String>;
```

### 3.2 13 个文件统一改造模式

```rust
// 原来的 in-process 模式:
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
fn setup() -> ExecutionEngine<MemoryStorage> { ... }
fn run() {
    let engine = setup();
    let rows = engine.execute(sql).unwrap();
    // 断言
}

// 改造后 wire 模式:
use common::MySqlTestClient;
mod tpch_wire_harness;
use tpch_wire_harness::*;

fn setup() -> MySqlTestClient { start_sf001() }  // 或 start_sf01()
fn run() {
    let mut client = setup();
    let rows = client.query_rows(sql).unwrap();
    // 断言 (rows 已经是 Vec<Vec<String>>)
}
```

### 3.3 关键不变量

- ✅ 13 个文件 `#[test]` 函数名**全部保留**
- ✅ baseline JSON 路径**全部保留**
- ✅ cargo 命令行入口 (`cargo test --test <name>`) **全部保留**
- ✅ 输出格式（`println!` 内容）**向后兼容**

---

## 4. 错误处理与稳定性

### 4.1 错误处理矩阵

| 错误类型 | 处理策略 | 影响 |
|----------|----------|------|
| `wire_err::Error` (连接/帧) | 立即 `panic!` with server 日志 | 测试失败 |
| Server 返回 ERR packet (SQL 错) | 收集到 `TestReport`；标记 known-fail | 不 panic 单 query |
| 单 query 失败 | 22 query 统计；总通过数 gate | 22/22 必须通过 |
| `LOAD DATA` 失败 | 整测试 `panic!` | fixture 严重错误 |
| Per-query 超时 | 标记 timeout；继续其他 query | 不 panic 单 query |
| Float 比较不等 | 标记 `[FLOAT-TOLERANCE]` 不 panic | SF=0.1 已知浮点累积差 |

### 4.2 已知不通过 query 处理

参考 `E2E_MIGRATION_MASTER_PLAN.md` 提到的 G11/G13 invalid、Q1 row count 4 (vs 6):

- 不修改 baseline JSON
- 测试输出 `[KNOWN-ISSUE] Q{N}: ...` 警告
- 不 panic
- 末尾统计: `22/22 passed, N known-issues`

### 4.3 资源约束

| 资源 | 限制 | 控制方式 |
|------|------|----------|
| Server RSS | 2 GB | ulimit -v (test 启动前) |
| 进程 FD | 512 | ulimit -n |
| 数据库大小 | 500 MB | SF=0.1 600K 行约 ~50MB，远低于 500MB |
| Per-test 超时 | 60s | `set_timeouts(read=60s, write=60s)` |
| 22 query 总时长 | SF=0.001: 30s, SF=0.1: 5min | `Instant::now()` 累计 |

### 4.4 CI 集成

**主路径**:
```bash
cargo test --test tpch_gate_test
cargo test --test tpch_full_22_test
# ... 13 个文件
```

**Gitea CI workflow** (`.gitea/workflows/tpch_e2e.yml`):

```yaml
- name: TPC-H E2E wire (default scale)
  run: |
    cargo build --all-features
    cargo test --test tpch_gate_test --test tpch_full_22_test \
               --test tpch_sf01_inprocess_test --test tpch_value_test_v2 \
               --test tpch_value_correctness_test --test tpch_bug_regression_test \
               --test tpch_q9_audit --test tpch_per_query_timeout_test \
               --test tpch_hash_test -- --nocapture

- name: TPC-H E2E wire (slow, nightly)
  run: |
    cargo test --test tpch_sf01_perf_baseline_test \
               --test tpch_q8_q21_perf_regression_test \
               --test tpch_sf01_22_vs_sqlite \
               --test tpch_sf01_22_vs_3engines -- --nocapture
  # 仅夜间跑, 慢测试
```

---

## 5. 阶段性实施 (P0/P1/P2)

### P0 (1 周) — 核心 G1 gate
- [ ] 新建 `tests/common/tpch_wire_harness.rs`
- [ ] 改造 `tests/tpch_gate_test.rs` (G1 gate)
- [ ] 改造 `tests/tpch_full_22_test.rs` (22 query 跑通)
- [ ] 改造 `tests/tpch_sf01_inprocess_test.rs` (SF=0.1 sanity)
- [ ] PR: `feature/tpch-e2e-p0-gate`
- [ ] 验证: `cargo test --test tpch_gate_test --test tpch_full_22_test --test tpch_sf01_inprocess_test`

### P1 (1 周) — 值正确性 + bug regression
- [ ] 改造 `tests/tpch_value_test_v2.rs` (对比 SQLite baseline)
- [ ] 改造 `tests/tpch_value_correctness_test.rs` (数值正确性)
- [ ] 改造 `tests/tpch_bug_regression_test.rs` (3 bugs 回归)
- [ ] 改造 `tests/tpch_q9_audit.rs` (Q9 + SQLite ground-truth)
- [ ] 改造 `tests/tpch_per_query_timeout_test.rs` (per-query timeout)
- [ ] 改造 `tests/tpch_hash_test.rs` (SHA-256 对比)
- [ ] PR: `feature/tpch-e2e-p1-correctness`
- [ ] 验证: 全部 7 个 P0+P1 测试通过

### P2 (1 月) — 性能 + 跨引擎对比
- [ ] 改造 `tests/tpch_sf01_perf_baseline_test.rs` (perf baseline)
- [ ] 改造 `tests/tpch_q8_q21_perf_regression_test.rs` (Q8/Q21 perf budget)
- [ ] 改造 `tests/tpch_sf01_22_vs_sqlite.rs` (跨 SQLite)
- [ ] 改造 `tests/tpch_sf01_22_vs_3engines.rs` (跨 MariaDB/PG)
- [ ] PR: `feature/tpch-e2e-p2-perf`
- [ ] 验证: 13 个 E2E 测试全过；perf baseline JSON 重新生成

---

## 6. 验证策略 (5 步)

| 步骤 | 命令 | 期望 |
|------|------|------|
| 1. 编译 | `cargo build --all-features` | 0 errors, 0 warnings |
| 2. Clippy | `cargo clippy --all-features -- -D warnings` | 0 errors |
| 3. Format | `cargo fmt --check --all` | 0 diffs |
| 4. 单文件测试 | `cargo test --test tpch_gate_test -- --nocapture` | 22/22 PASS |
| 5. 全量测试 | `cargo test --tests --all-features` | 全部 PASS |

### grep 验证 (防止 in-process 残留)
```bash
# 必须全部 0 结果
grep -l "ExecutionEngine::new(MemoryStorage" tests/tpch_*.rs
grep -l "bulk_insert_records" tests/tpch_*.rs
grep -l "\.execute(sql)" tests/tpch_*.rs | grep -v "// " | grep -v wire_smoke
```

---

## 7. 完成标准 (Definition of Done)

1. ✅ `tests/common/tpch_wire_harness.rs` 存在，所有 API 文档化
2. ✅ 13 个 in-process 测试文件中**无 `ExecutionEngine::new(MemoryStorage)` 残留**
3. ✅ 13 个测试文件中**无 `bulk_insert_records` 残留**
4. ✅ 13 个测试文件中**无 `.execute(sql)` 直接调用 engine**
5. ✅ 所有 22 个 TPC-H query 通过 wire protocol 跑通（22/22 或带 known-issue）
6. ✅ `cargo test --tests --all-features` 退出码 0
7. ✅ `docs/releases/v3.9.0/E2E_MIGRATION_MASTER_PLAN.md` §1.1 表更新（标记 ✅ done）
8. ✅ 新增实施报告 `docs/releases/v3.9.0/TPCH_E2E_MIGRATION_REPORT.md`

---

## 8. 风险与缓解

| 风险 | 等级 | 缓解 |
|------|------|------|
| Wire protocol 比 in-process 慢导致超时 | 中 | `set_timeouts(60s)` + per-test 60s timeout |
| `LOAD DATA LOCAL INFILE` 大数据量失败 (EAGAIN) | 中 | SF=0.001 无问题；SF=0.1 用 8MB chunk (已实现) |
| 已知不通过 query (G11/G13) 导致 CI 永远红 | 高 | known-issue 标签，不 panic |
| 13 文件同时改造 PR 太大 | 高 | 拆 P0/P1/P2 三批 PR (3+6+4) |
| SF=0.1 LOAD DATA 60K lineitem 慢 | 低 | 实测 1-2min, 可接受 |
| 外部引擎 (MariaDB/PG) 环境缺失 | 中 | P2 跨引擎测试在 CI 用 `command -v` 探测，缺失则 skip |
| Float 比较累积差 | 中 | `compare_cells(float_tol=0.01)` |
| server 启动慢 | 低 | `start_ephemeral` 启动 <100ms |

---

## 9. 与 governance 一致性

- ✅ 5-step governance 流程 (DOC_CHECK_CORRECTION_RULES.md)
- ✅ 最小修改原则 (不改 baseline JSON, 不改 SQL, 不改 schema DDL)
- ✅ 不破坏现有工作流 (cargo test 命令行入口保留)
- ✅ 4-remote 同步 (后续 PR 阶段)
- ✅ 文档更新 (E2E_MIGRATION_MASTER_PLAN.md + 本文件 + 实施报告)

---

## 10. 后续行动 (下一步)

1. **立即**: 在 `feature/tpch-e2e-migration` 分支上实施 P0
2. **P0 完成后**: PR review + merge to `develop/v3.9.0`
3. **P1/P2 实施**: 按 §5 阶段性 PR

---

Last updated: 2026-06-13
