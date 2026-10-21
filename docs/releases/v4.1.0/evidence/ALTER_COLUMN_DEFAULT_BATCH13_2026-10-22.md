# 迭代 5 · 批次 13 —— `ALTER COLUMN SET/DROP DEFAULT` 在落盘后端上可用（#5471 / #5472）

**关闭**：#5471 / #5472；#5414 已在本 PR 之前修好，本 PR 附证据关单
**基线**：`develop/v4.1.0` @ `1c096c9148b5`（批次 12 的 merge commit）

---

## 0. 一句话

`ALTER TABLE t ALTER COLUMN c SET DEFAULT 99` 与 `DROP DEFAULT`
在 `MemoryStorage` 之外**根本不存在** —— trait 默认直接返回
「not supported by this storage engine」；
而 `MemoryStorage` 自己那一份虽然能跑，**列名匹配却是大小写敏感的**，
与同一 impl 里的三个兄弟方法不一致。

本批次把这条 DDL 补到**生产存储栈**上，并把列名匹配对齐。

---

## 1. #5471 —— 落盘后端完全没有这条 DDL

### 1.1 trait 默认

`crates/storage/src/engine.rs`：

```rust
/// V313-followup-1 / Issue #4154: SET/DROP DEFAULT on a column;
/// default impl returns "not supported" so backends opt in.
fn set_column_default(&mut self, _table: &str, _column: &str, _default_value: Option<String>)
    -> SqlResult<()>
{
    Err(SqlError::ExecutionError(
        "set_column_default not supported by this storage engine".to_string(),
    ))
}
```

全仓的实现只有两处：trait 默认本身，和 `MemoryStorage`。
⇒ **没有任何落盘后端 opt-in** ⇒ 生产部署（`FileStorage` 链）**无法**设置或清除列默认值。

### 1.2 生产栈是「一串包装」，少一环就重新落回 trait 默认

```text
ExecutionEngine<BoxStorageEngine>          ← crates/mysql-server/src/lib.rs
  → BoxStorageEngine                       ← crates/storage/src/binary_storage.rs
    → ParallelWalStorage                   ← crates/storage/src/parallel_wal_storage.rs
      → MvccStorage                        ← crates/storage/src/mvcc_storage.rs
        → FileStorage                      ← crates/storage/src/file_storage.rs
```

`ExecutionEngine<S>` 是泛型的（`src/execution_engine.rs:81`
`pub(crate) storage: Arc<parking_lot::RwLock<S>>`），
`src/engine_ddl.rs:1392` 直接 `storage.set_column_default(...)`。
⇒ **每一层都必须显式转发**，否则 trait 默认重新接管。

> 这不是理论上的担心：`modify_column` / `rename_column` / `drop_column`
> 在 `MvccStorage` / `WalStorage` / `ParallelWalStorage` / `BoxStorageEngine`
> 上**一个都没有转发** —— 同一族缺陷，本批次只修 `set_column_default`
> 这一条（范围见 §5）。

### 1.3 `FileStorage` 的实现要点：默认值住在 schema 里，不在行里

`ColumnDefinition::default_value` 是 **schema** 字段
（`crates/storage/src/engine.rs:901`），落盘时由
`save_table_full` 写进 `<table>.json` 的 `columns` 段
（`crates/storage/src/file_storage.rs:1794`）。

而 `save_table` 的 full/delta 判定是**按行数**的
（`crates/storage/src/file_storage.rs:1676`）：

```rust
if total_rows == 0 || last_saved == 0 { return self.save_table_full(...); }
if total_rows <= last_saved            { return self.save_table_full(...); }
// 其余走 delta：只追加 rows[last_saved..]，**不重写 schema**
```

⇒ 一个 schema 变更如果落在 delta 路径上，**默认值会静默丢失**。

⇒ 本实现直接调 **`save_table_full`**：schema 的写不依赖行数启发式。

> `save_table_full` 自己的文档注释早就写着
> 「Called by `save_table` on the first write, **after a schema change**, …」
> —— 那是**意图**；`save_table` 的代码只靠行数相等**碰巧**实现了它。

---

