# E5-5281 存储身份调用链审计

类型：只读源码审计 ｜ 关联：#5281、#5280、#5277
基线：`3e262692e3` ｜ 测试：`test/5281-rowid-lifecycle` @ `97c3f0e7f8`（2 PASS / 3 FAIL）

本报告只陈述源码证据。**未执行的验证一律标注为未知项**，
不以现有测试结果推断未覆盖的结论。

---

## 一、调用链总览

```
SQL Executor  execute_update / execute_delete
      │  传递的是「列值」——filters: &[Value]、updates: &[(usize, Value)]
      ▼
StorageEngine trait  update(table, filters, updates) / delete(table, filters)
      │  签名中没有任何身份参数
      ▼
MvccStorage::update  (mvcc_storage.rs:796)
      │  以 row.first() 派生版本链 Key
      ▼
FileStorage::update_at  (file_storage.rs:4914)
      │  按 Vec 下标定位，改动后位置移动
      ▼
WAL  WalEntry  (wal_legacy.rs:83)
         key: Option<Vec<u8>> —— 字节序列，无类型化身份
```

**身份在整条链路上都不存在。** 每一层都只看到值。

---

## 二、逐节点证据

### 节点 1：SQL Executor

| 位置 | 函数 | 身份输入/输出 |
|---|---|---|
| `src/engine_dml.rs:944` | `execute_update` | 无。单表路径直接构造 `(filters, updates)` |
| `src/engine_dml.rs:1014-1034` | 无 WHERE 的「简单路径」 | **无身份** |

`:1014` 是最严重的证据：

```rust
// If no WHERE clause, use the simple storage.update() path
let sample_row: Vec<Value> = {
    engine.scan_for_reader_in_db(&*storage, &stmt_db, &table_name)
        .ok()
        .and_then(|rows| rows.first().cloned())      // ← 取第一行当样本
        .unwrap_or_else(|| /* 全 NULL */)
};
```

UPDATE 的目标行由**扫描结果的第一行的内容**代表。这不是身份，
而是「取任意一行，用它的值去匹配所有行」。

- `execute_delete`（`:1389`）走同一形态
- 多表路径 `execute_update_multi_table`（`:1692`）、
  `execute_delete_multi_table` 未在本次审计范围内 —— **未知项**

### 节点 2：StorageEngine Trait

```rust
fn update(&mut self, table: &str, _filters: &[Value],
          _updates: &[(usize, Value)]) -> SqlResult<usize>;   // engine.rs:1219
fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize>;   // engine.rs:1207
```

**签名中不存在身份参数。** 存储层无法知道要改哪一条逻辑行，
只能按值匹配。

`engine.rs` 中 `_filters` 的下划线前缀表明这些参数在默认实现里被忽略。

### 节点 3：MvccStorage

`row.first()` 共 **5 处**：

| 行 | 函数 | 读/写 |
|---|---|---|
| `:504` | `scan_in_tx_db` | 读 |
| `:549` | `scan_with_filter_in` | 读 |
| `:612` | `scan` | 读 |
| `:652` | `scan_with_filter` | 读 |
| **`:796`** | **`update`** | **写 —— 唯一的问题点** |

`:796` ：

```rust
let updated_rows = self.inner.scan(table)?;
for row in updated_rows {
    let Some(pk) = row.first().cloned() else { continue; };
    if !filters.is_empty() && !filters.contains(&pk) { continue; }
    mvcc.put(pk, row, ts, tx_id);
}
```

**问题**：UPDATE 后的行与 filters 比较，取 `row.first()` 作版本链 Key。
无 PK 表 → 首列值相同的多行共用一个 Key；
PK 非首列表 → 用错列，完全对不上声明的主键。

`delete` 路径同样按值定位 —— 对应 `delete_removes_only_the_matching_row` 失败。

### 节点 4：FileStorage

`update_at`（`:4914`）签名 `(db, table, filters: &[Value], updates)`。

**行定位方式**：`TableData.rows: Vec<Record>`，`engine.rs:913`。
行没有身份，只能按**下标**访问。下标不是稳定身份：

- `filter_map` / 压缩会移动行
- 删除后下标不复用语义
- MVCC 不可见时物理行仍在

因此即使 MVCC 记对了身份，物理层也无法按身份定位 —— **这是 P0-a 必须解决的下游阻塞**。

### 节点 5：WAL

`crates/storage/src/wal_legacy.rs:83`：

```rust
pub struct WalEntry {
    pub tx_id: u64,
    pub entry_type: WalEntryType,
    pub table_id: u64,
    pub key: Option<Vec<u8>>,     // ← 字节序列，无类型化身份
    pub data: Option<Vec<u8>>,
    pub lsn: u64,
    pub timestamp: u64,
    ...
}
```

`key` 是 `Vec<u8>` —— **无类型、无作用域、无不可复用保证**。
注释显示 `table_id` 是表名的 31 进制哈希（`WalStorage::table_name_to_id`），
说明当前连「哪个表」都需要额外字段补救。

**重放时如何重建逻辑行身份 —— 未审计，未知项。**

### 节点 6：包装器

`ParallelWalStorage` / `WalStorage` 的 `update` / `delete` 委托路径
**未逐一核查，未知项**。#5280 已确认 `BoxStorageEngine::scan_in_tx_db`
正确转发三个参数，但这不构成对 `update` / `delete` 的证明。

---

