# #4948 — COMMIT 不再深拷贝全库：`tables` / `committed_tables` Arc 化

- **Date**: 2026-10-07
- **Branch**: `feat/4948-committed-tables-arc`（PR #5071）+ `feat/4948-memory-curve`（PR #5074）
- **Base**: `gitea252/develop/v4.1.0` = `75683f1cc6`（PR #5070 合并后）
- **Issue**: #4948（B2.5 / #4915 F-11）

---

## 1. 起点：先量，再决定

issue 的 Task 第 1 条要求「先决定 `tables` 是否也 Arc 化」。这是个设计决定，先拿数据。

新增测量夹具 `crates/storage/tests/measure_commit_clone_4948.rs`（**`#[ignore]`**，手动跑）：

```text
$ cargo test -p sqlrustgo-storage --all-features \
    --test measure_commit_clone_4948 -- --ignored --nocapture

tables   rows/table   total rows   commit ms
------------------------------------------------
1        1000         1000         0.103
10       1000         10000        1.428
10       10000        100000       9.624
50       10000        500000       49.127
```

测量的是**空事务**的 COMMIT：`begin_transaction()` 之后立刻 `commit_transaction_with_log()`，不做任何写入。因此这段时间完全是 `self.committed_tables = self.tables.clone()` 的开销。

约 **0.1 ms / 千行**，严格线性。50 万行的库，**每次 COMMIT（哪怕是空事务）都要 49 ms**。验收标准第 1 条描述的问题属实。

做成 `#[ignore]` 而不是普通断言：`crates/storage/src/wal_legacy.rs:1473` 的
`test_wal_perf_1000_insert` 硬编码 `<2s` 墙钟，PR #5070 验证时实测 4 跑挂 1。
同样的错误不在这里重犯。

## 2. 事实更正一：`tables` 不是 `HashMap<String, TableData>`

issue 原文：

> `tables: HashMap<String, TableData>`，其中 `TableData` 内含 `rows: Vec<Record>`

实际 `crates/storage/src/engine.rs:1678`：

```rust
tables: HashMap<String, Vec<Record>>,
committed_tables: HashMap<String, Vec<Record>>,   // :1721
```

`TableData { info, rows }` 是另一个结构（`engine.rs:913`）。**这让 Arc 化比 issue
预估的简单一层** —— 没有 `TableData` 包装要处理。

issue References 写的 `engine.rs:1478 — committed_tables 字段定义` 也是过期行号，
实际在 **1721**。

## 3. 事实更正二：这不是 breaking change，`SchemaSnapshot` 无需改动

issue 把「内部字段 Arc 化」和「公共类型变 breaking」当成同一件事：

> 其字段 `tables: HashMap<String, Vec<Record>>` 是公共 API 的一部分。Arc 化会
> 改变公共类型签名，需要单独的版本或兼容层

两者是**不同结构**。`SchemaSnapshot` 由 `snapshot_schema()` 产出，而它本来就在做
一次全量深拷贝（`engine.rs:2000-2009`）：

```rust
pub fn snapshot_schema(&self) -> SchemaSnapshot {
    SchemaSnapshot {
        tables: self.committed_tables.clone(),   // <-- 这里物化成 owned
        ...
    }
}
```

只要在**这个边界**物化，公共类型一个字段都不用动：

```rust
tables: self.committed_tables.iter()
    .map(|(k, v)| (k.clone(), (**v).clone()))
    .collect(),
```

**验收标准第 3 条「若为 breaking change，CHANGELOG 与迁移说明齐备」不触发** ——
公共 API 不变，不需要 CHANGELOG / 迁移说明。

`snapshot_schema()` 仍是 O(总行数)，但它只在**新建具名连接**时调用（见其 doc
comment），不在 COMMIT 热路径上，不违反验收标准第 1 条。

## 4. 采纳的设计

| 字段 | 之前 | 之后 |
|---|---|---|
| `tables` | `HashMap<String, Vec<Record>>` | `HashMap<String, Arc<Vec<Record>>>` |
| `committed_tables` | `HashMap<String, Vec<Record>>` | `HashMap<String, Arc<Vec<Record>>>` |
| `SchemaSnapshot.tables` | `HashMap<String, Vec<Record>>` | **不变** |

`self.committed_tables = self.tables.clone()` 退化为 HashMap 浅拷贝 —— O(表数) 而非
O(行数)。

### 代价（不是白赚的）