## 2. #5472 —— `MemoryStorage` 的列名匹配大小写敏感

| 方法 | 匹配方式 |
|---|---|
| `drop_column` | `c.name.to_lowercase() == column.to_lowercase()` |
| `modify_column` | `c.name.to_lowercase() == column.to_lowercase()` |
| `rename_column` | `c.name.to_lowercase() == old_name.to_lowercase()` |
| **`set_column_default`** | **`c.name == column`** ← 唯一的异类 |

为什么这会咬人：存储态的列名**不是用户写法的稳定函数** ——
`create_table_in_key` 只小写化**表名**、列名原样保留，
而 `modify_column` / `rename_column` 会把列名**小写化**。

⇒ 一次 ALTER 之后，列在磁盘上叫 `b`，DDL 里还写 `B`，
而 SET/DROP DEFAULT 是**唯一**再也找不到它的列级 DDL。

⇒ 本批次改为大小写不敏感，与三个兄弟方法一致。

---

## 3. 修法

| # | 位置 | 改动 |
|---|---|---|
| 1 | `crates/storage/src/engine.rs` | `MemoryStorage::set_column_default` 的列名匹配改为大小写不敏感（#5472） |
| 2 | `crates/storage/src/file_storage.rs` | 新增 `FileStorage::set_column_default`：解析 scoped key → 定位列 → 写 `default_value` → **`save_table_full` 落盘**（#5471） |
| 3 | `crates/storage/src/mvcc_storage.rs` | 转发 `set_column_default` 到内层引擎 |
| 4 | `crates/storage/src/wal_storage.rs` | 转发 |
| 5 | `crates/storage/src/parallel_wal_storage.rs` | 转发 |
| 6 | `crates/storage/src/binary_storage.rs` | `BoxStorageEngine` 转发 |

### 明确**不做**的两件事

1. **`BinaryTableStorage` 不实现**。
   BINT 格式里**根本没有 `default_value` 这个字段**
   （`crates/storage/src/bin_format.rs` 全文无 `default_value`）
   ⇒ 实现了也存不下来，重启即丢 ⇒ **静默的错误答案比崩溃更危险**。
   它保持 trait 默认的「not supported」——那是**诚实的失败**。
   同族的 `BinaryTableStorageV2` / `AppendOnlyStorage` 同理
   （它们连 `modify_column` 都没有）。
2. **不改 `FileStorage` 其它 DDL 的大小写语义**。
   本实现沿用本后端 `drop_column` / `modify_column` 的**大小写敏感**写法，
   不在单个方法上另开一个约定。`FileStorage` 的 DDL 列名匹配
   **整体**大小写敏感（且 `rename_column` 会小写化新名 ⇒ 存储态取决于列名来历）
   已单独立 **#5509** 跟踪。

---

## 4. 判别力

判别力不是「测试通过」，而是**「把改动撤掉，测试必须红」**。

| 变异 | 观察到的失败 |
|---|---|
| **M1** 把 `MemoryStorage` 的列名匹配退回大小写敏感 | `set_default_finds_the_column_case_insensitively` **FAILED**、`drop_default_finds_the_column_case_insensitively` **FAILED**（`Column not found: B`） |
| **M2** 撤掉 `FileStorage` 的实现 | **10 个用例 FAILED**，全部是 `ExecutionError("set_column_default not supported by this storage engine")` |
| **M3** 撤掉 `WalStorage` 的转发 | **恰好 1 个**用例 FAILED：`wal_storage_forwards_set_column_default` —— 其它 10 个仍绿 ⇒ 这个钉子精准命中那一环 |
| **M4** 把 `save_table_full` 换成 `save_table`（delta 路径） | **无红灯（exit 0）** ⇒ **等价变异**，见下 |

### M4 为什么没有红灯 —— 一个必须讲清楚的否定结论

`save_table` 只在 `rows.len() > last_saved_row_count` 时走 delta 路径。
要让这个条件在「schema 变更」的时刻成立，需要 `tables.rows` 在上次落盘之后增长过。

逐条核对增长点：

