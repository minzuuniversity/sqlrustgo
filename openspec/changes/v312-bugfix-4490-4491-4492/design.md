## Context

v3.12.0 RC 三个核心 bug 在 develop 分支 HEAD `e4c0325da` 上确认：

| Bug | 触发条件 | 影响文件 | 根因（已通过源码确认） |
|---|---|---|---|
| #4490 | `SELECT now()` 等内置函数 | `crates/executor/src/expr/mod.rs::eval_fn` | 11 个函数未注册，默认分支返 Null |
| #4490 | `SELECT year(now())` 嵌套 | `crates/parser/src/parser.rs:7513` | 通用 args 路径无条件消费一个多余 RParen |
| #4491 | JOIN `别名.列名` | `crates/executor/src/expr/mod.rs::find_column_index` | 限定符匹配在多表 JOIN 累积列名场景不完整 |
| #4491 | WHERE 标量子查询 | `crates/executor/src/` 子查询求值路径 | correlated subquery 输出未标量化 |
| #4492 | `char(n)` vs short string | `crates/executor/src/expr/mod.rs::eval_binary_op` | `=` 算子对 Text 未做 blank-padded 比较 |

`crates/parser/src/parser.rs:7513-7515` 已通过临时 probe test 复现 bug；通用 args 循环在 7413 行 `expect(RParen)` 后，到 7513 又额外消费一个 `RParen`，导致嵌套调用失败。

## Goals / Non-Goals

**Goals:**
- 修复三个 bug 的最小改动集，每个修复附独立回归测试
- 不引入新依赖
- 保持现有 PASS 测试不回归（必要时不惜调整空白边）
- 修复后保持 `Value::Null` 在"未知函数"路径上的现状（**不**改静默行为，避免破坏其他依赖此行为的代码；改为在文档中标注）

**Non-Goals:**
- 不重写函数注册表为宏化版本
- 不引入完整日期库；用系统时间 + 简单字符串解析
- 不实现 `EXTRACT(field FROM expr)` 之外的窗口函数
- 不修改 `Value::PartialEq` 的全局语义（避免影响 Hash/sort）；blank-padded 比较限定在 `eval_binary_op` 的字符串操作符路径

## Decisions

### D1: 函数注册策略 — 扩展 `eval_fn` 单文件
**选择**：在 `eval_fn` match 中直接添加新分支（NOW/CURDATE/CURTIME/YEAR/MONTH/DAY/DATEDIFF/ROUND/RAND/LENGTH/DATE_ADD），不动既有结构。
**理由**：与现有 TRIM/LPAD/RPAD 等函数实现风格一致；改动局部化，review 简单。
**备选**：抽出 `register_function!` 宏——超出 bugfix 范围，留待后续 refactor。

### D2: 默认分支保留 Null
**选择**：`eval_fn` 的 `_ => Value::Null` 保持现状。
**理由**：改动该默认行为可能影响 #4432 (TPC-H Q17 性能) 等其他路径的容错；issue #4490 的"建议"是"未实现函数应报错"，但报错会破坏现有依赖静默 Null 容错的测试。本次只保证**已注册函数返回正确结果**，未知函数行为留待独立变更。

### D3: parser 多消费 RParen 修复 — 删除冗余消费
**选择**：删除 `crates/parser/src/parser.rs:7513-7515` 的 `if matches!(self.current(), Some(Token::RParen)) { self.next(); }`。
**理由**：行 7413 `expect(RParen)` 已经处理通用情况；该 if 块的注释声称用于 CAST 特例，但实际位置在 Identifier(args) 通配 else 分支里，无差别作用于所有函数调用。CAST 的 AS TYPE 路径在 7474-7504 行已单独消费其尾部。
**备选**：把该 if 用 `if name_upper == "CAST" && matches!(...RParen)` 限定——更保守，但删了更干净。已通过临时 probe 测试确认删除后 `SELECT foo(now())` 解析成功。

