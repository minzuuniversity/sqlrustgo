# SQLRustGo

> **更新日期**: 2026-08-14
> **最新稳定版**: v3.11.0 GA (2026-08-09)
> **当前开发版**: v3.12.0 ALPHA (DRAFT -> ALPHA: 2026-08-12)
> **当前开发目标**: 面向 GMP 内审检索系统的 SQLRustGo 数据库、内部向量检索、SQL-backed graph projection 和可审计 evidence bundle
> **真实性边界**: README 只陈述已有文档或实测证据支持的状态；未完成项标为 `PARTIAL`、`DEFERRED` 或 `OPEN`。属于 v3.12 初始生产边界的 `PARTIAL` 必须绑定 [PARTIAL 功能整改 Issue 计划](docs/releases/v3.12.0/PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md)，不能作为无闭环生产能力宣传。

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85+-dea584?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/v3.11.0-GA-blue?style=flat-square" alt="v3.11.0 GA">
  <img src="https://img.shields.io/badge/v3.12.0-ALPHA-orange?style=flat-square" alt="v3.12.0 ALPHA">
  <img src="https://img.shields.io/badge/TPC--H%20SF1-22%2F22%20completed-yellowgreen?style=flat-square" alt="TPC-H SF=1 22/22 completed">
  <img src="https://img.shields.io/badge/GMP%20Retrieval-v3.12%20target-informational?style=flat-square" alt="GMP retrieval target">
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License">
</p>

SQLRustGo 是一个纯 Rust 实现的 SQL 数据库项目，包含 SQL 解析、优化、执行、存储、事务、MySQL 风格 wire protocol、GMP/RAG 检索、内部向量检索和 SQL-backed 图投影等模块。

当前仓库的发布口径是：

- **v3.11.0 GA**: 可作为简单生产环境或受控场景的候选数据库版本，但不能宣称为完整 MySQL 5.7 替代品。
- **v3.12.0 ALPHA**: 正在补强 v3.11 的弱项，并面向 `~/gmp-platform` 的 GMP 合规内审检索系统建立数据库、向量检索和图投影能力。
- **v4.0.0 方向**: 才适合规划“通用向量数据库 / 通用图数据库 / 更广义生产替代”的产品目标。

---

## 目录

