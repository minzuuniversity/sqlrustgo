# Design — v312-63 executor batch 2

## 总体架构

三个 issue 各自独立但都属于"executor 跳过执行步骤"模式：

### #4624 Trigger 持久化

**现状**：
- MemoryStorage 完整实现 `create_trigger`/`list_triggers`/`drop_trigger`（`crates/storage/src/engine.rs:1855`）。
- FileStorage `create_trigger` 是 no-op（`crates/storage/src/file_storage.rs:1354` 测试甚至注释 "returns Ok but does nothing"）。
- BinaryStorage、AppendOnlyStorage 同样 no-op。
- SqliteMode 默认 backend 是 FileStorage，所以 CLI 下触发器永远不会被持久化、永远不触发。

**目标**：FileStorage 持久化 trigger metadata 到 `triggers.json`，启动时加载。

**设计**：
1. FileStorage 引入 `triggers: HashMap<String, TriggerInfo>`。
2. `create_trigger`：写入内存 + 持久化到 `<data_dir>/triggers.json`。
3. `list_triggers(table)`：从内存按 `table_name` 过滤。
4. `drop_trigger`：从内存 + 磁盘同步删除。
5. 启动时：从 `triggers.json` 加载填充内存。
6. 其他 backend（binary/append_only）保持 in-memory only（仅当前进程有效）。

**为什么不依赖 catalog**：catalog 是 view/table 的元数据存储（V312-55F），但 trigger 不属于 catalog schema。FileStorage 的 metadata 是与数据文件一同在 `data_dir/` 下管理的，trigger 走相同路径更一致。

### #4637 VARCHAR(N) 长度限制

**现状**：
- 整个仓库搜索 `string too long`/`too big`/`too long for column`/`string or blob too big` 全部 0 命中。
- `col.char_max_length` 是 `Option<usize>`，但只有 CHAR 补齐逻辑用到，VARCHAR 校验完全缺失。

**目标**：在 INSERT/UPDATE DML 路径上，如果列是 VARCHAR(N) 或 CHAR(N) 且传入字符串长度 > N，返回 strict 错误。

**设计**：
1. 在 `src/engine_dml.rs` INSERT 路径的 CHAR(N) 补齐代码（约 200 行）之后增加校验：遍历 `processed_records`，对每行每列检查 `col.char_max_length`。
2. 错误信息格式：`String too long for column 'c5' (length 6 > 5)`，对齐 MySQL 的 `Data too long for column` + SQLite 的 `string or blob too big`。
3. 同步应用到 UPDATE `apply_set_clauses` 与 ODKU `apply_odku`。
4. 不实现 truncate/warning 模式（YAGNI，issue 也是测 strict 行为）。

**为什么不改 catalog**：catalog 只存表/列定义，长度约束在 `ColumnDefinition::char_max_length` 已存在，只缺运行时校验。

### #4638 CREATE VIEW

**现状**：
- `execute_create_view` (`src/execution_engine.rs:1038`) 已正确将 view 写入 `self.views: HashMap<String, CreateViewStatement>`（PR #4567 修复）。
- `rewrite_view_from` (`src/engine_select.rs:387`) 实现 view 解析。
- 但 issue 报 `SELECT * FROM v1` 报 `Table not found: v1`，说明在 SqliteMode/CLI 路径上没命中。

**目标**：诊断 view 名归一问题或其他路径差异，修复以确保 SELECT FROM view 实际返回数据。

**设计**：
1. 阅读 `rewrite_view_from` 与 `execute_create_view`，确认 view 名 key 一致性。
2. 如果 view 名归一不一致（小写 vs 大写 vs 原样），强制统一为 lowercase。
3. 测试覆盖 CLI 路径与 engine.execute 路径。
4. 如果 view 实际已经能 SELECT（PR #4567 修复后），issue 可能已修复——需要在 SqliteMode 下实测确认。

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| FileStorage `triggers.json` 损坏 | 启动时加载失败则记录 warning 并以空 trigger 注册继续运行 |
| 现有 trigger 测试破坏 | PR-2760 已实现 drop_trigger，新实现与其对齐 |
| VARCHAR 校验破坏现有接受超长数据的测试 | 此类测试预期失败；记录 issue 但不在本次 batch 内放宽 |
| View 持久化 vs 内存视图差异 | 本次仅验证单 instance 内 view 工作；FileStorage view 持久化留 v313.0 follow-up |

## 测试策略

- 每个 issue 单元测试 + 集成测试 + CLI batch stdin 端到端复现 issue 例子。
- 新建 3 个集成测试文件：`tests/integration/sql/{trigger_persistence,varchar_length,view_resolution}_test.rs`。
- 共 12-18 个新测试。