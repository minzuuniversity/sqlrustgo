# 批次 46 证据 —— 逻辑备份/恢复真的做事（#5525）

| 字段 | 值 |
|---|---|
| gate_policy_eval_id | `v410-b46-logical-backup-001` |
| source_agent | openclaw-mcode (MiniMax Code) |
| source_run | 批次 46（本机实测，Mac-mini-7.local arm64 macOS 27.0.0） |
| timestamp | 2026-10-25T04:10:00+08:00 |
| 基线 | `b52a76d8d554c518c9e04498e7b2b060b0e63dff`（批次 45 之后、批次 46 之前） |
| 成果提交 | `95b136a0a5`（分支 `fix/logical-backup-restore-5525`，独立于 PR #5578） |
| 工作树 | `wt-v410-b40` |
| 关联 issue | #5525（逻辑备份/恢复都是骨架）；#5516（脚本侧入口与二进制名）；#5377（`CREATE INDEX` 库解析，本批只引用不修） |

---

## 0. 一句话

#5525 报的是**「看起来成功、实际上什么都没做」**。本批把每一处都改成
**「要么真做，要么明确报错」**，并且把实现路径上踩出来的**一个存储层缺陷**
一起修掉。

| 方向 | 修前 | 修后 |
|---|---|---|
| **枚举表** | `run_backup()` 递给备份一张**空的 `HashMap`** ⇒ 写出的 `.sql` 里 **0 张表**，却打印 `Backup completed successfully!` | `collect_all_tables()` 从 `StorageEngine` **真实枚举**；数据库没有表 ⇒ **错误** |
| **DDL** | 从**第一行的列名**猜（`first_row.keys()`）：无类型、无 NOT NULL、无主键 | **带类型**的 DDL：声明类型 / `NOT NULL` / `PRIMARY KEY` / `UNIQUE` / `FOREIGN KEY`，宽度也带 |
| **空表** | **只有至少有一行的表才写出 `CREATE TABLE`** ⇒ 空表彻底消失 | 空表**照写**（`CREATE TABLE` 在，没有任何 `INSERT`） |
| **转义** | 一律 `'{}'` 套壳 | `'` → `''`、反斜杠转义、`NULL` 不加引号、数字不加引号、BLOB 用 `X'..'` |
| **恢复** | 只**数了一下** `INSERT INTO` 行数，每张表 0 行，数据**没落进任何数据库** | `restore_into()` **真的写回** `StorageEngine`，并**逐表校验落库行数**，对不上即错误 |
| **开关** | `schema_only` / `compress` / `drop_first` / `backup_type` 被**接收后静默忽略** | `schema_only` 真生效；`compress` 与非 `full` 的 `backup_type` **明确报错**；`drop_first` 真生效 |
| **数据源** | 备份**没有任何数据源** | CLI **必须**给 `--data-dir`；拿不到 ⇒ **明确失败，绝不报成功** |

---

## 1. 修前的实测（issue 里的复现仍然成立）

```
$ sqlrustgo backup /tmp/b20_bk2 --database default --backup-type full
Starting backup for database: default
Backup completed successfully!          ← 报成功
Size: 62 bytes                          ← 62 字节 = 一个文件头
```

`crates/tools/src/backup_restore.rs` 的三处「空转」逐条对应 issue 的指控：

1. `let tables = std::collections::HashMap::new();` —— 备份拿到的是**空表清单**；
2. `create_backup()` 的 DDL 来自 `first_row.keys()` —— 没有类型、没有约束，
   而且**空表不写**；
3. `restore()` 只 `data.entry(table_name).or_default()` —— 每张表 **0 行**，
   数据**从未落库**。

---

## 2. 改了什么

### 2.1 类型化的备份内容（唯一真源）

新增 `BackupColumn` / `BackupTable` / `BackupContent`：

```rust
pub struct BackupColumn {
    pub name: String,
    pub data_type: String,            // 目录里记的类型，原样带出
    pub char_max_length: Option<usize>,
    pub primary_key: bool,
    pub not_null: bool,
}
pub struct BackupTable {
    pub name: String,
    pub columns: Vec<BackupColumn>,
    pub rows: Vec<Vec<Value>>,        // 与 columns 按位对齐
    pub unique_constraints: Vec<BackupUniqueConstraint>,
    pub foreign_keys: Vec<BackupForeignKey>,
}
```

