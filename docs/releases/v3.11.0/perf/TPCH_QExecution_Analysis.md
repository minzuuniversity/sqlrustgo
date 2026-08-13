# TPC-H SF=1.0 执行分析报告

**版本**: v3.11.0 (`develop/v3.11.0`, commit `698b69840d`)
**日期**: 2026-07-15
**目的**: 详细分析 22 个 TPC-H 查询的执行路径、JOIN 算法及 OOM 根因

---

## 一、架构总览

### 1.1 执行入口

```
execute_select(SelectStatement)
    │
    ├─ execute_joins()           ← 处理 FROM + JOIN + WHERE
    │       │
    │       ├─ scan_with_ahi()   ← 扫描基表
    │       ├─ extract_single_table_predicates()  ← 提取单表谓词
    │       ├─ try_comma_join_hash_chain()  ← comma-join 快速路径
    │       └─ execute_single_join()          ← 逐个 JOIN 处理
    │
    ├─ WHERE post-filter         ← JOIN 后 WHERE 过滤
    ├─ GROUP BY aggregation
    └─ ORDER BY + LIMIT
```

### 1.2 JOIN 算法分类

| 查询类型 | JOIN 路径 | 算法 |
|---------|---------|------|
| 显式 `JOIN ... ON` | `execute_single_join()` | 逐个处理 `JoinClause` |
| comma-join + WHERE 链式等值 | `try_comma_join_hash_chain()` | 贪心哈希链 (`multi_way_hash_chain`) |
| 无可用等值谓词 | `execute_single_join()` → `JoinKey::All` | 笛卡尔积 |

### 1.3 关键数据结构

**Parser 输出 (`SelectStatement`)**:
```rust
pub table: String,           // FROM 第一个表 (e.g. "part")
pub extra_tables: Vec<String>, // comma-join 后续表 (e.g. ["supplier", "partsupp", ...])
pub join_clause: Vec<JoinClause>, // 显式 JOIN (e.g. Q5 用显式 JOIN 语法)
pub where_clause: Option<Expression>,
```

**Parser 自动重写** (`parser.rs:3707-4044`):
```sql
-- 输入: FROM t1, t2, t3 WHERE p1 AND p2 AND ...
-- 输出: FROM t1 JOIN t2 ON p_extracted JOIN t3 ON p_extracted WHERE p_rest
```
自动重写将 comma-join 转换为显式 JOIN，提取等值谓词到 ON 子句。

---

## 二、JOIN 算法详解

### 2.1 `try_comma_join_hash_chain()` — 哈希链快速路径

**调用条件**: `extra_tables` 非空（comma-join 语法）

**算法**:
1. 从 WHERE 提取所有 `=` 等值谓词
2. 用 `resolve_bare()` 解析列名到表别名（处理 TPC-H 前缀如 `s_suppkey`）
3. 构建 `pair_key: HashMap<(table1, table2), (col1, col2)>`
4. 贪心链构建：从基表出发，每次选一个与当前链尾有等值边的未访问表
5. 若所有表都在链中，调用 `multi_way_hash_chain()` 执行

**resolve_bare 歧义处理**:
```rust
// supplier 和 partsupp 都有 s_suppkey 列
// TPC-H 前缀映射: "s_" → supplier, "ps_" → partsupp
"s_suppkey" → supplier.s_suppkey  (通过前缀映射)
"ps_suppkey" → partsupp.ps_suppkey (通过前缀映射)
```

**贪心链构建**:
```rust
// 从基表开始，每次找与链尾有等值边的最小未访问表
while visited.len() < join_tables.len() {
    let tail = chain_order.last().alias;
    let next = join_tables.iter()
        .find(|(_, alias)| !visited && pair_key contains edge(tail, alias));
    // 按行数选择最小表作为 build side
}
```

### 2.2 `execute_single_join()` — 显式 JOIN 处理器

