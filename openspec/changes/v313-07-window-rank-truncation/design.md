# Design: 修复窗口函数 Wire Protocol 截断 Bug

## 1. 问题分析

### 1.1 错误复现

```sql
CREATE TABLE t (a INT, b INT, c INT);
INSERT INTO t VALUES (1, 1, 1);
SELECT RANK() OVER (PARTITION BY a, b ORDER BY c) FROM t;
-- Error: "Protocol: row packet truncated: rpos=1 pkt_len=1"
```

错误信息表明：
- `rpos=1`：当前读取位置为 1
- `pkt_len=1`：数据包总长度仅为 1 字节
- 预期至少需要 8 字节（64位整数）

### 1.2 数据流

```
SQL: RANK() OVER (PARTITION BY a, b ORDER BY c)
  │
  ▼
WindowVolcanoExecutor::next()  ──►  Value::Integer(1)  [窗口函数结果]
  │
  ▼
ResultSet::encode_rows()  ──►  MySQL Row Data Packet 序列化
  │
  ▼
错误：pkt_len=1，rpos=1  →  数据被截断
```

### 1.3 根因假设

`ResultSet::encode_rows()` 在编码包含窗口函数结果的行时，可能存在以下问题：

1. **Length-encoded 编码误用**：窗口函数结果为整数，但编码器可能错误地使用了短长度前缀
2. **列计数错误**：窗口函数结果列未被计入总列数，导致提前终止
3. **NULL 值处理异常**：PARTITION BY 多列时，中间列值可能为 NULL，触发异常的短包逻辑

## 2. 修复方案

### 2.1 定位问题代码

在 `crates/mysql-server/src/` 中查找 `ResultSet` 或 `encode_rows` 的实现：

```bash
# 搜索结果集编码相关代码
grep -rn "encode_rows\|ResultSet\|row.*packet\|serialize.*row" crates/mysql-server/src/
```

关键路径（推断）：
- `crates/mysql-server/src/result_set.rs` 或等价模块
- 包含 `write_row()` / `encode_column()` / `write_length_encoded()` 方法

### 2.2 窗口函数结果编码

MySQL Row Data Packet 格式（每个列）：

| 类型 | 编码方式 |
|------|----------|
| NULL (列值为 NULL) | `0xFB` 单字节 |
| 整数 `INT` | Length-encoded integer（1/2/3/4/8 字节） |
| 字符串 / TEXT | Length-encoded string（长度前缀 + 数据） |

`RANK()` 返回 `Value::Integer(n)`，应使用标准长度编码整数。

### 2.3 截断 Bug 模式

推断问题代码结构：

```rust
// 错误模式（伪代码）
fn write_row_values(values: &[Value], output: &mut Vec<u8>) {
    for (i, val) in values.iter().enumerate() {
        match val {
            Value::Integer(n) => {
                // BUG: 当 i > 0 时，可能误用短编码
                if i > 0 {
                    // 错误地写入 0xFB（NULL标记）而非整数值
                    output.push(0xFB);  // ← 截断根因
                } else {
                    write_length_encoded_int(*n, output);
                }
            }
            _ => write_length_encoded(val, output),
        }
    }
}
```

实际测试应确认：当 `PARTITION BY a, b` 时，`a`、`b` 的列值被错误编码为 `0xFB`（NULL），导致：
- 第一行数据 `a=1, b=1, c=1, rank=1`
- `a=1` 编码为 1 字节（长度前缀 `0x01`）
- `b=1` 编码为 `0xFB`（NULL 误标记）→ `pkt_len` 异常
- `c=1` 及 `rank=1` 未被编码

### 2.4 修复策略

1. **单步调试**：在 `write_row` 或 `encode_row` 入口处添加日志，打印每个值的类型和编码后的十六进制
2. **隔离验证**：对简单整数列（非窗口函数）单独测试，确认基础行编码正确
3. **窗口函数结果隔离**：先对 `SELECT RANK() FROM t`（无 PARTITION BY）测试，确认单列窗口函数编码正确
4. **PARTITION BY 多列测试**：逐步增加列数，定位截断发生的精确列

### 2.5 修复检查表

| 步骤 | 操作 | 预期 |
|------|------|------|
| 1 | `SELECT 1 AS x` | 返回 `1` |
| 2 | `SELECT RANK() OVER () FROM t` | 返回 `1`（无 PARTITION BY） |
| 3 | `SELECT a, RANK() OVER (ORDER BY a) FROM t` | 返回 `a=1, rank=1` |
| 4 | `SELECT a, b, RANK() OVER (PARTITION BY a ORDER BY b) FROM t` | 多列 PARTITION BY |
| 5 | `SELECT RANK() OVER (PARTITION BY a, b ORDER BY c) FROM t` | 完整测试（当前失败点） |

