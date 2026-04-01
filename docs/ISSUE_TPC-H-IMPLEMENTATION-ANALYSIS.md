# TPC-H 实现状态分析报告

**日期**: 2026-04-02
**状态**: 🚨 需要实现
**优先级**: P0

---

## 执行摘要

| 问题 | 状态 | 影响 |
|------|------|------|
| TPC-H Q1-Q22 测试是 Stub | 🚨 严重 | 无法验证查询正确性 |
| BETWEEN 操作符未实现 | 🚨 严重 | Q6, Q18, Q21 失败 |
| DATE 字面量未实现 | 🚨 严重 | Q1, Q3, Q4, Q5 等失败 |
| IN value list 未实现 | 🚨 严重 | Q12, Q16, Q20, Q22 失败 |
| CASE 表达式未实现 | 🚨 严重 | Q8, Q12, Q14, Q19 失败 |
| COUNT(DISTINCT) 未实现 | ⚠️ 中等 | Q16 失败 |
| MockStorage::delete/update 是 Stub | 🚨 严重 | DML 测试无法验证 |

---

## 一、TPC-H 测试现状

### 1.1 测试文件分析

| 文件 | 测试数 | 状态 | 说明 |
|------|--------|------|------|
| `tpch_full_test.rs` | 28 | ❌ STUB | 只打印消息和 `assert!(true)` |
| `tpch_test.rs` | 5 | ⚠️ PARTIAL | 使用 MockStorage + 物理计划 |
| `tpch_benchmark.rs` | 11 | ⚠️ PARTIAL | 手动构造物理计划 |
| `mysql_tpch_test.rs` | 4 | ❌ 需要 MySQL | 环境依赖 |
| `postgres_tpch_test.rs` | 22 | ⚠️ PARTIAL | 对比测试框架 |

### 1.2 测试状态详情

**tpch_full_test.rs** (主要问题):
```rust
#[test]
fn test_tpch_q1_pricing_summary() {
    println!("\n=== TPC-H Q1: Pricing Summary Report ===");
    println!("Query: SELECT l_returnflag, l_linestatus, SUM(l_quantity)...");
    // SQLRustGo 实现 - 只打印，没有实际执行!
    let start = Instant::now();
    let elapsed = start.elapsed();
    println!("SQLRustGo: executed in {:?}", elapsed);
    assert!(elapsed.as_secs_f64() < 1.0);  // 假测试!
}
```

**tpch_test.rs** (部分实现):
```rust
#[test]
fn test_tpch_q1_projection() {
    let storage = MockStorage::with_data("orders", TestDataSet::simple_orders());
    let harness = TestHarness::<MockStorage>::new(storage);
    
    // 手动构造物理计划
    let scan = Box::new(SeqScanExec::new("orders".to_string(), schema.clone()));
    let plan = Box::new(ProjectionExec::new(scan, ...));
    
    let result = harness.execute(plan.as_ref()).unwrap();
    // 实际执行了，但只测试投影!
}
```

---

## 二、缺失的 SQL 特性

### 2.1 BETWEEN 操作符

**状态**: `Token::Between` 存在，但 `parse_comparison_expression` 未实现

**当前代码** (`parser.rs:1648-1670`):
```rust
fn parse_comparison_expression(&mut self) -> Result<Expression, String> {
    let left = self.parse_arithmetic_expression()?;
    
    let op = match self.current() {
        Some(Token::Equal) => "=",
        Some(Token::NotEqual) => "!=",
        Some(Token::Greater) => ">",
        Some(Token::Less) => "<",
        Some(Token::GreaterEqual) => ">=",
        Some(Token::LessEqual) => "<=",
        _ => return Ok(left),  // ❌ 没有 BETWEEN 处理!
    };
    // ...
}
```

**TPC-H Q6 使用 BETWEEN**:
```sql
SELECT SUM(l_extendedprice * l_discount) AS revenue
FROM lineitem
WHERE l_shipdate >= DATE '1994-01-01'
  AND l_shipdate < DATE '1995-01-01'
  AND l_discount BETWEEN 0.05 AND 0.07  -- ❌ 不支持
  AND l_quantity < 25
```

**需要 TPC-H 查询**: Q6, Q18, Q21

---

### 2.2 DATE 字面量

**状态**: `Token::Date` 存在于 lexer，但未实现日期字面量解析

**当前行为**:
```sql
SELECT * FROM t WHERE d = DATE '2024-01-01'
-- 解析失败: DATE 不是有效的 token
```

**TPC-H Q1 使用 DATE**:
```sql
SELECT l_returnflag, l_linestatus, 
       SUM(l_quantity) AS sum_qty,
       SUM(l_extendedprice) AS sum_base_price
FROM lineitem
WHERE l_shipdate <= DATE '1998-12-01'  -- ❌ 不支持
  AND l_shipdate > DATE '1998-10-01'
GROUP BY l_returnflag, l_linestatus
```