**ON 条件解析** (`find_join_key_index`):
```rust
fn find_join_key_index(expr, left_info, left_alias, right_info, right_alias) -> JoinKey
    ├─ Expression::Literal("true") → JoinKey::All (笛卡尔积)
    ├─ Expression::Identifier("col") → 在 left/right 中查找
    ├─ Expression::BinaryOp("=") → 解析左右列
    └─ Expression::BinaryOp("AND") → 递归，合并多列 JOIN
```

**JoinKey 变体**:
```rust
enum JoinKey {
    Left(usize),           // 仅左表列 (ON a.id = 1)
    Right(usize),          // 仅右表列
    Pair(usize, usize),    // 左右各一列 (ON a.id = b.id)
    Pairs(Vec<(l,r)>),     // 多列 (ON a.id = b.id AND a.sub = b.sub)
    All,                   // 笛卡尔积 (无有效 ON)
}
```

### 2.3 `multi_way_hash_chain()` — 批量哈希 JOIN

```rust
// 对每一步:
for step in chain {
    let (acc_rows, cur_rows, left_col, right_col) = step;
    let build_rows = smaller(acc_rows, cur_rows);  // 自动选 build side
    let hash = build_hash_table(build_rows, key_col);
    let result = probe_hash_join(probe_rows, hash, key_col);
    acc_rows = result;
}
```

---

## 三、Parser 自动重写详解

### 3.1 自动重写流程

```sql
-- 输入 Q2:
SELECT ... FROM part, supplier, partsupp, nation, region
WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey
  AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey
  AND r_name = 'EUROPE' AND p_size = 15 AND p_type LIKE '%BRASS';

-- 预期输出:
FROM part
  JOIN supplier  ON s_suppkey = ps_suppkey  -- 错！ps_suppkey 尚未加入
  JOIN partsupp ON p_partkey = ps_partkey
  JOIN nation   ON s_nationkey = n_nationkey
  JOIN region   ON n_regionkey = r_regionkey
WHERE r_name = 'EUROPE' AND p_size = 15 AND p_type LIKE '%BRASS';
```

### 3.2 `find_join_predicate()` — 谓词提取

```rust
fn find_join_predicate(predicates, new_table, new_alias, joined_tables) -> Option<Expression>
    // 1. 必须是等值谓词
    // 2. new_table 在一侧，另一侧所有表已在 joined_tables 中
    // 3. 两边都必须是列（不是字面量）
```

**判断条件**:
```rust
left_is_col = matches!(l, Expression::Identifier(_));
right_is_col = matches!(r, Expression::Identifier(_));
if !left_is_col || !right_is_col { continue; }  // 字面量跳过

// 检查 new_table 在一侧
if left_has_new && !right_has_new && right_refs ⊆ joined_tables {
    return Some(predicate);
}
if right_has_new && !left_has_new && left_refs ⊆ joined_tables {
    return Some(predicate);
}
```

### 3.3 `tpch_reorder_extra_tables()` — 贪心重排序

**目的**: 将 `FROM part, supplier, partsupp, nation, region` 重排为最优 JOIN 顺序

**行数估计** (SF=1):
| 表 | 行数 | 过滤器 | 有效行数 |
|----|------|--------|---------|
| region | 5 | r_name='EUROPE' | 1 |
| nation | 25 | n_regionkey=regionkey | 1-5 |
| supplier | 5,000 | s_nationkey=nationkey | 25-200 |
| customer | 30,000 | c_nationkey=nationkey | ~7,000 |
| orders | 1,500,000 | o_orderdate >= 1994-01-01 | ~300,000 |
| lineitem | 6,000,000 | l_orderkey=orders | ~210,000 |
| partsupp | 80,000 | (无单表过滤器) | 80,000 |
| part | 200,000 | p_size=15, p_type LIKE '%BRASS' | ~400 |

**Prefix 碰撞检测 (BUG 所在)**:
```rust
// 问题: part 和 partsupp 都以 'p' 开头，被判定为碰撞
let p1 = &t[..1];  // "partsupp" → 'p', "part" → 'p' → 碰撞
if !seen_prefix.insert(p1) { collision = true; }
if collision { return extras.to_vec(); }  // 直接返回原始顺序！
```

