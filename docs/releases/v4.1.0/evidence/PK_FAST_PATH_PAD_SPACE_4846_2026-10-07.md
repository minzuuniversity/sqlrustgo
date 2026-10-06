# #4846（WP-G）收尾：PK 点查快路径静默丢行

- **日期**：2026-10-07
- **Issue**：#4846 / #4944（WP-G legacy 回归矩阵）
- **分支**：`fix/4944-pk-fast-path-pad-space`
- **基线**：`develop/v4.1.0` @ `e47c37c0b9`

---

## 1. 现象

WP-G 的 9 个 `#[ignore]` 里，有两个记的是 #4846 的 headline：CHAR 主键的点查返回 0 行。

实测（修复前）：

```text
CREATE TABLE c(id CHAR(10), nm VARCHAR(20))
INSERT INTO c VALUES ('U1','tom'),('U2','jerry')

SELECT count(*) FROM c WHERE id = 'U1'          -> 0     <- 应为 1
SELECT count(*) FROM c WHERE id = 'U1' AND 1=1  -> 1     <- 加任意合取项就对了
SELECT count(*) FROM c WHERE nm = 'tom'         -> 1
SELECT count(*) FROM c                          -> 2
```

**同一张表、同一个谓词，仅仅合取项个数不同，答案就不同。** 这是典型的「走了另一条路径」
的信号，也是定位的切入点。

## 2. 根因：两个独立缺陷，同一个后果

快路径在 `src/engine_select.rs`：

```rust
let pk_lookup_rows = if let Some(pk_value) =
    crate::engine_select_pk::try_extract_pk_eq_with_col(&select.where_clause, &pk_column)
{
    let row = storage.scan_pk(lookup_table, &pk_column, &pk_value)?;
    row.map(|r| vec![r]).unwrap_or_default()
} else {
    self.scan_with_ahi(...)   // 全表扫描 + evaluate_where_clause
};
```

`try_extract_pk_eq_with_col` 只认**裸** `col = literal`，对 AND / OR 返回 `None`。
所以合取项 ≥2 时走扫描路径（正确），只有单谓词走快路径。而**快路径返回的行不再过
WHERE** —— 它假定 `scan_pk` 与 `sql_compare` 语义一致。这个假定在两处不成立：

### 缺陷一：`parse_literal_token` 不剥引号（影响所有文本主键）

`Expression::Literal` 携带的是原始 token，带引号。直接用会得到：

```text
'U1'  -> Some(Text("'U1'"))     <- 引号还在
U1    -> Some(Text("U1"))
1     -> Some(Integer(1))
1.5   -> Some(Float(1.5))
```

于是快路径拿 `Text("'U1'")` 去比存储的 `Text("U1")` → 不等 → 0 行。
**这一条 VARCHAR / TEXT 主键同样中招**，与 CHAR 无关：

```text
CREATE TABLE c(id VARCHAR(10), nm VARCHAR(20))
SELECT count(*) FROM c WHERE id = 'U1'          -> 0     <- 同样的病
SELECT count(*) FROM c WHERE id = 'U1' AND 1=1  -> 1
CREATE TABLE d(id INTEGER PRIMARY KEY, ...)
SELECT count(*) FROM d WHERE id = 1             -> 1     <- 数字 token 不需剥引号
```

### 缺陷二：`scan_pk` 无法表达 PAD SPACE（影响 CHAR）

CHAR(10) 存 `'U1'` 会补齐成 `"U1        "`。`sql_compare` 对 Text 相等应用 PAD SPACE
（尾部空格忽略），所以 `id = 'U1'` 应当匹配；但默认 `scan_pk` 是
`row.first() == Some(&pk)` —— 严格相等，没有任何填充规则。

## 3. 修法

### 缺陷一：剥引号，并还原 `''` 转义

```rust
if t.len() >= 2 && t.starts_with('\'') && t.ends_with('\'') {
    return Some(Value::Text(t[1..t.len() - 1].replace("''", "'")));
}
```

