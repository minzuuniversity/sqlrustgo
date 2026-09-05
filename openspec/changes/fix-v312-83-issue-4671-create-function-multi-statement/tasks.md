# Tasks — Issue #4671: CREATE FUNCTION 多语句函数体支持

## 分析
- [ ] 阅读 `crates/parser/src/parser.rs` 中 `parse_create_function`
- [ ] 阅读函数执行逻辑在 `crates/executor/src/stored_proc.rs`

## Parser
- [ ] 修改 `parse_create_function` 支持 BEGIN...END 块
- [ ] 支持 RETURNS TABLE 类型
- [ ] 支持 DECLARE 语句
- [ ] 更新 FunctionInfo 结构以支持多语句

## Executor
- [ ] 实现多语句函数执行器
- [ ] 实现语句序列解析和执行
- [ ] 实现表返回函数（RETURN QUERY）

## 测试
- [ ] 添加多语句函数测试
- [ ] 添加 RETURNS TABLE 测试
- [ ] 验证简单函数不受影响

## 验证命令
```bash
cargo test --all-features -- create_function
cargo test --all-features -- stored_function
```