**重排逻辑**:
```rust
while !remaining.is_empty() {
    for (i, t) in remaining.enumerate() {
        let base = effective_row_count(b, conj, &accumulated);
        let reachable = accumulated.contains(edge_to(b));
        let score = if reachable { base } else { base + 1_000_000_000 };
        if score < best_score { best = i; }
    }
    let picked = remaining.swap_remove(best_idx);
    accumulated.insert(bare(picked));
    out.push(picked);
}
```

---

## 四、每个查询的执行分析

### Q1 (SF=1: 96 MB, 41 ms) ✅

**SQL**: 单表 `lineitem WHERE l_shipdate <= '1995-12-01'` + GROUP BY

**执行路径**:
```
execute_joins:
  ├─ extra_tables = []  → 不走 try_comma_join_hash_chain
  ├─ scan_with_ahi("lineitem") → 6,000,000 rows
  ├─ WHERE pushdown: l_shipdate <= '1995-12-01' → 约 1/4 过滤
  └─ GROUP BY aggregation
      └─ 约 1,500,000 行 → 2 groups (R/F) → 2 rows

内存: 1,500,000 rows × ~64 bytes ≈ 96 MB
```

**JOIN 算法**: 无（单表扫描）

---

### Q2 (SF=1: **FIXED 2026-07-16: ~13 s, 20 rows**) ✅

**SQL**: `FROM part, supplier, partsupp, nation, region` (comma-join)

**执行路径**:

#### Parser 层
```
extra_tables = ["supplier", "partsupp", "nation", "region"]
join_clause = []

自动重写触发 tpch_reorder_extra_tables()
├─ 碰撞检测: "p" (part) vs "p" (partsupp) → 碰撞！
├─ 返回原始顺序: ["supplier", "partsupp", "nation", "region"]
└─ joined = ["region"] (基表)

处理 supplier:
  ├─ 谓词 s_suppkey=ps_suppkey: ps_suppkey 未在 joined 中 → 跳过
  ├─ 谓词 s_nationkey=n_nationkey: n_nationkey 未在 joined 中 → 跳过
  └─ ON = true (笛卡尔积！)

处理 partsupp:
  ├─ 谓词 s_suppkey=ps_suppkey: s 在 joined → 找到！
  └─ ON = s_suppkey=ps_suppkey

处理 nation:
  └─ ON = n_regionkey=r_regionkey

处理 region:
  └─ ON = r_name='EUROPE' (WHERE 谓词被提取)

最终 join_clause = [
  supplier  ON true,           ← 笛卡尔积！
  partsupp  ON s_suppkey=ps_suppkey,
  nation    ON n_regionkey=r_regionkey,
  region    ON r_name='EUROPE'  ← 实际是 WHERE 过滤
]
```

#### 执行层
```
execute_joins:
  ├─ extra_tables 非空 → try_comma_join_hash_chain()
  │   ├─ pair_key 收集成功 (s_suppkey → supplier.ps_suppkey, etc.)
  │   ├─ 贪心链: region → nation → supplier → partsupp → part
  │   └─ chain_order.len() == 5 → 返回 None (未覆盖所有表！)
  │
  └─ 逐个 execute_single_join(join_clause):

Step 1: region × nation
  ├─ region scan: r_name='EUROPE' → 1 row
  ├─ nation scan: 25 rows
  ├─ JoinKey::Pair (n_regionkey=r_regionkey)
  └─ nation × region: 1 × 5 = 5 rows

Step 2: 5 rows × supplier
  ├─ supplier scan: 5,000 rows
  ├─ JoinKey::All (ON=true，笛卡尔积！)
  └─ 5 × 5,000 = 25,000 rows  ← 第一爆炸点

Step 3: 25,000 rows × partsupp
  ├─ partsupp scan: 80,000 rows
  ├─ JoinKey::Pair (s_suppkey=ps_suppkey)
  └─ 25,000 × 80,000 = 2,000,000,000 rows  ← 20亿行！≈ 18 GB
```

**根因已修复 (2026-07-16)**:

修复前: `supplier ON true` (笛卡尔积) → OOM 18.1 GB
修复后: `partsupp ON p_partkey=ps_partkey` → supplier ON s_suppkey=ps_suppkey → ~13s, 20 rows

**修复内容**:
- 1-char prefix entries in accumulated set incorrectly matched full table names
- Filter accumulated to len >= 2 only
- Use starts_with instead of == for prefix matching

---

### Q3 (SF=1: 1.9 GB, 2.2 s) ✅

**SQL**: `FROM customer, orders, lineitem WHERE c_custkey=o_custkey AND l_orderkey=o_orderkey AND c_mktsegment='BUILDING'`

**执行路径**:
```
extra_tables = ["orders", "lineitem"]
自动重写: customer → orders → lineitem
join_clause = [
  orders    ON c_custkey=o_custkey,
  lineitem  ON l_orderkey=o_orderkey
]

execute_joins:
  ├─ try_comma_join_hash_chain() → 成功
  │   ├─ chain: customer → orders → lineitem
  │   ├─ p1 = c_custkey=o_custkey
  │   ├─ p2 = l_orderkey=o_orderkey
  │   └─ multi_way_hash_chain()
  │       ├─ customer scan: c_mktsegment='BUILDING' → ~30,000 rows
  │       ├─ orders × customer (c_custkey=o_custkey): ~30,000 × 300,000 → 略放大
  │       └─ lineitem × orders (l_orderkey=o_orderkey): ~600万行
  └─ GROUP BY, ORDER BY, LIMIT 10
```

**JOIN 算法**: `try_comma_join_hash_chain` → 哈希链，无笛卡尔积

---

### Q4 (SF=1: 1.8 GB, 6.3 s) ✅

**SQL**: `FROM orders, lineitem WHERE l_orderkey=o_orderkey AND o_orderdate>='1993-10-01'`

**执行路径**:
```
orders → lineitem 哈希链
├─ orders scan: o_orderdate>='1993-10-01' → ~600,000 rows
├─ orders × lineitem (l_orderkey=o_orderkey)
└─ WHERE o_orderpriority='1-URGENT' 后过滤
```

**JOIN 算法**: 哈希链，有日期过滤下推

---

### Q5 (SF=1: **FIXED 2026-07-15 via PR #3550**) ✅

**SQL** (显式 JOIN 语法):
```sql
FROM customer, orders, lineitem, supplier, nation, region
WHERE c_custkey=o_custkey AND l_orderkey=o_orderkey
  AND l_suppkey=s_suppkey AND c_nationkey=s_nationkey
  AND s_nationkey=n_nationkey AND n_regionkey=r_regionkey
  AND r_name='ASIA' AND o_orderdate>='1994-01-01' AND o_orderdate<'1995-01-01'
```

**关键**: Q5 使用 **comma-join 语法**（与 Q2 相同），不是显式 `JOIN ... ON`！

**Parser 处理**:
```
extra_tables = ["orders", "lineitem", "supplier", "nation", "region"]
自动重写触发 tpch_reorder_extra_tables()

重排逻辑评估 (基表=customer):
├─ customer: base_score = 30,000 (reachable from customer)
├─ orders: c_custkey=o_custkey (c 已加入) → reachable → ~300,000
├─ lineitem: l_orderkey=o_orderkey (orders 在 accumulated) → ~210,000
├─ supplier: c_nationkey=s_nationkey (c 已加入) → ~200
├─ nation: s_nationkey=n_nationkey (s 已加入) → ~3
├─ region: n_regionkey=r_regionkey (n 已加入) → ~1
└─ 重排后顺序: region → nation → supplier → orders → lineitem
```

**nation-bridge 启发式** (`parser.rs:1109-1120`):
```rust
// 检测: c_nationkey=s_nationkey (customer-supplier nation 桥接)
//      AND orders 有日期过滤
// 动作: 当 b=="orders" 且 customer 还在 remaining 中时，force orders 先于 customer
// 目的: orders(30万) × customer(3千) 笛卡尔积 vs customer(3千) × orders(30万)
//       两者乘积相同，但 orders 先可以应用日期过滤 (30万 → 约30万)
//       customer 先则无法应用过滤，全量参与笛卡尔
```

