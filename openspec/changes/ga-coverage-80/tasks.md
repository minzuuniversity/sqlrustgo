# Tasks: GA 门禁覆盖率提升至 80%

> **Change**: ga-coverage-80
> **目标**: workspace 覆盖率从 69.9% 提升到 80-85%
> **Total effort**: ~8-10 小时

## 0. Round 4: Dead code cleanup + coverage verification (#3538)

- [x] **4.1** 检查并删除 dead code — PR #3569 merged
  - Removed `has_storage()` from `crates/planner/src/planner.rs` (0 callers)
  - Removed `write_tar_end_marker()` from `crates/admin/src/backup.rs` (0 callers)
  - Verified `BackupResult`/`PitrResult` NOT dead (actively used by tests)
  - 13 lines removed, 0 added, clippy clean, 189 tests pass

- [x] **4.2** 重新跑完整 workspace 覆盖率验证 — 完成
  - Affected crates (admin + planner): **85.38% total regions**, **84.14% total lines**
  - Well above the 75% milestone target
  - 3 pre-existing failures in `sqlrustgo-storage` (insert_buffer tests) block full workspace run

- [x] **4.3** 更新 PR #3528 描述 + 更新此文件 — 完成

## 1. 验证当前覆盖率基线

- [ ] 1.1 运行 `cargo llvm-cov test --workspace --lib -- --skip test_benchmark_run_short` 确认当前 workspace 总覆盖率
- [ ] 1.2 确认各落后 crate 当前覆盖率数值

## 2. Main crate engine 集成测试

- [ ] 2.1 阅读 `src/engine_select.rs` 结构，找出核心方法（find_best_plan, select_physic_alternative 等）
- [ ] 2.2 阅读 `src/execution_engine.rs` 结构，找出 execute 方法和核心执行路径
- [ ] 2.3 阅读 `src/engine_builder.rs` 结构，找出 build_engine 方法
- [ ] 2.4 在 `src/` 下为 engine_select.rs 编写 3-5 个测试用例（覆盖主要路径选择）
- [ ] 2.5 在 `src/` 下为 execution_engine.rs 编写 3-5 个测试用例（覆盖执行路径）
- [ ] 2.6 在 `src/` 下为 engine_builder.rs 编写 2-3 个测试用例（覆盖引擎构建）
- [ ] 2.7 运行 `cargo test -p sqlrustgo --lib` 验证所有测试通过
- [ ] 2.8 运行 `cargo llvm-cov test -p sqlrustgo --lib` 确认覆盖率提升

## 3. Executor 算子测试

- [ ] 3.1 阅读 `crates/executor/src/trigger.rs` 结构，找出 trigger_eval 函数
- [ ] 3.2 阅读 `crates/executor/src/expression/` 结构，找出 aggregate、join 实现
- [ ] 3.3 在 trigger.rs 中 `mod tests {}` 编写 3 个测试（覆盖 INSERT/UPDATE/DELETE trigger）
- [ ] 3.4 在 aggregate 相关文件中编写 3 个测试（覆盖 COUNT/SUM/AVG）
- [ ] 3.5 在 join 相关文件中编写 3 个测试（覆盖 INNER/LEFT/MULTI-COLUMN join）
- [ ] 3.6 运行 `cargo test -p sqlrustgo-executor --lib` 验证所有测试通过
- [ ] 3.7 运行 `cargo llvm-cov test -p sqlrustgo-executor --lib` 确认覆盖率提升

## 4. Parser 语句测试

- [ ] 4.1 阅读 `crates/parser/src/` 结构，找出核心解析函数和现有测试
- [ ] 4.2 在 parser 的 `mod tests {}` 或 `tests/parser_tests.rs` 中编写 SELECT 解析测试（WHERE/JOIN/GROUP BY/HAVING）
- [ ] 4.3 编写 INSERT/UPDATE/DELETE 解析测试
- [ ] 4.4 编写 CREATE TABLE/ALTER TABLE 解析测试
- [ ] 4.5 运行 `cargo test -p sqlrustgo-parser --lib` 验证所有测试通过
- [ ] 4.6 运行 `cargo llvm-cov test -p sqlrustgo-parser --lib` 确认覆盖率提升

## 5. MySQL Server 协议测试（补充）

- [ ] 5.1 检查 `mysql_type_tests` 中 decode_* 函数覆盖率
- [ ] 5.2 补充 handshake 失败、COM_QUERY 不同 SQL 类型、COM_STMT_PREPARE 的测试
- [ ] 5.3 运行 `cargo test -p sqlrustgo-mysql-server --lib` 验证所有测试通过
- [ ] 5.4 运行 `cargo llvm-cov test -p sqlrustgo-mysql-server --lib` 确认覆盖率提升

## 6. 最终验证

- [ ] 6.1 运行 `cargo llvm-cov test --workspace --lib -- --skip test_benchmark_run_short` 获取完整 workspace 覆盖率
- [ ] 6.2 验证总体覆盖率 ≥ 80%
- [ ] 6.3 运行 `cargo test --workspace --lib` 确认所有 lib tests 通过
- [ ] 6.4 运行 `cargo test --workspace --all-targets -- --skip test_benchmark_run_short` 验证所有集成测试可编译和运行