- [当前真实状态](#当前真实状态)
- [快速开始](#快速开始)
- [架构概览](#架构概览)
- [功能矩阵](#功能矩阵)
- [TPC-H 与性能基准](#tpc-h-与性能基准)
- [GMP / RAG / Vector / Graph](#gmp--rag--vector--graph)
- [测试与质量门禁](#测试与质量门禁)
- [文档资源](#文档资源)
- [历史版本](#历史版本)
- [贡献指南](#贡献指南)
- [许可证](#许可证)

## 当前真实状态

| 项 | 当前状态 | 证据 / 说明 |
|---|---|---|
| v3.11.0 阶段 | GA | [v3.11 综合评估](docs/releases/v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md)、[v3.11 STAGE](docs/releases/v3.11.0/STAGE.yaml) |
| v3.11.0 生产边界 | 受控/简单生产候选 | 不等同完整 MySQL 5.7 替代；TPC-H correctness、LOAD DATA、recovery、upgrade 等仍需 v3.12 补强 |
| v3.12.0 阶段 | ALPHA | [v3.12 STAGE](docs/releases/v3.12.0/STAGE.yaml) 记录 DRAFT -> ALPHA 于 2026-08-12 完成 |
| v3.12.0 产品目标 | GMP 内审检索数据库 | [v3.12 README](docs/releases/v3.12.0/README.md)、[GMP 合规矩阵](docs/releases/v3.12.0/GMP_COMPLIANCE_MATRIX.md) |
| TPC-H SF=1 | 22/22 completed，但 correctness 仍有限定 | [v3.11 TPC-H 报告](docs/releases/v3.11.0/TPCH_SF1_22_22_PASS_REPORT.md)、[v3.12 TPC-H correctness](docs/releases/v3.12.0/evidence/tpch/V312-12-TPCH-CORRECTNESS.md) |
| TPC-H SF=10 | PARTIAL / 有整改 issue | harness 存在；3/8 SF=10 表已 parity match，剩余大表受 FileStorage 写放大/吞吐瓶颈阻塞；见 [#4020 evidence](docs/releases/v3.12.0/evidence/issue-4020/4020_evidence.md)、[#4217](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4217) |
| Bulk-load SF=10 | PARTIAL / OPEN | 不是 schema creation 阶段失败的旧状态；当前是 3/8 表 match，5/8 大表未完成；见 [#4020](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4020)、[#4217](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4217) |
| MySQL wire + LOAD DATA hardening | PARTIAL / 有整改 issue | [V312-13 报告](docs/releases/v3.12.0/evidence/wire_load_data/V312-13-REPORT.md) failed_steps=0，但完整 MySQL 5.7 兼容、prepared statement sysbench 路径、SF=10 full bulk-load 仍需 [#4223](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4223)、[#4211](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4211)、[#4020](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4020) |
| Sysbench | PARTIAL / 有整改 issue | read_only baseline 已捕获；write/read_write 因行级锁/隔离问题失败，prepared statement 兼容另有缺口；见 [#4210](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4210)、[#4211](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4211) |
| Prometheus metrics | endpoint 已实现并 E2E scrape | [#4021 evidence](docs/releases/v3.12.0/evidence/issue-4021/4021_evidence.md) |
| Slow query log | 已实现并测试 | [#4022 evidence](docs/releases/v3.12.0/evidence/issue-4022/4022_evidence.md) |
| SQLLogicTest | smoke gate 25/25 PASS；full official corpus 未声明完成 | [SQLLogicTest smoke report](docs/releases/v3.12.0/evidence/sqllogictest/smoke-report.md)、[v3.12 Scope Table](docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md)、[#4224](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4224) |
| 覆盖率 | 分层统计中；不再用单一 workspace 口径宣传全达标 | [综合测试框架与覆盖率基线](docs/releases/v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md) |

## 快速开始

```bash
# 克隆
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo

# 构建
cargo build --all-features

# 运行测试
cargo test --all-features

# 格式和 lint
cargo fmt --check --all
cargo clippy --all-features -- -D warnings

# MySQL 风格服务端
cargo run --bin sqlrustgo-mysql-server -- serve --host 127.0.0.1 --port 3307
```

> 注意：不同历史文档中曾经出现 `sqlrustgo`、`sqlrustgo-sql-cli` 等旧入口。自 v3.8 起，主要运行入口逐步收敛到 `sqlrustgo-mysql-server` 及 workspace crates。

## 架构概览

```text
SQL / MySQL Wire / Admin
        |
Parser -> Planner -> Optimizer -> Executor
        |                  |
        |                  +-- TPC-H / SQLLogicTest / MySQL compat paths
        |
Storage / Catalog / Transaction / WAL / MVCC
        |
GMP schema / chunks / versions / audit log / relations / embeddings
        |
Hybrid retrieval / Vector retrieval / SQL-backed graph projection / RAG evidence bundle
```

## 功能矩阵

状态说明：

- `DONE`: 有合并代码和测试/报告证据。
- `PARTIAL`: 主路径或 smoke 可用，但生产级闭环、边界测试或性能证据不足。
- `PARTIAL / 有整改 issue`: 仅允许作为 Alpha/Beta 过渡状态；若属于 v3.12 初始生产边界，GA 前必须按 [整改计划](docs/releases/v3.12.0/PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md) 关闭或降级。
- `DEFERRED`: 已有 owner/expiry/follow-up 或明确规划，当前版本不宣称完成。
- `OPEN`: 当前仍阻塞或未达到关闭标准。
- `UNSUPPORTED`: 明确不支持或不作为当前版本目标。

| 能力 | v3.11.0 GA | v3.12.0 当前 | 证据边界 |
|---|---:|---:|---|
| SQL 基础 DDL/DML | DONE | DONE / 持续硬化 | CREATE/INSERT/SELECT/UPDATE/DELETE 主路径可用，corner cases 由 SQLLogicTest 继续覆盖 |
| SQL-92 SELECT / JOIN / GROUP BY | DONE | DONE / 持续硬化 | TPC-H 和 SQL corpus 仍暴露 planner/semantic gap |
| CTE | DONE | DONE | 包括 CTE materialization 改进；递归和复杂兼容仍需按测试声明 |
| 窗口函数 | PARTIAL | PARTIAL / scope decision | MySQL compat 中 `window_rank_partition` 仍有 deferred/协议问题记录；3.12 是否交付受控子集由 [#4227](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4227) 决策 |
| Optimizer / CBO / Hash Join | DONE | DONE / 持续硬化 | #3909 已通过 PR #4087 close-out；性能债仍按后续 issue 跟踪 |
| WAL / MVCC | PARTIAL | PARTIAL / blocker | 主路径存在；crash recovery 28/31，backup/restore API drift 和 upgrade/downgrade 需 [#4222](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4222) 收口 |
| B+Tree / Hash index | DONE | DONE | 索引能力进入主路径；性能趋势需按具体 workload 阅读 |
| Clustered Index / AHI / Change Buffer / Double Write Buffer | DONE | DONE | v3.11 重点功能；仍建议配合 crash/fault injection 继续验证 |
| MySQL wire protocol | PARTIAL | PARTIAL / 有整改 issue | e2e/wire 测试有推进；完整 MySQL 5.7 兼容不可宣称；[#4223](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4223) 定义生产边界 |
| Prepared Statement | PARTIAL | PARTIAL / blocker | 基本回归有测试；Sysbench prepared statement 仍由 [#4211](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4211) 和 [#4223](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4223) 收口 |
| LOAD DATA | PARTIAL | PARTIAL / blocker | SF=1/smoke 与 wire gate 有证据；SF=10 full bulk-load 由 [#4020](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4020)、[#4217](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4217) 收口 |
| TLS / Compression | PARTIAL | PARTIAL / 有整改 issue | V312-13 有 typed wrapper / handshake / primitive 证据；生产客户端路径边界由 [#4223](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4223) 收口 |
| Sysbench OLTP | 未作为 GA 主证据 | PARTIAL / blocker | read_only: 2870.99 qps / 179.44 tps；write/read_write 因行级锁/隔离问题失败；见 [#4210](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4210)、[#4211](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4211) |
| Prometheus `/metrics` | N/A | DONE / 有限制 | endpoint 和 live scrape 已验证；query counter hot path 仍有 observability debt |
| Slow query log | N/A | DONE / 有限制 | 单元和集成测试通过；未在真实 TPC-H SF=10 长查询上捕获日志 |
| SQLLogicTest runner | 规划/非阻断 | DONE / smoke；RC-GA 扩展 | smoke gate 25/25 PASS；full SQLite official corpus 不宣称完成，RC/GA 扩展由 [#4224](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4224) 收口 |
| 覆盖率治理 | PASS with follow-up | PARTIAL / blocker | v3.12 采用 per-crate 分层口径；低覆盖 crate 和 SEM-4 gap 由 [#3943](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3943) 收口 |
| GMP schema / version / chunk / audit / relation | N/A | DONE | `sqlrustgo-gmp --lib` 154 tests PASS，含 hash-chain tamper tests |
| GMP Hybrid Retrieval | N/A | DONE / 受控 | RRF、filter、citation tests；目标是 GMP 内审检索，不是通用搜索引擎 |
| RAG Evidence Bundle | N/A | DONE / 受控 | citation/evidence_hash/answer envelope tests；需结合 GMP fixture 做质量评估 |
| Internal Vector Retrieval | PARTIAL | PARTIAL / blocker | v3.12 支持内部 GMP/RAG 检索用途；rebuild、dimension/hash、empty-index、质量 fixture 由 [#4225](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4225) 收口；不宣称通用独立向量数据库 |
| SQL-backed Graph Projection | N/A | DONE / 受控 | BFS 子图、EvidenceBundle、GraphStats；不宣称通用图数据库 |
| Row-Level Security / Column Privileges | DONE | DONE / 持续硬化 | v3.11 主路径能力；GMP 权限矩阵仍需 v3.12 生产路径验证 |
| 存储过程 | UNSUPPORTED | UNSUPPORTED | MySQL compat 明确 `CREATE PROCEDURE` 需要 stored procedure catalog |
| 通用复制 / 分布式 | UNSUPPORTED | UNSUPPORTED | 不作为 v3.12 当前目标 |
| 完整 MySQL 5.7 替代 | PARTIAL | OPEN / 非当前声明 | 需要 SQLLogicTest、TPC-H correctness、wire、LOAD DATA、recovery、upgrade 等全部闭环；[#4220](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4220) 要求 README 不得把悬空 PARTIAL 当生产能力 |
| 通用向量数据库 / 图数据库 | UNSUPPORTED | DEFERRED | v4.0.0 方向，不是 v3.12 对外声明 |

## TPC-H 与性能基准

### 口径说明

TPC-H 的 `22/22 completed` 表示 22 个 query 都跑完且没有 OOM/panic；它不自动等于跨引擎结果完全正确。若要宣称 correctness，需要 row count、canonical SHA256 和外部 oracle 对比。

### 历史与当前 TPC-H 状态

| 版本 / 场景 | 状态 | 可声明内容 | 不可声明内容 |
|---|---|---|---|
| v3.9.0 SF=0.1 | DONE / 历史基准 | SF=0.1 22 query 历史性能基准存在 | 不能外推为 SF=1/SF=10 生产能力 |
| v3.10.0 并行执行优化 | DONE / 历史优化 | 部分 TPC-H query 在大数据上有并行和 fast-load 优化记录 | 不能宣称所有 query 线性加速 |
| v3.11.0 SF=1 | DONE with correctness follow-up | 22/22 completed，519.15s，0 OOM，0 panic | 不能宣称 PostgreSQL/MySQL SHA256 零差异 |
| v3.12.0 SF=1 close-out | PARTIAL / blocker | row count baseline 和 zero-row explanation 已形成 | cross-engine SHA256 和 zero-row correctness 由 [#4221](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4221) 收口 |
| v3.12.0 SF=10 harness | PARTIAL / blocker | harness 可运行；当前不是完整 60M lineitem 生产证据 | 不能宣称真实 SF=10 全量 parity；[#4020](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4020)、[#4217](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4217) 继续整改 |
| v3.12.0 Bulk-load SF=10 | PARTIAL / OPEN | runner/gate/evidence 记录 3/8 表 match，5/8 大表未完成 | 不能宣称 8 表真实 bulk-load 完成 |

### v3.11.0 SF=1 关键数据

| 指标 | 结果 |
|---|---:|
| 数据规模 | lineitem 6,001,215 行，8 表约 8.66M 行 |
| 加载路径 | BINT mmap fixture，不是 LOAD DATA |
| 执行结果 | 22/22 query completed |
| 总耗时 | 519.15s |
| 稳定性 | 0 OOM / 0 panic |
| 结果边界 | 8 个 zero-row query 仍需外部 oracle correctness 验证 |

### v3.12.0 SF=10 当前边界

[#4018 evidence](docs/releases/v3.12.0/evidence/issue-4018/4018_evidence.md) 明确说明：

- SF=10 harness 和 gate 基础设施已交付。
- 最新证据中 3/8 TPC-H SF=10 表已完成 `parity=match`。
- 剩余大表受 FileStorage 全表重序列化导致的吞吐瓶颈阻塞，需 #4217 继续整改。
- 还没有 8/8 表完整 row-count/hash parity，因此不能声明 SF=10 生产完成。

[#4020 evidence](docs/releases/v3.12.0/evidence/issue-4020/4020_evidence.md) 明确说明：

- Bulk-load runner 存在。
- schema creation 失败是旧阶段问题，最新状态已推进到 3/8 表 `parity=match`。
- 5/8 大表仍未完成，不能宣称 8 表真实 bulk-load 完成。

因此 README 不再把 SF=10 写成完成状态。

## GMP / RAG / Vector / Graph

v3.12.0 的 GMP 方向是“受控内审检索系统数据库”，不是通用数据库产品宣传。

| 模块 | 当前状态 | 证据 |
|---|---:|---|
| GMP schema / version / chunk / relation / audit | DONE | [V312-02](docs/releases/v3.12.0/v312-02-gmp-schema-report.md) |
| Idempotent GMP markdown ingestion | DONE | [V312-03](docs/releases/v3.12.0/v312-03-gmp-ingestion-report.md) |
| Embedding provider | DONE / 受控 | [V312-04](docs/releases/v3.12.0/v312-04-embedding-provider-report.md) |
| Hybrid retrieval | DONE / 受控 | [V312-05](docs/releases/v3.12.0/v312-05-hybrid-retrieval-report.md) |
| SQL-backed graph projection | DONE / 受控 | [V312-06](docs/releases/v3.12.0/v312-06-graph-projection-report.md) |
| RAG evidence bundle | DONE / 受控 | [V312-07](docs/releases/v3.12.0/v312-07-rag-evidence-bundle-report.md) |
| GMP compliance audit controls | PARTIAL / blocker | [GMP 合规矩阵](docs/releases/v3.12.0/GMP_COMPLIANCE_MATRIX.md)、[V312-08](docs/releases/v3.12.0/v312-08-compliance-audit-report.md)；生产 ACL/audit-chain/tamper 全链路由 [#4226](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4226) 收口 |

允许的产品声明：SQLRustGo v3.12 支持受控 GMP 内审检索工作负载中的关系存储、chunk、embedding、audit trail、evidence relation、hybrid retrieval 和 SQL-backed graph projection。

禁止的产品声明：v3.12 是通用独立向量数据库、通用图数据库或完整 MySQL 5.7 替代品。

## 测试与质量门禁

v3.12.0 采用分层测试体系，避免把慢测试、性能测试、coverage 和 SOAK 混成一个不可维护的总门禁。

| 层级 | 类型 | 当前目标 |
|---|---|---|
| L0 | 单元测试 | 受影响 crate 和核心 crate 必须 PASS |
| L1 | 集成测试 | SQL / storage / transaction / GMP 主路径可复跑 |
| L2 | E2E / wire / SQLLogicTest smoke | 失败必须 issue-linked，不允许静默 ignore |
| L3 | per-crate coverage | 统一命令、统一报告；低覆盖必须 owner/expiry/issue |
| L4 | 性能测试 | TPC-H、Sysbench、bulk-load、RAG/vector 单独产出趋势和 artifact |
| L5 | SOAK / crash / recovery | Beta/RC/GA 前单独验收，不作为每 PR 门禁 |

关键文档：

- [v3.12 TEST_PLAN](docs/releases/v3.12.0/TEST_PLAN.md)
- [综合测试框架与覆盖率基线](docs/releases/v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md)
- [v3.12 Scope Table](docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md)
- [SQLLogicTest gate 报告](docs/releases/v3.12.0/sqllogictest-oracle-gate-report.md)

## 文档资源

| 文档 | 说明 |
|---|---|
| [CHANGELOG](CHANGELOG.md) | 版本变更历史 |
| [RELEASE_NOTES](RELEASE_NOTES.md) | 发行说明索引 |
| [v3.11 综合评估](docs/releases/v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md) | v3.11 GA 可信度、边界和遗漏测试 |
| [v3.11 TPC-H SF=1 报告](docs/releases/v3.11.0/TPCH_SF1_22_22_PASS_REPORT.md) | SF=1 22/22 可运行性证据 |
| [v3.12 README](docs/releases/v3.12.0/README.md) | v3.12 产品契约 |
| [v3.12 DEVELOPMENT_PLAN](docs/releases/v3.12.0/DEVELOPMENT_PLAN.md) | v3.12 开发计划 |
| [v3.12 FEATURE_CHECKLIST](docs/releases/v3.12.0/FEATURE_CHECKLIST.md) | v3.12 功能清单 |
| [v3.12 PARTIAL 功能整改 Issue 计划](docs/releases/v3.12.0/PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md) | README 中 PARTIAL/OPEN 功能的整改归属、issue 和关闭边界 |
| [v3.12 STAGE](docs/releases/v3.12.0/STAGE.yaml) | v3.12 阶段 SSOT |
| [governance](docs/governance/) | 真实性、门禁、Issue 关闭和多 AI 协作规范 |

## 历史版本

| 版本 | 阶段 / 定位 | 说明 |
|---|---|---|
| v3.12.0 | ALPHA | GMP 内审检索数据库 + v3.11 弱项硬化 |
| v3.11.0 | GA | 简单生产/受控场景候选；TPC-H SF=1 可运行性突破；仍有 correctness 和生产边界 |
| v3.10.0 | GA / 历史 | 并行执行器、fast-load、MySQL 5.7 替代方向推进；部分测试增强未成为阻断门禁 |
| v3.9.0 | GA / 历史 | TPC-H SF=0.1、长稳、治理真实性修复的重要版本 |
| v3.8.0 | GA / 历史 | canonical server 入口、WAL/recovery/文档治理演进 |
| v3.7.0 | GA / 历史 | truthfulness framework 和覆盖率争议治理成形 |
| v3.6.0 | GA / 历史 | 早期覆盖率和 Beta/GA 口径漂移需以后续版本纠正阅读 |

## 贡献指南

```bash
# 格式
cargo fmt --check --all

# 构建
cargo build --all-features

# 测试
cargo test --all-features

# Clippy
cargo clippy --all-features -- -D warnings

# 文档链接
bash scripts/gate/check_docs_links.sh
```

提交文档或关闭 Issue 前，请遵循：

- [ADR-001 Truthfulness Framework](docs/governance/adr/ADR-001-truthfulness-framework.md)
- [Anti-Fabrication Policy](docs/governance/ANTI_FABRICATION_POLICY.md)
- [Issue Closing Verification](docs/governance/ISSUE_CLOSING_VERIFICATION.md)
- [Document Correction Rules](docs/governance/DOC_CHECK_CORRECTION_RULES.md)
- [ADR-008 Test Claim Transparency](docs/governance/adr/ADR-008-test-claim-transparency.md)
- [ADR-014 Multi-AI Coordination](docs/governance/adr/ADR-014-multi-ai-coordination.md)

## 许可证

MIT License，详见 [LICENSE](LICENSE)。