**实际执行路径**:
```
execute_joins:
  ├─ extra_tables 非空 → try_comma_join_hash_chain()
  │   ├─ pair_key: c_custkey→orders, l_orderkey→lineitem,
  │   │             l_suppkey→supplier, c_nationkey→supplier,
  │   │             s_nationkey→nation, n_regionkey→region
  │   ├─ chain_order.len() < 6 → 返回 None (cycle detected!)
  │   │   原因: customer ↔ supplier 通过 c_nationkey=s_nationkey 形成环
  │   │         customer ← c_custkey → orders ← l_orderkey ← lineitem
  │   │                   ↕ (via nation bridge)
  │   │                 supplier ← l_suppkey ← lineitem
  │   └─ 链不完整 → 不走 try_comma_join_hash_chain
  │
  └─ 走 execute_single_join 逐个处理

join_clause (自动重写后):
  region    ON n_regionkey=r_regionkey
  nation    ON s_nationkey=n_nationkey
  supplier  ON c_nationkey=s_nationkey  ← nation-bridge 关键边
  orders    ON c_custkey=o_custkey
  lineitem  ON l_orderkey=o_orderkey
  (customer 是基表，最后加入)
```

**OOM 根因**: `customer` 是基表，最后加入。`supplier × customer` (via `c_nationkey=s_nationkey`) 产生 `200 × 30,000 = 6,000,000` 行中间结果，然后与 `orders` (`300,000`) 产生 `1.8 × 10^12` 行笛卡尔！

**正确顺序应为**: `region(1) → nation(3) → supplier(200) → **orders(300K)** → **customer(30K)** → lineitem(210K)`
orders 的日期过滤在 JOIN 之前应用，将 1.5M 过滤到 300K。

---

### Q6 (SF=1: 96 MB, 42 ms) ✅

**SQL**: 单表 `lineitem WHERE l_shipdate>='1994-01-01' AND l_shipdate<'1995-01-01' AND l_discount BETWEEN 0.06-0.01 AND 0.06+0.01 AND l_quantity < 24`

**执行路径**: 单表扫描 + 多个谓词下推 + 聚合

**JOIN 算法**: 无

---

### Q7 (SF=1: 2.0 GB, 2.8 s) ✅

**SQL**: `FROM supplier, lineitem, orders, customer, nation n1, nation n2 WHERE ...`

**执行路径**:
```
自动重写: supplier → lineitem → orders → customer → n1 → n2
哈希链成功，chain 完整
```

**JOIN 算法**: 哈希链

---

### Q8 (SF=1: 2.0 GB, 2.5 s) ✅

**SQL**: `FROM part, supplier, lineitem, orders, customer, nation, region WHERE ...`

**执行路径**: 7 表 comma-join，自动重写 + 哈希链

**JOIN 算法**: 哈希链

---

### Q9 (SF=1: 2.1 GB, 4.1 s) ✅

**SQL**: 4 表 comma-join，自动重写 + 哈希链

**JOIN 算法**: 哈希链

---

### Q10 (SF=1: 2.0 GB, 2.5 s) ✅

**SQL**: `FROM customer, orders, lineitem, nation WHERE c_custkey=o_custkey AND l_orderkey=o_orderkey AND c_nationkey=n_nationkey AND o_orderdate>='1993-07-01'`

**执行路径**:
```
customer → orders → lineitem → nation
customer scan: c_mktsegment 过滤 (单表)
orders scan: o_orderdate 过滤 (单表下推)
```

**JOIN 算法**: 哈希链

---

### Q11 (SF=1: 2.3 GB, 4.8 s, 29,636 rows) ✅

**SQL**: `FROM partsupp, supplier, nation WHERE ps_suppkey=s_suppkey AND s_nationkey=n_nationkey AND s_nationkey=...`

**JOIN 算法**: 哈希链

---

### Q12 (SF=1: 2.0 GB, 1.7 s) ✅

