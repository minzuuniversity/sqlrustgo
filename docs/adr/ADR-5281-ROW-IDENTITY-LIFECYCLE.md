# ADR-5281：MVCC 行身份生命周期

状态：草案，待审核
关联：#5281（P0）、#5280（MERGE HOLD）、#5277

## 背景

#5277 的根因是 `VersionedTable` 以**单列 `Value`** 作版本链 Key，
而写入路径传的是 `row[0].clone()`。无主键表中三行共享首列值时，
它们被解释为同一条记录的三个版本，扫描只返回最新的一个。

#5280 为无主键表引入了隐藏身份，修好了 INSERT / SELECT 路径。
E4-C 随后证明该身份**没有形成契约**：UPDATE 与 DELETE 无法定位它。

实测（`crates/storage/tests/mvcc_row_identity_lifecycle_5277.rs`）：

```
数据 (1,10),(1,20),(1,40)
UPDATE t SET b=99 WHERE a=1 AND b=20
  → 4 行：[[1,40], [1,10], [1,20], [1,40]]
```

`mvcc_storage.rs:796` 用 `row.first()` 作 Key，对无主键表生成新身份，
版本链读作新记录。

## 源码契约审计（E5 前置）

在提出新接口之前，先确认现有代码具备什么。

| 模块 | 审计结果 |
|---|---|
| `heap.rs:22` | **已有 `pub struct RowId { page_id: u32, slot: u16 }`** —— 但它是**物理页槽标识**，且**全仓无人使用**（`grep` 除定义外零引用） |
| `TableData`（`engine.rs:913`） | `rows: Vec<Record>` —— **行只有值，没有身份** |
| `FileStorage` | `insert_buffer: HashMap<String, Vec<Record>>`；行按 `Vec` 下标定位，**下标不是稳定身份**（`filter_map`、压缩、删除都会移动） |
| `StorageEngine::update` | `(table, filters: &[Value], updates: &[(usize, Value)])` —— **目标是谓词值，不是身份** |
| `StorageEngine::delete` | `(table, filters: &[Value])` —— 同上 |
| `VersionedTable` | `versions: RwLock<BTreeMap<Value, Vec<VersionedRow>>>`；Key 是 `Value` |
| `TransactionManager` | `commit(&mut self) -> Result<Option<u64>, _>`；**不绑定修改目标** |
| `MvccStorage::update` | 读 inner 全表后按 `filters` 匹配 `row.first()` 重投 —— 这就是 #5277 的 UPDATE 侧表现 |

**结论：现有代码没有可复用的稳定行身份。** `heap::RowId` 是物理位置语义，
与逻辑行身份不是一回事，不能直接复用（但其命名需要在方案中避让）。

#5280 引入的 `Value::Text("__mvcc_row_id__{n}")` 只是字符串前缀，
**不构成类型级隔离** —— 用户可以插入同值文本。本 ADR 不接受该做法。

## D1：RowId 所有权

**决策：权威 RowId 由 `FileStorage` 持有并持久化；`MvccStorage` 只消费，不生成。**

理由：

- `FileStorage` 是唯一同时掌握物理写入与恢复的层。身份必须与数据同生共死，
  否则恢复时无法重建。
- MVCC 生成身份会造成**恢复后身份漂移**：进程重启后计数器归零，
  新分配的身份会与恢复出来的身份冲突。
- 身份与事务无关，因此放在事务层之上、存储层之内最简单。

唯一性保证：

- 计数器在**数据目录内持久化**（单调序号文件或表级元数据）
- 恢复时先读到已持久化的最大序号，再继续分配 —— **绝不复用**
- 分配在写锁内完成，并发写不会取到相同序号

组合方式：`RowId { table_epoch: u32, seq: u64 }` 或全局单调 `u64` + 表名。
**不使用 `u128` 物理页槽形式**（那是 `heap::RowId` 的语义，且物理页会因分裂移动）。

## D2：扫描接口

**决策：新增带身份的扫描，不改变既有 `scan*` 的签名。**

```rust
pub struct RowId { /* opaque, 表内唯一且不跨恢复复用 */ }
pub struct IdentifiedRow { pub row_id: RowId, pub record: Record }
```