### 缺陷二：CHAR 列不走快路径

新增 `engine_select_pk::pk_fast_path_preserves_semantics(table_info, pk_column)`：

```rust
!table_info.columns.iter()
    .find(|c| c.name.eq_ignore_ascii_case(pk_column))
    .map(|c| c.data_type.trim().to_ascii_uppercase().starts_with("CHAR"))
    .unwrap_or(false)
```

快路径的判断加上这个前提；CHAR 主键改走常规扫描 + `evaluate_where_clause`。

**没有选择去改 `scan_pk` 实现 PAD SPACE**：B+Tree 的精确查找无法在不扫描的前提下表达
「忽略尾部空格」这一非前缀语义，强行实现反而更糟。VARCHAR 不在该集合内 —— 那里尾部空格
是值的一部分，PAD SPACE 不适用，这正是 `sql_compare` 按比较而非按存储值应用该规则的原因。

**代价**：CHAR 主键失去 O(log N) 点查。正确性优先，且 CHAR 主键是少数场景。

## 4. 测试

新增 `crates/executor/tests/pk_fast_path_char_pad_space_4846.rs`（5 项，全绿）：

| 用例 | 钉住什么 |
|---|---|
| `single_equality_on_char_column_matches` | #4846 headline |
| `conjunct_form_agrees_with_the_bare_form` | 合取形式必须与裸形式一致 |
| `projection_through_the_fast_path_is_correct` | 快路径直接返回 rows，计数对不代表行集对 |
| `varchar_still_uses_strict_equality` | **VARCHAR 不能被误伤**：`'U1'` 匹配，`'U1␠␠␠'` 不匹配 |
| `absent_key_still_returns_no_rows` | 改走扫描路径不能变成「全匹配」 |

同时摘掉 `wp_g_legacy` 里两个 CHAR PK 测试的 `#[ignore]`：4 通过/8 忽略 → **6 通过/6 忽略**。
两者原有的诊断注释（「partial-fix」等）一并更正为实际根因。

## 5. 变异验证

| 变异 | 做法 | 结果 | 判定 |
|---|---|---|---|
| **M13** | CHAR 守卫失效（快路径对 CHAR 重新启用） | 3 项 FAILED：`single_equality_on_char_column_matches`、`conjunct_form_agrees_with_the_bare_form`、`projection_through_the_fast_path_is_correct` | **CAUGHT** |
| **M14** | 撤掉引号剥离 | 1 项 FAILED：`varchar_still_uses_strict_equality` | **CAUGHT** |

M14 只被 VARCHAR 那一项抓住，CHAR 那两项在 M14 下仍然通过 —— 因为 CHAR 已经不走快路径，
引号剥离与否都无所谓。这说明两个变异各自钉住的是各自那半，互不遮蔽。

## 6. 一个值得记的排查教训

最初我写的 VARCHAR 测试断言是「`id = 'U1'` 应返回 1」，它失败了。**那次失败是真的**：
VARCHAR 主键同样坏在引号上。差点被我当成「修复引入了新问题」而回滚 —— 那正是我前几轮
反复犯的同一个错误：**看到测试失败就怀疑新改动，而不是先假设自己最早的判断可能是错的**。

实测确认：`VARCHAR(10)` 存 `'U1'` 后读回是 `[U1]`（**没有**被填充），所以问题不可能是
PAD SPACE，只能是 token 解析。定位到 `parse_literal_token` 后一切自洽。

## 7. 仍未闭合的 WP-G

剩余 6 个 `#[ignore]`：

- BETWEEN 与 CHAR（范围比较不 PAD）
- IN 列表与 CHAR（集合成员判断不 PAD）
- DISTINCT 跨字面量长度不折叠
- executor 静默接受 `CREATE PROCEDURE` / `CREATE FUNCTION`（属 WP-C）

这些走的是 `sql_compare` 之外的路径（范围比较、集合成员），需要各自单独判断，不是同一个
根因。
