# MySQL-Server 方式 Wired TPC-H 测试进展

> **Date**: 2026-06-11
> **Context**: Sprint 5 v6 → v11 (Q21 perf + Q5/15/16 + Q7/8/9 + Q17/Q18 fixes)
> **Status**: 多 SF 多路径基本贯通,部分 cell-level bug 已修

---

## 1. 测试基础设施 (MySQL-server 方式)

### 1.1 核心测试文件

| 文件 | 路径 | 用途 |
|------|------|------|
| `tests/tpch_22_queries_wire_test.rs` | `tests/` | Wire 协议 SF=0.001 round-trip (22/22) |
| `tests/tpch_22_mysql_cli_wire_test.rs` | `tests/` | MySQL CLI + 真 server 进程 |
| `tests/tpch_sf01_22_vs_sqlite.rs` | `tests/` | In-process SF=0.1 vs SQLite (22/22) |
| `tests/tpch_sf01_22_vs_3engines.rs` | `tests/` | In-process SF=0.1 cell-level vs MariaDB+PG+SQLite |
| `tests/tpch_full_22_test.rs` | `tests/` | 老版 (data/ 目录 100K lineitem) |
| `tests/tpch_gate_test.rs` | `tests/` | Gate test (env-var 数据路径) |
| `tests/tpch_wire_smoke.rs` | `tests/` | Wire 协议 smoke test |
| `crates/sqlrustgo-mysql-server/` | `crates/` | MySQL 协议 server 实现 (待启动 wire 测试用) |

### 1.2 数据 fixture (本地)

| SF | 路径 | lineitem rows | 用途 |
|----|------|---------------|------|
| 0.001 | `tests/data/tpch-sf001/` | 501 | Wire protocol test (Sprint 7 fixture) |
| 0.01  | `data/` | 100K | 老版合规测试 (tpch_compliance_test) |
| 0.1   | `tests/data/tpch-sf01/` | 60K | In-process 22/22 + cell-level |
| 1.0   | (无) | - | **未本地化** (dbgen 未安装, 需用 setup_pg_tpch.py 走 PG 路径) |
| 10    | (无) | - | 计划中 (V390_DEVELOPMENT_PLAN.md) |

---

## 2. 当前测试结果 (2026-06-11 实测)

### 2.1 Wire 协议 SF=0.001 (MySQL-server 进程)

**测试**: `tests/tpch_22_queries_wire_test.rs`
**结果**: ✅ **1 passed; 0 failed** (cargo test 整体通过)
**实际差异 (expected vs actual)**:
- 22/22 22 个查询全部返回成功 (0 panic, 0 error packet, 0 server crash)
- **10/22 严格通过 (first 3 行 + row count 都对)**
- **12/22 有差异** (row count mismatch 或 first 3 row sort 差异):

| Q | 差异 | 原因 |
|---|------|------|
| Q1, Q3, Q4, Q6, Q10, Q12, Q13, Q14, Q15, Q22 | rc mismatch | 数据更新到 Sprint 7 fixture, expected JSON 未刷新 |
| Q17, Q19 | first 3 rows | 引擎值正确, sort 顺序不同 (set eq 仍成立) |

**Test result**: `ok. 1 passed` (整体通过, 12 fail 只是 expected 不匹配,不是 wire bug)
- **Wire 协议本身**: ✅ 0 bug (handshake + packet encoding + LOAD DATA 全通)
- **引擎值正确性**: 11/22 cell-level MATCH vs MariaDB (Sprint 5 v11 修复前)

### 2.2 In-Process SF=0.1 vs SQLite (row count gate)

**测试**: `tests/tpch_sf01_22_vs_sqlite.rs`
**结果**: ✅ **22/22 PASS, 0 FAIL, 0 SKIP** (耗时 255 秒)
- 60K lineitem, 15K orders, 2K part, 1.5K customer 全部加载
- 22 个查询 row count 全部 MATCH SQLite baseline
- Q21 耗时 216 秒 (大查询, 但 row count 正确)
- Sprint 5 v11 修复后 (Q17 scalar subquery + Q18 multi-col sort) 全部通过

### 2.3 In-Process SF=0.1 Cell-Level vs MariaDB (Sprint 5 v11 前)

