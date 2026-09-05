# Tasks — Issue #4679: GROUP BY ROLLUP / CUBE

## 分析
- [ ] 阅读 `crates/parser/src/parser.rs` 中 GROUP BY 解析
- [ ] 阅读 `src/engine_select.rs` 中 GROUP BY 执行逻辑
- [ ] 理解 GroupByClause 结构

## Parser
- [ ] 添加 ROLLUP 语法解析
- [ ] 添加 CUBE 语法解析
- [ ] 添加 GROUPING SETS 语法解析
- [ ] 更新 GroupByClause 结构以支持这些语法

## Executor
- [ ] 实现 ROLLUP 分组扩展
- [ ] 实现 CUBE 分组扩展
- [ ] 实现 GROUPING SETS 分组扩展
- [ ] 处理 NULL 值的 GROUPING 函数

## 测试
- [ ] 添加 ROLLUP 测试（单列、多列）
- [ ] 添加 CUBE 测试
- [ ] 添加 GROUPING SETS 测试
- [ ] 验证普通 GROUP BY 不受影响

## 验证命令
```bash
cargo test --all-features -- group_by
cargo test --all-features -- rollup
cargo test --all-features -- cube
```