## 3. Fixture 设计

### 3.1 新增 Fixture

#### `tests/compat/mysql_v3_13/window_rank_partition_fix.sql`

```sql
# name: window_rank_partition_fix
# expect: PASS
# NOTE: 修复 V312-21 DEFERRED 状态；完整 PARTITION BY 场景

CREATE TABLE t (a INT, b INT, c INT);
INSERT INTO t VALUES (1, 1, 1);
INSERT INTO t VALUES (1, 1, 2);
INSERT INTO t VALUES (1, 2, 1);
INSERT INTO t VALUES (2, 1, 1);

-- PARTITION BY 单列
SELECT a, RANK() OVER (PARTITION BY a ORDER BY c) FROM t ORDER BY a, c;

-- PARTITION BY 多列（当前 Bug 触发点）
SELECT a, b, RANK() OVER (PARTITION BY a, b ORDER BY c) FROM t ORDER BY a, b, c;

-- DENSE_RANK
SELECT a, b, DENSE_RANK() OVER (PARTITION BY a ORDER BY c) FROM t ORDER BY a, c;

-- ROW_NUMBER
SELECT a, b, ROW_NUMBER() OVER (PARTITION BY a, b ORDER BY c) FROM t ORDER BY a, b, c;

DROP TABLE t;
```

#### `tests/compat/mysql_v3_13/window_rank_basic.sql`

```sql
# name: window_rank_basic
# expect: PASS
# NOTE: 基础窗口函数测试（无 PARTITION BY）

CREATE TABLE scores (name VARCHAR(10), score INT);
INSERT INTO scores VALUES ('Alice', 95);
INSERT INTO scores VALUES ('Bob', 87);
INSERT INTO scores VALUES ('Carol', 95);
INSERT INTO scores VALUES ('Dave', 87);

-- 无 PARTITION BY：RANK 全局排序
SELECT name, score, RANK() OVER (ORDER BY score DESC) FROM scores ORDER BY score DESC, name;

-- DENSE_RANK
SELECT name, score, DENSE_RANK() OVER (ORDER BY score DESC) FROM scores ORDER BY score DESC, name;

-- ROW_NUMBER
SELECT name, score, ROW_NUMBER() OVER (ORDER BY score DESC) FROM scores ORDER BY score DESC, name;

DROP TABLE scores;
```

### 3.2 更新现有 Fixture

#### `tests/compat/mysql_v3_12/window_rank_partition_unsupported.sql`

```sql
# name: window_rank_partition_unsupported
# expect: PASS
# NOTE: V313-07 修复后解除 DEFERRED 状态；PARTITION BY 窗口函数现已正常工作

CREATE TABLE t (a INT, b INT, c INT);
INSERT INTO t VALUES (1, 1, 1);
SELECT RANK() OVER (PARTITION BY a, b ORDER BY c) FROM t;
```

## 4. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| 调试方法 | 日志 / 断点 / 单元测试 | 单步隔离测试 | 避免大量日志影响生产代码 |
| 修复范围 | 只修 RANK / 修整个 encode_rows | 只修 RANK 结果编码 | 避免引入其他回归 |
| 测试策略 | 先修再测 / 先测再修 | 先测再修（隔离复现） | 确保 Bug 可复现后再修改 |

## 5. 验证方式

```bash
# 1. 构建并运行 compat-runner
cargo build -p compat-runner
./target/debug/compat-runner

# 2. 运行窗口函数单元测试（已有）
cargo test -p sqlrustgo-executor window

# 3. 运行新增 fixture
./scripts/gate/check_v313_07_window_rank.sh

# 4. 验证 V312-21 deferred fixture 升级为 PASS
# 检查 tests/compat/mysql_v3_12/window_rank_partition_unsupported.sql 的 # expect 行
```

## 6. 失败模式

- 若修复后 RANK 结果仍截断：说明 Bug 不在编码层，需检查 `WindowVolcanoExecutor::next()` 是否返回了异常短的值向量
- 若其他结果集（如普通 `SELECT col1, col2 FROM t`）回归失败：说明修复引入了通用编码回归，需回退并使用更窄的修复范围
- 若 `DENSE_RANK` / `ROW_NUMBER` 仍然失败：需确认 `WindowFunction` 枚举的所有变体在 ResultSet 编码层有一致的处理路径