`create_backup()` 的签名从
`HashMap<String, Vec<HashMap<String, String>>>` 改成 `&[BackupTable]` ——
**猜类型的那个 API 被删掉了**，不是被绕过。

### 2.2 三份产物，同一份内容渲染

| 文件 | 角色 |
|---|---|
| `<id>.sql` | 可移植 dump，`mysql < dump` 可直接载入 |
| `<id>.data.json` | **payload** —— `restore` 消费的就是它，自带 SHA-256 |
| `<id>.meta.json` | 元数据（id / type / size / checksum / 表清单） |

`.sql` 与 `.data.json` **都从同一个 `BackupContent` 渲染**，因此不可能漂移：
`.sql` 是 payload 的**渲染**，不是第二份真源。测试
`test_sql_and_payload_describe_the_same_backup` 逐表比对两者的表名与行数。

**为什么 restore 读 payload 而不是回读 SQL 文本**：SQL 字面量无法无损表达
JSON 文档、`NaN` 与 `-0.0`。payload 的 SHA-256 在**恢复前**校验，
损坏的备份是错误，不是半恢复（测试 `test_corrupted_payload_is_refused`）。

### 2.3 真实枚举 + 真实落库 + 校验

```rust
pub fn collect_all_tables(storage: &dyn StorageEngine, db: &str) -> Result<Vec<BackupTable>, String>
pub fn restore_into(&self, backup_id, storage: &mut dyn StorageEngine, db, drop_first)
    -> Result<RestoreResult, String>
```

`restore_into()` 每张表：存在且没给 `--drop-first` ⇒ **报错**（存储层的
`create_table_in_db` 是**静默覆盖**，不拦就会毁掉目标数据）；建表；插行；
**重新 scan 校验落库行数**，与备份不符即错误。

### 2.4 顺带修掉的存储层缺陷（被这条路径踩出来的）

`FileStorage::drop_table_in()` 只删 `tables` 里的条目，**不删 `insert_buffer`**：

```
CREATE TABLE t (id INT); INSERT INTO t VALUES (1);  -- 进 insert_buffer
DROP TABLE t;                                        -- buffer 留着
CREATE TABLE t (id INT);                            -- 同一个 key
SELECT * FROM t;                                    -- 1 行 —— 被 DROP 的表的行
```

逻辑恢复的 `--drop-first` 直接踩中：第一次恢复的缓冲行还在 buffer 里，
表被 drop 又重建，于是**落了两倍的行**（实测 `backup holds 2 row(s), 4 landed`）。

修复：`drop_table_in()` 同时 `s.insert_buffer.remove(&key)`。

**影响面分析**（GitNexus MCP 本会话不可用，手工分析）：

| 层级 | 对象 |
|---|---|
| 直接调用者 | `FileStorage::drop_table` / `FileStorage::drop_table_in_db`（trait 实现） |
| 上游调用者 | `src/execution_engine_methods.rs:642` `execute_drop_table`（SQL `DROP TABLE`）、`src/engine_create.rs:95/310`（建表替换路径）、`src/engine_cte.rs:373/422/437/448/692`（CTE 工作表清理） |
| 风险 | **LOW** —— 改动只删除属于「已被删掉的表」的状态。唯一会变的行为是「drop 后重建同名表时旧行复活」，而那本来就是缺陷 |
| 回归证据 | §5 的 A/B；`test_drop_table_keeps_the_other_tables_buffer` 是「不误伤其它表」的对照 |

### 2.5 CLI 与脚本

* `sqlrustgo backup` / `sqlrustgo restore`（`crates/mysql-server/src/main.rs`）：
  新增**必填** `--data-dir`；`backup` 新增 `--schema-only`，
  `restore` 新增 `--drop-first`。
* `sqlrustgo-cli`（`crates/sqlrustgo-cli/src/lib.rs`）：转发补上 `--data-dir`
  与 `--drop-first`。
* `scripts/backup.sh`：新增 `-D/--data-dir`（默认 `$SQLRUSTGO_DATA_DIR` 或
  `/tmp/sqlrustgo-data`）；`--schema-only` **放行**（已实现）；`--compress`
  仍拒绝（仍未实现）。

