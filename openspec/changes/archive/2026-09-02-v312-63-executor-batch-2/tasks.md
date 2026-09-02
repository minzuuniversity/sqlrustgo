# Tasks — v312-63 executor batch 2 (#4624/#4637/#4638)

## 1. Trigger persistence on FileStorage (#4624)

- [ ] 1.1 `crates/storage/src/file_storage.rs`: 定位 `create_trigger` no-op 占位实现的位置；引入内存 `triggers: HashMap<String, TriggerInfo>` 字段。
- [ ] 1.2 `create_trigger`：把 `TriggerInfo` 写入 `self.triggers`；同步写入 `<data_dir>/triggers.json`（与 table metadata 一致的目录）。
- [ ] 1.3 `drop_trigger`：从内存和 `triggers.json` 中删除（PR-2760 已实现内存删除但磁盘可能未删）。
- [ ] 1.4 `list_triggers(table)`：从 `self.triggers` 中按 `table_name` 过滤返回。
- [ ] 1.5 FileStorage 启动时加载 `triggers.json` 填充 `self.triggers`（如果存在）。
- [ ] 1.6 `crates/storage/src/binary_storage.rs`、`binary_storage_v2.rs`、`append_only_storage.rs`：把 `create_trigger` 升级为内存版（与 MemoryStorage 一致），`list_triggers` 同样实现。
- [ ] 1.7 集成测试 `tests/integration/sql/trigger_persistence_test.rs`：CLI batch stdin 端到端，复现 issue 例子，断言 `SELECT count(*) FROM log` 返回 `1`。
- [ ] 1.8 单元测试 `crates/storage/src/file_storage.rs`：create/drop/list round-trip。
- [ ] 1.9 验证：跨进程（关闭再打开 FileStorage）trigger 仍然存在并触发。

## 2. VARCHAR(N) 长度限制 (#4637)

- [ ] 2.1 `src/engine_dml.rs:200-222` 附近（CHAR(N) 补齐代码之后）：新增 VARCHAR/CHAR 长度校验。当 `col.char_max_length = Some(n)` 且 `Value::Text(s).len() > n` 时返回 `SqlError::ExecutionError` 带列名/实际长度信息。
- [ ] 2.2 同样的校验应用到 UPDATE 路径（`src/engine_helpers.rs:apply_set_clauses` 之后或之前）。
- [ ] 2.3 `apply_odku` 中同样加入校验（ODKU 通过 storage.update 路径）。
- [ ] 2.4 集成测试 `tests/integration/sql/varchar_length_test.rs`：覆盖 `(5)` 列收到 `'abcdef'` (6) 报错；`(5)` 列收到 `'abc'` (3) 正常；CHAR(5) 收到 `'abc'` 补齐到 `'abc  '` 但 `'abcdef'` 报错。
- [ ] 2.5 CLI batch stdin 端到端：复现 issue 例子 `INSERT INTO t VALUES ('abcdef')` 应返回非零 exit code。

## 3. CREATE VIEW 实际生效 (#4638)

- [ ] 3.1 `src/engine_select.rs:387 rewrite_view_from`：审查 view 名大小写归一；确保 view 名 key 与 `execute_create_view` 写入 key 一致。
- [ ] 3.2 `src/execution_engine.rs:1038 execute_create_view`：确认 `self.views` 写入并 `view.name` 是归一后的小写或大写 key。
- [ ] 3.3 `crates/sqlrustgo-cli/src/sqlite_mode.rs` 验证：单 instance 下 `run_batch("CREATE VIEW v1 ...")` 后再 `run_batch("SELECT * FROM v1")` 能命中。
- [ ] 3.4 如果 view 名归一有问题，修复 key 一致性。
- [ ] 3.5 集成测试 `tests/integration/sql/view_resolution_test.rs`：CLI batch stdin 复现 issue 例子，断言 SELECT * FROM v1 返回 1 行（1, 10）。
- [ ] 3.6 单元测试 `src/engine_select.rs` 或 `src/execution_engine.rs`：CREATE VIEW + SELECT round-trip。

## 4. Documentation & Verification

- [ ] 4.1 `docs/releases/v3.12.0/CHANGELOG.md` 追加 #4624/#4637/#4638 已解决条目。
- [ ] 4.2 `cargo build --all-features` 干净构建。
- [ ] 4.3 `cargo test --all-features` 全部 PASS（含新测试 12-20 个）。
- [ ] 4.4 `cargo clippy --all-features -- -D warnings` 无 warning。
- [ ] 4.5 `cargo fmt --check --all` 无 diff。
- [ ] 4.6 复现每个 issue 的 printf|cli 命令，确认行为符合预期。
- [ ] 4.7 openspec 校验：`openspec validate v312-63-executor-batch-2 --strict` 通过。
- [ ] 4.8 commit + push + `tea pr create` + `tea pr merge --style squash`。