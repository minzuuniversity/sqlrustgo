# 事务隔离设计：脏读与事务内重复读不稳定

> **provenance (ADR-014 5 evidence fields)**
> - `source_agent`: mcode (MCode root session)
> - `source_run`: snapshot-isolation-probe-2026-10-04
> - `timestamp`: 2026-10-04T20:35+08:00
> - `evidence_hash`: local-git:`17f40f0d69` (base = gitea252/develop/v4.1.0)
> - `conflict_resolution`: N/A — 首次测量，本文不含实现
>
> **政策**: Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-008 + ADR-014
> **状态**: DESIGN（只含实测证据与方案对比，**未改任何代码**）
> **关联**: #4960（本次调查的副产品，PR #4971）、#4910（单线程锁竞争）

---

## 0. TL;DR

实测确认当前实现**不提供任何事务隔离级别**，连 READ UNCOMMITTED 之外的两条底线都没守住：

| 现象 | 实测 | 判定 |
|---|---|---|
| 脏读：读到别人未提交的 INSERT | 复现 | 违反 READ UNCOMMITTED 底线 |
| 脏读：读到别人未提交的 UPDATE 值 | 复现 | 同上 |
| 事务内重复读不稳定 | 复现 | 连 READ COMMITTED 都不满足 |

**这不是可以顺手修的 bug。** 根因在 MVCC 的时间戳分配时机与读快照的取得方式，
两者都需要改结构。本文给出证据、方案对比与三个必须先定的决策点。

---

## 1. 实测证据

探针：`crates/mysql-server/tests/zz_probe_snapshot.rs`（真实 MySQL 协议 +
真实服务端；数据目录每次全新，避免上次运行的残留）。

### 1.1 脏读

```text
P0 before anything: []
P1 A 自己的事务内看到: ["1"]
P1 B 看到 A 未提交的写: ["1"]                                    ← 脏读
P2 A 看到 B 未提交的写: ["1", "2"]                              ← 双向脏读
P2 B 自己事务内看到: ["1", "2"]
P3 A 提交后 B 看到: ["1", "2"]
P4 都提交后 A 看到: ["1", "2"]
P5 A 事务内: ["1", "2"]
P5 B 看到 A 的未提交 UPDATE: [["1", "A-DIRTY"], ["2", "A-DIRTY"]]  ← 脏读
P6 A 回滚后 B 看到: [["1", "a-uncommitted"], ["2", "b-uncommitted"]]
```

`P6` 尤其说明问题：**B 读到了 `A-DIRTY` 这个从未被提交过的中间状态，
而 A 随后回滚，该值又消失了。** B 的读依赖于一个最终被丢弃的写。

`P1`/`P2` 说明脏读是**双向**的，不是单连接特有。

### 1.2 事务内重复读不稳定

```text
R1 A 事务内第一次读: ["1"]
R2 A 事务内第二次读: ["1", "2"]     ← 同一个事务内，B 提交后读到了新行
R2 一致? false
R3 A 提交后读: ["1", "2"]
```

A 的事务没有做任何写，纯粹在读。B 在 A 的事务进行中提交，A 的**第二次读
就看到了 B 的新行**。这连 READ COMMITTED 都不满足 —— READ COMMITTED 要求
每条语句取新快照，但同一个语句的结果必须自洽；这里是同一个事务的两次不同
读，第二次拿到了新数据。

严格说：这符合 READ COMMITTED 的定义（每语句新快照），但**不符合任何
InnoDB 默认语义**（REPEATABLE READ 要求事务内固定快照）。当前实现
`SET TRANSACTION ISOLATION LEVEL` 是否真正生效，也需要一并核实。

---

## 2. 根因定位

### 2.1 写侧：可见性时间戳在「写时」就分配了

`crates/storage/src/mvcc_storage.rs::insert`：

```rust
let ts = mvcc.next_snapshot_ts();   // 全局递增，立即取
mvcc.put(pk, row, ts, ts);
```

`VersionedTable::next_snapshot_ts` 是 `fetch_add(1) + 1`——单调递增且立即生效。
`VersionedTable::put(pk, row, visible_from_ts, tx_id)` 把这个 ts 同时当作
`visible_from_ts` 和 `tx_id` 写入版本链。