---

## 3. 修后的实测

```
$ sqlrustgo backup /tmp/bk --database default --data-dir /tmp/data
Starting backup for database: default
Backup completed successfully!
Backup ID: backup_1792868645
Tables backed up: 2
Rows backed up: 2
Size: 942 bytes
```

产出的 dump（节选）：

```sql
-- SQLRustGo logical backup
-- Database: default
-- Tables: 2
--
-- Table: users (2 row(s))
DROP TABLE IF EXISTS `users`;
CREATE TABLE `users` (
  `id` INTEGER NOT NULL,
  `name` TEXT NOT NULL,
  `email` TEXT(255),
  PRIMARY KEY (`id`)
);

INSERT INTO `users` (`id`, `name`, `email`) VALUES (1, 'alice', 'alice@example.com');
INSERT INTO `users` (`id`, `name`, `email`) VALUES (2, 'bob', NULL);

-- Table: audit_log (0 row(s))
DROP TABLE IF EXISTS `audit_log`;
CREATE TABLE `audit_log` (
  `id` INTEGER NOT NULL,
  `note` TEXT,
  PRIMARY KEY (`id`)
);
```

空表在 dump 里**在**，只是没有 `INSERT` —— 这正是修前「空表彻底消失」的反面。

---

## 4. 负对照（issue 明确要求的那条）

| 场景 | 期望 | 实测 |
|---|---|---|
| 备份**空数据库** | 必须失败 | `collect_all_tables` 报 `database \`default\` contains no tables`；CLI 退出码 ≠ 0 |
| 备份**空表清单** | 必须失败 | `create_backup` 报 `no tables to back up` |
| payload 被篡改 | 必须失败 | `backup payload ... is corrupt: checksum mismatch`；目标库**一张表都没建** |
| 恢复到已存在的表（无 `--drop-first`） | 必须失败 | `table \`users\` already exists in database \`default\`: refusing to overwrite it` |
| `LOCK TABLES`（未实现） | 必须失败 | `LOCK TABLES is not implemented: refusing to write a dump that promises table locking it does not take` |
| `--backup-type incremental`（未实现） | 必须失败 | `--backup-type \`incremental\` is not implemented; only \`full\` is supported` |
| `--compress`（未实现） | 必须失败 | `--compress is not implemented; refusing to write an uncompressed dump under a compressed name` |

---

## 5. 回归矩阵（本机实测）

| 套件 | 结果 |
|---|---|
| `cargo test -p sqlrustgo-tools` | **505 passed / 0 failed**（含新增 `logical_backup_roundtrip_5525.rs` 11 个） |
| `cargo test -p sqlrustgo-storage --features fault-injection` | **993 passed / 1 failed** —— 唯一失败是**既有红** `index_update_crash_does_not_serve_dangling_rows`，失败信息**逐字未变**：`ERROR 1146 (42000) at line 1: Table not found: t`（根因是 #5377 的 `CREATE INDEX` 库解析缺陷，与本批无关） |
| `cargo test -p sqlrustgo-mysql-server` | **388 passed / 0 failed**（`cov_v410_cli_subprocess` 的备份/恢复用例已按新契约重写并加入负对照） |
| `cargo test -p sqlrustgo-cli` | **87 passed / 0 failed** |

### 5.1 A/B —— 存储层修复的判别力

`crates/storage/tests/drop_table_discards_buffered_rows_5525.rs`（3 个用例）：

| 版本 | 结果 |
|---|---|
| **BASE**（撤掉 `insert_buffer.remove`） | **2 red**：`the dropped table's buffered rows came back`（left: 1, right: 0） |
| **FIXED** | **3 green** |

第三个用例 `test_drop_table_keeps_the_other_tables_buffer`（不误伤其它表）在两个版本
都绿 —— 它是**反向对照**，证明判据不是「一律报错」。

### 5.2 一次必须记录的环境陷阱

跑回归时 `crash_wal_index_b1` 的 `wal_append_crash_recovers_gracefully_without_resurrecting_rows`
红了（`server never reached fault point after_wal_append within 60s`）。

**这不是代码回归**，A/B 证明：

