# V313-08 Tasks: 修复 SQLite VALUES Constructor 解析错误

## 1. 定位问题代码

- [ ] 1.1 在 `crates/parser/src/parser.rs` 中定位 `parse_insert` 方法（约 line 5185-5320）
- [ ] 1.2 确认 VALUES Constructor 解析逻辑（lines 5209-5259）
- [ ] 1.3 分析行解析循环中的 token 处理逻辑，找出提前终止的原因
- [ ] 1.4 检查 `parser.rs.bak` 是否存在，记录其内容（用于确认无遗漏代码）

## 2. 分析 Bug 根因

- [ ] 2.1 在 `parse_values_row` 或等价内层循环中添加临时日志，输出每个 token 的类型
- [ ] 2.2 运行 `INSERT INTO t VALUES (1, 2), (3, 4)` 验证多行解析问题
- [ ] 2.3 运行 `INSERT INTO t VALUES (1)` 验证单行解析正常
- [ ] 2.4 确认 `con1` 连接标记是否被正确识别（sqllogictest 多连接场景）
- [ ] 2.5 确认 bug 是出现在行内解析还是行间解析

## 3. 修复 VALUES Constructor 解析逻辑

- [ ] 3.1 根据分析结果，修复行解析循环中的提前终止 bug
- [ ] 3.2 修复后再次运行步骤 2.2，确认多行 VALUES 正确解析
- [ ] 3.3 运行 `cargo test -p sqlrustgo-parser` 确认 parser 无回归
- [ ] 3.4 运行 `cargo fmt` 格式化代码

## 4. 删除过时备份文件

- [ ] 4.1 确认 `crates/parser/src/parser.rs.bak` 为过时备份
- [ ] 4.2 删除 `crates/parser/src/parser.rs.bak`
- [ ] 4.3 验证删除后 `cargo build --all-features` 仍可正常编译

## 5. 创建新增 Parser 单元测试

- [ ] 5.1 新建 `crates/parser/tests/values_constructor_test.rs`：
  - 包含单行 VALUES 测试
  - 包含多行 VALUES 测试
  - 包含列数不匹配测试
- [ ] 5.2 运行新增测试确认通过：`cargo test -p sqlrustgo-parser values_constructor`

## 6. 运行 Sqllogictest 验证

- [ ] 6.1 运行 `insert__test_insert_invalid.test`：`cargo test -p sqlrustgo_sqllogictest insert__test_insert_invalid`
- [ ] 6.2 运行 `insert__test_insert.test`：`cargo test -p sqlrustgo_sqllogictest insert__test_insert`
- [ ] 6.3 运行 `update__test_update.test`：`cargo test -p sqlrustgo_sqllogictest update__test_update`
- [ ] 6.4 确认所有受影响的测试返回 PASS

## 7. 全面回归测试

- [ ] 7.1 运行 `cargo test --all-features` 确认无通用回归
- [ ] 7.2 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 7.3 运行 `cargo fmt --check` 确认代码格式正确

## 8. PR 与合并

- [ ] 8.1 提交所有变更到特性分支
- [ ] 8.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 8.3 获得至少 1 个 reviewer 批准
- [ ] 8.4 合并到 develop/v3.13.0
- [ ] 8.5 更新 ISSUE #3898，记录 PR 链接
