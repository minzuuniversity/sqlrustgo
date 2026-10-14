# N5 Independent Root-Cause Report — #5281 / #5285

| 项 | 值 |
|---|---|
| 报告类型 | N5 独立根因审查（**非** Reviewer-A 复核、不引用其结论） |
| 基线 | `01df92a5d1`（冻结 #5283 HEAD） |
| 审查目标 | UPDATE/DELETE 的 MVCC 可见性失效 |
| 测试 base | `crates/storage/tests/mvcc_row_identity_lifecycle_5277.rs`（HEAD 实测 2 PASS / 3 FAIL） |
| 角色约束 | 见 `n5-reviewer-role.md` 硬约束 1–8 |
| 隔离 | 全程在 HEAD（`01df92a5d1`）只读源码 + 已存在测试断言上完成；不修改任何生产文件 |

---

## 一、最终判定

> **`ROOT_CAUSE_CONFIRMED`**（高置信度）

根因：**`MvccStorage` 在 UPDATE/DELETE 路径上未使用行身份作为版本链 Key，而是用
`row[0]`（首列值）当 Key**。这导致：

1. UPDATE 在版本链上向一个「新聚合链」追加新版本，**原链保持原状**，扫描既见到
   旧版本也见到新版本。
2. DELETE 在「新聚合链」上落 tombstone，**原链没有任何 tombstone**，被删除的行
   仍通过原链可见。
3. 即使声明 PK 在非首列，原 INSERT 是按声明 PK 值落链的，UPDATE 改按 `row[0]`，
   链 Key 完全错位；原 PK 链保持原状，新的非 PK 链覆盖不到原行。

---

## 二、调用链与关键 file

### 2.1 完整调用链（SQL → SQL 读）

```
SQL Executor  execute_update / execute_delete          src/engine_dml.rs:945 / :1390
   │
   ▼
StorageEngine trait  update(table, filters, updates)  crates/storage/src/engine.rs:1219
   │                delete(table, filters)            crates/storage/src/engine.rs:1207
   │  ← 签名中没有任何身份参数
   ▼
MvccStorage::update                                   crates/storage/src/mvcc_storage.rs:755-806
   │  row.first().cloned()  ←──── ✱ 根因点 RC1
   ▼
FileStorage::update_at                                crates/storage/src/file_storage.rs:4903-4982
   │  按 filters 列位置匹配（非身份）
   ▼
WAL  WalEntry.key: Option<Vec<u8>>                    crates/storage/src/wal_legacy.rs:83
      字节序列，无类型化身份

DELETE 路径：

MvccStorage::delete_in_db                             crates/storage/src/mvcc_storage.rs:1191-1218
   │  removed_pks 来自 ↓
   ▼
FileStorage::delete_collect_pks_at                    crates/storage/src/file_storage.rs:5036-5180
   │  filters 列位置匹配；line 5103 `filter_map(|row| row.first().cloned())`
   │  ←──── ✱ 根因点 RC2（被包装器吞噬后传染到 MVCC）
   ▼
返回 row.first() 当作「PK」，调用 mvcc.delete(pk)  ←── 落到错误链
```

### 2.2 关键 file:line 表

| 位置 | 角色 |
|---|---|
| `crates/storage/src/mvcc_storage.rs:791` | `pk = row.first().cloned()` — UPDATE 链 Key 错误 |
| `crates/storage/src/mvcc_storage.rs:797` | `filters.contains(&pk)` — PK-集合语义，但 pk 已是错误值 |
| `crates/storage/src/mvcc_storage.rs:800` | `mvcc.put(pk, row, ts, tx_id)` — 把新版本落到错链 |
| `crates/storage/src/mvcc_storage.rs:1192-1194` | `inner.delete_collect_pks_in_db(...)` 拿到的 PK 是首列值 |
| `crates/storage/src/mvcc_storage.rs:1212-1213` | `mvcc.delete(pk, ts, tx_id)` — 把 tombstone 落到错链 |
| `crates/storage/src/file_storage.rs:5095-5104` | `data.rows.iter().filter(match).filter_map(|row| row.first().cloned())` — PK 定义为列 0 |
| `crates/storage/src/file_storage.rs:5071-5074` | `filters.iter().enumerate().all(|(i, f)| row.get(i) == f)` — 列位置匹配 |
| `crates/storage/src/mvcc.rs:133` | `pub fn put(&self, pk: Value, ...)` — `BTreeMap.entry(pk)` |
| `crates/storage/src/mvcc.rs:227` | `pub fn delete(&self, pk: &Value, ...)` — 追加 tombstone 到 `pk` 链 |
| `crates/storage/src/mvcc.rs:383-395` | `find_visible` — 链尾向头走，首个 `visible_from_ts <= snapshot_ts` |
| `crates/storage/src/mvcc_storage.rs:301-322` | `row_identity()` — 无 PK → `__mvcc_row_id__N`，有 PK → PK 列值 |