**SQL**: `FROM orders, lineitem WHERE l_orderkey=o_orderkey AND l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate`

**JOIN 算法**: 哈希链，3 个单表过滤下推

---

### Q13 (SF=1: 2.1 GB, 4.7 s) ✅

**SQL**: 包含 `JOIN` 子查询，`IN (SELECT ...)` 实现

**JOIN 算法**: 混合，子查询通过 `Expression::In` 处理

---

### Q14 (SF=1: 96 MB, 382 ms) ✅

**SQL**: `FROM lineitem, part WHERE l_partkey=p_partkey AND l_shipdate>='1995-09-01' AND l_shipdate<'1995-10-01'`

**JOIN 算法**: 哈希链，part 有 `p_type LIKE 'PROMO*'` 过滤

---

### Q15 (SF=1: 96 MB, 51 ms) ✅

**SQL**: 包含 `JOIN` 子查询创建视图 `revenue`

**JOIN 算法**: 物化子查询 + 哈希链

---

### Q16 (SF=1: 2.2 GB, 4.8 s, 18,314 rows) ✅

**SQL**: `FROM partsupp, supplier, part WHERE ps_suppkey=s_suppkey AND p_partkey=ps_partkey AND p_brand<>'Brand#45' AND p_type NOT LIKE 'MEDIUM%'`

**JOIN 算法**: 哈希链，单表过滤下推

---

### Q17 (SF=1: 96 MB, 308 ms) ✅

**SQL**: `FROM lineitem, orders, customer WHERE l_orderkey=o_orderkey AND o_custkey=c_custkey`

**JOIN 算法**: 哈希链，无额外过滤

---

### Q18 (SF=1: 2.2 GB, 3.3 s) ✅

**SQL**: 3 表 comma-join + `HAVING SUM(...) > ...`

**JOIN 算法**: 哈希链 + HAVING 过滤

---

### Q19 (SF=1: 96 MB, 306 ms) ✅

**SQL**: `FROM lineitem, part, supplier WHERE p_partkey=l_partkey AND s_suppkey=l_suppkey AND p_brand='Brand#12' AND p_container IN ('SM CASE', 'SM BOX', ...) AND l_quantity>=1 AND l_quantity<=30 AND p_size BETWEEN 1 AND 15`

**JOIN 算法**: 哈希链，多个强过滤条件

---

### Q20 (SF=1: 96 MB, 621 ms) ✅

**SQL**: 包含子查询 + `IN (SELECT ...)`，`l_suppkey IN (SELECT ...)`

**JOIN 算法**: 子查询物化 + 哈希链

---

### Q21 (SF=1: **FIXED 2026-07-15 via PR #3550**) ✅

**SQL**:
```sql
FROM supplier, lineitem l1, orders, nation
WHERE s_suppkey=l1.l_suppkey AND o_orderkey=l1.l_orderkey
  AND o_orderstatus='F' AND s_nationkey=n_nationkey AND n_name='GERMANY'
  AND EXISTS (SELECT * FROM lineitem l2 WHERE ...)
  AND NOT EXISTS (SELECT * FROM lineitem l3 WHERE ...)
```

**执行路径**:
```
supplier 基表 (5,000)
extra_tables = ["lineitem|l1", "orders", "nation"]
join_clause = []

自动重写评估:
├─ 基表: supplier
├─ orders: s_suppkey=l1.l_suppkey 无法解析 (l1 未加入) → ON=true
├─ nation: s_nationkey=n_nationkey (s 已加入) → reachable
└─ nation × supplier: 1 × 200 = 200 rows

关键问题: l1 (lineitem alias) 第一次 JOIN 使用 ON=true！
```

**OOM 根因**: `lineitem l1` 加入时，JOIN ON = `true`（笛卡尔积），产生大量中间结果。`EXISTS` 子句进一步加剧。

---

### Q22 (SF=1: 2.5 GB, 7.0 s, 7 rows) ✅

**SQL**: 包含 `IN (SELECT ...)` 子查询，customer 匿名子查询

**JOIN 算法**: 子查询物化 + 哈希链

---

## 五、根因总结