Arc 引入后，提交后**首次写入**某张表会经 `Arc::make_mut` 触发该表行的一次拷贝 ——
这条开销今天不存在。

| 场景 | 之前 | 之后 |
|---|---|---|
| 事务只改 100 张表中的 1 张 | 全库拷贝 | 仅拷贝 1 张表 |
| 事务改遍所有表 | 全库拷贝 | 全库拷贝（**持平，非改善**） |
| 空事务 | 全库拷贝 | O(表数) |

对验收标准第 1 条是达成；对「改遍所有表」是持平。不宣称全面提速。

## 5. 修复后的曲线

同一夹具、同一命令：

```text
tables   rows/table   total rows   commit ms
------------------------------------------------
1        1000         1000         0.000
10       1000         10000        0.001
10       10000        100000       0.001
50       10000        500000       0.003
```

| 总行数 | 之前 (ms) | 之后 (ms) |
|---|---|---|
| 1 000 | 0.103 | 0.000 |
| 10 000 | 1.428 | 0.001 |
| 100 000 | 9.624 | 0.001 |
| **500 000** | **49.127** | **0.003** |

曲线从线性变为平坦 —— 这是验收标准第 1 条的直接证据。

### 验收标准第 2 条：内存曲线 —— 实测（补于 2026-10-07，PR #5074）

#### 先记一次失败的方法

第一版用 `/usr/bin/time -l` 的 `maximum resident set size` 直接比较进程峰值，
结果**两版毫无差别**（10 表 × 5 万行）：

```text
pre-Arc  : 484.2 / 483.0 / 483.0 / 483.0 / 483.0 MB
post-Arc : 483.0 / 483.0 / 482.9 / 483.0 / 484.1 MB
```

（各 5 次重复，离散度 ±0.3%，不是噪声。）

原因是**峰值在构建夹具时就已达到**，commit 的那份额外拷贝抬不高它：

```rust
// MemoryStorage::insert 的 autocommit 分支本来就同时写两张表
if let Some(log) = self.tx_log.as_mut() { ... } else {
    committed_tables.entry(..).or_default().extend(padded.iter().cloned());
}
tables.entry(..).or_default().extend(padded);
```

也就是说数据集一装载，常驻就已经约 2× 了 —— 这正是 issue 说的「当前常驻约 2×」，
但它出现在**装载阶段**而不是 commit 阶段。峰值 RSS 因此看不见本 issue 关心的东西。

#### 改测「commit 本身带来的增量」

`getrusage(RUSAGE_SELF).ru_maxrss` 是单调高水位，因此在 commit **前后各采样一次**，
差值就精确隔离出 commit 新分配的内存。没有引入 `libc` 依赖 —— C 结构体就地声明
（`ru_maxrss` 在 macOS 是字节，Linux 是 KB）。

夹具：`crates/storage/tests/measure_commit_memory_4948.rs`（`#[ignore]`，手动跑）。
10 表 × 4 列 TEXT，行数由 `SQLRUSTGO_4948_ROWS` 控制。

#### 结果

| 总行数 | pre-Arc commit 增量 | post-Arc commit 增量 |
|---|---|---|
| 250 000 | **68.7 MB** | **0.0 MB** |
| 500 000 | **142.9 MB** | **0.1 MB** |
| 1 000 000 | **291.2 MB** | **0.0 MB** |

pre-Arc 折合约 **290 字节/行**，随行数严格线性 —— 即 commit 每次都把整个数据集再
复制一份，常驻从约 1× 变成约 2×。post-Arc 增量恒为 0（500K 那次的 0.1 MB 是
HashMap 本身重新分配，与行数据无关）。

**验收标准第 2 条达成。** 顺带说明：这个夹具自带变异验证能力 —— pre-Arc 版本等价于
「把 commit 退回深拷贝」，它测出 291 MB，Arc 版测出 0 MB，说明该测量确实能分辨
这两者，而不是在测噪声。

## 6. 测试

### 6.1 新增（`crates/storage/src/engine.rs` 内单元测试，需访问私有字段）

| 测试 | 断言 |
|---|---|
| `commit_shares_row_vectors_instead_of_deep_copying_4948` | **两个** commit 入口下，每张表的 live 与 snapshot `Arc::ptr_eq` 且 `strong_count == 2` |
| `commit_with_log_shares_row_vectors_instead_of_deep_copying_4948` | 同上，`commit_transaction_with_log` 入口 |
| `writing_one_table_after_commit_clones_only_that_table_4948` | 写 t1 后 t1 分离、snapshot 看不到事务内行；**t2 仍然共享**（证明只拷贝被写的那张表） |
| `rollback_actually_undoes_an_insert_4948` | `#[ignore]` —— 见 §8 的既有缺陷 |