新增 trait 方法（概念签名，未核对最终形态）：

```rust
fn scan_identified_in_tx_db(&self, db: &str, table: &str, reader_tx: u64)
    -> SqlResult<Vec<IdentifiedRow>>;
```

WHERE / JOIN 的传递方式：

- 谓词求值在**内存中的 `IdentifiedRow` 列表**上进行，产出命中的 `RowId` 集合
- Executor 不再把「命中的行」交给存储，而是把 `RowId` 集合交给存储
- JOIN 需要两侧的身份，但**两侧身份属于不同表**，各自独立即可

快照一致性：`scan_identified_in_tx_db` 与既有 `scan_in_tx_db` 取同一
`reader_tx` 快照，两者在同一查询内必须一致 —— 这一点需要测试固定。

## D3：修改接口

**决策：新增按身份修改的方法，既有 `update`/`delete` 保持不变。**

```rust
fn update_by_ids_in_db(&mut self, db, table, ids: &[RowId], updates: &[(usize, Value)], tx) -> SqlResult<usize>;
fn delete_by_ids_in_db(&mut self, db, table, ids: &[RowId], tx) -> SqlResult<usize>;
```

语义要求：

- 原子：要么全部命中身份，要么一个不动
- 冲突检测：身份不存在（已被其他事务删除）→ 明确报错，不静默成功
- 墓碑走同一身份：`delete_by_ids` 在 `VersionedTable` 上对给定身份写 tombstone，
  **不新增 Key**

**为什么保留旧的 `update`/`delete`**：它们是「按值匹配」的 SQL 语义入口，
`UPDATE ... WHERE a=1` 本身就要匹配所有满足谓词的行。身份路径是 Executor
在谓词求值之后**追加**的一层，两者并存不冲突。

## D4：持久化协议

WAL 与恢复必须保存身份与版本关系：

| 阶段 | 要求 |
|---|---|
| WAL 记录 | 携带 `RowId`，不能只记录行值 |
| COMMIT | 身份与版本的绑定落盘 |
| ROLLBACK | 身份不被回收，可继续复用序号空间但不复用该 RowId |
| 恢复 | 先重建 `FileStorage` 的身份表，再重建 MVCC 链；**新分配的 RowId 不得与恢复出的冲突** |

**已损坏数据**：现有版本可能已把多条逻辑行写进同一条版本链。
`FileStorage` 中的物理行仍然完整（E2 已证磁盘有全部 6 行），
因此**可通过重建恢复**：`rebuild_from_inner` 按物理行重新分配身份，
每行一个。**无法恢复的是仅存在于 MVCC 链、未落盘的行** ——
这类数据在当前架构下本就不持久，需在迁移中明确丢弃并记录。

## E5-A：设计必须通过的反例

| # | 反例 | 当前状态 |
|---|---|---|
| 1 | 两条内容完全相同的记录仍有不同 RowId | ✅ #5280 已满足（计数器） |
| 2 | UPDATE 修改全部业务列，RowId 不变 | ❌ FAILED |
| 3 | PRIMARY KEY 被修改，逻辑行身份不变 | ⚠️ 当前 PK 值**就是**身份，改 PK 即改身份 —— 违反不变量 |
| 4 | 按 RowId 删除不误删同值的另一条 | ❌ FAILED |
| 5 | 重启恢复与回滚后身份不重用、不漂移 | ⚠️ #5280 的计数器**不持久化**，重启后从 0 重新分配 |

**反例 3 与 5 是当前设计尚未满足的**，且都不在 #5280 的验证范围内。
反例 3 意味着即便修好 UPDATE/DELETE，「以主键值为身份」本身仍与
「身份稳定」冲突 —— 这支持统一 RowId 架构（方案 B）。

## 方案 A：最小增量（仅修无主键表生命周期）

**内容**

1. `FileStorage` 持久化身份计数器与「行 → RowId」映射
2. 新增 `scan_identified_*` 与 `update_by_ids_*` / `delete_by_ids_*`
3. `MvccStorage::update` / `delete` 改走身份路径
4. `rebuild_from_inner` 为每条物理行分配独立身份（顺带修复恢复时的合并）
5. 保留「PK 值即身份」的现状

