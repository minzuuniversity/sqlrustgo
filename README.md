# SQLRustGo

> **当前开发线**: `develop/v3.12.0`
> **当前开发版**: v3.12.0
> **发布状态**: **GA promotion authorized**（2026-09-08，72/72 gate PASS）
> **可信状态入口**: [docs/releases/v3.12.0/STAGE.yaml](docs/releases/v3.12.0/STAGE.yaml)
> **GA 发布证据入口**: [docs/releases/v3.12.0/GA_PUBLICATION_EVIDENCE_INDEX.md](docs/releases/v3.12.0/GA_PUBLICATION_EVIDENCE_INDEX.md)

---

## 0. 项目概述

SQLRustGo 是一个用 Rust 实现的 SQL 数据库项目，包含 MySQL 风格服务器、嵌入式 SQL 执行路径、存储/事务组件和基于证据的质量门禁治理体系。

当前 **v3.12.0** 版本定位为 **GMP 合规性内审检索系统** 数据库底座：

- SQLRustGo 管理的 GMP 文档、chunk、版本、审计记录、证据关系和检索元数据的关系存储
- 面向 GMP/RAG 工作负载的内部向量检索和混合检索
- 用于证据导航的 SQL-backed 图投影
- 仅在验证的兼容性边界内提供 MySQL 风格和 sqlite-like 入口

**禁止声明**：
- ❌ 通用向量数据库
- ❌ 通用图数据库
- ❌ 广泛的 MySQL/SQLite 替代品

---

## 1. 当前状态（2026-09-08）

### GA 门禁状态

| Gate | 状态 |
|------|------|
| BETA | 40/40 ✅ PASS |
| RC | 11/11 ✅ PASS |
| GA | 8/8 ✅ PASS |
| thresholds_override | 13/13 ✅ PASS |
| **总计** | **72/72 ✅ PASS** |

- commit: `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4`
- post-cut docs/evidence refresh HEAD: `9febebb255f984387ac78c26510d5b46d73f6046`
- mode: `full`
- blockers: 0

### 已知限制 — GA-claim-caveat

| Issue | 区域 | 限制范围 |
|-------|------|----------|
| #4846 | 执行器/类型语义 | `CHAR(n)` 字节填充主键点查排除 |
| #4847 | 事务语义 | 显式 `BEGIN`/`COMMIT`/`ROLLBACK` 排除 |
| #4848 | 存储/DDL | `ALTER TABLE ... RENAME COLUMN` 排除 |

详见 [CLAIM_DOWNGRADE_MANIFEST.md](docs/releases/v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md)

---

## 2. 功能完整性矩阵

### 2.1 主路径能力（v3.11 GA vs v3.12 GA）

| # | 能力 | v3.11 GA | v3.12 GA | 证据 |
|---|------|---------|---------|------|
| 1 | TPC-H SF=1 in-process 22/22 | PASS | **PASS** | Q17 61.6s PASS |
| 2 | TPC-H SF=1 wire round-trip 22/22 | PASS | **PASS** | cross-engine matrix |
| 3 | TPC-H SF=10 bulk load | 部分 | **PASS** | LOAD DATA SF=10 PASS |
| 4 | GMP 文档/Chunk/Embedding/Audit schema | 无 | **PASS** | RC1 wrapper |
| 5 | GMP hybrid retrieval | 无 | **PASS** | RC2 wrapper |
| 6 | SQL-backed graph projection | 无 | **PASS** | depth-limited paths + neighbors |
| 7 | 审计 hash-chain tamper fail-closed | 无 | **PASS** | RC1/RC4 |
| 8 | SQLLogicTest smoke 25/25 | 无 | **PASS** | smoke-report.md |
| 9 | V312-57 sqlite3-like CLI | 无 | **PASS** | RC9 + RC10 |
| 10 | MySQL wire 协议 typed wrappers | 部分 | **PASS** | RC7 wrapper |
| 11 | MySQL TLS 1.3 / compression | 部分 | **PASS** | V312-13-REPORT.md |
| 12 | MySQL prepared statement params | 部分 | **PASS** | V312-13-REPORT.md |
| 13 | LOAD DATA SF=1 / SF=10 | 部分 | **PASS** | V312-13-REPORT.md |
| 14 | Backup / restore (GMP preserved) | 部分 | **PASS** | RC3 wrapper |
| 15 | Crash recovery 7+4+4 | 部分 | **PASS** | RC8 wrapper |
| 16 | RBAC + 安全扫描 | 部分 | **PASS** | RC4 wrapper |
| 17 | 168h SOAK | PASS | **PARTIAL** | GA2 mixed demo PASS；scaffold 就绪 |
| 18 | per-crate 覆盖率 ≥80% | 部分 | **PARTIAL** | 分层口径 |
| 19 | RAG inverted index + rerank | 部分 | **PASS** | RC4 + RC7 |
| 20 | 教学 REPL + 内审检索 demo | 部分 | **PASS** | V312-57 week01-06 |
| 21 | 文档治理 0 overclaim | 部分 | **PASS** | CLAIM_DOWNGRADE_MANIFEST |

