# Tasks — Issue #4669: DROP INDEX / 函数索引 / 部分索引

## 分析
- [ ] 阅读 `crates/storage/src/engine.rs` 中 StorageEngine trait
- [ ] 阅读 `crates/storage/src/file_storage.rs` 中 drop_index 函数
- [ ] 阅读 CREATE INDEX 解析逻辑

## DROP INDEX
- [ ] 实现 FileStorage.drop_index
- [ ] 实现 MemoryStorage.drop_index
- [ ] 在 catalog 中移除索引元数据
- [ ] 删除磁盘上的索引文件

## 函数索引
- [ ] 修改 CREATE INDEX 解析以支持表达式列
- [ ] 在 create_index 时解析和求值表达式
- [ ] 或返回错误：函数索引暂不支持

## 部分索引
- [ ] 在 IndexInfo 中存储 WHERE 条件
- [ ] 在 scan 时应用部分索引过滤

## 测试
- [ ] 添加 DROP INDEX 测试
- [ ] 添加函数索引测试（或验证返回合适错误）
- [ ] 添加部分索引测试

## 验证命令
```bash
cargo test --all-features -- drop_index
cargo test --all-features -- create_index
```
