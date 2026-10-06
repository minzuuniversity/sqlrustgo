# #5025 收尾：命名数据库重启后数据不可见（新增回归）

- **日期**：2026-10-06
- **Issue**：#5025
- **分支**：`fix/5060-5059-storage-write-paths`（worktree `~/workspace/dev/sqlrustgo-worktrees/wt-5055`）
- **基线**：`gitea252/develop/v4.1.0` = `32e0c46b6e`

---

## 1. 发现方式

修 #5059/#5060 跑全量 storage 测试时，`storage_e2e_test::test_e2e_index_survives_restart`
是基线上就存在的失败。追下去发现 `load_all_indexes` 与 `load_all_tables` 存的是**裸表名**，
而 `has_index`/`get_index` 用 `self.tbl()` 的**作用域键**查询 —— 键根本不匹配。

顺带把范围查清楚后，发现问题比索引严重得多。

---

## 2. 探针实测（修复前）

探针：`crates/storage/tests/probe_5025_restart_scoped.rs`

```
S1 has_index(t,id) = true
S1 rows = Ok(1)
--- disk ---
shop
  t.json
  t_idx_id.json
S2 get_table(t).is_some() = false      ← 数据在磁盘上，缓存里没有
S2 contains_table(t)      = false
S2 scan rows              = Ok(0)      ← 静默：一张有 1 行的表读出 0 行
S2 has_index(t,id)        = false
```

`S1` 能读到 1 行，`S2` 读到 0 行，而 `data/shop/t.json` 一直在磁盘上。**这是一次静默的数据不可见。**

---

## 3. 根因

#5025 把每个库的文件移到 `data/<db>/`，并把内存缓存键改成作用域键
（`scoped_key(db, table)`）。但它只改了**写侧**和**读helper**，两个启动 loader 没跟上：

| 位置 | #5025 改造后 | 应有行为 |
|---|---|---|
| `load_all_tables` | 只遍历 `data_dir` 根目录 | 遍历每个库目录 |
| `load_all_indexes` | 只遍历 `data_dir` 根目录 | 遍历每个库目录 |
| `load_all_indexes` 存入的键 | `(table, column)` 裸元组 | `(scoped_key(db, table), column)` |
| `has_index` / `get_index` 查询的键 | `(self.tbl(table), column)` | 同上 |

于是三个失败同时成立：

1. 命名库里的表写进了 `data/<db>/t.json`，下次打开**根本不被读**（`scan` 返回 0 行）；
2. 命名库里的索引文件同样不被读；
3. 连**默认库**的索引也读不回来 —— 它被存到裸键下，而 `has_index` 查的是作用域键。

第 3 条正是既存失败 `test_e2e_index_survives_restart` 一直在报的东西；第 1、2 条此前**没有任何测试覆盖**。

对照：`load_all_tables` 在 #5025 里是改过的（键加了 `self.tbl(&name)`），只是没扩展到子目录；
`load_all_indexes` 连键都没改。属于改造做了一半。

---

## 4. 修复

`crates/storage/src/file_storage.rs`

新增「库名显式」的一组原语，让启动 loader 能逐库读取，而**不必**去翻转
`current_db`（那是进程级设置，其它连接也会读）：

- `db_dir_for(db)` / `current_db_name()` —— `db_dir()` 的具名版本；`current_db_name`
  克隆出 `String` 再传给 `*_in`，避免在 parking_lot 读锁上嵌套读锁（有写者排队时会死锁）。
- `table_path_in` / `index_path_in` / `delta_path_in` —— 对应 `*_path` 的具名版本。
- `load_table_in` / `load_table_delta_in` / `load_index_in` —— 对应 `load_*` 的具名版本。
- `db_dirs()` —— 列出磁盘上所有库：隐式默认库（目录就是 `data_dir` 本身）+ 每个子目录。
  子目录只可能由 `create_database` 创建。