**优点**：改动面小；能修复 #5281 的两个失败用例；不动 WAL 格式
**风险**：
- 反例 3 未解决 —— 改主键会改身份
- 有 PK 与无 PK 表走两套身份语义，长期分叉
- `FileStorage` 现有 `Vec<Record>` 结构需要并行维护身份映射，
  每次压缩/过滤都要同步，是新的不一致来源

**工作量**：中。约 6–8 个文件，MVCC + FileStorage + Executor 三层。

## 方案 B：统一 RowId 架构

**内容**

1. 引入独立 `RowId` 类型（非 `Value`），由 `FileStorage` 持久化分配
2. 所有扫描路径返回 `IdentifiedRow`；`Record`-only 接口作为适配层保留
3. UPDATE / DELETE / UPSERT 一律按 RowId
4. **主键降级为「SQL 值 → RowId」的唯一索引**，不再是身份本身
5. WAL 记录携带 RowId；恢复先建身份表
6. `heap::RowId` 重命名或明确区分（物理页槽 vs 逻辑行身份）

**优点**：
- 满足全部五个反例，含反例 3
- 列名 `d1`/`d2`、值重复、PK 更新都不再影响身份
- 身份契约单一，未来新增索引/优化器路径不再需要重新推导
- 直接落实本次确立的不变量

**风险与成本**：
- **WAL 格式变更** —— 已持久化的 WAL 记录不含 RowId，需要版本号或拒绝加载旧 WAL
- **数据迁移** —— 现有 `t.json` 的 `rows: Vec<Record>` 不含身份，
  需在加载时重建（旧文件可从物理行重建，不丢数据）
- **影响面大** —— Storage trait、Executor、MVCC、WAL、恢复全链路
- 事务协调器（`TransactionManager::commit` 目前不绑定修改目标）需要能表达「本次修改了哪些身份」

**工作量**：大。估计 15–25 个文件，分多个 PR。

## 推荐

**长期目标采用方案 B，分阶段落地；第一阶段执行方案 A 的第 1、4 步。**

理由：

- 反例 3 无法用方案 A 解决。以 PK 值作身份意味着 `UPDATE t SET id = 9`
  会换身份 —— 而 SQL 允许更新主键。这不是边界情况，是标准 SQL。
- 「有 PK 用 PK、无 PK 用隐藏身份」的双轨制会让下一个接触这段代码的人
  再次面对「什么时候能用列值当身份」这个已经出过三次的问题。
- 但方案 B 的全量改造不适合在 P0 修复窗口内完成。

**建议分期**

| 阶段 | 内容 | 对应门禁 |
|---|---|---|
| P0-a | 方案 A 第 1、4 步：身份持久化 + 恢复重建 | #5281 的 UPDATE/DELETE 用例 + 重启用例 |
| P0-b | 方案 A 第 2、3 步：身份化扫描与修改接口 | 反例 2、4 |
| P1 | 方案 B 第 4 步：主键降级为索引 | 反例 3 |
| P1 | 方案 B 第 5、6 步：WAL 与事务携带身份 | 崩溃恢复验收 |

在 P1 完成前，**必须**在代码与文档中明确标注：
「当前主键值即行身份，更新主键会改变身份」是**已知限制**，
不得被后续改动当作既定契约依赖。

## 不做的事

以下方案已评估并否决：

- **按记录内容反查身份** —— `(1,99)` 不含 `RowId=102`；重复记录下不唯一
- **`row[0]` 推断身份** —— #5277 的原始缺陷
- **扫描结果按位置绑定身份** —— `filter_map`/压缩会移动行
- **复用 `heap::RowId`** —— 物理页槽语义，页分裂后不稳定，且当前零引用
- **`Value::Text` 前缀作为隐藏身份** —— 无类型级隔离，用户可插入同值

## 验收

本 ADR 的通过标准不是编译成功，而是能从接口契约上证明：

- 任何合法 `UPDATE` 都不会改变目标行的身份
- 任何合法 `DELETE` 都不会误删身份不同的同值行
- `COMMIT` / `ROLLBACK` / 恢复后身份不重用、不漂移
- 上述性质由**接口签名与测试**保证，而非依赖调用方自觉