**当前解析问题**:
- `Token::Date` 被识别为类型名
- 没有 `Token::DateLiteral` 来处理 `DATE 'yyyy-mm-dd'` 语法

**需要 TPC-H 查询**: Q1, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10, Q12, Q14, Q17, Q18, Q19, Q20, Q21

---

### 2.3 IN value list

**状态**: `Token::In` 不存在（只有 `Token::Into`, `Token::Index`）

**当前行为**:
```sql
SELECT * FROM t WHERE a IN (1, 2, 3)
-- 解析失败: IN 不是有效的 token
```

**TPC-H Q16 使用 IN**:
```sql
SELECT p_brand, p_type, p_size,
       COUNT(DISTINCT ps_suppkey) AS supplier_count
FROM partsupp, part
WHERE p_partkey = ps_partkey
  AND p_brand <> 'Brand#45'
  AND p_type NOT LIKE 'MEDIUM%'  -- ❌ NOT LIKE 问题
  AND p_size IN (9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20)  -- ❌ 不支持
GROUP BY p_brand, p_type, p_size
```

**需要 TPC-H 查询**: Q12, Q16, Q20, Q22

---

### 2.4 CASE 表达式

**状态**: `Token::Case` 不存在

**TPC-H Q12 使用 CASE**:
```sql
SELECT l_shipmode,
       SUM(CASE WHEN o_orderpriority = '1-URGENT'
            OR o_orderpriority = '2-HIGH'
            THEN 1 ELSE 0 END) AS high_line_count,
       SUM(CASE WHEN o_orderpriority <> '1-URGENT'
            AND o_orderpriority <> '2-HIGH'
            THEN 1 ELSE 0 END) AS low_line_count
FROM orders, lineitem
WHERE o_orderkey = l_orderkey
  AND l_shipmode IN ('MAIL', 'SHIP')
GROUP BY l_shipmode
```

**需要 TPC-H 查询**: Q1, Q8, Q12, Q14, Q19

---

### 2.5 COUNT(DISTINCT)

**状态**: 未实现

**TPC-H Q16 使用 COUNT(DISTINCT)**:
```sql
SELECT p_brand, p_type, p_size,
       COUNT(DISTINCT ps_suppkey) AS supplier_count  -- ❌ 不支持
FROM partsupp, part
...
```

**需要 TPC-H 查询**: Q16

---

### 2.6 NOT LIKE / LIKE

**状态**: `Token::Like` 不存在

**TPC-H Q16 使用 NOT LIKE**:
```sql
AND p_type NOT LIKE 'MEDIUM%'  -- ❌ 不支持
```

---

## 三、MockStorage 问题

### 3.1 Stub 实现

**文件**: `crates/executor/src/mock_storage.rs:169-179`

```rust
fn delete(&mut self, _table: &str, _filters: &[Value]) -> SqlResult<usize> {
    Ok(0)  // ❌ 总是返回 0!
}

fn update(
    &mut self,
    _table: &str,
    _filters: &[Value],
    _updates: &[(usize, Value)],
) -> SqlResult<usize> {
    Ok(0)  // ❌ 总是返回 0!
}
```

### 3.2 影响

| 功能 | 影响 |
|------|------|
| TPC-H 测试 | 无法验证 UPDATE/DELETE |
| FK 约束测试 | delete/update 操作返回 0 |
| 性能测试 | 不准确 |

---

## 四、TPC-H Q1-Q22 SQL 特性依赖

| Query | BETWEEN | DATE | IN | CASE | DISTINCT | LIKE/NOT LIKE |
|-------|---------|-------|-----|------|----------|----------------|
| Q1 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q2 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Q3 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q4 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q5 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q6 | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q7 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q8 | ❌ | ✅ | ❌ | ✅ | ❌ | ❌ |
| Q9 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q10 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q11 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Q12 | ❌ | ❌ | ✅ | ✅ | ❌ | ❌ |
| Q13 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Q14 | ❌ | ✅ | ❌ | ✅ | ❌ | ✅ |
| Q15 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Q16 | ❌ | ❌ | ✅ | ❌ | ✅ | ✅ |
| Q17 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q18 | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q19 | ❌ | ✅ | ❌ | ✅ | ❌ | ✅ |
| Q20 | ❌ | ✅ | ✅ | ❌ | ❌ | ✅ |
| Q21 | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Q22 | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ |

**总计**: 22 个查询中，0 个能完全执行 ❌

---

## 五、实施计划

### Phase 1: 语法支持 (Week 1-2)

| Task | Duration | Priority |
|------|----------|----------|
| 实现 BETWEEN 操作符 | 1-2 days | P0 |
| 实现 DATE 字面量 | 1-2 days | P0 |
| 实现 IN value list | 2-3 days | P0 |
| 实现 CASE 表达式 | 2-3 days | P0 |