**后果**：一行刚被写入（哪怕事务尚未提交）就带上了「从现在起可见」的语义。
任何之后取快照的读者（`begin_snapshot()` 返回当前计数器值，必然更大）
都会通过 `find_visible` 的 `visible_from_ts <= snapshot_ts` 判定，看到它。

结构上，**当前实现在写入时刻就已经决定了可见性，之后无法补救**。

### 2.2 读侧：快照不是「固定」的，而是「取当前值」

`crates/storage/src/mvcc.rs`：

```rust
pub fn begin_snapshot(&self) -> u64 {
    self.snapshot_counter.load(Ordering::Acquire)   // 每次读都取当前值
}
```

每次 `scan` / `scan_with_filter` / `get_visible` 都重新调 `begin_snapshot()`。
事务与快照之间没有任何绑定关系——`MvccStorage` 根本不知道「当前连接是否在
事务中、在哪个事务」，`scan` 是 `&self` 且无任何事务参数。

这直接解释了 `R2`：A 的第二次读重新取了当时的计数器，自然看到 B 刚提交的行。

### 2.3 为什么不能简单地把 `ts` 推迟到提交时

把 `visible_from_ts` 改成「提交时分配」是必要条件，但不充分——还需要：

1. **未提交版本的可见性判定**：`find_visible` 只看 `visible_from_ts`，无法区分
   「已提交」与「未提交」。需要引入 `committed: bool` 或让未提交版本用
   `visible_from_ts = u64::MAX`（但那会破坏链的有序性，因为 `find_visible` 是
   `iter().rev().find()`，依赖链按 ts 有序）。
2. **读事务需要固定自己的快照 ts**（见 §2.2），这需要连接级状态。
3. **GC 必须知道哪些版本不能回收**（见 §3.3）。

---

## 3. 三个必须先定的决策点

以下三点没有唯一正确答案，必须由人决定或由外部约束（兼容性要求）确定。

### 3.1 目标隔离级别

| 目标 | 含义 | 代价 |
|---|---|---|
| **READ UNCOMMITTED（现状）** | 承认现状，不修 | 脏读永久存在 |
| **READ COMMITTED** | 语句级快照 | 改动较小，但 `R2` 的行为仍"合法" |
| **REPEATABLE READ（InnoDB 默认）** | 事务级固定快照 | 与 §3.3 的 GC 强耦合 |
| **SERIALIZABLE** | 最强 | 需要锁或谓词冲突检测，代价最大 |

**建议**：至少 READ COMMITTED（消除脏读），理想 REPEATABLE READ（对齐 MySQL 默认）。
注意 #4910 已记录当前 8 线程 sysbench TPS 仅 164、BETA 目标 400——**隔离级别提升
有明确的性能代价，必须实测**，不能凭直觉决定。

### 3.2 连接级快照存在哪里

这是**最关键的决策点**，也是 #4960 撞过墙的地方。

| 方案 | 存放位置 | 优点 | 致命问题 |
|---|---|---|---|
| **A. 存在 `MvccStorage`** | `HashMap<conn_id, u64>` | 实现直观 | **存储是全局单例**（`Arc<RwLock<S>>`），需引入连接标识贯穿整条调用链。#4960 已实测：连接级状态放共享存储会导致暂存键错配、A 回滚清掉 B 的数据 |
| **B. 存在 `ExecutionEngine`** | `TxSession.snapshot_ts` | 天然 per-connection，无跨连接污染 | `storage.scan(&self)` 拿不到它。需要 `scan_at(table, snapshot_ts)` 之类的新接口，且所有读路径要改 |
| **C. 存在 `TxSession`，读时以参数传入** | 同 B，但显式传 | 无隐式状态，最易测 | 改动面最大：每个 `scan` 调用点都要传 snapshot |

**建议 B**：这是 #4960 的教训——**per-connection 状态必须放在 per-connection 的
结构里**（`ExecutionEngine` 是每连接一个），不能放在共享存储里。

