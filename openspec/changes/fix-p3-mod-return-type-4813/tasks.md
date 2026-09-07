# Tasks — Issue #4813: MOD() returns Integer remainder

## 1. 现状审查

- [ ] 1.1 `grep -rn "MOD\|builtin_mod\|fn.*mod_op" src/` 找到当前
      MOD 函数实现。
- [ ] 1.2 记录当前 MOD 逻辑(可能是 `a / b` 或浮点除法)。
- [ ] 1.3 验证 parser 接受 `MOD(a, b)` syntax。
- [ ] 1.4 写 anchor failing 测试:`SELECT MOD(10, 3)` 应返回 `1`
      (Integer),当前可能返回 `3.3333`。

## 2. MOD 函数实现修复

- [ ] 2.1 在 `src/expression.rs` (或对应文件) 重写 MOD:
      ```rust
      fn mod_op(a: Value, b: Value) -> Value {
          match (a, b) {
              (Value::Integer(x), Value::Integer(y)) => {
                  if y == 0 { Value::Null } else { Value::Integer(x % y) }
              }
              (Value::Float(x), Value::Float(y)) => {
                  if y == 0.0 { Value::Null } else { Value::Float(x % y) }
              }
              (Value::Integer(x), Value::Float(y)) |
              (Value::Float(y), Value::Integer(x)) => {
                  if y == 0.0 { Value::Null } else { Value::Float(x % y) }
              }
              (Value::Null, _) | (_, Value::Null) => Value::Null,
              _ => Value::Null,  // 类型不匹配
          }
      }
      ```
- [ ] 2.2 在 `src/types/cast.rs` 或 column type derivation 路径,确保
      `MOD(int, int)` 的 column type = Integer(不是 Float)。
- [ ] 2.3 验证 NULL operands 和 zero divisor 都返回 NULL。
- [ ] 2.4 验证负数行为:`MOD(-7, 3) = -1` (truncated, 不是 Euclidean 
      的 2)。

## 3. Tests (8 tests)

- [ ] 3.1 `mod_integer_basic` — anchor:`MOD(10, 3)` = `1`。
- [ ] 3.2 `mod_integer_division_zero` — `MOD(0, 5)` = `0`。
- [ ] 3.3 `mod_integer_negative_dividend` — `MOD(-7, 3)` = `-1`。
- [ ] 3.4 `mod_integer_negative_divisor` — `MOD(7, -3)` = `1`。
- [ ] 3.5 `mod_zero_divisor_returns_null` — `MOD(10, 0)` = NULL。
- [ ] 3.6 `mod_with_float_returns_float` — `MOD(10.0, 3.0)` = Float。
- [ ] 3.7 `mod_column_values` — `SELECT MOD(col1, col2) FROM t` 表
      上正确。
- [ ] 3.8 `mod_uses_correct_return_type` — column type 是 Integer。

## 4. Verification

- [ ] 4.1 `cargo build --all-features` clean。
- [ ] 4.2 `cargo test --test p3_mod_return_type_4813_test` 8/8 PASS。
- [ ] 4.3 `cargo test --all-features --lib` no regression。
- [ ] 4.4 `cargo test --test arithmetic_test --test parser_e2e_test`
      全绿。
- [ ] 4.5 `openspec validate fix-p3-mod-return-type-4813 --strict`
      valid。

## 5. Commit + Memory

- [ ] 5.1 Commit message:
      `fix(P3 / #4813): MOD() returns integer remainder, not float 
      division`。
- [ ] 5.2 在 `memory/` 新增 `p3-mod-return-type-4813.md` 记录 MOD
      truncated 语义与 PostgreSQL/SQLite 对照。
- [ ] 5.3 `openspec archive fix-p3-mod-return-type-4813` after 
      merge。

## 6. Out of Scope

- [ ] 6.1 Numeric arbitrary precision MOD — defer。
- [ ] 6.2 MOD Euclidean semantics (Python-style) — 选择 PostgreSQL 
      truncated。
- [ ] 6.3 BigInt MOD — 假设 int64 足够。