### Phase 2: 聚合函数 (Week 2-3)

| Task | Duration | Priority |
|------|----------|----------|
| 实现 COUNT(DISTINCT) | 1-2 days | P1 |
| 实现 LIKE/NOT LIKE | 1-2 days | P1 |

### Phase 3: 测试框架 (Week 3-4)

| Task | Duration | Priority |
|------|----------|----------|
| 修复 MockStorage::delete/update | 1 day | P0 |
| 实现真实 TPC-H Q1-Q22 测试 | 2-3 days | P0 |
| 集成测试套件 | 1 day | P1 |

### Phase 4: 性能优化 (Week 4-5)

| Task | Duration | Priority |
|------|----------|----------|
| TPC-H 性能基准测试 | 2 days | P2 |
| 查询优化 | Ongoing | P2 |

---

## 六、技术方案

### 6.1 BETWEEN 实现

```rust
// parser.rs - parse_comparison_expression 添加:
if self.current() == Some(Token::Between) {
    self.next(); // consume BETWEEN
    let low = self.parse_arithmetic_expression()?;
    self.expect_keyword("AND")?;
    let high = self.parse_arithmetic_expression()?;
    return Ok(Expr::Between {
        expr: Box::new(expr),
        low: Box::new(low),
        high: Box::new(high),
    });
}
```

### 6.2 DATE 字面量实现

```rust
// parser.rs - parse_primary_expression 添加:
Some(Token::Date) => {
    self.next(); // consume DATE
    if let Some(Token::StringLit(date_str)) = self.current() {
        self.next();
        return Ok(Expr::DateLiteral(date_str));
    }
    return Err("Expected DATE 'yyyy-mm-dd'".to_string());
}
```

### 6.3 IN value list 实现

```rust
// 需要添加 Token::In
// parser.rs - parse_comparison_expression 添加:
if self.current() == Some(Token::In) {
    self.next(); // consume IN
    self.expect(Token::LParen)?;
    let values = self.parse_comma_separated(Parser::parse_value)?;
    self.expect(Token::RParen)?;
    return Ok(Expr::InList {
        expr: Box::new(expr),
        values,
    });
}
```

---

## 七、验收标准

### 7.1 Phase 1 验收

| 测试 | 标准 |
|------|------|
| `SELECT * FROM t WHERE a BETWEEN 1 AND 10` | 返回正确结果 |
| `SELECT * FROM t WHERE d = DATE '2024-01-01'` | 返回正确结果 |
| `SELECT * FROM t WHERE a IN (1, 2, 3)` | 返回正确结果 |
| `SELECT CASE WHEN a > 0 THEN 'positive' ELSE 'negative' END` | 返回正确结果 |

### 7.2 Phase 2 验收

| 测试 | 标准 |
|------|------|
| `SELECT COUNT(DISTINCT col) FROM t` | 返回正确去重计数 |
| `SELECT * FROM t WHERE name LIKE 'A%'` | 返回正确结果 |

### 7.3 Phase 3 验收

| 测试 | 标准 |
|------|------|
| `cargo test --test tpch_full_test` | Q1-Q22 全部通过 |
| `MockStorage::delete` | 返回实际删除行数 |
| `MockStorage::update` | 返回实际更新行数 |

### 7.4 Phase 4 验收

| 测试 | 标准 |
|------|------|
| `cargo test --test tpch_benchmark` | 性能数据可记录 |
| Q6 执行时间 | < 1s (1000 rows) |

---

## 八、风险和缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| Parser 修改破坏现有功能 | 高 | 每个任务后运行完整测试 |
| DATE 比较性能问题 | 中 | 使用字符串比较验证 |
| IN list 性能问题 | 低 | 当前保证正确性，后续优化 |
| MockStorage 修复复杂 | 中 | 使用 MemoryStorage 作为参考 |

---

## 九、建议的下一步行动

1. **立即**: 在 `parser.rs` 中实现 BETWEEN 操作符
2. **立即**: 实现 DATE 字面量解析
3. **短期**: 实现 IN value list 和 CASE 表达式
4. **中期**: 修复 MockStorage::delete/update
5. **长期**: 完整的 TPC-H Q1-Q22 测试实现

---

## 十、相关文档

| 文档 | 描述 |
|------|------|
| `docs/v2.1.0-IMPLEMENTATION-STATUS.md` | 实现状态总报告 |
| `docs/v2.1.0-TEST-MATRIX.md` | 功能矩阵和测试详情 |
| `tests/integration/tpch_full_test.rs` | TPC-H 测试 (当前 Stub) |
| `tests/integration/tpch_test.rs` | TPC-H 基础测试 |

---

*报告生成: 2026-04-02*
*分析工具: SQLRustGo Implementation Checker*
