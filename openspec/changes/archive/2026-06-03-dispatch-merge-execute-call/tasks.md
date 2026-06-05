# Tasks: LocalExecutorDml MERGE dispatch

## 1. OpenSpec 准备

- [x] 1.1 openspec init in worktree
- [x] 1.2 openspec new change dispatch-merge-execute-call
- [x] 1.3 编写 proposal.md
- [x] 1.4 编写 specs/merge-dispatch-g3/spec.md
- [x] 1.5 编写 design.md

## 2. 实现 LocalExecutorDml 改造

- [ ] 2.1 添加 `storage: Arc<RwLock<dyn StorageEngine>>` 字段
- [ ] 2.2 添加 `new_with_storage(storage, engine)` 构造函数
- [ ] 2.3 修改 `new()` 使用 `MemoryStorage` 默认 storage
- [ ] 2.4 添加 `execute_dml(sql: &str)` 方法
- [ ] 2.5 添加私有 `convert_parser_merge_to_planner` 函数
- [ ] 2.6 添加私有 `convert_expr` 辅助函数
- [ ] 2.7 添加私有 `convert_clause` 辅助函数
- [ ] 2.8 验证：cargo check -p sqlrustgo-executor

## 3. 单元测试

- [ ] 3.1 新增 `test_execute_dml_routes_merge_to_executor`
- [ ] 3.2 新增 `test_execute_dml_rejects_non_merge`
- [ ] 3.3 新增 `test_execute_dml_returns_parse_error_for_malformed`
- [ ] 3.4 新增 `test_new_with_storage_constructor`
- [ ] 3.5 验证：cargo test -p sqlrustgo-executor --all-features --lib

## 4. 静态检查

- [ ] 4.1 cargo fmt --all
- [ ] 4.2 cargo clippy -p sqlrustgo-executor --all-features -- -D warnings

## 5. 提交与 PR

- [ ] 5.1 git add crates/executor/src/local_executor_dml.rs openspec/
- [ ] 5.2 git commit -m "feat(executor): dispatch MERGE from LocalExecutorDml to MergeExecutor (G3, #2810)"
- [ ] 5.3 git push -u gitea fix/issue-2810-merge-dispatch
- [ ] 5.4 创建 PR (head=fix/issue-2810-merge-dispatch, base=develop/v3.8.0)
- [ ] 5.5 合并 PR (force_merge)

## 6. openspec 归档

- [ ] 6.1 openspec archive dispatch-merge-execute-call
- [ ] 6.2 提交归档
- [ ] 6.3 创建 + 合并归档 PR

## 7. 收尾

- [ ] 7.1 删除本地 + 远程 fix 分支
- [ ] 7.2 删除 worktree
- [ ] 7.3 关闭 issue #2810
- [ ] 7.4 评论添加 PR 链接
