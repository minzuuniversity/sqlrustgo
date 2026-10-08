docs(#5057): 其余 4 个存储引擎不参与多库路径 —— 无需补 `*_in_db`

`FileStorage` 在 #5127 达到 14/14 后，`StorageEngine` 的 8 个实现中还有 6 个
的 `*_in_db` 全部走 trait 默认（共 98 处）。本 PR 判定其中 4 个**不参与多库
路径**，无需照搬覆写；另 2 个（`BoxStorageEngine` / `ScanCountingStorage`）是
包装层与测试替身，`db` 对它们无意义。

纯文档，无代码变更。

## 判据一：这些引擎没有库隔离的概念

| 引擎 | `scoped_key` | `new_database` | 表键构造 |
|---|---|---|---|
| `AppendOnlyStorage` | 0 | 0 | `self.tables.get(table)` |
| `ColumnarStorage` | 0 | 0 | `self.tables.get(table)` |
| `BinaryTableStorageV2` | 0 | 0 | 无表键 —— `scan` 是 `Ok(vec![])` 桩 |
| `BinaryTableStorage` | 0 | 0 | `current_db` 仅出现在 `BoxStorageEngine` 的转发 |

`AppendOnlyStorage` 与 `ColumnarStorage` 的表**直接用裸表名做键**。`MemoryStorage`
与 `FileStorage` 的 `#5025` 改造把它们统一成了 `scoped_key(db, table)` 形式。
这两个引擎没有做这项改造 —— 它们与库隔离是两个正交的设计轴，补 98 处覆写
等于给一个单库引擎硬塞一个它不使用的参数。

`BinaryTableStorageV2` 情况更进一步：其 `StorageEngine` 实现（134 行，30 个方法）
中 15 个的形参带 `_` 前缀且直接返回常量 —— `scan` 返回 `Ok(vec![])`、
`delete` / `delete_if` 返回 `Ok(0)`。半数方法未接上存储逻辑，给它补库隔离
没有意义。

## 判据二：没有生产路径使用它们

全部 4 个引擎的构造点**只出现在各自的单元测试内**：

```
AppendOnlyStorage::new       → crates/storage/src/append_only_storage.rs:537   (单测)
ColumnarStorage::new         → crates/storage/src/columnar/storage.rs:1013    (单测)
BinaryTableStorageV2::new   → crates/storage/tests/binary_v2_direct_v3_12.rs:14
BinaryTableStorage::new     → crates/storage/tests/binary_storage_direct_v3_12.rs:46
```

## 判据三：唯一的引擎工厂点不产出它们

`src/engine_builder.rs` 是全仓唯一的引擎构造点，9 个公开构造器
（`with_memory` / `with_memory_and_cbo` / `with_wal` / `with_wal_file` /
`with_wal_recovery` …）只产出两种：

```rust
MemoryStorage::new()                        // with_memory 等 5 个
FileStorage::new_with_wal(data_dir)         // with_wal / with_wal_file / with_wal_and_checkpoint
```

`engine_builder.rs` 对 `AppendOnly` / `Columnar` / `Binary` 的引用数为 **0**。
仓库内也不存在引擎枚举或 `storage_factory` 之类的分派层。

**`MemoryStorage` 与 `FileStorage` 就是全部可达的存储后端** —— 前者是 CLI
默认与 `--memory` 模式，后者是落盘模式。二者均已 14/14。

## 因此不做的事

不改这 6 个引擎的 `*_in_db`。若将来某个引擎要接入多库路径，正确的顺序是：

1. 先做 `scoped_key` 式的表键改造（`#5025` 做的事）
2. 再谈 `*_in_db` 覆写

顺序颠倒会得到一堆把 `db` 参数原样丢弃的桩方法 —— 那正是 `MemoryStorage`
在 #5081 之后的状态：14 个方法一个没实现，全部退化为
`{ let _ = db; self.op(table, ..) }`。

## 附：本 PR 顺带记录的两个盘点坑

排查过程中我自己踩了两个坑，都会导致盘点结论错误，值得留在案：

1. **HEAD 停在旧提交**：首次盘点时读到 `MemoryStorage` 0/14，与刚完成的
   14/14 直接矛盾 —— 实际跑在 `eafce9729a`（#5057 第一步）上。盘点任何覆盖
   情况前先确认 HEAD。
2. **块定位被注释行劫持**：`engine.rs:1970` 的注释里字面写着
   `` // `impl StorageEngine for MemoryStorage` cannot carry extra ``，子串
   匹配命中该行，把 2000+ 行的 inherent 块当成 trait impl，同样读出 0/14。
   须取**最后一个** `strip()` 后以 `impl` 开头的匹配。

## 验证

无代码变更。仅文档与盘点结论。

Refs: #5057, #5025, #5127