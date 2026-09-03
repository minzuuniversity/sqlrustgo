# Tasks — issue-4677 LIKE ESCAPE executor 语义

## T1: 匹配器

- [x] T1.1 新增 `sql_like_match_esc(text, pattern, escape)` 公开函数（`None` 时委托原 `sql_like_match` 语义）
- [x] T1.2 `like_match_recursive` 增加 `escape: Option<u8>` 参数
- [x] T1.3 转义字节字面匹配：`%`/`_`/任意字符被转义后按字面比较，失配走既有回溯分支
- [x] T1.4 文本耗尽后残余 pattern 仅接受未转义 `%`（转义 `%` 无法匹配空尾）
- [x] T1.5 悬垂转义符（pattern 末尾）宽松处理：不崩溃，按失配处理

## T2: 求值路径

- [x] T2.1 `src/expr_utils.rs::Like` arm 传入 parser 解析出的 escape 字符
- [x] T2.2 `src/expr_utils.rs::NotLike` arm 同样传入 escape

## T3: 测试

- [x] T3.1 `test_like_match_escape_wildcards`：`100!%` 只匹配 `100%`；`a!_b` 只匹配 `a_b`；混合通配
- [x] T3.2 `test_like_match_escape_backslash`：反斜杠转义（MySQL 惯用法）
- [x] T3.3 `test_like_match_escape_dangling_and_escaped_escape`：`!!` 匹配字面 `!`；悬垂 `!` 失配
- [x] T3.4 `test_like_match_no_escape_backward_compat`：无 ESCAPE 行为不回归
- [x] T3.5 旧测试 `test_like_match_backtracking` / `test_like_match_with_special_chars` 适配新签名（传 `None`）

## T4: 验证

- [x] T4.1 `cargo test -p sqlrustgo-executor --lib --all-features like` 15 passed
- [x] T4.2 `cargo check --all-features` 无新增警告
- [x] T4.3 `cargo test --test v312_62_issue_batch_test` 32 passed（3 failed 为 develop 基线已有，与本改动无关）
