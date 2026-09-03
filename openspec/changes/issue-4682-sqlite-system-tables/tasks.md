# Tasks — Issue #4682: SQLite 系统表 / 虚拟表兼容补全

- [x] 1. lexer：`"SQL"` 改为 `Token::Identifier("SQL")`，确认无
      `Token::SQL` 消费点（parser 全仓 grep）。
- [x] 2. engine_select：`sqlite_sequence` 合成
      （`query_sqlite_sequence`，AUTOINCREMENT 表 max(id)）。
- [x] 3. engine_select：分发识别 `sqlite_temp_master`（恒空集）。
- [x] 4. engine_select：系统视图 fast-path 应用 WHERE
      （`eval_predicate` + 合成 TableInfo）。
- [x] 5. engine_select：fast-path 应用 SELECT-list 投影
      （`evaluate_expression` 逐列，支持 `*` 展开与限定名）。
- [x] 6. parser：`CREATE VIRTUAL TABLE ... USING module(args)`
      解析（裸参数→TEXT 列，`key=value` 选项忽略，IF NOT EXISTS）。
- [x] 7. 顺带门禁修复：EXECUTE USING 字面量；clippy/fmt 阻断项
      （collapsible_match / nonminimal_bool / doc list / unused /
      dead_code）。
- [x] 8. 新增集成测试 `v312_76_sqlite_system_tables_test.rs`（10
      用例）并注册 Cargo.toml test target。
- [x] 9. 门禁实跑：`cargo clippy --all-features -- -D warnings`
      PASS；改动文件 `rustfmt --check` PASS；parser lib 660 测试
      PASS（含修复后的 test_execute_with_params）；集成测试
      v312-76/75/71 共 20 用例 PASS。
- [ ] 10. PR 合并后实跑 issue 复现 SQL 回归并关闭 issue。