**总计**：19/21 PASS，2/21 PARTIAL

### 2.2 TPC-H SF=1 Cross-Engine 对比

| 引擎 | 覆盖数 | 说明 |
|------|--------|------|
| PostgreSQL | 22/22 | 参考 Oracle |
| SQLite | 22/22 | 参考 Oracle |
| MySQL | 18/22 | Q2/Q11/Q12/Q17 deferred（v3.13） |
| **SQLRustGo** | **22/22** | **Q17 61.6s PASS（was TIMEOUT 1042s）** |

详见 [GA5_TPCH_SF1_REPORT.md](docs/releases/v3.12.0/evidence/v312-59/GA5_TPCH_SF1_REPORT.md)

### 2.3 SQLLogicTest 状态

| 类别 | 状态 | 说明 |
|------|------|------|
| smoke | **25/25 PASS** | [smoke-report.md](docs/releases/v3.12.0/sqllogictest-baseline/smoke-report.md) |
| curated selected | **16/21 PASS** | 5 EXCLUDED（issue-linked） |
| historical exclusions | **16/16 closed** | v3.12.0 内关闭 |
| full SQLite corpus | 待 v3.13 | RC/GA expansion |

---

## 3. 稳定性与性能评估

### 3.1 TPC-H SF=1

| 维度 | 结论 | 证据 |
|------|------|------|
| In-process 22/22 执行 | **PASS** | SUMMARY.json |
| Cross-engine 4 engine | **sqlrustgo 22/22** | 4 engine × 22 query |
| Q17 SF=1 | **PASS 61.6s** | PR #4550 (commit `640d672bf8`) |
| Cell-level 匹配 | **22/22** | Q17_SF1_CELLDIFF.json |

### 3.2 SOAK

| 阶段 | 状态 | 证据 |
|------|------|------|
| v3.11 GA 168h SOAK | PASS（343h37m） | SOAK_168H_REPORT.md |
| v3.12 GA mixed demo | PASS | GA2_MIXED_SOAK_DEMO_REPORT.md |
| 168h scaffold | 就绪 | tests/soak/v312_mixed_soak.rs |

### 3.3 LOAD DATA / Bulk

| 测试 | 状态 | 证据 |
|------|------|------|
| SF=0.0001 smoke | PASS | V312-13-REPORT.md step 6.5 |
| SF=1 | PASS | step 7 |
| SF=10 | PASS | step 8 |
| TLS handshake | PASS | step 9 |
| Compression | PASS | step 10 |

### 3.4 Crash Recovery

| 场景 | 状态 | 证据 |
|------|------|------|
| 7+4+4 scenarios | PASS | V312-14-CRASH-RECOVERY-RECHECK.md |

---

## 4. 安全与合规

| 项 | 状态 | 证据 |
|----|------|------|
| RBAC role-based access | PASS | GA-4 wrapper |
| Audit hash-chain tamper fail-closed | PASS | RC1/RC4 |
| 文档 claim 清理 | PASS | 4 ALLOWED / 14 DISALLOWED / 0 OVERCLAIM |
| 安全扫描 | PASS | GA3_SECURITY_SCAN_REPORT.md |
| Secret 扫描 | PASS | secret_scan_v312.txt |
| Plaintext password 扫描 | PASS | plaintext_pw_scan_v312.txt |

---

## 5. MySQL 5.7 替代能力判断

**v3.12.0 GA 不可宣称 MySQL 5.7 替代品**。已具备的 MySQL-style 子集：

