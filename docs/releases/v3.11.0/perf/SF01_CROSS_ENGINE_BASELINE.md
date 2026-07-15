# TPC-H SF=0.1 Cross-Engine Baseline Report

**日期**: 2026-07-15  
**Scale Factor**: 0.1  
**Fixture**: `tests/data/tpch-sf01/*.tbl` (dbgen 格式)  
**SQLite 版本**: 3.45.1

## 数据库规模 (SF=0.1)

| 表 | 行数 | 说明 |
|----|------|------|
| region | 5 | 5 个地区 |
| nation | 25 | 25 个国家 |
| supplier | 1,000 | 1,000 个供应商 |
| customer | 15,000 | 15,000 个客户 |
| part | 20,000 | 20,000 个零件 |
| partsupp | 80,000 | 80,000 个零件供应关系 |
| orders | 150,000 | 150,000 个订单 |
| lineitem | 600,572 | 600,572 个订单明细 |

## SQLite SF=0.1 性能数据

> 测试条件: SQLite 3.45.1, 64MB cache, Intel Xeon Gold 6138, Ubuntu Linux

| Q | 行数 | SQLite (ms) | 备注 |
|----|------|-------------|------|
| Q1 | 4 | 448.1 | 单表聚合 |
| Q2 | 20 | 21.2 | 5 表 join + 过滤 |
| Q3 | 15,224 | 248.1 | 3 表 join + group by |
| Q4 | 5 | 21.0 | 2 表 join + 存在性子查询 |
| Q5 | 5 | 185.0 | 6 表 join + 聚合 |
| Q6 | 1 | 94.5 | 单表 + 复杂过滤 |
| Q7 | 7 | 293.2 | 6 表 join + self-join nation |
| Q8 | 7 | 597.5 | 8 表 join + CASE 表达式 |
| Q9 | 0 | 482.2 | 5 表 join + LIKE 过滤 |
| Q10 | 20 | 156.6 | 4 表 join + 聚合 |
| Q11 | 2,541 | 67.0 | 3 表 join + HAVING |
| Q12 | 2 | 139.0 | 2 表 join + 复杂过滤 |
| Q13 | 37 | 141.4 | 子查询 + LEFT OUTER JOIN |
| Q14 | 1 | 113.6 | 2 表 join + 聚合 |
| Q15 | 1,000 | 126.3 | CTE + 物化子查询 |
| Q16 | 2,358 | 32.6 | 3 表 join + 聚合 |
| Q17 | 1 | 459.0 | 4 表 join + 相关子查询 |
| Q18 | 5 | 347.7 | 3 表 join + HAVING |
| Q19 | 1 | 358.3 | 3 表 join + UNION ALL |
| Q20 | 0 | 0.2 | 子查询 + NOT EXISTS |
| Q21 | 50 | 256.5 | 4 表 join + 双 EXISTS |
| Q22 | 7 | 5.4 | 聚合 + NOT EXISTS |

## 对比: sqlrustgo (历史数据, SF=0.1)

> 来自 `docs/releases/v3.10.0/perf/SF0.1_BASELINE_REPORT.md` (v3.10.0)

| Q | sqlrustgo (ms) | SQLite (ms) | sqlrustgo 相对 SQLite |
|----|----------------|-------------|----------------------|
| Q1 | 41.2 | 448.1 | **0.09x** faster |
| Q2 | 2,288.6 | 21.2 | 108x slower |
| Q3 | 11.9 | 248.1 | **0.05x** faster |
| Q4 | 41.5 | 21.0 | 2.0x slower |
| Q5 | 9,946.5 | 185.0 | 54x slower |
| Q6 | 0.2 | 94.5 | **460x** faster |
| Q7 | 58.7 | 293.2 | **0.20x** faster |
| Q8 | 50.3 | 597.5 | **0.08x** faster |
| Q9 | 112.9 | 482.2 | **0.23x** faster |
| Q10 | 24.6 | 156.6 | **0.16x** faster |
| Q11 | 411.6 | 67.0 | 6.1x slower |
| Q12 | 0.4 | 139.0 | **347x** faster |
| Q13 | 97.9 | 141.4 | **0.69x** faster |
| Q14 | 19.3 | 113.6 | **0.17x** faster |
| Q15 | 41.5 | 126.3 | **0.33x** faster |
| Q16 | 413.8 | 32.6 | 12.7x slower |
| Q17 | 15.2 | 459.0 | **0.03x** faster |
| Q18 | 49.1 | 347.7 | **0.14x** faster |
| Q19 | 56.7 | 358.3 | **0.16x** faster |
| Q20 | 71.8 | 0.2 | 359x slower |
| Q21 | 41.5 | 256.5 | **0.16x** faster |
| Q22 | 173.9 | 5.4 | 32x slower |

## 观察

### sqlrustgo 优势查询 (快 10x 以上)
- **Q6**: 460x 快 — 单表扫描 + 强过滤
- **Q12**: 347x 快 — 2 表 join + 强过滤
- **Q17**: 33x 快 — 相关子查询优化
- **Q1**: 11x 快 — 单表聚合

### sqlrustgo 劣势查询 (慢 10x 以上)
- **Q20**: 359x 慢 — NOT EXISTS 子查询
- **Q2**: 108x 慢 — 5 表 join, O(n²) 中间结果
- **Q5**: 54x 慢 — 6 表 join, nation-bridge 循环
- **Q11**: 6x 慢 — 3 表 join
- **Q16**: 13x 慢 — 3 表 join + 聚合
- **Q22**: 32x 慢 — 聚合 + NOT EXISTS

### OOM 问题 (SF=1.0)
在 SF=1.0 下，以下查询会 OOM:
- **Q2**: 18.1 GB — supplier × partsupp 笛卡尔积
- **Q5**: 18.1 GB — 循环 join 图
- **Q21**: 18.9 GB — lineitem 别名表笛卡尔积

## 环境

- **SQLite**: 3.45.1 (Ubuntu 24.04)
- **PostgreSQL**: 16.14 (可用但需密码认证)
- **MySQL/MariaDB**: 未安装
- **sqlrustgo**: v3.10.0 / v3.11.0
- **CPU**: Intel Xeon Gold 6138 @ 2.00 GHz
- **RAM**: ~31 GB

## 下一步

1. 配置 PostgreSQL 无密码访问或提供密码
2. 安装 MariaDB 并加载 TPC-H SF=0.1 数据
3. 修复 Q2/Q5/Q21 OOM 问题
4. 在 SF=1.0 上重新运行 sqlrustgo 性能测试
