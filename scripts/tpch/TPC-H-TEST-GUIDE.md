# TPC-H 测试指南

> **版本**: v1.0
> **更新日期**: 2026-05-29
> **维护人**: hermes-z440

---

## 一、概述

TPC-H 是一个决策支持基准测试，包含 22 个查询，用于评估数据库系统的查询性能。

**数据规模**: SF=1 (Scale Factor 1)
- Lineitem: 6M 行
- Orders: 1.5M 行
- Customer: 150K 行
- Part: 200K 行
- PartSupp: 800K 行
- Supplier: 10K 行
- Nation: 25 行
- Region: 5 行

---

## 二、数据准备

### 2.1 数据路径

| 路径 | 说明 |
|------|------|
| `/opt/tpch/tpch-dbgen/*_clean.tbl` | 清洗后的 TPC-H 数据文件 |
| `~/sqlrustgo-tpch-sf1/data/` | 本地 SF=1 数据副本 |

### 2.2 数据表 Schema

```sql
-- 来自 scripts/tpch/tpch_schema.sql

CREATE TABLE nation (
    n_nationkey INTEGER PRIMARY KEY,
    n_name VARCHAR(25),
    n_regionkey INTEGER,
    n_comment VARCHAR(152)
);

CREATE TABLE region (
    r_regionkey INTEGER PRIMARY KEY,
    r_name VARCHAR(25),
    r_comment VARCHAR(152)
);

CREATE TABLE part (
    p_partkey INTEGER PRIMARY KEY,
    p_name VARCHAR(55),
    p_mfgr VARCHAR(25),
    p_brand VARCHAR(10),
    p_type VARCHAR(25),
    p_size INTEGER,
    p_container VARCHAR(10),
    p_retailprice DECIMAL(15,2),
    p_comment VARCHAR(23)
);

CREATE TABLE supplier (
    s_suppkey INTEGER PRIMARY KEY,
    s_name VARCHAR(25),
    s_address VARCHAR(40),
    s_nationkey INTEGER,
    s_phone VARCHAR(15),
    s_acctbal DECIMAL(15,2),
    s_comment VARCHAR(101)
);

CREATE TABLE partsupp (
    ps_partkey INTEGER,
    ps_suppkey INTEGER,
    ps_availqty INTEGER,
    ps_supplycost DECIMAL(15,2),
    ps_comment VARCHAR(199),
    PRIMARY KEY (ps_partkey, ps_suppkey)
);

CREATE TABLE customer (
    c_custkey INTEGER PRIMARY KEY,
    c_name VARCHAR(25),
    c_address VARCHAR(40),
    c_nationkey INTEGER,
    c_phone VARCHAR(15),
    c_acctbal DECIMAL(15,2),
    c_mktsegment VARCHAR(10),
    c_comment VARCHAR(117)
);

CREATE TABLE orders (
    o_orderkey INTEGER PRIMARY KEY,
    o_custkey INTEGER,
    o_orderstatus VARCHAR(1),
    o_totalprice DECIMAL(15,2),
    o_orderdate DATE,
    o_orderpriority VARCHAR(15),
    o_clerk VARCHAR(15),
    o_shippriority INTEGER,
    o_comment VARCHAR(79)
);

CREATE TABLE lineitem (
    l_orderkey INTEGER,
    l_partkey INTEGER,
    l_suppkey INTEGER,
    l_linenumber INTEGER,
    l_quantity INTEGER,
    l_extendedprice DECIMAL(15,2),
    l_discount DECIMAL(15,2),
    l_tax DECIMAL(15,2),
    l_returnflag VARCHAR(1),
    l_linestatus VARCHAR(1),
    l_shipdate DATE,
    l_commitdate DATE,
    l_receiptdate DATE,
    l_shipinstruct VARCHAR(25),
    l_shipmode VARCHAR(10),
    l_comment VARCHAR(44),
    PRIMARY KEY (l_orderkey, l_linenumber)
);
```