| 步骤 | 结果 |
|---|---|
| 整树回到父提交 `b52a76d8d5` 重跑 | **同样红** |
| 只回退 `file_storage.rs` 一处重跑 | **同样红** |
| 带上本批改动、**单独**重跑（先 `cargo build -p sqlrustgo-mysql-server --features sqlrustgo-mysql-server/fault-injection`） | **绿**（6.28 s） |

根因：**同一个 `target/` 目录里并行跑两套 cargo 任务**，其中
`cargo test -p sqlrustgo-mysql-server` 会**不带 `fault-injection` feature**
重建 `target/debug/sqlrustgo-mysql-server`，把崩溃测试需要的那个二进制
覆盖掉 ⇒ 故障点永远不会被触发。

> **给下一个人的硬规则**：跑崩溃测试之前，必须
> `cargo build -p sqlrustgo-mysql-server --features sqlrustgo-mysql-server/fault-injection`，
> 且**不要**与其它 cargo 任务并行。

---

## 6. 改动清单

| 文件 | 改什么 |
|---|---|
| `crates/tools/src/backup_restore.rs` | 重写：类型化备份内容、真实枚举、三份产物、真恢复 + 校验、开关显式化 |
| `crates/tools/src/lib.rs` | 导出新类型与 `collect_all_tables` / `collect_table` / `render_sql` |
| `crates/tools/Cargo.toml` | 加 `sha2`（payload 校验和；旧的 `md5_simple` 是 32 位非密码学哈希，检测不出损坏，保留仅为 API 兼容并明确标注） |
| `crates/tools/tests/logical_backup_roundtrip_5525.rs` | **新增** 11 个端到端用例（真实 `FileStorage` 落盘 + 重开验证 + 负对照） |
| `crates/tools/tests/{backup_restore,tools_backup_manager,tools_backup_restore,tools_test,unit_tests}.rs` | 按新契约更新 |
| `crates/storage/src/file_storage.rs` | `drop_table_in()` 同时清 `insert_buffer` |
| `crates/storage/tests/drop_table_discards_buffered_rows_5525.rs` | **新增** 3 个回归用例 |
| `crates/mysql-server/src/main.rs` | `backup` / `restore` 子命令加 `--data-dir`（必填）、`--schema-only`、`--drop-first` |
| `crates/mysql-server/tests/cov_v410_cli_subprocess.rs` | 备份/恢复用例重写：真造数据、断言行落库、加空库负对照 |
| `crates/sqlrustgo-cli/src/lib.rs` | 转发 `--data-dir` / `--drop-first` |
| `scripts/backup.sh` | 加 `-D/--data-dir`；放行 `--schema-only` |

---

## 7. 未验证 / 需要属主裁决

1. **本批没有跑 `cargo test --workspace`**（全量 ~30 min，且会覆写
   `docs/releases/v4.1.0/evidence/a04-5226/wire/*.txt`）。跑的是四个受影响
   crate 的全量套件。
2. **`restore` 目前只吃本实现写出的 payload**，不解析任意外部 mysqldump 文件。
   这是有意取舍（文本往返无法无损表达 JSON / `NaN` / `-0.0`），但意味着
   `mysql < dump` 与 `sqlrustgo restore` 是两条路径。
3. **`Value::Json` / `Value::Point` 在 `.sql` 里按文本渲染**（payload 里无损）。
   dump 头部会显式写出「有 N 个 JSON/POINT 值按文本渲染」。是否需要为它们
   引入 JSON 字面量语法，属于产品口径问题。
4. **`CHECK` 约束没有进备份**（`CheckConstraint` 持 AST `Expression`，
   序列化代价高）。`UNIQUE` / `FOREIGN KEY` 已进。
5. **GA 三次阶段跃迁仍需人类审批**；本批不做任何 tag / release。

---

## 8. 与既有 issue 的关系

| issue | 关系 |
|---|---|
| #5525 | 本批关闭对象 |
| #5516 | 脚本侧入口与二进制名 —— 本批在其之上补 `--data-dir` 链路 |
| #5377 | `CREATE INDEX` 库解析缺陷 —— 既有红 `index_update_crash_does_not_serve_dangling_rows` 的根因，**本批只引用不修** |
| #5531 | CI 槽位互杀台账 —— 本批独立分支，不与 #5578 抢同一个 PR |
