# Tasks: LocalExecutor engine 字段重构

## 1. OpenSpec 准备

- [x] 1.1 openspec init in worktree
- [x] 1.2 openspec new change refactor-local-executor-engine-field
- [x] 1.3 编写 proposal.md
- [x] 1.4 编写 specs/local-executor-engine-field/spec.md
- [x] 1.5 编写 design.md

## 2. LocalExecutor 结构改造

- [ ] 2.1 移除 `<'a>` 生命周期参数
- [ ] 2.2 `storage` 改为 `Arc<RwLock<dyn StorageEngine>>`
- [ ] 2.3 添加 `engine: Arc<Mutex<dyn ExecutionEngine>>` 字段
- [ ] 2.4 创建 `LocalExecutorEngine` adapter struct (私有)
- [ ] 2.5 实现 `ExecutionEngine` for `LocalExecutorEngine`
- [ ] 2.6 实现 `new()` 构造函数 (owned storage, auto-construct engine)
- [ ] 2.7 实现 `new_with_engine()` 构造函数
- [ ] 2.8 验证：cargo check -p sqlrustgo-executor

## 3. 更新 call sites

- [ ] 3.1 `crates/server/src/connection_pool.rs`: 2 处 `LocalExecutor::new`
- [ ] 3.2 `crates/executor/src/harness.rs`: 1 处调用
- [ ] 3.3 `crates/executor/src/local_executor.rs` mod tests: 17 处测试
- [ ] 3.4 其他：grep 全局验证无遗漏
- [ ] 3.5 验证：cargo build --all-features

## 4. 测试

- [ ] 4.1 新增单元测试：`test_local_executor_engine_field_arc_cloneable`
- [ ] 4.2 新增单元测试：`test_local_executor_new_with_engine_constructs_merge_executor`
- [ ] 4.3 验证：cargo test -p sqlrustgo-executor
- [ ] 4.4 验证：cargo test --all-features

## 5. 静态检查

- [ ] 5.1 cargo fmt --all
- [ ] 5.2 cargo clippy --all-features -- -D warnings

## 6. 提交与 PR

- [ ] 6.1 git add crates/
- [ ] 6.2 git commit -m "refactor(executor): add engine field to LocalExecutor for VTU/MERGE support (G4, #2811)"
- [ ] 6.3 git push -u gitea fix/issue-2811-local-executor-engine
- [ ] 6.4 创建 PR (head=fix/issue-2811-local-executor-engine, base=develop/v3.8.0)
- [ ] 6.5 合并 PR (force_merge)

## 7. openspec 归档

- [ ] 7.1 openspec archive refactor-local-executor-engine-field
- [ ] 7.2 提交归档
- [ ] 7.3 创建 + 合并归档 PR

## 8. 收尾

- [ ] 8.1 删除本地 + 远程 fix 分支
- [ ] 8.2 删除 worktree
- [ ] 8.3 关闭 issue #2811
- [ ] 8.4 评论添加 PR 链接