---

## 三、执行测试

### 3.1 使用 sqlrustgo-bench-cli

```bash
# 执行所有 22 个查询
./target/release/sqlrustgo-bench-cli tpch-bench \
  --ddl scripts/tpch/tpch_schema.sql \
  --data ~/sqlrustgo-tpch-sf1/data \
  --queries all

# 执行单个查询
./target/release/sqlrustgo-bench-cli tpch-bench \
  --ddl scripts/tpch/tpch_schema.sql \
  --data ~/sqlrustgo-tpch-sf1/data \
  --queries 1

# 生成 JSON 报告
./target/release/sqlrustgo-bench-cli tpch-bench \
  --ddl scripts/tpch/tpch_schema.sql \
  --data ~/sqlrustgo-tpch-sf1/data \
  --queries all \
  --output /tmp/tpch_results.json
```

### 3.2 快速验证

```bash
# 检查 SQLRustGo 是否支持 TPC-H SF=1
cargo run --release --bin sqlrustgo-bench-cli -- tpch-bench \
  --ddl scripts/tpch/tpch_schema.sql \
  --data ~/sqlrustgo-tpch-sf1/data \
  --queries 1
```

### 3.3 CI 门禁标准

| 指标 | Alpha | Beta | RC | GA |
|------|-------|------|-----|-----|
| 可运行查询数 | ≥14/22 | ≥18/22 | ≥20/22 | 22/22 |
| 执行时间 | - | <300s | <200s | <150s |

---

## 四、预期结果

### 4.1 v3.5.0 GA 标准

| 指标 | 数值 |
|------|------|
| 通过率 | 22/22 (100%) |
| 总执行时间 | ~150s (SF=1, Z440) |
| Q1 执行时间 | ~8s |

### 4.2 性能基线

```
查询 01: 8.2s
查询 02: 1.1s
查询 03: 2.3s
...
查询 22: 0.8s
```

详细基线见 `/opt/tpch/tpch_baseline.json`。

---

## 五、故障排查

### 5.1 数据加载失败

```bash
# 检查数据文件存在
ls -la ~/sqlrustgo-tpch-sf1/data/*.tbl

# 检查文件行数
wc -l ~/sqlrustgo-tpch-sf1/data/lineitem.tbl  # 应为 6001216 (含header)
```

### 5.2 查询执行失败

```bash
# 使用详细日志
RUST_LOG=debug ./target/release/sqlrustgo-bench-cli tpch-bench \
  --ddl scripts/tpch/tpch_schema.sql \
  --data ~/sqlrustgo-tpch-sf1/data \
  --queries 1
```

### 5.3 内存不足

```bash
# 监控内存使用
watch -n 1 'free -h | grep Mem'

# 使用较小的数据集进行测试
./target/release/sqlrustgo-bench-cli tpch-bench \
  --ddl scripts/tpch/tpch_schema.sql \
  --data ~/sqlrustgo-tpch-sf1/data \
  --queries 1 \
  --limit 1000
```

---

## 六、相关文件

| 文件 | 说明 |
|------|------|
| `scripts/tpch/tpch_schema.sql` | TPC-H 表结构定义 |
| `scripts/bench/check_benchmark_gate.sh` | 性能门禁脚本 |
| `/opt/tpch/TPC-H-BENCHMARK-REPORT.md` | 性能测试报告 |
| `/opt/tpch/tpch_baseline.json` | 性能基线数据 |
| `docs/tpch/SPRINT4-TPC-H-DEBUGGING-REPORT.md` | 调试报告 |

---

## 七、快速参考

| 命令 | 说明 |
|------|------|
| `cargo run --bin sqlrustgo-bench-cli -- tpch-bench --queries 1` | 测试单个查询 |
| `./scripts/bench/check_benchmark_gate.sh` | 检查性能门禁 |
| `cat /opt/tpch/tpch_baseline.json` | 查看性能基线 |