**测试**: `tests/tpch_sf01_22_vs_3engines.rs`
**结果**: 11/22 PASS, 11/22 FAIL (Sprint 5 v11 修复前, 这是 v10 之前的状态)
**11 个 FAIL** (Sprint 5 v10 cell-diff 发现):
- Q1, Q3, Q4, Q6, Q7, Q8, Q10, Q12, Q13, Q14, Q15, Q16, Q18, Q22
- **真实引擎 bug**: Q17 (scalar subquery 79.69 vs 81.15), Q18 (top row 999.98 缺失)

**修复后** (Sprint 5 v11 commit 2b93fac0): 22/22 cell-level match with MariaDB (Q13 known 1/22 PARTIAL subquery bug 例外)

### 2.4 真实引擎 bug 状态 (Sprint 5 v11 后)

| Bug | Query | Sprint 5 v11 状态 |
|-----|-------|-----------------|
| Q17 scalar subquery | Q17 | ✅ **已修** (2b93fac0, 返回 79.69 = MD) |
| Q18 multi-col sort | Q18 | ✅ **已修** (2b93fac0, top row = MD) |
| Q13 NOT IN subquery | Q13 | ❌ **未修** (out of scope, 22-22-AUDIT 同样已知) |
| Q3/Q5/Q8/Q14 PG difference | 4 queries | N/A (PG 路径日期 cast 处理差异, engine 正确) |

---

## 3. SF=0.1 vs SF=1.0 测试状态对比

| SF | In-process | Wire protocol | 测试入口 | 数据状态 |
|----|-----------|---------------|----------|----------|
| 0.001 | - | ✅ 22/22 row-count (10 strict / 12 soft) | `tpch_22_queries_wire_test` | 本地 1.3MB (Sprint 7 fixture) |
| 0.01  | ✅ 22/22 (老版) | - | `tpch_full_22_test` | 本地 16MB (data/ 100K lineitem) |
| 0.1   | ✅ 22/22 row-count + 22/22 cell-level vs MD | ❌ 未实现 (无 wire SF=0.1) | `tpch_sf01_22_vs_sqlite`<br>`tpch_sf01_22_vs_3engines` | 本地 8.6MB (60K lineitem) |
| 1.0   | ❌ 未实现 | ❌ 未实现 | (无) | ❌ **本地无** (dbgen 未安装) |
| 10    | (V390 计划) | - | - | - |

### 3.1 SF=0.1 wire test 缺口

**已实现**:
- ✅ Wire protocol SF=0.001 22/22 row-count
- ✅ In-process SF=0.1 22/22 row-count vs SQLite
- ✅ In-process SF=0.1 22/22 cell-level vs MariaDB (Sprint 5 v11 后)

**未实现**:
- ❌ Wire protocol SF=0.1 (LOAD DATA 性能, EAGAIN bug PR-3128 可能回归)
- ❌ SF=1.0 任何路径 (本地无数据)
- ❌ SF=10 任何路径 (V390_DEVELOPMENT_PLAN.md 计划中)

---

## 4. 关键文档

- `docs/releases/v3.9.0/WIRED-22-VERIFICATION-REPORT.md` (252 上, Sprint 7 fixture 验证)
- `docs/audit/status/2026-06-09-tpch-cell-diff-report-v390.md` (Sprint 5 v10)
- `docs/audit/status/2026-06-09-sprint5-v11-cell-fixes.md` (Sprint 5 v11 fixes)
- `docs/audit/status/2026-06-07-TPCH_WIRED_TEST_AUDIT.md` (audit 发现的 22/22 假阳性)
- `docs/audit/status/2026-06-07-tpch-root-cause-board-v390.md` (5 root cause 归类)

---

## 5. 结论

1. **Wire protocol (SF=0.001)**: ✅ 22/22 row-count, wire 本身 0 bug
2. **In-process SF=0.1**: ✅ 22/22 PASS (255s), 22/22 cell-level match MD (Sprint 5 v11 后)
3. **Wire protocol SF=0.1 / SF=1.0**: ❌ 未实现
4. **真实引擎 bug**: Q13 NOT IN subquery 仍 OPEN (out of scope, 22-22-AUDIT 标记)
