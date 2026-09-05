# Tasks — Issue #4639: RIGHT JOIN / FULL OUTER JOIN

## 分析
- [ ] 阅读 `src/engine_select.rs` 中 `execute_joins` 函数
- [ ] 阅读 `src/executor/join/hash_join.rs` 中 HashJoin 实现
- [ ] 分析 JoinType 枚举定义

## Parser
- [ ] 确认 JoinType::Right 和 JoinType::Full 已正确解析

## Executor
- [ ] 在 `execute_joins` 中添加 RIGHT JOIN 处理分支
- [ ] 在 `execute_joins` 中添加 FULL OUTER JOIN 处理分支（或返回错误：SQLite 不支持）
- [ ] 修改 HashJoin 执行器支持 RIGHT JOIN（记录未匹配右表行）
- [ ] 修改 HashJoin 执行器支持 FULL OUTER JOIN（记录所有未匹配行）

## 测试
- [ ] 添加 RIGHT JOIN 测试用例
- [ ] 添加 FULL OUTER JOIN 测试用例（期望返回错误或空结果）
- [ ] 验证 LEFT JOIN 不受影响（回归测试）
- [ ] 验证 INNER JOIN 不受影响（回归测试）

## 验证命令
```bash
cargo test --all-features -- right_join
cargo test --all-features -- full_outer_join
```
