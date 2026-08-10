# V313-15 Tasks: SQLite Harness 指令支持

## 1. 分析当前 SltDb 和 Runner 架构

- [ ] 1.1 读取 `crates/sqlrustgo_sqllogictest/src/main.rs` 全文，确认 `SltDb` 结构、`Runner::new()` 用法以及文件遍历循环
- [ ] 1.2 读取 `sqllogictest = "0.29"` 的 `register_directive` API 文档（在 `~/.cargo/registry/src/` 或 docs.rs 上确认签名）
- [ ] 1.3 读取 `crates/sqlrustgo_sqllogictest/Cargo.toml`，确认所有依赖版本

## 2. 扩展 SltDb 结构

- [ ] 2.1 在 `crates/sqlrustgo_sqllogictest/src/main.rs` 顶部添加 `use std::collections::HashMap;` 和 `use std::sync::Arc;`（若尚未引入）
- [ ] 2.2 在 `crates/sqlrustgo_sqllogictest/src/main.rs` 添加 `use parking_lot::RwLock;`（若尚未引入）
- [ ] 2.3 将 `SltDb` 结构从：
  ```rust
  pub struct SltDb {
      engine: MemoryExecutionEngine,
  }
  ```
  改为：
  ```rust
  pub struct SltDb {
      engine: MemoryExecutionEngine,
      variables: Arc<RwLock<HashMap<String, String>>>,
  }
  ```
- [ ] 2.4 更新 `impl Default for SltDb`，添加 `variables: Arc::new(RwLock::new(HashMap::new()))`
- [ ] 2.5 更新 `SltDb::new()` 工厂方法，接收 `Arc<RwLock<HashMap<String, String>>>` 参数
- [ ] 2.6 添加 `SltDb::new_with_shared_vars(vars: Arc<RwLock<HashMap<String, String>>>) -> Self` 辅助构造器
- [ ] 2.7 运行 `cargo build -p sqlrustgo_sqllogictest` 确认结构体修改无编译错误

## 3. 注册 set variable 指令处理器

- [ ] 3.1 在 `async_main()` 中创建 `shared_vars: Arc<RwLock<HashMap<String, String>>>` 实例
- [ ] 3.2 在 `Runner::new()` 调用前，将 `Arc::clone(&shared_vars)` 传入 `SltDb::new_with_shared_vars`
- [ ] 3.3 在 runner 初始化后、文件循环前，调用 `tester.register_directive(...)` 注册 handler：
  - handler 闭包接收 `&str` 类型的行内容
  - 匹配 `set variable <name> <value>` 格式（注意指令行可能以空白前缀）
  - 提取 `name` 和 `value`，写入 `shared_vars`
  - 无法解析的行不 panic，让 sqllogictest 继续处理（return 或不操作）
- [ ] 3.4 运行 `cargo build -p sqlrustgo_sqllogictest` 确认 directive 注册无编译错误

## 4. 编译验证

- [ ] 4.1 运行 `cargo build -p sqlrustgo_sqllogictest` 确认全量编译通过
- [ ] 4.2 运行 `cargo clippy -p sqlrustgo_sqllogictest --all-features -- -D warnings` 确认 lint 通过
- [ ] 4.3 运行 `cargo fmt --check -p sqlrustgo_sqllogictest` 确认代码格式正确

## 5. 运行验证

- [ ] 5.1 运行 `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata --filter quantile_fun` 确认：
  - 不再报 `parse error: invalid line: "set variable sf 0.001"`
  - 输出显示文件进入 SQL 执行阶段（PASS 或 fail，而非 parse error）
- [ ] 5.2 运行 `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata --filter aggregate__quantile_fun` 确认同样消除 parse error
- [ ] 5.3 运行 `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata --filter sql__quantile_fun` 确认同样消除 parse error
- [ ] 5.4 运行完整 suite：`cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata`，确认 pass rate 提升（3 个文件从 parse error 转为 SQL 阶段执行）

## 6. 更新 Exclusion 记录

- [ ] 6.1 读取 `docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml`
- [ ] 6.2 从 `exclusions` 数组中删除以下 3 个条目：
  - `aggregate__quantile_fun.test`（follow_up: `v313-15-sqlite-harness-directives`）
  - `sql__quantile_fun.test`（follow_up: `v313-15-sqlite-harness-directives`）
  - `quantile_fun.test`（follow_up: `v313-15-sqlite-harness-directives`）
- [ ] 6.3 保存 `exclusions.yml`，确认 YAML 格式有效（YAML parse 无 error）
- [ ] 6.4 读取 `docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json`
- [ ] 6.5 更新 3 个文件的 `status` 字段：`"excluded"` → `"unknown"`
- [ ] 6.6 保存 `sqlite-corpus-manifest.json`，确认 JSON 格式有效

## 7. PR 与合并

- [ ] 7.1 提交所有变更到特性分支
- [ ] 7.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 7.3 获得至少 1 个 reviewer 批准
- [ ] 7.4 合并到 develop/v3.13.0
