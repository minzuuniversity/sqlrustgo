# V313-12 Tasks: 修复 SQLite 兼容层 NOT NULL 约束强制执行

## 1. 定位 INSERT/UPDATE 执行器代码

- [ ] 1.1 在 `crates/executor/src/` 中搜索 `Insert` / `Update` 语句处理代码，确认执行器结构
- [ ] 1.2 在 `crates/executor/src/` 中搜索 `ColumnDefinition` / `nullable` 的使用位置，确认表结构信息如何传递
- [ ] 1.3 确认 `SqlError::ConstraintViolation` 错误类型是否已存在

## 2. 分析约束缺失根因

- [ ] 2.1 读取 `stored_proc.rs` 中 INSERT 分支，确认数据写入路径
- [ ] 2.2 读取 `stored_proc.rs` 中 UPDATE 分支，确认数据更新路径
- [ ] 2.3 确认 `ColumnDefinition.nullable` 是否在执行时可访问

## 3. 实现 INSERT NOT NULL 约束检查

- [ ] 3.1 在 INSERT 执行路径中添加 NULL 检查逻辑
- [ ] 3.2 违反约束时返回 `SqlError::ConstraintViolation`
- [ ] 3.3 测试 `INSERT INTO t VALUES (NULL)` 被正确拒绝

## 4. 实现 UPDATE NOT NULL 约束检查

- [ ] 4.1 在 UPDATE 执行路径中添加 NULL 检查逻辑
- [ ] 4.2 违反约束时返回 `SqlError::ConstraintViolation`
- [ ] 4.3 测试 `UPDATE t SET col=NULL` 被正确拒绝

## 5. 运行 Fixture 验证

- [ ] 5.1 运行 `constraints__test_not_null.test` 确认 PASS
- [ ] 5.2 运行 `test_constraint_with_updates.test` 确认 PASS
- [ ] 5.3 运行 `cargo test --all-features` 确认无回归

## 6. Lint 和格式检查

- [ ] 6.1 运行 `cargo clippy --all-features -- -D warnings`
- [ ] 6.2 运行 `cargo fmt --check`

## 7. PR 与合并

- [ ] 7.1 提交所有变更到特性分支
- [ ] 7.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 7.3 获得至少 1 个 reviewer 批准
- [ ] 7.4 合并到 develop/v3.13.0