### 2.3 INSERT 路径的链 Key（对照组）

| 路径 | 链 Key 来源 | file:line |
|---|---|---|
| `MvccStorage::insert_in_db` | `row_identity(row, info)`（无 PK 走计数器、有 PK 走 PK 列） | `mvcc_storage.rs:301-322` |
| `MvccStorage::rebuild_from_inner` | 同上 | `mvcc_storage.rs:1011+` |

**结论**：INSERT/REBUILD 走身份；UPDATE/DELETE 走 `row[0]`。**两者从未在同一语义上。**

---

## 三、最小失败复现

HEAD 上 `cargo test -p sqlrustgo-storage --test mvcc_row_identity_lifecycle_5277 --all-features --no-fail-fast` 实测：

```
test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

| 测试 | 期望 | 实测 | 失败行 |
|---|---|---|---|
| C1 `update_changes_one_row_without_growing_the_table` | 3 行，含 `(1,99)`，原 `(1,10)` 仍存在 | `[[1,40], [1,10], [1,20], [1,40]]` — 4 行，无 `(1,99)`，多一个 `(1,40)` | C1:88-89（行数错） |
| C1b `delete_removes_only_the_matching_row` | 2 行，无 `(1,40)` | `[[1,10], [1,20], [1,40]]` — 3 行，`(1,40)` 仍在 | C1b:115-116 |
| C4 `primary_key_need_not_be_first_column` | 2 行，含 `(1,201)` | `[[1,201], [1,100], [1,200]]` — 3 行，`(1,200)` 未改 | C4:233-234 |
| C2 `rollback_restores_the_previous_state` | 3 行 | ✓ PASS | — |
| C3 `rebuild_keeps_repeated_rows_and_accepts_more` | 3 → 4 行 | ✓ PASS | — |

### 3.1 C1 复现推导

数据：`(1,10), (1,20), (1,40)` 三行，INSERT 后 MVCC 链：

```
__mvcc_row_id__0 → [(1,10)]
__mvcc_row_id__1 → [(1,20)]
__mvcc_row_id__2 → [(1,40)]
```

执行 `UPDATE t SET b=99 WHERE a=1 AND b=20`（filter=`[1,20]`，update=`(1,99)`）：

1. `MvccStorage::update` (:755) 调 `self.inner.update([1,20], (1,99))`
2. `FileStorage::update_at(:4903)` 列位置匹配：col[0]=1 AND col[1]=20 → 只改 `(1,20)`
3. 内层状态：`[(1,10), (1,99), (1,40)]`
4. `self.inner.scan` 返回这 3 行
5. **每个行都取 `row.first() = 1`**（:791）→ `filters.contains(&1) = true` → 都过 `filters.contains` 检查
6. `mvcc.put(1, row, ts, tx_id)` 三次（:800） → 全部 3 行追加到链 `1`

最终 MVCC 状态：

```
1              → [(1,10), (1,99), (1,40)]    ← 新聚合链，全部 3 行被错误归并
__mvcc_row_id__0 → [(1,10)]                  ← 原链未被任何 update 触达，仍可见
__mvcc_row_id__1 → [(1,20)]
__mvcc_row_id__2 → [(1,40)]
```

`mvcc.scan_visible(snapshot)` 取每个链的尾版本：

```
1              → (1, 40)   ← 链尾是 (1, 40)，非 (1, 99)
__mvcc_row_id__0 → (1, 10)
__mvcc_row_id__1 → (1, 20)
__mvcc_row_id__2 → (1, 40)  ← 与链 1 的 (1, 40) 重复
```

`scan_in_tx_db` 在 mvcc 输出 4 行 / inner 3 行时，`inner_rows.len() > out.len()` 为
false → 不合并 inner → 直接返回 4 行。**实测输出与失败信息字字相符**：

```
[[Integer(1), Integer(40)], [Integer(1), Integer(10)],
 [Integer(1), Integer(20)], [Integer(1), Integer(40)]]
