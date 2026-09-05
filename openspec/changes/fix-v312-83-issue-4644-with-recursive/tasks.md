# Tasks — Issue #4644: WITH RECURSIVE

## 分析
- [ ] 阅读 `crates/parser/src/parser.rs` 中 `parse_with_clause` 函数
- [ ] 阅读 CTE 执行逻辑在 `src/engine_select.rs`

## Parser
- [ ] 修改 `parse_with_clause` 支持 RECURSIVE 关键字
- [ ] 解析递归 CTE 的锚点和递归成员
- [ ] 更新 CommonTableExpression 结构以支持递归标记

## Executor
- [ ] 实现递归 CTE 执行器
- [ ] 实现迭代执行直到收敛（无新行）
- [ ] 处理 UNION ALL 累积

## 测试
- [ ] 添加递归 CTE 测试用例（层次遍历树结构）
- [ ] 验证普通 WITH 不受影响（回归测试）

## 验证命令
```bash
cargo test --all-features -- with_recursive
cargo test --all-features -- cte
```
