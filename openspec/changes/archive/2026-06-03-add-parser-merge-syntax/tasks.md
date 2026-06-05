# Tasks: Parser MERGE 语法实现

## 1. OpenSpec 准备

- [x] 1.1 openspec init in worktree
- [x] 1.2 openspec new change add-parser-merge-syntax
- [x] 1.3 编写 proposal.md
- [x] 1.4 编写 specs/parser-merge-statement/spec.md
- [x] 1.5 编写 design.md

## 2. Token 层

- [ ] 2.1 `crates/parser/src/token.rs`: 在 Keyword 段（Insert/Update/Delete 附近）新增 `Merge,`
- [ ] 2.2 `crates/parser/src/lexer.rs`: 在关键字识别 match 块新增 `"MERGE" => Token::Merge,`
- [ ] 2.3 验证：cargo check -p sqlrustgo-parser

## 3. AST 定义

- [ ] 3.1 `crates/parser/src/parser.rs`: 新增 `pub struct MergeStatement`
- [ ] 3.2 新增 `pub enum MergeSource` (Table, Subquery)
- [ ] 3.3 新增 `pub struct MergeWhenClause` + `pub enum MergeAction` (Update/Insert/Delete)
- [ ] 3.4 在 `Statement` enum 新增 `Merge(MergeStatement)` 变体
- [ ] 3.5 验证：cargo check

## 4. 解析函数

- [ ] 4.1 `crates/parser/src/parser.rs`: 新增 `parse_merge()` 主函数
- [ ] 4.2 新增 `parse_merge_source()` (Table or Subquery)
- [ ] 4.3 新增 `parse_merge_when_clause()`
- [ ] 4.4 新增 `parse_merge_action()` (Update or Insert；Delete 留作 follow-up)
- [ ] 4.5 新增 `parse_optional_alias()` 辅助函数
- [ ] 4.6 `parse_statement()` 新增 `Some(Token::Merge) => self.parse_merge()` 匹配臂
- [ ] 4.7 验证：cargo check

## 5. lib.rs 导出

- [ ] 5.1 `crates/parser/src/lib.rs` re-export 新增类型
- [ ] 5.2 验证：cargo check -p sqlrustgo-parser

## 6. 测试

- [ ] 6.1 新增 lexer test: `test_lexer_merge_keyword` (uppercase/lowercase/mixedcase)
- [ ] 6.2 新增 parser test: `test_parse_merge_basic_when_matched_update`
- [ ] 6.3 新增 parser test: `test_parse_merge_when_matched_and_not_matched`
- [ ] 6.4 新增 parser test: `test_parse_merge_with_aliases`
- [ ] 6.5 新增 parser test: `test_parse_merge_with_subquery_source`
- [ ] 6.6 新增 parser test: `test_parse_merge_rejects_missing_using`
- [ ] 6.7 新增 parser test: `test_parse_merge_rejects_missing_on`
- [ ] 6.8 验证：cargo test -p sqlrustgo-parser

## 7. 全量验证

- [ ] 7.1 cargo build --all-features
- [ ] 7.2 cargo clippy --all-features -- -D warnings
- [ ] 7.3 cargo test -p sqlrustgo-parser

## 8. 提交与 PR

- [ ] 8.1 git add crates/parser/ openspec/
- [ ] 8.2 git diff --staged --stat 验证
- [ ] 8.3 git commit -m "feat(parser): add MERGE statement syntax (G2, #2809)"
- [ ] 8.4 git push -u gitea fix/issue-2809-merge-parser
- [ ] 8.5 创建 PR (head=fix/issue-2809-merge-parser, base=develop/v3.8.0)
- [ ] 8.6 合并 PR（force_merge）

## 9. openspec 归档

- [ ] 9.1 openspec archive add-parser-merge-syntax
- [ ] 9.2 提交归档
- [ ] 9.3 创建 + 合并归档 PR

## 10. 收尾

- [ ] 10.1 删除本地 + 远程 fix 分支
- [ ] 10.2 关闭 issue #2809
- [ ] 10.3 评论添加 PR 链接