```

### 3.2 C1b 复现推导

`DELETE t WHERE a=1 AND b=40`（filter=`[1,40]`）：

1. `MvccStorage::delete_in_db(:1191)` 调 `inner.delete_collect_pks_in_db([1,40])`
2. `FileStorage::delete_collect_pks_at(:5036)` 列位置匹配：col[0]=1 AND col[1]=40 → 命中 `(1,40)`
3. 内层删除该行，返回 `removed_pks = [1]`（行 `first()` 即 col[0]，**line 5103**）
4. `MvccStorage::delete_in_db(:1212-1213)`：`for pk in &removed_pks { mvcc.delete(pk, ts, tx_id); }` → `mvcc.delete(1, ts, tx_id)`
5. **链 `1` 在 MVCC 中不存在**（插入时用的是 `__mvcc_row_id__2`） → `mvcc.delete` 在空链上追加 tombstone，**无可见性副作用**

最终 MVCC 状态：

```
__mvcc_row_id__0 → [(1,10)]                  ← 原链未被任何 delete 触达
__mvcc_row_id__1 → [(1,20)]
__mvcc_row_id__2 → [(1,40)]                  ← 关键：原链无 tombstone
1              → [(1,40) tombstone]          ← 链 1 上无业务行，tombstone 落空
```

扫描输出 3 行。**实测输出 `[[1,10], [1,20], [1,40]]` 与推导完全一致**。

### 3.3 C4 复现推导

表 schema `a INT, b INT PK`，初始 `(1,100), (1,200)`：

```
INSERT 走 row_identity → PK 列 b 在 index=1 → 链 Key = row[1] = 100, 200
100 → [(1,100)]
200 → [(1,200)]
```

`UPDATE t SET b=201 WHERE a=1 AND b=200`：

1. `inner.update([1, 200], (col=1, 201))` → 列位置匹配 → 改 `(1,200)` 为 `(1,201)`
2. 内层：`[(1,100), (1,201)]`
3. `self.inner.scan` 返回 2 行 → 每个 `row.first() = 1` → `filters.contains(&1) = true` → 都 put
4. MVCC：

```
1   → [(1,100), (1,201)]    ← 用错的 row[0]，而非新行 b=201
100  → [(1,100)]             ← 未触达
200  → [(1,200)]             ← 关键：PK 链未被任何更新覆盖
```

扫描输出：`[(1,201), (1,100), (1,200)]` — 3 行。**实测输出字字相符**。

> **关键观察**：C4 的失败揭示出**「PK 是第二列」+「首列值重复」同时发生时，错误形态
> 与 C1 一模一样**。这进一步证明：`MvccStorage::update` 始终在用 `row[0]`，从不查表元
> 数据的 PK 列位置，从不查 `row_identity`。

---

## 四、已证实根因（CONFIRMED）

### RC1 — UPDATE 链 Key 错误（`MvccStorage::update:791`）

**机制**：

- `let Some(pk) = row.first().cloned()` 把列 0 当作链 Key
- INSERT 路径走 `row_identity(row, info)`（`mvcc_storage.rs:301-322`）：
  - 无 PK → `Value::Text("__mvcc_row_id__{n}")` 计数器
  - 有 PK → PK 列对应位置的值（**不是**列 0，除非 PK 恰好在列 0）
- UPDATE 路径取 `row[0]` → 与 INSERT 不在同一语义

**后果**：

- 无 PK 表：所有 UPDATE 后行被错误归并到同一聚合链。聚合链的尾版本是 inner 最后一行
  的快照，**不等于**该行的「UPDATE 后状态」。
- 有 PK 表（PK 不在列 0）：UPDATE 后行被归并到 `row[0]` 值链；原 PK 链保持不变。
- **UPDATE 永远无法正确更新 MVCC 版本**。

**证据**：

- 源码：`crates/storage/src/mvcc_storage.rs:789-801`
- 现有失败用例：C1、C4
- 自洽推演：C1 输出字字相符；C4 输出字字相符

### RC2 — DELETE 链 Key 错误（`MvccStorage::delete_in_db:1212`）

**机制**：

- `inner.delete_collect_pks_in_db` 内部调用 `FileStorage::delete_collect_pks_at(:5036)`
- 后者在 `line 5103` 用 `filter_map(|row| row.first().cloned())` 返回「PK」
- `MvccStorage::delete_in_db` 拿这个首列值当 MVCC 链 Key，调用 `mvcc.delete(pk)`
- INSERT 用的不是首列值 → tombstone 落到错误链

**后果**：

- 内层该行被删，但 MVCC 原链没有任何 tombstone
- 任何后续扫描都会返回该行（来自 MVCC 原链），绕过 inner 的删除结果
- 测试断言的「delete removes only the matching row」字面上成立 — 内层确实只移除了匹配行；
  **MVCC 路径独立地把那个行复活**

**证据**：

- 源码：`crates/storage/src/mvcc_storage.rs:1191-1218`、`file_storage.rs:5036-5180`（特别是 `line 5103`）
- 现有失败用例：C1b
- 自洽推演：C1b 输出字字相符

### RC3 — `scan_in_tx_db` 的非对称 inner-merge（次级放大器）

**机制**：

`scan_in_tx_db` 在合并 inner 与 mvcc 输出时：

```rust
if !self.snapshot_bound(reader_tx) && inner_rows.len() > out.len() {
    // inner 行数 > mvcc 行数时，把 inner 注入 out
}
```

- C1 场景：mvcc=4，inner=3 → 不合并 → mvcc 的 4 行（含重复 `(1,40)`）原样输出
- C1b 场景：mvcc=3，inner=2 → 不合并 → mvcc 的 3 行（含 deleted `(1,40)`）原样输出

**判定**：

- **不是独立根因**，但放大了 RC1/RC2 的症状
- 即便修了 RC1/RC2，只要 inner 与 mvcc 行数差异方向与当前相反，仍需面对 merge 逻辑
- 修复方案 A/B 落地时必须把这条非对称条件同时处理（见第六节）

### RC4 — INSERT 路径为何失效

INSERT 在修复后**仍然正确**：链 Key 来自 `row_identity`（`mvcc_storage.rs:301-322`）。
但 INSERT 路径埋的**身份不变量在 UPDATE/DELETE 上没有任何消费者**。换言之：

- `__mvcc_row_id__N` 只在 INSERT 时被铸造，UPDATE/DELETE 路径既不读取、也不维持它
- C3（`rebuild_keeps_repeated_rows_and_accepts_more`）通过正是这个事实：它只测 INSERT
- C2（`rollback_restores_the_previous_state`）通过是因为回滚只回滚自己未提交的版本，
  未触及已有链

**结论**：RC1/RC2 是唯一主因；RC3 是放大器；RC4 不是 bug，是**已存在但未被利用的机制**。

---

## 五、已排除假设

| 假设 | 排除依据 |
|---|---|
| H-A：`find_visible` 算法错误回退到旧版本 | `find_visible`（`mvcc.rs:383-395`）本身按 `visible_from_ts <= snapshot_ts` 顺序取首项。**算法本身正确**。C1b 的「删除的行仍可见」不是 find_visible 错走旧版本，而是根本没走到 tombstone — tombstone 在错链上。 |
| H-B：`snapshot_ts` / `commit_ts` 时序错乱 | autocommit 路径下 `mvcc.delete` 直接落 `committed=true, visible_from_ts=ts`；事务路径下 `commit_tx_auto` 用 commit_ts 覆盖。扫描端 snapshot_ts 来自 `mvcc.begin_snapshot()`，时序单调。**与本 bug 无关**。 |
| H-C：SQL 扫描路径没走 MVCC 过滤 | `scan_in_tx_db` 实调 `mvcc.scan_visible(snapshot_ts, reader_tx)`（已读源码）。过滤是在的。 |
| D-Q5：UPDATE 走 delete+insert 破坏原子性 | MVCC 层的 UPDATE **不**走 delete+insert；它走「inner update + 内层全表扫描 + 每行 put」。即便在 wrapper 上挂 delete+insert（`engine_dml.rs:1340-1347` 的 WHERE-path），也用了正确的 `pk_idx`（`engine_dml.rs:1335-1339`）。**事务原子性未被 UPDATE 直接破坏**，破坏发生在 MVCC 层的 put 错链。 |
| H-E：rollback 路径异常 | C2 已通过，rollback 路径未触达本 bug。 |
| H-F：rebuild 路径异常 | C3 已通过，rebuild 正确铸造每行身份。 |
| H-G：WAL 是根因 | WAL 是 `key: Option<Vec<u8>>`，但 WAL 层在 `MvccStorage::update` 之前介入；本 bug 触发在 WAL 写入之前，**WAL 不是根因**，是上游修复时要考虑的扩展面。 |

---

## 六、最小修复边界与架构影响

### 6.1 最小修复边界（不属本报告执行范围，仅列出）

| 入口 | 应改为何 |
|---|---|
| `MvccStorage::update:791` | 改用 `row_identity(row, info)` 取链 Key，**不要** `row.first()`。需要 table 元数据 — 在 trait 里 `update` 没有 info 字段（已与 ADR D3 一致指出需新方法）。 |
| `MvccStorage::delete_in_db:1212` | 改用 `row_identity` 路径拿到目标链 Key 后 tombstone，**不要**用 inner 返回的 `row.first()`。 |
| `MvccStorage::update_if:818-823`（同模式） | 同步修正：扫描可见链 + put 用正确 Key，或改为 delete_by_ids + insert_by_ids（取决于方案 A/B）。 |
| `FileStorage::delete_collect_pks_at:5095-5104` | 不需要在此处修 PK 定义；让它返回列 0 即可（语义上是「内层识别」），MVCC 层改用自己的 row_identity 路径。 |
| `scan_in_tx_db` 的非对称 merge | 修复 RC1/RC2 后此条件依然非对称；建议明确 inner 与 mvcc 哪个是权威来源（见 RC3）。 |

### 6.2 架构影响

- **方案 A（最小增量，ADR-5281 推荐 P0-a）**：在 `MvccStorage::update` /
  `delete_in_db` 内改用 `row_identity`，不修改 StorageEngine trait。改动面 2–3 个文件。
  **与本报告根因定位一致**。
- **方案 B（统一 RowId 架构）**：需新增 `RowId` 类型、`scan_identified_*`、
  `update_by_ids_*` / `delete_by_ids_*`，WAL 携带 RowId，恢复先建身份表。改动面
  15–25 个文件。本报告**不**裁决方案选型。
- **共同点**：无论 A/B，都必须在 RC3（merge 非对称）处加测试 — 这是修复后唯一能保证
  「内层与 MVCC 一致」的回归点。

### 6.3 与上游文档的一致性

- E5-5281-AUDIT 已正确预测 RC1（`mvcc_storage.rs:796` 即本报告 `:791`，文档版本漂移一行）
- ADR-5281 D7（UPDATE 保持身份）/ D8（DELETE 定位）已列为 ✗ FAILED，本报告证实之
- E5-A 反例 2（UPDATE 不改身份）/ 4（按 RowId 删除不误删）失败状态由本报告确认

---

## 七、修复前应增加的红灯测试

> 以下测试**应在修复前提交 HEAD**，作为修复方案的「不可回退」断言。N5 **不**提交
> 测试代码；以下为规格说明。

### RT-1：UPDATE 不增长链数

```
seeded([(1,10),(1,20),(1,40)])
UPDATE WHERE a=1 AND b=20 SET b=99
assert mvcc.chain_count == 3     // 修复前：4（多 1 个聚合链）
assert scan.len() == 3
assert scan contains (1, 99)     // 修复前：缺失
```

### RT-2：DELETE 链 tombstone 落到正确链

```
seeded([(1,10),(1,20),(1,40)])
DELETE WHERE a=1 AND b=40
assert mvcc.chain_for_row[(1,40)].last == tombstone  // 修复前：原链无 tombstone
assert scan.len() == 2
assert !scan contains (1, 40)
```

### RT-3：UPDATE 后同值异行不串链（覆盖 RC1 + 行数可计数）

```
seeded([(1,10), (2,10)])        // 两行 col[0] 不同，但 col[1] 都是 10
UPDATE WHERE b=10 SET b=99
assert scan.len() == 2
assert scan contains (1, 99)
assert scan contains (2, 99)
assert !scan contains (1, 10)
assert !scan contains (2, 10)
```

### RT-4：UPDATE PK-on-second-column 命中正确 PK 链（覆盖 C4 + RC1）

```
schema a INT, b INT PK; seeded([(1,100),(1,200)])
UPDATE WHERE b=200 SET b=201
assert scan.len() == 2
assert scan contains (1, 201)
assert !scan contains (1, 200)
```

### RT-5：DELETE 不误删同首列的异行（覆盖 C1b + RC2）

```
seeded([(1,10),(1,20),(1,40)])
DELETE WHERE a=1 AND b=40       // 只删 (1, 40)
assert scan.len() == 2
assert scan contains (1, 10)
assert scan contains (1, 20)
assert !scan contains (1, 40)
```

### RT-6（升级）：UPDATE 改主键列，逻辑行身份不变（方案 B 验收前置）

```
schema a INT, b INT PK; seeded([(1,100),(1,200)])
UPDATE WHERE b=200 SET b=201    // 改了 PK 列
assert scan.len() == 2
assert scan contains (1, 201) and (1, 100)
assert row_with_pk_201 has same RowId as original row_with_pk_200
```

> RT-6 在方案 A 下不可达（PK 改即身份改，违反 ADR E5-A 反例 3），因此 RT-6 是
> 方案 B 的入门门槛，不属 P0-a。

---

## 八、N5 必答问题逐条答复

| # | 问题 | 答复 |
|---|---|---|
| Q1 | MVCC tombstone 与被删除记录是否同 RowId / 版本链？ | **否（RC2）**。tombstone 落在 `row[0]` 值链；被删除记录的实际链是 `row_identity` 铸造的链（无 PK → `__mvcc_row_id__N`，有 PK → PK 列值）。二者**永远不是同一链**。证据：`mvcc_storage.rs:1212`、`file_storage.rs:5103`。 |
| Q2 | `find_visible` 是否错误回退到旧版本？ | **`find_visible` 本身不犯错**（`mvcc.rs:383-395` 顺序取首项的语义正确）。表面上的「旧版本可见」来自 RC2：tombstone 在错链，原链无 tombstone → 原行就是当前链的「首项可见」。**算法无需修；数据流方向需修**。 |
| Q3 | `snapshot_ts` 与删除事务 `commit_ts` 关系？ | **满足预期**（在 RC1/RC2 修复后能满足；当前 bug 下无关）。autocommit 路径 `mvcc.delete` 直接落 `committed=true, visible_from_ts=ts`；事务路径 `commit_tx_auto` 用 commit_ts 覆盖。 |
| Q4 | SQL 扫描路径是否真的调 MVCC 可见性过滤？ | **是**。`MvccStorage::scan_in_tx_db` 调用 `mvcc.scan_visible(snapshot_ts, reader_tx)`。但 inner-merge 条件 `inner_rows.len() > out.len()`（`mvcc_storage.rs:497` 附近）是非对称的，在 RC1/RC2 触发的「mvcc 比 inner 多」场景下不会合并 inner — **这放大了 RC1/RC2 的可见症状**（RC3）。 |
| Q5 | UPDATE 的 delete+insert 是否破坏 RowId 身份或事务原子性？ | MVCC 路径不删+insert；事务原子性未被该路径破坏。Executor WHERE-path（`engine_dml.rs:1340-1347`）用 `pk_idx` 正确取 PK，再 `delete_in_db(&[pk_val])` + `insert_in_db(new_row)` — 这条路径**能**正确传递身份，前提是 `MvccStorage::update` 修复后整体语义连贯。**根因不在 Executor 的 delete+insert**。 |

---

## 九、最终判定与交付

- **判定**：`ROOT_CAUSE_CONFIRMED`
- **置信度**：HIGH
- **证据强度**：源码 + 现有失败用例双线对齐，无需新增实验
- **建议**：交付架构裁决；本报告**不**提交修复
- **修复前红灯测试**：见第七节 RT-1 到 RT-5（规格说明，未实现）
- **下游文档一致性**：与 E5-5281-AUDIT、ADR-5281 E5-A 反例 2 / 4 失败状态一致；
  本报告不复述上述文档结论，仅独立核对源码与测试

— 报告结束 —