两个 loader 改为遍历 `db_dirs()`，并用 `scoped_key(&db, table)` 存键。

顺带：`load_all_tables` 现在显式跳过含 `_idx_` 的文件名，不再靠「解析失败」隐式排除索引文件。

---

## 5. 测试

新增：`crates/storage/tests/index_and_table_survive_restart_5025.rs`（5 项，全绿）

| 用例 | 钉住什么 |
|---|---|
| `table_in_a_named_database_survives_restart` | 命名库的表重启后可见、行数正确；先断言 `data/shop/t.json` 确实在磁盘上，防止「因错误的原因通过」 |
| `index_in_a_named_database_survives_restart` | 命名库的索引重启后可见 |
| `index_in_the_default_database_survives_restart` | 默认库的索引重启后可见（即既存失败的根因） |
| `same_table_name_in_two_databases_stays_separate_across_restart` | 两个库各有同名表 `shared`，重启后仍各自只看到自己那 1 行（11 / 22） |
| `index_files_are_not_mistaken_for_tables` | 索引文件不会被当成表列出 —— **但见下，此用例未钉住任何东西** |

修复后既存的 `storage_e2e_test::test_e2e_index_survives_restart` 由 FAILED 转 PASS。

---

## 6. 变异验证

测试文件 `index_and_table_survive_restart_5025.rs` + 既存 `storage_e2e_test`。

| 变异 | 做法 | 结果 | 判定 |
|---|---|---|---|
| **M1** | `load_all_indexes` 存回裸键 `(table, column)`（#5025 之前的行为） | 5 项中 3 项 FAILED：`index_in_the_default_database_survives_restart`、`index_in_a_named_database_survives_restart`、`same_table_name_in_two_databases_stays_separate_across_restart`；同时既存 `storage_e2e_test::test_e2e_index_survives_restart` 复现 FAILED | **CAUGHT** |
| **M2** | `load_all_tables` 只遍历 `data_dir` 根目录（不扫子目录） | 5 项中 2 项 FAILED：`table_in_a_named_database_survives_restart`、`same_table_name_in_two_databases_stays_separate_across_restart` | **CAUGHT** |
| **M3** | 删除 `load_all_tables` 里的 `_idx_` 跳过 | 5 项全 PASS | **NOT CAUGHT（无效变异）** |

### M3 为什么无效

`_idx_` 那个跳过**不是**承重结构。索引文件是序列化的 `BPlusTree`，拿去按
`StoredTableData` 解析必然失败，外层 `if let Ok(..)` 早把它吞掉了 —— 跳过与否行为一致。

处置：**保留**该跳过（它把意图写明白，而不是依赖一次 serde 不匹配），但在源码注释和
测试 doc comment 里**明写它没有被任何变异钉住**，不拿它当证据。真正钉住本次修复的是前 4 个用例。

（这正是 `ANTI_FABRICATION_POLICY.md` 要求的：无效变异要显式标注，不能拿「5/5 通过」
冒充覆盖度。）

---

## 7. 仍未闭合的 #5025 部分（本 PR 不含）

| 缺口 | 状态 | 说明 |
|---|---|---|
| `USE db` 从未到达 storage | **未修** | executor 层没有任何 `current_db` 处理；`Statement::UseDatabase` 被解析但无人执行。故 `SELECT DATABASE()` 恒为 `default`（issue 评论 id=180641 已实测记录）。 |
| per-command TOCTOU 窗口 | **未修** | `probe_5025_conn_db_race.rs` 演示了窗口结构；上一轮实测已通过。 |
| 启动时全库加载的内存代价 | **已知的取舍** | 修复后启动会把所有库的表读进内存（此前只读根目录）。仓库里没有按库惰性加载的基础设施，全量加载是与既有行为一致的选择；若日后需要，应另做惰性加载，不在本 PR 范围。 |

因此本 PR 只闭合 #5025 的「数据在重启后不可见」这一条；#5025 保持 open。