| 子集 | 状态 |
|------|------|
| COM_QUERY + COM_STMT_PREPARE/EXECUTE | PASS（typed wrappers） |
| LOAD DATA LOCAL INFILE | PASS（SF=1/10） |
| TLS 1.3 | PASS |
| Compression | PASS |
| prepared statement params | PASS |
| Information schema 部分 | PARTIAL（继承 v3.11） |
| Procedure / Trigger | PARTIAL（继承 v3.11） |

---

## 6. 快速开始

```bash
# 克隆项目
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo

# 构建
cargo build --all-features

# 测试
cargo test --all-features

# 启动 MySQL 风格服务器
cargo run --bin sqlrustgo-mysql-server -- serve --host 127.0.0.1 --port 3307

# sqlite-like 教学 CLI
cargo run --bin sqlrustgo -- sqlite --batch path/to/database.sqlrg < script.sql
```

---

## 7. 架构

```
SQL text / MySQL wire / sqlite-like CLI
        |
Parser -> Planner -> Optimizer -> Executor
        |
Catalog / Storage / Transaction / WAL / MVCC
        |
GMP documents / chunks / audit trail / relations / embeddings
        |
Hybrid retrieval / internal vector retrieval / SQL-backed graph projection
```

工作空间 crate 位于 [crates](crates/)，集成和兼容性测试位于 [tests](tests/)。

---

## 8. 版本定位

| 版本 | 阶段 | 边界 |
|------|------|------|
| **v3.12.0** | GA authorized | GMP 合规性内审检索系统数据库底座 |
| v3.11.0 | GA | 受控/简单生产候选；非完整 MySQL 5.7 替代品 |
| v3.10.0 及更早 | 历史 | 用于追溯功能演进，非当前版本证据 |

---

## 9. 发布规范

SQLRustGo 发布文档遵循证据优先治理原则：

- 文档声明 ≠ 执行证据
- PASS/GA 声明需要实际命令输出、commit、时间戳和证据制品
- Open issues 只能通过显式 release-claim downgrade 排除
- Issue 关闭需要合并的 PR 证据和相关验证

核心策略：
- [ADR-001 真实性框架](docs/governance/adr/ADR-001-truthfulness-framework.md)
- [反伪造政策](docs/governance/ANTI_FABRICATION_POLICY.md)
- [门禁条件](docs/governance/GATE_CONDITIONS.md)
- [Issue 关闭验证](docs/governance/ISSUE_CLOSING_VERIFICATION.md)
- [文档修正规则](docs/governance/DOC_CHECK_CORRECTION_RULES.md)

---

## 10. 开发检查

```bash
# 格式检查
cargo fmt --check --all

# Lint 检查
cargo clippy --all-features -- -D warnings

# 测试
cargo test --all-features

# 文档链接检查
bash scripts/gate/check_docs_links.sh
bash scripts/gate/check_docs_links_v312.sh
bash scripts/gate/check_docs_consistency_v312.sh

# v3.12.0 GA 检查
bash scripts/gate/check_ga_v3.12.0.sh --full
```

不要使用旧的 gate 报告或历史 issue 数量作为新鲜 GA 决策的替代。

---

## 11. 关键文档

| 文档 | 用途 |
|------|------|
| [v3.12.0 README](docs/releases/v3.12.0/README.md) | 当前 v3.12.0 范围、阻塞项和 GA 状态 |
| [v3.12.0 STAGE](docs/releases/v3.12.0/STAGE.yaml) | 阶段 SSOT 和升级要求 |
| [v3.12.0 GA 门禁报告](docs/releases/v3.12.0/GA_GATE_REPORT.md) | GA verdict map 和证据边界 |
| [v3.12.0 综合评估报告](docs/releases/v3.12.0/COMPREHENSIVE_ASSESSMENT_REPORT.md) | GA 综合评估 |
| [v3.12.0 测试框架](docs/releases/v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md) | 测试框架和覆盖率基线 |
| [Claim 降级清单](docs/releases/v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md) | Release-claim 排除和关闭台账 |
| [v3.12.0 测试计划](docs/releases/v3.12.0/TEST_PLAN.md) | 测试层、门禁和证据要求 |
| [v3.12.0 性能报告](docs/releases/v3.12.0/PERFORMANCE_REPORT.md) | 性能测试汇总 |
| [CHANGELOG](CHANGELOG.md) | 历史版本变更 |

---

## 12. 许可

MIT License。详见 [LICENSE](LICENSE)。
