## Context

Issue #4813 — `MOD(10, 3)` 返回 `3.3333` (Float) 而非 `1` (Integer)。

Bug 推测:
1. `MOD(a, b)` 实现为 `a / b` (错误,这是除法)。
2. 或实现为 `a - b * (a / b_f64)` (浮点除法),导致精度问题。
3. 或实现为 `a % b_f64` (Rust `%` 要求同类型,如果强制 as f64 会丢
   整数语义)。

正确实现:
```rust
fn mod_op(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => {
            if y == 0 {
                Value::Null
            } else {
                Value::Integer(x % y)  // Rust int % 是 truncated
            }
        }
        (Value::Float(x), Value::Float(y)) => {
            if y == 0.0 {
                Value::Null
            } else {
                Value::Float(x % y)
            }
        }
        // Mixed: coerce to float
        (Value::Integer(x), Value::Float(y)) | (Value::Float(y), Value::Integer(x)) => {
            if y == 0.0 { Value::Null } else { Value::Float(x % y) }
        }
        (Value::Null, _) | (_, Value::Null) => Value::Null,
        _ => Value::Null,  // type mismatch
    }
}
```

注意:`Value::Integer(x) % 0` 在 Rust 中是 panic。必须先 check `y == 0`。

## Approach

### A1. 找到当前 MOD 实现

```bash
grep -rn "MOD\|fn mod_op\|builtin_mod" src/executor/ src/expression.rs
```

### A2. 重写为 truncated integer / float modulo

按上面模板,在 `src/expression.rs` 或 `src/executor/builtin_fn.rs`
中替换实现。

### A3. 验证 type inference

在 SELECT projection 中,`MOD(int, int)` 的 column type 必须 integer
而非 float。检查 column type derivation code。

### A4. NULL 传播

`MOD(NULL, 3)` 和 `MOD(3, NULL)` 都返回 NULL。`MOD(10, 0)` 返回
NULL(不 panic)。

### A5. NULL/Zero consistency

PostgreSQL 在 `10 % 0` 报 `division by zero`,SQLite 返回 NULL。本
PR 选 NULL (更宽松,符合 SQL standard)。文档说明。

## Files Changed

| File | Lines | Purpose |
|------|-------|---------|
| `src/expression.rs` 或 `src/executor/builtin_fn.rs` | +30 | 重写 MOD 函数 |
| `src/types/cast.rs` (可能) | +10 | integer mod 类型推导 |
| `tests/integration/sql/p3_mod_return_type_4813_test.rs` | +150 | 8 tests |

Total: ~190 行, ~3 files touched。

## Verification

| Test | Expected |
|------|----------|
| `cargo build --all-features` | clean |
| `p3_mod_return_type_4813_test` | 8/8 PASS |
| `arithmetic_test` | no regression |
| `integer_test` | no regression |
| `parser_e2e_test` | 249/249 PASS |

## Non-Goals

- Numeric (arbitrary precision) MOD
- MOD 与 Python `%` (Euclidean) 对齐(选择 truncated)
- 大整数 BigInt MOD (假设 int64 足够)

## Difficulty Tier

**🟢 EASY** — 单点函数实现修正,bug 是简单 div-vs-mod 错用。10-30 
行 fix + 8 个 test。