## Why

三个 executor 层 issue 在 develop/v3.12.0 (HEAD `0209980749`) 上仍然阻断标准 SQL 用例，与 batch 1 的"parser 接受但不执行"模式一脉相承：

1. **#4624** — `CREATE TRIGGER ... BEFORE INSERT ON t FOR EACH ROW BEGIN INSERT INTO log(msg) VALUES ('fired'); END` 静默成功，后续 `INSERT INTO t` 触发器不执行、`log` 表保持空。`crates/executor/src/trigger.rs` 解析 trigger 语法 OK，DML 路径 (`src/engine_dml.rs:192/686/920`) 调用 `trigger_executor.execute_before/after_insert/update/delete`，但 `crates/storage/src/file_storage.rs` 与 `binary_storage.rs` 的 `create_trigger` 是 no-op（返回 `Ok(())` 但 `self.triggers` 不会写入），`list_triggers` 也返回空。MemoryStorage 实现完整但 FileStorage（SqliteMode 默认 backend）完全丢弃。
2. **#4637** — `VARCHAR(5)` 列接受 `'abcdef'`（6 字符）静默保存，长度限制未生效。整个仓库内无 `validate_value_length`/`too long for column`/`string or blob too big` 任何错误信息，亦无对应检查点。
3. **#4638** — `CREATE VIEW v1 AS SELECT * FROM t` 接受但 `SELECT * FROM v1` 报 `Table not found: v1`。当前代码已经在 `execute_create_view` (`src/execution_engine.rs:1038`) 中将 view 写入 `self.views: HashMap<String, CreateViewStatement>`，并在 `engine_select.rs:387 rewrite_view_from` 实现 view 解析。但 SqliteMode/CLI 路径走 `engine.execute_select`，需要验证该路径是否完整闭环。同时 `execute_select` 经过 `rewrite_view_from` 后是否依赖 `self.views` 在 SELECT 调用前可访问——如果 catalog 不持久化或 SELECT 走另一个 engine 实例，view 会消失。FileStorage 不持久化 view 也可能是因子。

## What Changes

- **FileStorage trigger 持久化 (`crates/storage/src/file_storage.rs`)**：
  - 替换 `create_trigger` 的 no-op 占位实现为：把 `TriggerInfo` 写入磁盘上的 `triggers.json`（或追加到现有 metadata 文件），同时填充内存 `self.triggers` 以便 `list_triggers` 直接返回。
  - `drop_trigger` 已在 PR-2760 实现，需校验是否会从磁盘上同步删除（如果尚未实现则补充）。
  - `list_triggers(table)` 实现真实地从持久化读取并按 `table_name` 过滤。
- **PR-2760 关联补丁 (`crates/storage/src/binary_storage.rs`、`crates/storage/src/binary_storage_v2.rs`、`crates/storage/src/append_only_storage.rs`)**：把它们的 `create_trigger` no-op 升级为内存版（与 MemoryStorage 一致），不强制要求持久化但至少在当前进程内生效；触发器需要持久化的 backend 仅 FileStorage。
- **DML VARCHAR 校验 (`src/engine_dml.rs` INSERT 路径，约 200 行 + UPDATE 路径)**：
  - 在已存在的 `CHAR(N)` 空格补齐代码之后增加 VARCHAR/CHAR 长度校验：当 `col.char_max_length = Some(n)` 且 `record[i]` 是 `Value::Text(s)` 且 `s.len() > n` 时，按当前 SQL mode 返回错误：
    - 默认 strict：返回 `SqlError::ExecutionError("string too long for column 'c5' (length 6 > 5)")`。
    - 不在本次 scope 加 truncate/warning 模式（per-batch YAGNI）。
  - 同样逻辑应用到 UPDATE SET 阶段（`apply_set_clauses` 或之前）。
  - 测试用 strict 模式（默认就是 strict），issue 复现脚本是 `assert_ne!(exit, 0)`，匹配。
- **CREATE VIEW 验证 + 持久化（`src/engine_select.rs:387`）**：
  - 检查并修复 `rewrite_view_from`：要求单表 FROM、view 名（含大小写归一）能在 `self.views` 中命中；定义 view 时使用 `view.name.clone()`（已是大写归一 key）。
  - FileStorage 不持久化 view 是预期行为（`engine.views: HashMap` 是 in-memory 状态），但 SqliteMode 单进程单 engine 实例，view 应在 `run_batch` 之间保持。需要在 `SqliteMode` 内确认 view 在同一 instance 内可见。
  - 如果是 view 名归一问题（大小写、`rewrite_view_from` 未命中），在 view 名归一上对齐。
  - 测试覆盖：CLI batch stdin 端到端、`ExecutionEngine::execute` 路径。

## Capabilities

### New Capabilities

- `executor-trigger-fire-on-dml`: `CREATE TRIGGER` 在 FileStorage 上持久化，DML 路径上 `list_triggers` 命中并执行 trigger body。
- `executor-varchar-length-validation`: INSERT/UPDATE 时 `VARCHAR(N)`/`CHAR(N)` 长度校验，超长字符串按 strict 模式拒绝。

### Modified Capabilities

（无现有 spec 受影响——#4567 已记录 view 持久化但 in-memory 仅满足单 instance session，本次保持该行为。）

## Impact

- 受影响 crates: `crates/storage/src/file_storage.rs`、`crates/storage/src/binary_storage.rs`、`crates/storage/src/binary_storage_v2.rs`、`crates/storage/src/append_only_storage.rs`、`src/engine_dml.rs`、`src/execution_engine.rs`、`src/engine_select.rs`。
- 新增磁盘文件：`triggers.json` 在 FileStorage 的 data dir 下；启动时 FileStorage 加载该文件并填充 `self.triggers`。
- 不修改 wire 协议，不修改 WAL，不修改 catalog schema。
- 测试矩阵：3 个 issue 各 3-5 个集成测试（CLI batch stdin + REPL + engine execute 路径），共 12-20 个新测试。