# #4944 WP-G 收尾：CHAR PAD SPACE 在 BETWEEN / IN 上缺失

- **日期**：2026-10-07
- **Issue**：#4944（WP-G legacy 回归矩阵）、#4846
- **分支**：`fix/4944-wpg-char-remaining`
- **基线**：`develop/v4.1.0` @ `77b1738f7c`（含 PR #5068）

---

## 1. 起点

PR #5068 修完 CHAR 主键点查后，WP-G 的 `#[ignore]` 从 8 个降到 6 个，其中 **3 个其实已经能过**
（只是没摘掉标记）。摘掉后剩 3 个真失败：

```text
char_between_inclusive_bounds              FAILED
char_in_list                              FAILED
char_vs_varchar_strict_when_varchar_is_target  FAILED
```

## 2. 实测的语义缺口

`CHAR(5)` 存 `'abc'`（补齐成 `"abc  "`）：

| 表达式 | 实际 | 期望 | 修复前 |
|---|---|---|---|
| `WHERE ch = 'abc'` | 1 | 1 | ✅（#5068 已修） |
| `WHERE ch IN ('abc','def','ghi')` | **0** | 3 | ❌ |
| `WHERE ch BETWEEN 'abc' AND 'ghi'` | **2** | 3 | ❌ |
| `WHERE ch > 'abc'` | **4** | 3 | ❌（PAD SPACE 下 `'abc  '` 等于 `'abc'`） |

## 3. 根因：两个 issue 做了相反的决定，落在不同代码路径

这是 WP-G 剩余缺口的结构，不是三个独立的 bug：

| issue | 决定 | 落在 | 影响 |
|---|---|---|---|
| **#4612** | BINARY 语义，**尾部空格有意义** | `compare_values` 的 Text/Text 分支（`l.cmp(r)`），注释里明确写着「Removed: PAD SPACE (Issue #4846)」 | `BETWEEN`、`IN`、`NOT IN`、`NOT BETWEEN`、ORDER BY、GROUP BY |
| **#4846** | PAD SPACE，**尾部空格忽略** | `sql_compare` 的 `=` / `!=` | `WHERE col = 'x'` |

所以同一个 CHAR(5) 列，`WHERE ch = 'abc'` 匹配得到，`WHERE ch IN ('abc')` 匹配不到 ——
不是某一处写错了，而是**同一个语义问题有两套实现**。

## 4. 修法

新增 `executor::expr::compare_values_pad_space(left, right)`：Text/Text 时先 `trim_end()`
两侧再交给 `compare_values`，其余类型原样委托。

接到四处：

- `eval_between`
- `eval_not_between`
- `engine_utils.rs` 的 `InList` 成员判断
- `engine_utils.rs` 的 `NotInList` 成员判断

**刻意不改 `compare_values` 本身**：ORDER BY 与 GROUP BY 的 key 都走它，改那里是比
#4846 的谓词缺口大得多的决定，不该混在一次修复里。这条边界写进了函数的 doc comment。

**遗留的不一致（未修）**：`WHERE ch > 'abc'` 仍然返回 4。排序比较（`>` / `<`）走
`sql_compare` 的 `compare_values` 委托，属于第三种情形，本次未动。见 §6。

## 5. 测试

`crates/executor/tests/wp_g_legacy.rs`：**4 通过 / 8 忽略 → 11 通过 / 1 忽略**。

摘掉 5 个 `#[ignore]`：

- 3 个因 #5068 而已能过的（headline、短字面量 WHERE、补空格字面量）
- 2 个因本次修复而能过的（`BETWEEN`、`IN`）

每个摘掉的测试都补了说明，注明是哪个 PR / 哪次修复让它通过的，以及原本的诊断为何不准。

## 6. 变异验证

| 变异 | 做法 | 结果 | 判定 |
|---|---|---|---|
| **M15** | `eval_between` 退回 `compare_values` | `char_between_inclusive_bounds` FAILED | **CAUGHT** |
| **M16** | `InList` 成员判断退回 `compare_values` | `char_in_list` FAILED | **CAUGHT** |

两者各自精确命中，没有互相遮蔽。

## 7. 未闭合：最后一个 `#[ignore]`

`char_vs_varchar_strict_when_varchar_is_target`：

```text
CREATE TABLE t(v VARCHAR(10))
WHERE v = 'abc   '   当前返回 1，期望 0
```

VARCHAR 的尾部空格是值的一部分，不该 PAD SPACE。但 `sql_compare(op, left, right)` 只拿到
两个 `Value`，**拿不到列身份** —— 它无法知道这列声明的是 CHAR 还是 VARCHAR。`=` 对所有
Text 一律 trim，是 #4846 修复时的过度应用。

要正确修需要把列类型送进比较核心：`evaluate_where_clause` / `eval_predicate` 手上确实有
`table_info`，所以技术上可行，但那是影响**全引擎每一次 Text 比较**的设计变更，风险面与
本次修复完全不同量级。**本 PR 不做**，保持 `#[ignore]` 并在测试里写明性质。

顺带记录一个由此产生的观察：`VARCHAR` 列的裸形式 `WHERE id = 'x'` 走 PK 快路径时是严格
比较，而合取形式（走扫描 + `sql_compare`）会 trim —— **两种写法对 VARCHAR 给出不同答案**。
这与上一条同源，一并留给列类型感知的那次改动。

## 8. 门禁

executor crate 全量 **0 失败**（含 778 项 lib 测试）。
