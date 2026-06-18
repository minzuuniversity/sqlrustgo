<!-- env:blocked:no-ci -->

# openspec/3518 - TPC-H SF01/SF1 Fixture Data Generation

> **Issue**: [#3518](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3518)
> **作者**: Hermes Agent
> **日期**: 2026-06-18
> **Phase**: 1 (W1-2)
> **工作量**: 16h (~2 days)
> **优先级**: P1 (gate-infrastructure)
> **Milestone**: v3.9.0 (id=32, due 2026-09-23)
> **Label**: ai-task, gate-infrastructure, integration, mysql-server

## 一、问题分析

### 1.1 背景

TPC-H fixture 文件目前是 130B stubs (仅 3 行)，无法进行真实的 TPC-H SOAK 测试。

**当前状态**:
```
tests/data/tpch-sf01/
├── customer.tbl    131 bytes (stub)
├── lineitem.tbl    132 bytes (stub)
├── nation.tbl      128 bytes (stub)
├── orders.tbl      132 bytes (stub)
├── part.tbl        131 bytes (stub)
├── partsupp.tbl    131 bytes (stub)
├── region.tbl      128 bytes (stub)
├── supplier.tbl   131 bytes (stub)
```

**期望状态**:
- SF01 (Scale Factor 0.01, 0.1%): 约 1-10 MB 总数据
- SF1 (Scale Factor 1.0, 1%): 约 10-100 MB 总数据

### 1.2 影响

- TPC-H SOAK 测试结果不准确
- G1 门禁 (22/22 TPC-H) 无法验证真实性能
- 跨引擎对比 (sqlrustgo vs MySQL vs PostgreSQL) 无意义

## 二、变更设计

### 2.1 数据生成策略

**工具选择**:
1. **dbgen** (官方 TPC-H 工具) - 首选
2. **tpch-data-gen** (如果已集成)
3. **自定义生成器** - 最后备选

**dbgen 编译**:
```bash
# 下载 TPC-H 工具
wget https://www.tpc.org/tpc-documents-currentVersions/downloadprocurementfiles/tpch_tool.zip

# 编译
cd tpch_tool/dbgen
make clean
make MACHINE=Linux DATABASE=SQLSERVER

# 生成数据
./dbgen -s 0.01 -f -d    # SF0.01 (SF01)
./dbgen -s 1 -f -d       # SF1
```

### 2.2 文件格式转换

**dbgen 输出格式** (默认):
```
kroon|parkers|1993-07-01|123.45|N|O|1993-08-01|1993-07-31|TAKE BACK RETURN|RAIL|
```

**sqlrustgo 期望格式** (| 分隔):
```
kroon|parkers|1993-07-01|123.45|N|O|1993-08-01|1993-07-31|TAKE BACK RETURN|RAIL|
```

实际上格式相同，无需转换。

### 2.3 目录结构

```
tests/data/
├── tpch-sf01/          # SF0.01 (0.1%)
│   ├── customer.tbl
│   ├── lineitem.tbl
│   ├── nation.tbl
│   ├── orders.tbl
│   ├── part.tbl
│   ├── partsupp.tbl
│   ├── region.tbl
│   ├── supplier.tbl
│   └── sqlrustgo.wal   # 清空或删除
├── tpch-sf1/           # SF1.0 (1%) - 新建
│   ├── customer.tbl
│   ├── lineitem.tbl
│   ├── nation.tbl
│   ├── orders.tbl
│   ├── part.tbl
│   ├── partsupp.tbl
│   ├── region.tbl
│   ├── supplier.tbl
│   └── sqlrustgo.wal
└── tpch-gen/           # TPC-H dbgen 工具 (可选)
    ├── Makefile
    ├── dbgen.c
    └── ...
```

### 2.4 数据加载

**通过 LOAD DATA**:
```sql
-- 加载 SF01
LOAD DATA LOCAL INFILE 'tests/data/tpch-sf01/customer.tbl' INTO TABLE customer FIELDS TERMINATED BY '|';
LOAD DATA LOCAL INFILE 'tests/data/tpch-sf01/lineitem.tbl' INTO TABLE lineitem FIELDS TERMINATED BY '|';
-- ... 其他表

-- 加载 SF1 (切换到不同目录)
LOAD DATA LOCAL INFILE 'tests/data/tpch-sf1/customer.tbl' INTO TABLE customer FIELDS TERMINATED BY '|';
```

**通过测试 harness** (更可靠):
```rust
// tests/tpch_full_22_test.rs
async fn load_tpch_data(scale: ScaleFactor) {
    let data_dir = match scale {
        ScaleFactor::SF01 => "tests/data/tpch-sf01",
        ScaleFactor::SF1 => "tests/data/tpch-sf1",
    };
    // 使用 sqlrustgo 的 LOAD DATA 或直接插入
}
```

## 三、测试验证

### 3.1 数据完整性验证

| 表 | 行数 (SF01) | 行数 (SF1) |
|----|-------------|-------------|
| customer | 1,500 | 15,000 |
| lineitem | ~6,000 | ~60,000 |
| nation | 25 | 25 |
| orders | 1,500 | 15,000 |
| part | 2,000 | 20,000 |
| partsupp | 8,000 | 80,000 |
| region | 5 | 5 |
| supplier | 100 | 1,000 |

### 3.2 TPC-H 22/22 验证

**SF01** (快速冒烟测试):
```bash
./gate/check_g1_tpch_22_22.sh --scale sf01
```

**SF1** (完整性能测试):
```bash
./gate/check_g1_tpch_22_22.sh --scale sf1
```

### 3.3 门禁检查

**G1 门禁**:
- SF01: 22/22 queries 在 5 分钟内完成
- SF1: 22/22 queries 在合理时间内完成 (TBD)

## 四、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| dbgen 下载/编译失败 | 中 | 使用预编译二进制或替代方案 |
| 数据文件过大 (SF1 ~100MB) | 中 | 仅在 CI gate 时使用 SF01 |
| LOAD DATA 性能问题 | 中 | 批量插入优化 |
| 数据格式不匹配 | 高 | 验证每行字段数 |

## 五、实施步骤

| # | 步骤 | 文件 | 工作量 |
|---|------|------|--------|
| 1 | 编译 TPC-H dbgen | 外部工具 | 2h |
| 2 | 生成 SF01 数据 | tests/data/tpch-sf01/*.tbl | 2h |
| 3 | 生成 SF1 数据 | tests/data/tpch-sf1/ (新建) | 2h |
| 4 | 更新 test harness | tests/tpch_full_22_test.rs | 2h |
| 5 | 验证数据完整性 | — | 2h |
| 6 | 运行 TPC-H 22/22 SF01 | — | 2h |
| 7 | 运行 TPC-H 22/22 SF1 | — | 4h |
| **合计** | | | **16h** |

## 六、交付物清单

| 类别 | 文件 | 大小预估 |
|------|------|----------|
| Data | tests/data/tpch-sf01/*.tbl (8 files) | ~10 MB |
| Data | tests/data/tpch-sf1/*.tbl (8 files, 新建) | ~100 MB |
| Fix | tests/tpch_full_22_test.rs (更新) | +50 行 |
| Doc | docs/openspec/3518-tpch-fixture-data-generation.md | 200 行 |
| **合计** | | **~110 MB + 250 行** |

## 七、门禁 (G1)

**位置**: `gate/check_g1_tpch_22_22.sh` (更新)

**检查项**:
1. `tests/data/tpch-sf01/*.tbl` 文件大小 > 1MB (总)
2. TPC-H SF01 22/22 queries 全部 PASS
3. TPC-H SF1 22/22 queries 全部 PASS (可选，CI gate)

## 八、Issue 关闭条件

满足 4 项:
1. ✅ SF01 数据文件 > 1MB (非 stub)
2. ✅ SF1 数据文件存在 (100 MB 左右)
3. ✅ `gate/check_g1_tpch_22_22.sh --scale sf01` 22/22 PASS
4. ✅ 数据完整性验证通过 (行数匹配)

## 九、参考

- Issue #3518: <http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3518>
- gate/check_g1_tpch_22_22.sh
- tests/tpch_full_22_test.rs
- TPC-H Specification: <https://www.tpc.org/tpch/>
