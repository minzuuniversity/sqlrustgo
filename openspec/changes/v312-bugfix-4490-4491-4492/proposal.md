## Why

修复 v3.12.0 RC 三个阻塞课程上机的核心 bug，详见 Gitea issues #4490/#4491/#4492：

- **#4490** `executor` 11 个常用内置函数（now/curdate/curtime/year/month/day/datediff/round/rand/length/date_add）静默返回 Null；`year(now())`/`month(now())` 报 parse error。原因是 `crates/executor/src/expr/mod.rs::eval_fn` 大多数内置函数未注册，且通用函数参数解析循环对单参数内嵌函数调用多消费一个 `)` 导致嵌套解析失败。
- **#4491** JOIN 后 `别名.列名` 限定引用解析在 binder/executor 路径下丢列；WHERE 中标量子查询未将结果作为标量参与比较。原因是 `find_column_index` 的限定符 strip 语义对 JOIN 累积列名（`a_join_b.t.col`）处理不完整，correlated subquery 输出未标量化。
- **#4492** `char(n)` vs 短字符串比较失败（`'F '` != `'F'`）。原因是 `Value::Text` 的 `PartialEq` 不做 blank-padded 比较，且 INSERT 时未做 padding。

三者单独影响：清华 MySQL 课程第 2/5/9 章上机结果不正确，教学场景不可用。

## What Changes

- **bugfix(executor)**: 在 `crates/executor/src/expr/mod.rs::eval_fn` 注册缺失的日期/算术函数：`NOW`/`CURDATE`/`CURTIME`/`YEAR`/`MONTH`/`DAY`/`DATEDIFF`/`ROUND`/`RAND`/`LENGTH`。
- **bugfix(executor)**: `eval_fn` 默认分支不再静默返 Null；改为 `Value::Null`（保持现状）但保证已注册函数返回正确结果。
- **bugfix(parser)**: 修复 `crates/parser/src/parser.rs` 通用 `Identifier(args)` 函数调用解析路径对单参数内嵌调用多消费一个 `)` 的 bug（行 7513 处的 CAST 修复无差别应用于所有函数调用）。
- **bugfix(binder/executor)**: 修复 `find_column_index`（`crates/executor/src/expr/mod.rs`）对 JOIN 累积列名（如 `a_join_b.t.col`）的限定符解析，使 `别名.列名` 正确解析到对应表列。
- **bugfix(executor)**: 子查询结果作为标量参与外层 WHERE 比较，确保 `(SELECT min(x) FROM ...)` 返回单值与外层列比较正确。
- **bugfix(types)**: 字符串比较算子（`=`）对两个 `Value::Text` 做 blank-padded 比较（短串右补空格到较长串长度再比，或 trim 末尾空格后比）；其他算子（`<`/`>`/`<=`/`>=`）也按相同语义处理。
- **test**: 为每个修复添加至少 1 个回归测试，覆盖 issue 中的最小复现用例。

## Capabilities

### New Capabilities
<!-- Capabilities being introduced. Replace <name> with kebab-case identifier. Each creates specs/<name>/spec.md -->
- `builtin-function-coverage`: 内置函数注册表覆盖度与默认行为（未知函数不静默 Null）
- `string-comparison-semantics`: 字符串比较的 blank-padded 语义（char(n) vs short string）
- `nested-function-call-parse`: 通用函数调用解析对单参数内嵌调用的正确性

### Modified Capabilities
<!-- Existing capabilities whose REQUIREMENTS are changing -->
（无 — 所有变更都是 bug 修复，不修改既有 capability 的需求）

## Impact

- 影响范围：核心 executor（`crates/executor/src/expr/mod.rs`）、parser（`crates/parser/src/parser.rs`）、types 间接通过 `eval_binary_op` 的字符串处理。
- 兼容性：现有 PASS 测试不应回归；少数原本依赖 `eval_fn` 静默 Null 的代码路径需要确认。
- 风险：`blank-padded` 比较语义是行为变更——若现有测试断言严格 `=` 比较 `char` 列，可能失败。
- 范围：本变更独立于其他 v3.12.0 RC 流程；可单独 ship。