| 增长点 | 是否在同一临界区内落盘 |
|---|---|
| `insert_buffered`（`file_storage.rs`） | **是** —— `data.rows.extend(...)` 之后立刻 `save_table_window(..., total_rows)` |
| `flush_buffer`（`file_storage.rs`） | **是** —— 同样紧跟 `save_table_window(..., total_rows)` |

⇒ 每一条让 `tables.rows` 增长的路径都**当场落盘**
⇒ `last_saved == rows.len()` 恒成立于两次落盘之间
⇒ **delta 路径在 schema 变更点不可达**
⇒ `save_table` 与 `save_table_full` 在这个调用点上**当前等价**。

**但仍然用 `save_table_full`**，理由是：
schema 的写不应该**依赖**「行数增长必然伴随落盘」这个
**住在另一个函数实现细节里**的不变量。
一旦有人为了性能把 `flush_buffer` 改成不落盘，
`save_table` 会**静默丢掉** schema 变更，而 `save_table_full` 不会。

⇒ 这一条记在证据里，是因为**「没有红灯」必须被解释，而不是被跳过**。

### 变异脚本自身的一个坑（记录下来）

第一版变异脚本的 `restore()` 用了 `git checkout -- crates/storage/src/`，
把**未纳入备份**的三个包装层转发一起抹掉了
⇒ M2/M3/M4 的失败被污染，M4 的真实效果根本没被证明
（当时 M4 显示 3 个失败，但那 3 个其实是 `not supported`，
来自被误删的转发，不是来自 delta 路径）。

⇒ 改成「备份整个目录的 diff」后，M3 的失败数从 4 降到 **1**
（其余 3 个是被污染的连带失败），M4 变成 **0 失败**。

⇒ 记录在此，是因为**污染过的实验比没有实验更危险**：
它会给出一个「看起来有判别力」的假结论。

---

## 5. 测试钉住的行为

### `crates/storage/tests/alter_column_default_5471.rs`（11 个用例）

| 用例 | 钉住的行为 |
|---|---|
| `file_storage_accepts_set_column_default` | 落盘后端**接受**这条 DDL |
| `file_storage_persists_the_default_across_a_reopen` | 默认值**重启后还在**（schema 落盘） |
| `file_storage_persists_drop_default_across_a_reopen` | `DROP DEFAULT` 同样落盘 |
| `file_storage_persists_the_default_when_the_table_grew_since_the_last_save` | 表长过、建过索引之后，默认值**依然**落盘 |
| `file_storage_reports_a_missing_table` | 表不存在 → 仍然报 `Table not found`（不许变成静默成功） |
| `file_storage_reports_a_missing_column` | 列不存在 → 仍然报 `Column not found` |
| `wal_storage_forwards_set_column_default` | `WalStorage` 转发 |
| `mvcc_storage_forwards_set_column_default` | `MvccStorage` 转发 |
| `box_storage_engine_forwards_set_column_default` | `BoxStorageEngine` 转发 |
| `the_production_storage_stack_accepts_set_column_default` | **生产栈形态**（`ParallelWalStorage<MvccStorage<FileStorage>>`）整体可用 |
| `memory_storage_still_accepts_set_column_default` | 内存后端**不回归** |

### `crates/storage/tests/alter_column_default_case_5472.rs`（4 个用例）

| 用例 | 钉住的行为 |
|---|---|
| `set_default_finds_the_column_case_insensitively` | 列存 `b`、DDL 写 `B` → **命中** |
| `drop_default_finds_the_column_case_insensitively` | 同上，`DROP DEFAULT` 也命中 |
| `a_missing_column_still_errors` | 列不存在 → 仍然报错 |
| `the_exact_case_spelling_still_works` | 完全一致的大小写**仍然**可用（修法是放宽，不是替换） |

---

## 6. #5414 —— 已在本 PR 之前修好，本 PR 附证据关单

#5414 说的是 `MemoryStorage::set_column_default` 拿**裸表名**去查
（`table_infos.get_mut(&table.to_lowercase())`），
而表实际存在 scoped key `default\u{1}t` 下 ⇒ 必然报 `Table not found`。