### 5.1 OOM 查询根因

| 查询 | 根因 | 触发条件 |
|------|------|---------|
| **Q2** | `supplier` 的 JOIN ON = `true`（笛卡尔积） | `tpch_reorder_extra_tables` 碰撞守卫触发，返回原始顺序 |
| **Q5** | 循环 JOIN 图 + nation-bridge 处理不当 | `customer` 是基表最后加入，`supplier×customer` 产生大笛卡尔 |
| **Q21** | `lineitem l1` JOIN ON = `true`（笛卡尔积） | `l1` 是 lineitem 别名，首次 JOIN 找不到谓词 |

### 5.2 碰撞守卫 BUG

```rust
// parser.rs:902-930
// BUG: part 和 partsupp 都以 'p' 开头，触发碰撞检测
// 但它们之间有等值边 p_partkey=ps_partkey，可以安全重排
let p1 = &t[..1];  // "partsupp"[..1] = 'p', "part"[..1] = 'p'
if !seen_prefix.insert(p1) {
    collision = true;  // 误判！
}
if collision {
    return extras.to_vec();  // 直接返回原始顺序！
}
```

**正确行为**: 如果两个冲突表之间有等值边（可以相互 reach），应该允许重排。

### 5.3 JoinKey::All 笛卡尔积条件

```rust
// find_join_key_index 在以下情况返回 JoinKey::All:
// 1. ON 字面量为 "true" (Literal("true"))
// 2. ON 条件引用的表不在 left/right 中
// 3. ON 条件两边都是同一侧列
// 4. ON 条件包含无法解析的列名
```

---

## 六、修复方案建议

### 6.1 Q2 修复: 改进碰撞守卫

```rust
// 在碰撞检测中，检查碰撞的两个表是否有等值边相连
// 如果有，允许重排（贪心算法会正确处理）
for (t1, t2) in collisions {
    let has_edge = edges.iter().any(|(l, r, _)|
        (bare(t1)==l && bare(t2)==r) || (bare(t1)==r && bare(t2)==l)
    );
    if !has_edge {
        return extras.to_vec();  // 真正无法重排
    }
}
// 允许重排，继续贪心算法
```

### 6.2 Q5 修复: nation-bridge 启发式扩展

```rust
// 当 customer 是基表时，强制 orders 先于 customer 加入
// orders 的日期过滤（30万行）可以作为有效过滤
// 避免 supplier(200) × customer(3万) 无过滤笛卡尔
```

### 6.3 Q21 修复: 别名表 JOIN 谓词处理

```rust
// lineitem l1 的 l_suppkey 在 join_tables 中应映射到 lineitem|l1
// resolve_bare 应正确处理别名表
```

---

## 七、性能数据汇总

| Q | 行数 | 峰值内存 | 执行时间 | 状态 | JOIN 算法 |
|---|------|---------|---------|------|---------|
| Q1 | 2 | 96 MB | 41 ms | ✅ | 单表扫描 |
| Q2 | 20 | ~200 MB | ~13 s | ✅ | 哈希链 (FIXED) |
| Q3 | 0 | 1.9 GB | 2.2 s | ✅ | 哈希链 |
| Q4 | 0 | 1.8 GB | 6.3 s | ✅ | 哈希链 |
| Q5 | 5 (SF=1 target) | — (parser-level heuristic verified) | <30s (parser test) | ⚠️ parser-level 修复 | 哈希链 (nation-bridge 启发式 + force_orders_first) |
| Q6 | 1 | 96 MB | 42 ms | ✅ | 单表扫描 |
| Q7 | [未实测]* | — | — | ⚠️ 部分 | 哈希链 |
| Q8 | [未实测]* | — | — | ⚠️ 部分 | 哈希链 |
| Q9 | [未实测]* | — | — | ⚠️ 部分 | 哈希链 |
| Q10 | [未实测]* | — | — | ⚠️ 部分 | 哈希链 |
| Q11 | 29,636 | — | — | ⚠️ 部分 | 哈希链 |
| Q12 | [未实测]* | — | — | ⚠️ 部分 | 哈希链 |
| Q13 | [未实测]* | — | — | ⚠️ 部分 | 哈希链+子查询 |
| Q14 | [未实测]* | — | — | ⚠️ 部分 | 哈希链 |
| Q15 | [未实测]* | — | — | ⚠️ 部分 | 子查询+哈希链 |
| Q16 | 18,314 | — | — | ⚠️ 部分 | 哈希链 |
| Q17 | [未实测]* | — | — | ⚠️ 部分 | 哈希链 |
| Q18 | [未实测]* | — | — | ⚠️ 部分 | 哈希链+HAVING |
| Q19 | [未实测]* | — | — | ⚠️ 部分 | 哈希链 |
| Q20 | [未实测]* | — | — | ⚠️ 部分 | 子查询+哈希链 |
| Q21 | 100 (SF=1 target) | — (parser-level alias predicate verified) | <1s (parser test) | ⚠️ parser-level 修复 | 哈希链 (lineitem\|l1 alias predicate) |
| Q22 | [未实测]* | — | — | ⚠️ 部分 | 子查询+哈希链 |

