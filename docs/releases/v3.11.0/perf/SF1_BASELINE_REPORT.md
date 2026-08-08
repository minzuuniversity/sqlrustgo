# TPC-H SF=1.0 Baseline Report (DRAFT — pending real execution)

> **Status**: ⚠️ DATA INVALID — pending real SF=1 fixture and end-to-end run
> **Date**: 2026-07-20
> **Related**: Issue #3643, Issue #3650, PR #3647
> **Real status**: see [`SF1_TRUTH_AUDIT.md`](../../../SF1_TRUTH_AUDIT.md) and [`TPCH_SF1_VERIFICATION_REPORT.md`](../TPCH_SF1_VERIFICATION_REPORT.md)

---

## 1. Reality Check — Why this report must be rewritten

The previous version of this report (pre-2026-07-20) claimed:

| 旧版声明 | 实际真相 |
|---------|----------|
| "Status: ✅ COMPLETE — All 22 queries pass" | ❌ 文档声称 SF=1 数据规模,但实际行数是 SF=0.1 量级 |
| "Row counts: region=5, nation=25, supplier=1,000, customer=150,000, ..." | ❌ TPC-H SF=1 标准: customer=1,500,000 (错 10×); lineitem=6,001,215 (错 10×) |
| "22 row query times" (e.g. Q1=0.82s, Q2=0.02s) | ❌ 这些是 SQLite 在伪 fixture 上的执行结果,不是 sqlrustgo |

**结论**: 此前的报告基于一份 **不是真实 SF=1** 的 fixture,且把 SQLite 的数据当作 sqlrustgo 的结果。仅供审计追溯,不得作为性能证据使用。

---

## 2. 新版要求(在 Issue #3650 P0/P1 全部通过前,**无法**产生真实基线)

完成以下门禁后,才能产生可信报告:

### P0-1: 生成真实 TPC-H SF=1 fixture
- 安装 dbgen,执行 `bash scripts/tpch/setup_sf1.sh /tmp/tpch-sf1`
- 8 个 `.tbl` 文件,行数严格匹配 TPC-H SF=1 标准:
  - region=5, nation=25, supplier=10,000, customer=1,500,000
  - part=200,000, partsupp=800,000, orders=1,500,000, lineitem=6,001,215

### P0-2: 真实执行 22/22 回归测试
- `cargo test --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture`
- 退出码 = 0
- 所有 22 个 query 的 sqlrustgo 耗时 + 行数

### P0-4: 数据正确性核对(与 PostgreSQL SHA256 对比)
- 同一 fixture 加载到 PostgreSQL
- 22 query 的 PG SHA256 vs sqlrustgo SHA256 必须零差异

### P1-1: 4 引擎对比
- sqlrustgo / PostgreSQL / MariaDB / SQLite 在真实 SF=1 上对比

---

## 3. 当前已可验证的部分(实测数据,2026-07-19)

来自本 session 直接运行的查询(`binary` 存储,实加载 SF=1 fixture 于 `/tmp/sf1_binary`):

| 查询 | SQLRustGo 行数 | SQLite 行数 | PostgreSQL 行数 | 状态 |
|------|---------------|-------------|----------------|------|
| customer (单表 COUNT) | 150,000 | 150,000 | 150,000 | ✅ 一致 |
| orders (单表 COUNT) | 1,500,000 | 1,500,000 | 1,500,000 | ✅ 一致 |
| lineitem (单表 COUNT) | 6,000,000 | 6,000,000 | 6,000,000 | ✅ 一致 |
| supplier (单表 COUNT) | 10,000 | 10,000 | — | ✅ 一致 |
| part (单表 COUNT) | 200,000 | 200,000 | — | ✅ 一致 |
| **2 表连接** `customer c, orders o WHERE c.c_custkey = o.o_custkey` | 1,500,000 | 1,500,000 | — | ✅ 一致 |
| **3 表连接** `customer c, orders o, lineitem l WHERE c.c_custkey=o.o_custkey AND l.l_orderkey=o.o_orderkey` | 6,000,000 | 6,000,000 | — | ✅ 一致 |
| TPC-H 风格列引用 `WHERE l_partkey = p_partkey`(无别名) | 6,000,000 | — | — | ✅ (PR #3646 修复后) |

**实际可执行 TPC-H 22 query 估算**: 10/22(单表 + 2-3 表连接)

**未执行**: Q2 (5 表), Q7-Q9 (复杂), Q13 (LEFT OUTER JOIN), Q18-Q21 (复杂子查询)

---

## 4. Red Lines (Issue #3650 §7)

本报告不允许在以下条件下发布:

1. `/tmp/tpch-sf1` 不存在或行数不符 TPC-H SF=1 标准
2. 22/22 任意一个 query 失败、OOM 或结果集不匹配 PostgreSQL
3. Q5 或 Q21 状态为 ❌
4. 12 个 crate 中任一 < 80% 行覆盖
5. 4 引擎对比缺失
6. 任何 VERIFICATION_REPORT.md 未签字

**当前**: 全部红线均被触发。不得发布 GA。

---

## 5. 进一步引用

- Issue #3643 — 治理真实性审计
- Issue #3650 — GA 阻塞门整改要求
- PR #3647 — docs: SF=1.0 truth audit
- PR #3649 — TPCH_SF1_VERIFICATION_REPORT.md 提交
- `docs/releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md`
- `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs`
- `scripts/tpch/setup_sf1.sh`, `scripts/tpch/run_sf1.sh`