## 三、三个失败测试对应的真实路径

| 测试 | Executor | 存储调用 | MVCC | 断点 |
|---|---|---|---|---|
| `update_changes_one_row_without_growing_the_table` | `:1014` 样本行路径 | `update_at(filters=[1,20])` | `:796` `row.first()`=1 → 与 filters 不匹配 → **`continue` 跳过**，该行未更新；同时新值行被 `put` 到另一个 Key → 表变 4 行 | `mvcc_storage.rs:796` |
| `delete_removes_only_the_matching_row` | `execute_delete` | `delete(filters=[1,40])` | 按首列定位，`[1,10],[1,20],[1,40]` 首列同为 1 → 无法区分 | `mvcc_storage.rs` delete 路径 |
| `primary_key_need_not_be_first_column` | `:1014` | `update_at` | `:796` `row.first()`=**a 列的值**（1），而声明 PK 是 b 列（200）→ Key 错误 | `mvcc_storage.rs:796` |

**第三个测试最能说明问题**：有显式主键时同样错误，
因为 `MvccStorage::update` **从不查表元数据**，也**不查主键列位置**。

---

## 四、必须改变 / 可兼容保留的接口

| 接口 | 裁决 |
|---|---|
| `StorageEngine::update` / `delete` | **必须改变** —— 新增身份参数或新增身份化方法 |
| `StorageEngine::scan*` | **可兼容保留** —— 旧签名作为适配层，新增 `scan_identified_*` |
| `ExecutionEngine::execute_update` 的无 WHERE 路径 | **必须改变** —— 样本行机制无法定位目标 |
| `MvccStorage::update` / `delete` | **必须改变** —— 改用身份，而非 `row.first()` |
| `FileStorage::update_at` | **必须改变** —— 需支持按身份定位 |
| `TableData.rows: Vec<Record>` | **必须改变** —— 需承载身份 |
| `WalEntry.key: Vec<u8>` | **需重新定义** —— 类型化、带作用域、保证不复用 |
| `heap::RowId` | **不可复用** —— 物理页槽语义，全仓零引用 |

---

## 五、RowId 所有权边界（初步）

| 层 | 角色 |
|---|---|
| `FileStorage` | **权威所有** —— 分配、持久化、恢复 |
| `MvccStorage` | **消费** —— 不分配、不猜测 |
| `ExecutionEngine` | **不持有** —— 传递 |
| WAL | **记录** —— 需类型化身份 |

**与 ADR D1 的方向一致**，但本审计新增一条约束：

> `FileStorage` 现有 `rows: Vec<Record>` 使物理层无法按身份定位。
> 身份必须**下沉到行的存储结构本身**，而非旁挂映射 ——
> 否则 MVCC 与物理层会持有两份可能漂移的身份。

---

## 六、兼容性与风险

| 项 | 风险 |
|---|---|
| 旧 `t.json` | `rows` 不含身份，**但物理行完整**（#5277 E2 已证），可按顺序重建 |
| WAL | `key` 无类型；旧 WAL 重放后身份从哪来 —— **未验证** |
| 并发 | 分配器需在写锁内，且高水位需持久化 —— 现有 `FileStorage` 无此设施 |
| 已损坏数据 | 若多条逻辑行已合并进同一版本链，`FileStorage` 物理行仍完整，**可重建**；仅存于 MVCC 未落盘的行本就无持久性 |

**未知项（需 Reviewer 或后续验证）**

- `execute_update_multi_table` / `execute_delete_multi_table` 的目标确定方式
- WAL 重放如何重建身份
- `ParallelWalStorage` / `WalStorage` 对 `update` / `delete` 的委托是否保真
- 并发 UPDATE 同一行时的冲突检测
- `rebuild_from_inner` 在身份持久化后的行为

---

## 七、对 ADR D1–D12 的证据状态

| ID | 状态 | 说明 |
|---|---|---|
| D1 所有权 | **部分证据** | FileStorage 无行身份设施，需新建；方向与 ADR 一致 |
| D2 身份与值同存 | **支持** | 节点 4 证明必须下沉到行结构 |
| D3 独立类型 | **支持** | `heap::RowId` 不可复用；`Value::Text` 前缀不构成隔离 |
| D4 PK 与 RowId | **缺口** | `MvccStorage::update` 从不查主键列位置 |
| D5 无 PK 重复值 | **已验证** | 测试 1、3 |
| D6 身份传递 | **阻断** | Trait 签名无身份参数 |
| D7 UPDATE 保持身份 | **失败** | 测试 1、3 |
| D8 DELETE 定位 | **失败** | 测试 2 |
| D9 版本链 | **已验证** | `VersionedTable.versions: BTreeMap<Value, …>` |
| D10 WAL 身份 | **缺口** | `key: Vec<u8>` 无类型 |
| D11 重启恢复 | **未知** | 未审计 Recovery |
| D12 旧数据迁移 | **部分证据** | 物理行完整可重建；WAL 侧未知 |

---

## 八、结论

身份在**整条调用链上都不存在** —— 从 Executor 到 WAL，每一层只有值。

这解释了为什么三个失败用例横跨「无 PK」与「PK 非首列」两个域：
**不是身份实现有 bug，而是身份从未被实现。**

P0-a 的实际起点不是「让 `MvccStorage` 用对 Key」，而是
**从 `StorageEngine` trait 开始建立身份通道**，并在 `FileStorage` 的行存储中落地。