用 `Arc::strong_count` / `Arc::ptr_eq` 做**结构性**断言而非墙钟，理由同 §1。

### 6.2 测量夹具

`crates/storage/tests/measure_commit_clone_4948.rs`（`#[ignore]`）。

## 7. 变异验证

| ID | 变异 | 结果 | 命中 |
|---|---|---|---|
| M21 | `commit_transaction_with_log` 退回深拷贝 | **CAUGHT** — 3 FAILED | 全部 3 项 |
| M22 | trait `commit_transaction` 退回深拷贝 | **CAUGHT** — 1 FAILED | `commit_shares_row_vectors_...`（参数化那个） |

无无效变异；M22 只命中为它写的那个测试，无过耦合。

### 7.1 M22 首轮存活 —— 真实的覆盖缺口

第一版 `commit_shares_row_vectors_instead_of_deep_copying_4948` 只调
`commit_transaction_with_log`。**M22 存活**：把 trait 的 `commit_transaction`
单独退回深拷贝，测试全绿。

而 trait 的 `commit_transaction` 恰恰是**引擎实际走的热路径** —— autocommit 写入和
显式 COMMIT 都经由它（例：`commit_implicit_dml_tx` 里的 `storage.commit_transaction()?`）。
也就是说第一版测试保护的是冷路径，放过了热路径。

已把该测试参数化为 `[trait_commit, with_log_commit]` 两轮遍历。补强后 M22 被精准
抓住，且只被它抓住。

## 8. 顺带挖出的既有缺陷（`#[ignore]`，未修）

`MemoryStorage::rollback_transaction` 对 TxLog 里**已经作用域化**的表名又套了一层
作用域：

```rust
// insert() 存进去的是已作用域的 key：
let table_key = self.tbl(table);                    // "default\x01t1"
log.inserted.push((table_key.clone(), ...));

// ...rollback 又作用域一次：
let key = self.tbl(table);                          // "default\x01default\x01t1"
```

undo 永远找不到表，**静默什么也不做**。与我先前修的 #5059 blanket sweep 是同一根因，
不同位置。

既有测试 `test_rollback_removes_inserted_rows` 抓不到：它用**未作用域**的裸 key
`"t"` 预置 `storage.tables`，而 `insert` 写到 `"default\x01t"`，于是 `scan("t")`
无论 rollback 有没有跑都只看到 1 行 —— **断言碰巧成立，实则 rollback 完全失效**。

本 PR 的 `rollback_actually_undoes_an_insert_4948` 通过 `create_table` 正常建表，
key 对齐后失败是真的。标 `#[ignore]` 而非删除：缺陷不由 #4948 修（那是 commit 成本
问题，不是正确性问题），但复现用例应当留到有人修为止。

## 9. 触及面

- `crates/storage/src/engine.rs`：`tables` / `committed_tables` 两个字段；2 处
  `self.committed_tables = self.tables.clone()`；24 处 `self.tables` 访问
  （读侧解引用，写侧 `Arc::make_mut` / `.map(Arc::make_mut)`）；`snapshot_schema`
  与 `apply_schema` 两个边界；`partition_rows` 的返回值物化；`parallel_scan` 的
  `Arc::clone(data)`（原本每次扫描克隆整行集，现直接共享手柄）。
- 新增测试 4 项（含 1 项 `#[ignore]`）、测量夹具 1 个。

## 10. 证据字段（ADR-014）

- `source_agent`: mcode（MCode desktop session）
- `source_run`: 变异矩阵 `bg_1196c49b-67ca-4282-9893-5f812fed6742`；基线
  `engine` 单元 3 passed / 0 failed / 1 ignored
- `timestamp`: 2026-10-07
- `conflict_resolution`: issue 称这是 breaking change（要求 CHANGELOG 与迁移说明），
  实测发现 `SchemaSnapshot` 与内部字段是两个结构，在 `snapshot_schema` 边界物化即可
  保持公共 API 不变。按「公共 API 不变」结案，issue 的 breaking 前提不成立。
  issue 称 `tables` 是 `HashMap<String, TableData>`，实为
  `HashMap<String, Vec<Record>>`，按实际类型实施。