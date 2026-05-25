# 开源数据库测试与文档体系调研报告

> 调研对象：SQLite、PostgreSQL、MySQL、DuckDB
> 日期：2026-05-25
> 目标版本：v3.4.0 — 打造具有完整测试体系的示范版本

---

## 一、测试体系横向对比

### 1.1 测试框架与覆盖率

| 数据库 | 单元测试框架 | 覆盖率目标 | 测试分层 |
|--------|-------------|-----------|---------|
| **SQLite** | TCL + TH3（自研）+ sqllogictest | 100% MC/DC（分支覆盖） | 4个独立测试引擎 |
| **PostgreSQL** | TAP（Perl）+ pg_regress | 无硬性指标 | regression/isolation/performance |
| **MySQL** | mysql-test-run（自研TCL） | 无公开要求 | unit/integration/mtr |
| **DuckDB** | sqllogictest + C++ | 无硬性指标，PR前必须通过 | unit(~1min)/allunit(~1h) |
| **SQLRustGo** | cargo test + 门禁脚本 | 85%（当前82.88%） | Alpha/Beta/RC/GA 四阶段 |

### 1.2 测试类型对比

| 测试类型 | SQLite | PostgreSQL | MySQL | DuckDB | SQLRustGo |
|---------|--------|-----------|-------|--------|-----------|
| 单元测试 | ✅ TCL/TH3 | ✅ TAP | ✅ mtr | ✅ sqllogictest | ✅ cargo test |
| 集成测试 | ✅ SQL tests | ✅ pg_regress | ✅ mtr suites | ✅ sqllogictest | ✅ |
| Chaos Testing（OOM/IO/Crash） | ✅ TH3专项 | ❌ | ❌ | ❌ | ❌ |
| Fuzz Testing | ✅ AFL+OSS-Fuzz+dbsqlfuzz | ❌ | ✅ | ❌ | ❌ |
| 性能/Benchmark | ❌ | ✅ pgbench | ✅ | ✅ benchmark_runner | ✅ TPC-H SF=1 |
| Release Checklist | ✅ ~200项 | ❌ | ❌ | ❌ | ❌ |

### 1.3 SQLite 异常测试亮点

- OOM 测试：模拟内存耗尽，验证优雅降级
- I/O Error 测试：模拟磁盘故障，验证数据完整性
- Crash 测试：模拟进程崩溃，验证 WAL recovery
- Compound failure：多重故障叠加

### 1.4 DuckDB 测试文档（最具参考价值）

- `make unit` — 快速测试（~1分钟）
- `make allunit` — 完整测试（~1小时）
- Benchmark Guide 独立文档

---

## 二、文档体系横向对比

| 数据库 | 用户文档 | 开发者文档 | 架构文档 | Release Notes |
|--------|---------|-----------|---------|--------------|
| **DuckDB** | ✅ duckdb.org/docs | ✅ CONTRIBUTING | Engineering Blog | GitHub Releases |
| **SQLite** | ✅ sqlite.org/ | 代码注释+wiki | 设计文档 | 极详细 |
| **PostgreSQL** | ✅ postgresql.org/docs | 有限 | 有限 | 每个版本分页 |
| **MySQL** | ✅ dev.mysql.com | 分散 | 有限 | Oracle官方 |
| **SQLRustGo** | ⚠️ docs/ | ❌ 缺失 | ⚠️ 部分 | ⚠️ 刚补全 |

---

## 三、v3.4.0 问题清单

### 测试体系

| 问题 | 严重性 |
|------|--------|
| 门禁脚本 A5 bug | P0 |
| 门禁脚本 G6/G7 bug | P0 |
| 覆盖率 82.88% < 85% | P0 |
| G-TI 前缀缺失 | P1 |
| 无 Chaos Testing | P1 |
| 无统一测试命令 | P1 |
| 无 Fuzz Testing | P2 |
| 无 Release Checklist | P2 |

### 文档体系

| 问题 | 严重性 |
|------|--------|
| 无 CONTRIBUTING.md | P0 |
| 无 BUILD.md | P0 |
| 无 TESTING.md | P1 |
| 无统一 Makefile | P1 |
| 无架构设计文档体系 | P2 |

---

## 四、v3.4.0 改进计划

| 阶段 | 任务 | 优先级 |
|------|------|--------|
| **立即** | 修复门禁脚本 bug | P0 |
| **立即** | 覆盖率提升到 85%+ | P0 |
| **本周期** | 创建 CONTRIBUTING.md | P0 |
| **本周期** | 创建 BUILD.md | P0 |
| **本周期** | 创建 TESTING.md | P1 |
| **本周期** | 创建标准 Makefile | P1 |
| **下个版本** | 增加 Chaos Testing | P1 |
| **下个版本** | 建立 Release Checklist | P2 |
| **长期** | Fuzz Testing 集成 | P2 |
| **长期** | 性能回归测试自动化 | P2 |
| **长期** | CHANGELOG 自动生成 | P2 |