代价是存储层要新增 `scan_at` / `get_visible_at` 一类接口，且 `ExecutionEngine`
的所有读路径要改为传入本事务的快照。**改动面覆盖所有查询**，需完整回归。

### 3.3 GC 对未提交版本怎么办

当前 GC（`crates/storage/src/mvcc.rs::gc`）按 `visible_from_ts < cutoff` 回收。
引入未提交版本后有两种处理：

| 策略 | 做法 | 风险 |
|---|---|---|
| **提交前不进 MVCC 链** | 未提交的写只存在于别处，提交时才进链 | 需要 §3.2 的连接级暂存，回到 #4960 的老路 |
| **进链但标记未提交** | GC 跳过 `!committed` 的版本 | **长事务会无限增长**：GC 的 `cutoff` 只看时间，一个跑了几小时的事务的未提交版本永远达不到 cutoff，链会无限膨胀。必须再加「最老未提交版本年龄」或「未提交版本数量上限」的保护 |

**这是方案二（进链 + 标记）的隐藏成本**，也是它比方案一复杂的地方。设计时
必须回答：长事务下如何回收未提交版本？

---

## 4. 两套完整方案

### 方案一：承认 READ UNCOMMITTED，只修 ROLLBACK（最小）

- **做什么**：什么都不做。当前行为即为 READ UNCOMMITTED 语义。
- **代价**：MySQL 默认是 REPEATABLE READ，很多应用依赖它。用 sqlrustgo 的
  应用若在事务里做「先查后改」会读到别人未提交的数据。
- **何时选**：需要尽快发布且能接受弱隔离。

### 方案二：实现 READ COMMITTED（推荐路线）

- **做什么**：
  1. `MvccStorage` 增加 per-connection 快照（放 `ExecutionEngine::TxSession`）
  2. 写侧：`insert/update/delete` 在事务内**不立即写 MVCC 链**，改为记录
     `visible_from_ts = 提交时才分配`；autocommit 路径保持现状
  3. 读侧：`scan` 走 `scan_at(table, snapshot_ts)`，snapshot 取自本事务，
     无事务时取当前值
  4. GC：提交前不进链（沿用 §3.3 方案一），避免未提交版本的回收问题
- **代价**：
  - 改动覆盖所有读路径，回归面是整个测试套件
  - 写侧需要暂存机制——**这正是 #4960 试过并失败的「暂存层挂共享存储」**，
    但这次有 §3.2 的明确结论：暂存必须在 `ExecutionEngine` 侧
  - 有性能代价，需按 #4910 的口径实测
- **风险**：中等。#4960 的教训是架构前提必须先验证；本文 §3 的三个决策点
  就是为了在动手前把前提定死。

---

## 5. 回归面估计

| 变更点 | 受影响测试 |
|---|---|
| `MvccStorage::insert/update/delete` | storage 全部 + 依赖 autocommit 的所有测试 |
| `MvccStorage::scan` / `scan_with_filter` | 同上（#4962 刚改过 `scan_with_filter` 签名，注意叠加） |
| `ExecutionEngine` 读路径 | 根 crate 全部 |
| 新增 `scan_at` 接口 | 所有存储后端（FileStorage / MemoryStorage / BinaryStorage / WalStorage） |

`cargo test -p sqlrustgo-storage`（当前 39 个 target）、
`-p sqlrustgo-mysql-server`（20 个 target）、`-p sqlrustgo`（根 crate）
是最低门槛。**注意当前 develop 正被其他 agent 高频合入，测试基线在漂移**，
测量与回归需固定 develop commit。

---

## 6. 给评审者的三个问题

1. **目标隔离级别定到哪一档？** 需要考虑 #4910 的 164 → 400 TPS 目标。
2. **连接级快照放 `ExecutionEngine`（方案 B）还是接受改动面更大但更显式的方案 C？**
3. **长事务下未提交版本的回收策略能否接受「提交前不进 MVCC 链」**（即暂存层
   放在 `ExecutionEngine` 侧）？这是 #4960 已经验证过「放共享存储会出事」的位置。

在这三点确定之前动手，大概率重演 #4960 的过程：实现数百行、跑出结果、
然后发现架构前提不成立。
