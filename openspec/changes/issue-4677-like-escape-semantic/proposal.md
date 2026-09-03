# Issue #4677: LIKE ... ESCAPE parser 已接受但 executor 语义未生效

## Why

回归验证（2026-09-03, HEAD `11c0ac08d1`）确认 #4677 的 LIKE ESCAPE 部分
仍未修复：

```sql
-- 期望只返回字面 '100%' 一行, 实际返回空集
SELECT name FROM t WHERE name LIKE '100!%' ESCAPE '!';
-- 期望只返回 'a%bc', 实际返回 'a%bc' 和 'abc' (escape 未应用)
SELECT name FROM t WHERE name LIKE 'a\%bc' ESCAPE '\';
```

根因：PR #4725 (v312-70) 只修复了 parser 的多字符 ESCAPE 接受度，
`src/expr_utils.rs` 的 `Expression::Like` / `Expression::NotLike` arm
把 escape 参数绑到 `_escape` 丢弃，求值一律调无 escape 的
`sql_like_match(text, pattern)`。executor 层匹配器本身也无 escape 概念。

## What Changes

### 匹配器（crates/executor/src/expr/mod.rs, ~60 行）

- 新增 pub `sql_like_match_esc(text, pattern, escape: Option<char>)`，
  `sql_like_match` 保留为 `escape=None` 的包装（向后兼容，TPC-H 调用方零改动）。
- `like_match_recursive` 增加 `escape: Option<u8>` 参数：
  - pattern 中 escape 字节后跟任意字节 → 该字节按字面匹配（消耗 2 个
    pattern 字节、1 个 text 字节）；mismatch 走既有回溯分支，
    `%`-rewind 语义保持。
  - 文本耗尽后的尾部循环只消耗**未转义**的 `%`（被转义的 `%` 是字面量，
    不能匹配空尾部）。
  - 尾部 dangling escape（pattern 以 escape 结尾）按字面处理（宽松）。

### 求值路径（src/expr_utils.rs, ~15 行）

- `Expression::Like(expr, pattern, escape)` → `sql_like_match_esc(..., *escape)`。
- `Expression::NotLike(left, pattern, escape)` → 同上取反。

### 测试（expr/mod.rs, ~45 行）

- `test_like_match_escape_wildcards`: `!` 转义 `%`/`_`、转义与通配混用、大小写。
- `test_like_match_escape_backslash`: MySQL 反斜杠 idiom。
- `test_like_match_escape_dangling_and_escaped_escape`: 尾部 dangling、`!!` → 字面 `!`。
- `test_like_match_no_escape_backward_compat`: 无 ESCAPE 行为不变。

## Impact

- `sql_like_match` 签名与行为不变，所有既有调用方（TPC-H Q9/Q16 等）
  零影响；仅显式带 `ESCAPE` 子句的查询进入新分支。
- LIKE 的大小写不敏感语义在 escape 场景保持（escape 字节与文本统一
  lowercase 后比较）。

## Verification

```sql
CREATE TABLE w(v TEXT);
INSERT INTO w VALUES ('a%bc'),('abc'),('aXbc');
SELECT v FROM w WHERE v LIKE 'a\%bc' ESCAPE '\';  -- 仅 a%bc (修复前 2 行)
SELECT v FROM w WHERE v LIKE 'a%bc';              -- 全部 3 行 (无 ESCAPE 不变)

CREATE TABLE u(v TEXT);
INSERT INTO u VALUES ('100%'),('100X'),('100');
SELECT v FROM u WHERE v LIKE '100!%' ESCAPE '!';  -- 仅 100% (修复前空集)

-- NOT LIKE ESCAPE:
SELECT v FROM t WHERE v NOT LIKE 'a!_%' ESCAPE '!'; -- a%b, axb (a_b 被 LIKE 命中)
```

- cargo test -p sqlrustgo-executor --lib: 764 passed (含 4 个新 LIKE escape 测试)
- cargo test -p sqlrustgo --lib: 115 passed
