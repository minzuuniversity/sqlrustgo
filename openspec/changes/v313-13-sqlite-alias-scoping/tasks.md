# V313-13 Tasks: 修复 SELECT 别名在 WHERE 子句中的错误引用

## 1. 定位别名作用域检查代码

- [ ] 1.1 在 `crates/sqlrustgo-binder/src/` 中搜索 `alias`、`scope`、`where.*alias`、`10057` 相关实现
- [ ] 1.2 读取错误码 10057 的定义来源，确认别名作用域错误的错误码
- [ ] 1.3 定位 WHERE 子句列引用解析的入口方法
- [ ] 1.4 确认 SELECT 别名是在哪个阶段被注册到作用域的

## 2. 分析 Bug 根因

- [ ] 2.1 运行原始失败测试 `binder__alias_error_10057.test`，确认当前行为
- [ ] 2.2 在 Binder 中添加临时日志，输出 WHERE 子句解析时引用的列名
- [ ] 2.3 确认 `new_column` 是否在 WHERE 阶段被错误解析为有效列引用
- [ ] 2.4 确认 SELECT 别名是在什么时候被加入到可用命名空间的
- [ ] 2.5 分析 DuckDB 正确拒绝此查询的原因，推断正确的作用域边界

## 3. 修复 Binder 别名作用域逻辑

- [ ] 3.1 在 WHERE 子句列引用解析逻辑中，添加对 SELECT 别名的检查
- [ ] 3.2 确保 ORDER BY 仍可引用 SELECT 别名（不破坏现有功能）
- [ ] 3.3 修复后运行 `cargo test -p sqlrustgo_binder` 确认 Binder 无回归
- [ ] 3.4 运行 `cargo test --all-features` 确认无通用回归

## 4. 创建新增 Fixture

- [ ] 4.1 新建 `tests/compat/sqlite_v3_13/alias_where_error.sql`：
  - 包含 WHERE 子句引用 SELECT 别名的错误场景
  - 包含 ORDER BY 引用 SELECT 别名的正确场景
  - 包含 CTE 场景（同 original test）
- [ ] 4.2 新建 `tests/compat/sqlite_v3_13/alias_where_error.out`：
  - 记录每条查询的期望输出（错误信息）
- [ ] 4.3 将 `tests/compat/sqlite_v3_13/` 路径加入 compat-runner 扫描范围（若尚未支持）

## 5. 恢复 Excluded Fixture

- [ ] 5.1 修改 `crates/sqlrustgo_sqllogictest/testdata/duckdb_full/exclusions.yml`：
  - 移除 `binder__alias_error_10057.test` 的 exclusion 条目
- [ ] 5.2 确认 `binder__alias_error_10057.test` 的 `statement error` 预期现在能正确通过
- [ ] 5.3 删除旧的 exclusion 相关注释或日志（若存在）

## 6. 运行 Compat Runner 验证

- [ ] 6.1 构建：`cargo build -p compat-runner`
- [ ] 6.2 运行完整兼容测试：`./scripts/gate/run_compat_tests.sh`（或等效命令）
- [ ] 6.3 确认 `alias_where_error` fixture PASS
- [ ] 6.4 确认 `binder__alias_error_10057.test` 不再出现在 exclusion 列表中
- [ ] 6.5 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 6.6 运行 `cargo fmt --check` 确认代码格式正确

## 7. PR 与合并

- [ ] 7.1 提交所有变更到特性分支
- [ ] 7.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 7.3 获得至少 1 个 reviewer 批准
- [ ] 7.4 合并到 develop/v3.13.0
- [ ] 7.5 更新 ISSUE（若存在 related issue），记录 PR 链接