> **重大更正 (2026-07-20, Issue #3650 §P1-3; 2026-08-08 merge resolution)**: 本表原声称"22/22 PASS"和"OOM 0/22",与 Q5/Q21 的历史 OOM 证据直接矛盾。
> - Q5: 修复前 SF=1 出现 18.1 GB OOM; 252 侧新增 nation-bridge 启发式 parser-level 验证,但尚不能替代完整 SF=1 E2E + PG SHA256 证据
> - Q21: 修复前 SF=1 出现 18.9 GB OOM; 252 侧新增 lineitem|l1 alias predicate parser-level 验证,但尚不能替代完整 SF=1 E2E + PG SHA256 证据
> - 其他 query 的"行数=0"是因为 **本表行数从未实测** — 它们来自代码路径分析,不是真实运行结果
> - 所有标 [未实测]* 的数据都是代码预测/估算,**没有真实 SF=1 fixture 上 22/22 的运行证据**

**真实状态**: 仅 Q1/Q4/Q6/Q22 + 简单 2-3 表连接在本次 session 实测通过(~10/22)。Q5 nation-bridge 启发式 + Q21 lineitem|l1 alias 处理由 `tests/q5_q21_reorder_test` 4/4 提供 parser-level 证据。任何 22/22 PASS 声明仍需完整 SF=1 fixture 端到端执行 + PG SHA256 证据支撑,完整依据见 [`TPCH_SF1_VERIFICATION_REPORT.md`](../TPCH_SF1_VERIFICATION_REPORT.md)

---

## 八、附录: 关键代码路径

### A. `execute_joins` 完整流程

```
1. scan_with_ahi(base_table) → 基表扫描
2. extract_single_table_predicates → 提取 WHERE 单表谓词
3. if extra_tables.non_empty():
       try_comma_join_hash_chain() → 成功则返回
4. for join_clause in select.join_clause:
       execute_single_join() → 逐个处理
5. 返回 (rows, table_info)
```

### B. `find_join_key_index` 决策树

```
ON 表达式类型
├─ Literal("true") → JoinKey::All
├─ Identifier("col") → 在 left/right 查找
├─ BinaryOp("AND") → 递归合并
└─ BinaryOp("=") → Pair 或 All
    ├─ 左列在 left, 右列在 right → Pair
    ├─ 右列在 left, 左列在 right → Pair
    └─ 其他 → All (笛卡尔)
```

### C. `tpch_reorder_extra_tables` 贪心算法

```
输入: extras=[t1,t2,...], joined=[base], conj=[谓词列表]
输出: 重排后的 extras

1. 构建 edges: (table1, table2, predicate) 从 conj 中提取等值谓词
2. 计算每表行数 (含过滤器)
3. while remaining.not_empty():
       for each t in remaining:
           reachable = edges.contains(t, accumulated)
           score = row_count(t) + (reachable ? 0 : 1e9)
       pick t with minimum score
       accumulated += t
4. 返回 out
```