**基线里这一行已经是 `let key = self.tbl(table);`** ⇒ 缺陷已被修复，
只是 issue 没关。仓库里已有对应回归用例
`crates/storage/tests/memory_set_column_default_4154.rs`（4 例）。

本 PR 在基线上实测：

```
test drop_column_default_finds_the_table_it_modifies ... ok
test set_column_default_finds_the_table_it_modifies ... ok
test set_column_default_on_a_missing_table_still_errors ... ok
test set_column_default_uses_the_same_key_scheme_as_get_table_info ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

⇒ **4/4 绿** ⇒ #5414 可以关单（关单评论里给出这条证据）。

---

## 7. 回归

| 检查 | 结果 |
|---|---|
| `cargo test -p sqlrustgo-storage --test alter_column_default_5471` | **11 passed / 0 failed** |
| `cargo test -p sqlrustgo-storage --test alter_column_default_case_5472` | **4 passed / 0 failed** |
| `cargo test -p sqlrustgo-storage --test memory_set_column_default_4154`（#5414 的既有用例） | **4 passed / 0 failed** |
| 变异实验（4 组，见 §4） | **3 组产生独立红灯；M4 为等价变异，已解释** |
| `cargo test -p sqlrustgo-storage --all-features` | 存储层全量（结果见 §8） |
| `cargo fmt --check --all` | PASS |
| `cargo clippy -p sqlrustgo-storage --all-features --lib -- -D warnings` | exit 0 |
| `cargo clippy -p sqlrustgo-storage --all-features --test alter_column_default_5471 --test alter_column_default_case_5472 -- -D warnings` | exit 0 |
| 本批次改动面 | `crates/storage/` 6 个源文件 + 2 个新测试文件 |

> `cargo clippy -p sqlrustgo-storage --all-features --all-targets -- -D warnings`
> 仍是 **exit 101**，但那 4 个错误全在本批次**未触碰**的
> `crates/storage/tests/file_storage_concurrent_same_row_5059.rs`
> ⇒ **基线既存红**，与批次 12 记录的同一条（见 #5508）。

---

## 8. 存储层全量

| 目标 | 结果 |
|---|---|
| 92 个目标 | 见下 |
| 2 个 crash 目标（`crash_commit_fsync_b1` / `crash_wal_index_b1`） | **NOT-RUN**：测试从 `<workspace>/target/debug/sqlrustgo-mysql-server` 解析 server 二进制，本批次用独立的 `CARGO_TARGET_DIR` ⇒ 报 `server binary not found` ⇒ **受控原因码 `server-binary-missing`**，**不是代码红** |

---

## 9. 本批次**没有**做的事

1. **没有**动 `FileStorage` 其它 DDL 的大小写语义 ⇒ 单独立 **#5509**。
2. **没有**给 `BinaryTableStorage` / `BinaryTableStorageV2` / `AppendOnlyStorage`
   实现这条 DDL ⇒ BINT 格式不带 `default_value`，实现了也存不下来。
3. **没有**顺手修 `modify_column` / `rename_column` / `drop_column` 在
   `MvccStorage` / `WalStorage` / `ParallelWalStorage` / `BoxStorageEngine`
   上**同样没有转发**的问题 ⇒ 那是同一族的另一个范围，另立 issue 跟踪。
4. **没有**动空闲连接回收器（#5464）—— 另一个批次。
5. **没有**动 `MemoryStorage` 的提交快照机制（#5507）—— GA 后做。

---

## 10. 引用

- `crates/storage/src/engine.rs` — trait 默认、`MemoryStorage::set_column_default`
- `crates/storage/src/file_storage.rs` — `set_column_default`、`save_table` / `save_table_full`
- `crates/storage/src/mvcc_storage.rs` / `wal_storage.rs` / `parallel_wal_storage.rs` / `binary_storage.rs` — 转发
- `crates/storage/tests/memory_set_column_default_4154.rs` — #5414 的既有回归用例
- `crates/storage/tests/alter_column_default_5471.rs` / `alter_column_default_case_5472.rs` — 本批次新增
- `src/engine_ddl.rs:1392` — 引擎侧调用点
- **#5509**（本批次新立）— FileStorage 的 DDL 列名匹配整体大小写敏感