### D4: JOIN 限定符解析 — 增强 `find_column_index`
**选择**：在 `find_column_index`（`crates/executor/src/expr/mod.rs:515-565`）的限定符分支增加"严格 `qualifier.col` 匹配"——当 `col_name = "s.name"` 时，要求累积列名的首段等于 `s`（或末段等于 `col` 的 multi-join 形式）。当前实现在单表 JOIN 场景下 strip 限定符后能匹配，但在多表 JOIN 累积列名（`s_join_sc.s.name`）上 strip 限定符后变 `s.name` ——若列名实际是 `name`（仅末尾段），也能命中。
**理由**：通过临时 probe 验证：单表 + GROUP BY 场景下累积列名是 `sc.sid`、`s.name` 等。`find_column_index` 的多 join 分支应扩展为：若用户限定符等于累积列名的**第一段**（而非仅末段），也命中。
**关键修改**：限定符分支先尝试"首段相等"再 fallback "末两段相等"。

### D5: 标量子查询求值 — 输出标量化
**选择**：在 correlated subquery / scalar subquery 的执行路径中，确保当外层依赖的子查询只产出一行时，输出 `Value`（而非 `Vec<Value>`）到 WHERE 比较。
**理由**：issue #4491(b) 显示当前实现对外层比较返回 0 行或全 Null 行——意味着子查询结果**未标量化**即传给外层。
**实施点**：`crates/executor/src/executor.rs` / `crates/planner/src/planner.rs` 中 `ScalarSubquery` 物理节点返回路径；调研后再定精确修改点。

### D6: blank-padded 字符串比较 — 比较时 trim
**选择**：在 `crates/executor/src/expr/mod.rs::eval_binary_op` 的 `=`/`<`/`>`/`<=`/`>=` 操作符路径上，当两个操作数都是 `Value::Text` 时，先 `trim_end()` 两侧（短串补空格等价于 trim 后等长比较），再比较。
**理由**：MySQL 标准对 char(n) 是比较时做 blank-padded；不在存储层改，避免破坏既有数据。trim 后比较与 padding 后比较在等长情况下等价。
**关键决策**：不修改 `Value::PartialEq`（影响 Hash/sort/聚合），只在 `eval_binary_op` 字符串路径处理。
**测试**：issue #4492 的最小复现 + 反例 `'abc' = 'abc '` 应返 true。

## Risks / Trade-offs

- **R1 (D2)**: 未知函数静默 Null 不变，issue #4490 期望"应报错"未满足。建议在 docs/reference 中标注此行为限制。
- **R2 (D3)**: 删除 7513-7515 行 RParen 消费可能影响极少数测试——已计划运行 `cargo test --all-features` 全量回归。
- **R3 (D4)**: JOIN 限定符的"首段相等"语义可能误匹配——例如 `s.name` 在包含多个 `s_*` 表的 JOIN 中可能解析到错误列。计划用临时 test 验证。
- **R4 (D6)**: blank-padded 仅在 `eval_binary_op` 处理，不影响 JOIN/HASH/SORT 的等值判断。若有测试断言严格 char 比较，可能需更新——风险中等。
- **R5**: 三个 fix 都涉及 `crates/executor/src/expr/mod.rs`（D1/D4/D6），单一文件改动较集中，建议每个 fix 一个独立 commit 以便 bisect。

## Test Plan

1. **Parser test**: `SELECT foo(now())` / `SELECT foo(x, now())` 解析成功（`crates/parser/tests/`）
2. **Executor test**: `SELECT now()` / `SELECT round(3.14, 2)` 返回正确值（`crates/executor/tests/`）
3. **Executor test**: JOIN `s.name` + GROUP BY 场景返正确值（`crates/executor/tests/`）
4. **Executor test**: WHERE 子查询作标量比较（`crates/executor/tests/`）
5. **Executor test**: `char(2)` 列存 `'F'`，WHERE `sex = 'F'` 返 1 行（`crates/executor/tests/`）
6. **回归**: `cargo test --all-features --workspace` 全量通过

## Implementation Order

1. parser RParen 修复（#4490 阻塞 parser 测试）
2. eval_fn 补 11 个函数（#4490）
3. blank-padded 比较（#4492，独立）
4. JOIN 列解析 + 标量子查询（#4491，可能需要更多调研）
5. 全量回归 + clippy + fmt