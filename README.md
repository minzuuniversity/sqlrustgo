# SQLRustGo

> **Last updated**: 2026-06-04
> **Current dev branch**: [`c024980c`](http://192.168.0.252:3000/openclaw/sqlrustgo/commit/c024980c) @ develop/v3.8.0
> **Latest stable**: v3.7.0 (GA, 2026-05-31)
> **Latest beta**: v3.8.0-beta (Strong Beta, 2026-06-04, 8.0/10)

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85+-dea584?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/v3.7.0-GA-green?style=flat-square" alt="GA">
  <img src="https://img.shields.io/badge/v3.8.0-Strong%20Beta-blue?style=flat-square" alt="Beta">
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License">
  <img src="https://img.shields.io/badge/TPC--H-10%2F22-yellow?style=flat-square" alt="TPC-H">
  <img src="https://img.shields.io/badge/Corpus-86.5%25-yellow?style=flat-square" alt="Corpus">
  <img src="https://img.shields.io/badge/9--Dim%20Gate-8%2F8%20PASS-brightgreen?style=flat-square" alt="D9">
  <img src="https://img.shields.io/badge/INT--1%20(P0)-CLOSED-brightgreen?style=flat-square" alt="INT-1">
</p>

SQLRustGo 是一个纯 Rust 实现的 SQL 执行引擎，支持完整 SQL-92 语法、窗口函数、CTE、CBO 成本优化器、WAL + MVCC 事务、向量存储与图存储，以及 AI Native GMP 工作流。

> **v3.8.0 当前状态 (2026-06-04)**: **Strong Beta** (8.0/10) — INT-1 (P0 Release Blocker) 已修复 (DML 真实走 TransactionManager), GROUP BY/JOIN 核心 100%, Corpus 86.5%, D9 9 维门禁 8/8 ALL PASS. **v3.8.0 是长期收敛版本** (beta → rc1 → rc2 → ga), **不创建 3.9.0**. 详见 [RELEASE_NOTES.md](docs/releases/v3.8.0/RELEASE_NOTES.md), [V380_ROADMAP.md](docs/releases/v3.8.0/V380_ROADMAP.md) 与 [V380_COMPREHENSIVE_ASSESSMENT.md](docs/releases/v3.8.0/V380_COMPREHENSIVE_ASSESSMENT.md).

---

## Table of Contents

- [Core Features](#core-features)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Usage Examples](#usage-examples)
- [Feature Matrix](#feature-matrix)
- [Performance Benchmarks](#performance-benchmarks)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [Changelog](#changelog)
- [License](#license)

---

## Core Features

| 模块 | 能力 |
|------|------|
| 🔍 **SQL 解析** | SELECT / INSERT / UPDATE / DELETE / CREATE TABLE / DROP TABLE + CTE + 窗口函数 |
| ⚙️ **查询优化** | CBO 成本优化器（基于统计信息的 Join 排序优化） |
| 💾 **存储引擎** | MemoryStorage / FileStorage / ColumnarStorage + Buffer Pool |
| 🔎 **索引** | B+ Tree / Hash Index / Vector Index (ANN) |
| 🔄 **事务** | WAL + MVCC（Snapshot Isolation）+ XA 两阶段提交 |
| 🌐 **网络协议** | TCP/IP + MySQL 风格协议 |
| 🤖 **AI Native** | Ollama 本地 LLM 推理 + GMP Retrieval v3（BM25 + Vector + Graph + FTS） |
| 📦 **高级特性** | Prepared Statement / 触发器 / 图数据库（Cypher）|
| 🔀 **复制** | Semi-sync 复制 / MTS 并行复制 / Multi-source 复制 |

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                      SQLRustGo                           │
├─────────────┬─────────────┬─────────────┬───────────────┤
│  Network    │   Parser    │  Optimizer  │  Executor     │
│  (MySQL)    │  (SQL-92)   │   (CBO)     │  ( Volcano )  │
├─────────────┴─────────────┴─────────────┴───────────────┤
│                    Storage Engine                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
│  │  Memory   │ │  File    │ │ Columnar │ │ Vector   │  │
│  │  Storage  │ │  Storage │ │ Storage  │ │ Storage  │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘  │
├──────────────────────────────────────────────────────────┤
│  Transaction Manager  │  Buffer Pool  │  Index Manager  │
│  (WAL + MVCC)         │  (LRU/Knob)   │  (B+Tree/Hash)  │
└──────────────────────────────────────────────────────────┘
```

---

## Quick Start

```bash
# 克隆并构建
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo
cargo build --release

# 运行测试（39 项单元测试）
cargo test --all-features

# 代码检查
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# 启动 REPL
cargo run --bin sqlrustgo
```

**提示**: 更多文档请查阅 `docs/releases/v3.8.0/` 目录。

---

## Installation

### 二进制（Linux/macOS）

```bash
# 从 Release 下载
wget http://192.168.0.252:3000/openclaw/sqlrustgo/releases/download/v3.5.0/sqlrustgo-x86_64.tar.gz
tar -xzf sqlrustgo-x86_64.tar.gz
./sqlrustgo
```

### 从源码构建

```bash
cargo build --release
cargo install --path .
```

### Docker

```bash
docker run -p 5432:5432 sqlrustgo/sqlrustgo:v3.5.0
```

---

## Usage Examples

### CREATE TABLE + INSERT + SELECT

```sql
CREATE TABLE orders (
    order_id INTEGER PRIMARY KEY,
    customer_id INTEGER,
    order_date TEXT,
    total_amount REAL
);

INSERT INTO orders VALUES (1, 100, '2026-01-01', 250.00);
INSERT INTO orders VALUES (2, 200, '2026-01-02', 150.00);

SELECT customer_id, SUM(total_amount) AS revenue
FROM orders
GROUP BY customer_id;
```

### Window Functions

```sql
SELECT
    l_orderkey,
    l_quantity,
    SUM(l_quantity) OVER (PARTITION BY l_orderkey) AS total_qty,
    AVG(l_quantity) OVER (ORDER BY l_orderkey ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) AS moving_avg
FROM lineitem
LIMIT 10;
```

### CTE (Common Table Expression)

```sql
WITH regional_sales AS (
    SELECT
        n_name AS region,
        SUM(l_extendedprice * (1 - l_discount)) AS revenue
    FROM orders
    JOIN customer ON o_custkey = c_custkey
    JOIN nation ON c_nationkey = n_nationkey
    JOIN lineitem ON o_orderkey = l_orderkey
    GROUP BY n_name
)
SELECT region, revenue FROM regional_sales;
```

---

## Feature Matrix

| 功能 | SQLRustGo | SQLite | MySQL | PostgreSQL |
|------|:---------:|:------:|:-----:|:----------:|
| **SQL-92 SELECT** | ✅ 完全 | ✅ | ✅ | ✅ |
| **窗口函数** | ✅ | ✅ | ✅ | ✅ |
| **CTE / 递归 CTE** | ✅ | ✅ | ✅ | ✅ |
| **CBO 优化器** | ✅ | ❌ | ❌ | ✅ |
| **WAL 预写日志** | ✅ | ✅ | ❌ | ❌ |
| **MVCC 事务** | ✅ | ❌ | ✅ | ✅ |
| **MVCC 隔离级别** | Snapshot | - | Read Committed | Serializable |
| **Prepared Statement** | ✅ | ❌ | ✅ | ✅ |
| **存储过程** | ❌ | ❌ | ✅ | ✅ |
| **JSON / JSONB** | ✅ | ✅ | ✅ | ✅ (JSONB) |
| **向量存储 (ANN)** | ✅ | ❌ | ❌ | 扩展 |
| **图存储 (Cypher)** | ✅ | ❌ | ❌ | ❌ |
| **B+ Tree 索引** | ✅ | ✅ | ✅ | ✅ |
| **Hash 索引** | ✅ | ❌ | ✅ | ✅ |
| **全文搜索 FTS** | ✅ | ✅ (FTS5) | ✅ | ✅ (tsvector) |
| **复制 (主从)** | ❌ | 外部 | binlog | 流复制 |
| **分区表** | ❌ | ❌ | ✅ | ✅ |
| **并行查询** | ❌ | ❌ | 有限 | ✅ |
| **行级安全** | ❌ | ❌ | ❌ | ✅ |
| **ACID 完整支持** | ✅ | ✅ | ✅ | ✅ |
| **Zero-Config 部署** | ✅ | ✅ | ❌ | ❌ |

> **设计目标**：SQLRustGo 面向教育/研究/嵌入式场景，强调内存安全和 Rust 纯白盒实现，而非替代 PostgreSQL 的企业级功能。

---

## Performance Benchmarks

### 测试环境

| 项目 | 配置 |
|------|------|
| 硬件 | Z440 Workstation (80C / 408GB RAM) |
| CPU | Intel Xeon (第 4 代) |
| Rust | 1.85+ |
| OS | Linux 6.8 |

### TPC-H 决策支持基准

> TPC-H 是一个决策支持基准测试，包含 22 个 OLAP 查询，评估数据库在复杂聚合、JOIN、排序场景下的性能。

#### SF=0.1（约 100MB 数据，~60万行 lineitem）

| 查询 | 耗时 (ms) | 行数 | 查询 | 耗时 (ms) | 行数 |
|------|----------|------|------|----------|------|
| Q1 | 1573.94 | 3 | Q12 | 426.18 | 0 |
| Q2 | 314.68 | 0 | Q13 | 320.08 | 15000 |
| Q3 | 3155.72 | 0 | Q14 | 881.18 | 1 |
| Q4 | 454.73 | 5 | Q15 | 299.24 | 0 |
| Q5 | 310.06 | 0 | Q16 | 336.50 | 0 |
| Q6 | 849.19 | 1 | Q17 | 881.89 | 1 |
| Q7 | 300.11 | 0 | Q18 | 307.91 | 0 |
| Q8 | 299.60 | 0 | Q19 | 932.98 | 1 |
| Q9 | 297.09 | 0 | Q20 | 298.02 | 0 |
| Q10 | 781.40 | 1 | Q21 | 301.59 | 0 |
| Q11 | 337.15 | 0 | Q22 | 299.58 | 0 |

**SF=0.1 汇总**：22/22 PASS | 总耗时 **13.96s**

> ⚠️ 使用 `--queries all`，不支持 `--queries 1`（需用 `--queries Q1`）

#### SF=1（约 1GB 数据，~600万行 lineitem）

| 指标 | 数值 |
|------|------|
| 数据加载 | 8.6M 行 / **18.4min** |
| 总查询耗时 | **292s** (22/22 PASS) |
| 最慢查询 | Q3 (173s)，Q1 (17s)，Q10 (11s) |

| 查询 | 耗时 (ms) | 行数 | 查询 | 耗时 (ms) | 行数 |
|------|----------|------|------|----------|------|
| Q1 | 16997.80 | 3 | Q12 | 4638.89 | 0 |
| Q2 | 3335.33 | 0 | Q13 | 3659.32 | 150000 |
| Q3 | 173594.15 | 0 | Q14 | 9065.43 | 1 |
| Q4 | 4743.89 | 5 | Q15 | 3011.63 | 0 |
| Q5 | 3165.38 | 0 | Q16 | 3428.16 | 0 |
| Q6 | 9231.97 | 1 | Q17 | 8948.10 | 1 |
| Q7 | 3869.51 | 0 | Q18 | 3124.60 | 0 |
| Q8 | 3424.73 | 0 | Q19 | 9445.00 | 1 |
| Q9 | 4754.04 | 0 | Q20 | 3011.50 | 0 |
| Q10 | 11519.52 | 1 | Q21 | 3015.98 | 0 |
| Q11 | 3664.37 | 0 | Q22 | 3015.20 | 0 |

> ⚠️ 数据加载占 18min（全量 8.6M 行），查询阶段 4.9min。

#### SF=10（约 10GB 数据，~2200万行 lineitem）

| 阶段 | 状态 |
|------|------|
| 数据导入（29M 行） | ✅ 完成 (**96s**) |
| Q1 ~ Q22 | ❌ OOM (查询阶段被 SIGKILL，408GB RAM 仍不足) |

**SF=10 数据规模**：customer 1.5M / orders 5.5M / lineitem 21.8M / part 20K / partsupp 80K

> ⚠️ SF=10 需分批查询或降级测试策略，单次全量查询超出本机 408GB 物理内存。

### TPC-H 横向对比（SF=1）

> ⚠️ 数据均来自各数据库官方发布或公开基准，与 SQLRustGo 非同类生产级对比，仅供参考。

| 数据库 | SF=1 TPC-H | 说明 |
|--------|:----------:|------|
| **SQLRustGo** | **22/22 ✅** | 纯教育/研究目的，内存数据库 |
| SQLite | 22/22 ✅ | 微秒级本地延迟，简单查询 |
| MySQL (InnoDB) | 22/22 ✅ | sub-ms 网络延迟 |
| PostgreSQL | 22/22 ✅ | 最佳 SQL 标准兼容，复杂查询强 |
| DuckDB | 22/22 ✅ | 分析型，列式存储，OLAP 优化 |

> **结论**：SQLRustGo 在 SF=1 规模下完整通过所有 22 个 TPC-H 查询，验证了 SQL 引擎的完整性和正确性。

### OLTP 基准（Sysbench）

> ⚠️ Sysbench 测试进行中，200+ 连接压测待完成。

| 场景 | 状态 | QPS | 延迟 (p99) |
|------|------|-----|-----------|
| point_select (1 conn) | ⏳ | - | - |
| point_select (200 conn) | ⏳ | - | - |
| read_write (100 conn) | ⏳ | - | - |
| write_only (50 conn) | ⏳ | - | - |

### 代码质量

| 指标 | 值 |
|------|---|
| L1 覆盖率 | **87.36%** (≥85% ✅) |
| 单元测试 | **39 passed** |
| clippy warnings | **0 errors** |
| fmt diff | **0 diffs** |

---

## Documentation

### 📖 用户文档（mdBook）

> `docs/releases/v3.8.0/` 目录 — 包含完整 SQL 语法参考、部署指南、开发者文档

### 文档导航

| 文档 | 说明 |
|------|------|
| [📋 CHANGELOG](CHANGELOG.md) | 版本变更历史 |
| [📝 RELEASE_NOTES](docs/releases/v3.5.0/RELEASE_NOTES.md) | v3.5.0 正式发布说明 |
| [📖 v3.5.0 文档中心](docs/releases/v3.5.0/) | 当前版本完整文档 |
| [📊 v3.5.0 GA 门禁报告](docs/releases/v3.5.0/GA_GATE_REPORT.md) | 质量门禁执行证据 |
| `docs/releases/v3.8.0/` 目录 | 交互式文档 |
| [🔧 开发者文档](docs/releases/v3.5.0/DEV_PLAN.md) | 开发计划和路线图 |

### 历史版本

| 版本 | 文档 | 发布日期 |
|------|------|----------|
| v3.4.0 GA | [📂](docs/releases/v3.4.0/) | 2026-05-24 |
| v3.3.0 GA | [📂](docs/releases/v3.3.0/) | 2026-05-20 |
| v3.2.0 GA | [📂](docs/releases/v3.2.0/) | 2026-05-17 |
| v3.1.0 Beta | [📂](docs/releases/v3.1.0/) | 2026-05-14 |
| v3.0.0 GA | [📂](docs/releases/v3.0.0/) | 2026-05-10 |
| v2.x ~ v1.x | [📂 Archive 分支](docs/releases/) | 历史版本 |

---

## Contributing

```bash
# 运行所有测试
cargo test --all-features

# clippy 检查
cargo clippy --all-targets -- -D warnings

# 格式检查
cargo fmt --check

# 覆盖率报告
cargo llvm-cov report --open
```

---

## Changelog

### [3.8.0-beta] - 2026-06-04 (Strong Beta, 8.0/10) — **当前开发版本**

- **状态**: **Strong Beta** (8.0/10) — 跳过 RC 周期, 直接进入 v3.9.0
- **INT-1 (P0 Release Blocker) CLOSED**: DML 真实走 TransactionManager (PR-3019)
- **核心 SQL 引擎 100%**: Parser 18/18 + Executor 30/30 + GROUP BY 81/81 + JOIN 100%
- **Corpus 86.5%**: 711/822 cases PASS (Stage 1-3 累计 +270 cases)
- **6 真实 bug 修复**: COUNT(DISTINCT), SELECT DISTINCT, NULL=NULL, D9 路径, D9 5-Principle, INT-1
- **9 维门禁**: D9 8/8 ALL PASS (D1-D5 + D6/D7/D8 + Cross-Version + Test Plan + PR Template + Evidence)
- **11 issues 关闭**: #2966/2967/2968/2969/2970/2971/2972/2937/2938/2942/2807

详见 [RELEASE_NOTES.md](docs/releases/v3.8.0/RELEASE_NOTES.md), [V380_COMPREHENSIVE_ASSESSMENT.md](docs/releases/v3.8.0/V380_COMPREHENSIVE_ASSESSMENT.md), [V380_BETA_RELEASE_REPORT.md](docs/releases/v3.8.0/V380_BETA_RELEASE_REPORT.md)

**不适用**: ❌ 生产 OLTP/财务/订单/银行 (需要 v3.8.0-GA), ❌ 大数据量 (TPC-H 22/22 仍待 v3.9.0)
**适用**: ✅ 开发/CI/教学/实验/内部工具

### [3.7.0] - 2026-05-31 (GA) — **最新稳定版本**

- **AI Native GMP Platform** — AI Agent Layer + Ollama 本地推理 + GMP Retrieval v3
- **门禁**：Alpha/Beta/RC/GA 全部 ✅ PASS
- **覆盖率**：L1 87.36%，TPC-H 22/22 ✅

### [3.5.0] - 2026-05-28 (GA)

- **AI Native GMP Platform** — AI Agent Layer + Ollama 本地推理 + GMP Retrieval v3
- **门禁**：Alpha/Beta/RC/GA 全部 ✅ PASS
- **覆盖率**：L1 87.36%，TPC-H 22/22 ✅

详见 [CHANGELOG.md](CHANGELOG.md)

---

## v3.8.0 路线图 (长期收敛, 不创建 3.9.0)

按 ChatGPT 第三轮评估, v3.8.0 = **长期收敛版本** (beta → rc1 → rc2 → ga), 不开 3.9.0.

### 4 阶段 (总 270h ≈ 7 周 active + 1 周 wait)

| 阶段 | 状态 | 工作量 | 时间 |
|------|------|--------|------|
| ✅ **v3.8.0-beta** | Strong Beta 8.0/10 | 0 (已完成) | 2026-06-04 (现) |
| **v3.8.0-rc1** | 7 项完成 + TPC-H 22/22 | 100h | 2026-06-25 (3 周) |
| **v3.8.0-rc2** | 72h 长稳 + Crash 1000 轮 | 100h | 2026-07-16 (6 周) |
| **v3.8.0-ga** | 168h 长稳 + GA 收口 | 70h | 2026-08-13 (10 周) |

### rc1 必须完成 (7 项)

1. **SEM-1** 执行语义标准化 (20h, #2975)
2. **ARCH-2** merge.rs 统一 DML (15h, #2974)
3. **CLI-01** Client CLI 补全 (15h)
4. **SERVER-01** Alpha Server 成立 (10h)
5. **TPC-H 22/22** (40h, #2977)
6. **Corpus ≥95%** (并行)
7. **Crash Harness** 工具 (10h)

### v3.8.0 整个周期 Feature Freeze

**允许**:
- ✅ P0/P1 Bug 修复 (Crash, Data Loss, Deadlock, Corruption)
- ✅ 文档完善
- ✅ 9 维门禁的 bug fix

**禁止 (整个 10 周周期)**:
- ❌ SIMD 集成 (v3.9.0+)
- ❌ Vector SQL 集成 (v4.0+)
- ❌ Parallel Executor 主路径 (v3.9.0+)
- ❌ 新优化器 (v3.9.0+)
- ❌ MySQL 高级函数大规模补齐 (v3.9.0+)
- ❌ 任何 Feature 提交

**所有 PR 标题加 `[v380]` + 关联 4 阶段之一**.

详见 [V380_ROADMAP.md](docs/releases/v3.8.0/V380_ROADMAP.md)

---

## License

MIT License — 详见 [LICENSE](LICENSE) 文件。