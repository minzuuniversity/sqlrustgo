## Why

Issue #4813: `SELECT MOD(10, 3)` 在 sqlrustgo 中返回 `3.3333...` (Float)
而不是 `1` (Integer)。

SQL 标准的 `MOD(a, b)`:
- PostgreSQL: 返回 integer if both integer; numeric if either numeric;
  返回 `a - b * floor(a/b)`。
- MySQL: `MOD(int, int) → int`, `MOD(numeric, numeric) → numeric`。
- SQLite: `a % b` operator,返回 integer。
- 标准 SQL:1999: MOD returns the exact integer or numeric remainder。

当前 bug 推测:`MOD(a, b)` 被实现为 `a - b * (a / b)` (浮点除法),
导致 10 - 3 * (10 / 3) = 10 - 3 * 3.3333 = 10 - 10.0 ≈ 0.000001。
或者直接实现为 `a / b` (错误函数)。

后果:
- 类型推断错误(应该 Integer 返回 Float)
- 除零错误模式不可控
- 与 SQLite/PostgreSQL 不兼容,migration 时类型断言失败

## What Changes

1. **运算符选择**: MOD 应该是整数模 (`a % b`),不是浮点除 (`a / b`)。
2. **类型推导**: 当 a, b 都是 Integer 时,MOD 返回 Integer;否则按
     SQLite/PostgreSQL 规则升级。
3. **负数语义**: SQLite/PostgreSQL 用 truncated division,所以
     `MOD(-7, 3) = -1` (因为 -7 = 3 * (-2) + (-1))。不是 Euclidean 
     mod (`MOD(-7, 3) = 2`)。
4. **Zero divisor**: `MOD(a, 0)` 应该返回 NULL(SQLite 行为)或报错
     (PostgreSQL 行为)。

## Capabilities

### New Capabilities

- `executor-mod-integer-modulo`: `MOD(integer, integer)` MUST 返回
  integer remainder;`MOD(10, 3)` MUST 返回 `1`(不是 `3.3333`)。

## Out of Scope

- `MOD(numeric, numeric)` 高精度算术:本 PR 不优化精度,只用 Rust 
  f64 即可。
- `MOD()` 与 `%` 运算符差异(应该等价):本 PR 保证 `%` 走相同路径。
- PowerShell-style `-mod`:不相关。

## Verification

- 新测试 `tests/integration/sql/p3_mod_return_type_4813_test.rs`:
  - `mod_integer_basic` — issue anchor:`SELECT MOD(10, 3)` 返回 `1`。
  - `mod_integer_division_zero` — `MOD(0, 5)` 返回 `0`。
  - `mod_integer_negative_dividend` — `MOD(-7, 3)` 返回 `-1` (SQLite 
    truncated)。
  - `mod_integer_negative_divisor` — `MOD(7, -3)` 返回 `1` (PostgreSQL:
    `7 - (-3) * trunc(7/-3) = 7 - (-3) * (-2) = 7 - 6 = 1`)。
  - `mod_zero_divisor_returns_null` — `MOD(10, 0)` 返回 NULL。
  - `mod_with_float_returns_float` — `MOD(10.0, 3.0)` 返回 float。
  - `mod_column_values` — `SELECT MOD(col1, col2) FROM t` 在表数据
    上正确。
  - `mod_uses_correct_return_type` — 验证返回 column type 是 Integer
    而非 Float。

Total: 8 tests。

- `cargo test --test p3_mod_return_type_4813_test` 8/8 PASS。
- 回归: arithmetic_test, integer_test。

## Risks / Trade-offs

- **负数语义选择**: PostgreSQL/SQLite 用 truncated division,
  MOD(-7, 3) = -1;Euclidean (Python `%`) 用 floored, MOD(-7, 3) = 2。
  选择 PostgreSQL 语义(SQL 标准),写 spec 明确。
- **NULL 传播**: `MOD(NULL, 3)` 应该返回 NULL(标准 SQL)。
- **NULL divisor**: 与 NULL operands 一致返回 NULL(SQLite-style)或
  报错(PostgreSQL)。本 PR 选 NULL return (更宽松)。