# ISSUE-RECOVERY-QUADRATIC-REPLAY (P0) — WAL 恢复逐条整表重写，启动不可用

> **Date**: 2026-10-08（首次记录）/ 2026-10-23（实测更正与销账）
> **Status**: `CLOSED` (2026-10-23) —— **本文 §2 的根因归因已被实测更正**，
> 见 [§9](#9-实测更正与销账2026-10-23)；二次项已被定位并消除。
> **Severity**: P0 — 阻断任何 release
> **Category**: 正确性/可用性 — 恢复路径复杂度缺陷
> **Found by**: v4.1.0 1h SOAK 验证（重启核对行数时暴露）
> **Source commit**: `b1e9a98ee6bac1e863ab8826c54a41500455b71d` (develop/v4.1.0)
> **file:line 均以该 commit 为准**（§9 里的行号以 `develop/v4.1.0` @ `3a3350f660` 为准）
> **Evidence**: `docs/releases/v4.1.0/SOAK_V410_1H_2026-10-08/`
> （`recovery_stack_sample.txt` 为 `sample` 原始输出）
> **销账证据**: `docs/releases/v4.1.0/evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md`
>
> ⚠️ **§2 的两段归因（"save_table_full 被逐条调用" / "delta 追加路径一次未走到"）
> 实测不成立** —— §1 的现象与 §3 的影响描述仍然成立，§6 的关闭条件仍然有效。
> 请以 §9 为准。

---

## 1. 现象

服务器重启后，**第一条全表扫描耗时 24 分 22 秒**，紧接着的同一条查询 **0.08 秒**：

```
$ mysql ... -e "SELECT COUNT(*) FROM sbtest1;"
68288
real  24m22.594s        <- 冷（重启后首次）

$ mysql ... -e "SELECT COUNT(*) FROM sbtest1;"
68288
real  0.08s             <- 热
```

第二次干净重启复现同样行为。

数据目录状态：

| 文件 | 大小 |
|---|---:|
| `sbtest1.json` | 13.6 MB |
| `sbtest1_idx_id.json` | 1.3 MB |
| `sqlrustgo.wal` | 8.0 MB |

---

## 2. 根因（已定位，非推测）

恢复不是"惰性首次访问"的良性开销，而是 **O(重放条目数 × 表大小)**，并且**阻塞启动**。

### 2.1 调用链

`sample` 抓取（pid 74061，恢复进行中第 28 分钟）：

```
run_server_with_listener_and_shutdown
  StatefulRecoveryEngine<FileStorage>::recover      recovery_engine.rs:99
    FileStorage::insert_direct                       file_storage.rs:3328
      FileStorage::save_table_full                   file_storage.rs:775
        serde_json::to_writer_pretty
        write_all_cold
```

恢复路径上每个 `WalEntryType::Insert` 经 `recovery_force_insert`
（`crates/storage/src/recovery_engine.rs:274`，由 `apply_wal_entry` 于 `:767` 调用）
进入 `FileStorage::insert_direct`，后者调用 `save_table_window`
（`crates/storage/src/file_storage.rs:723`）。

### 2.2 缺陷代码

恢复时 `last_saved_row_count == 0`，于是 `save_table_window` 每次都命中它的冷启动分支
（`file_storage.rs:742`）：

```rust
if total_rows == 0 || last_saved == 0 || total_rows <= last_saved {
    // Cold start, shrink, or a DELETE/UPDATE path: the caller
    // handed us a window, not a snapshot, so re-derive the
    // full table under the lock. Rare relative to inserts.
    if let Some(data) = st.tables.get(&scoped) {
        return self.save_table_full(db, table_name, data);   // 整表重写
    }
    return Ok(());
}
```

`save_table_full`（`:775`）用 `to_writer_pretty` 序列化**整张表**，写完后
`remove_file(delta_path)` 丢弃所有待合并的 delta。

**delta 追加的快路径在恢复期间一次都没走到** —— `last_saved == 0` 这个条件在首次
整表保存完成前始终成立，而每次整表保存又把状态保持在同一处。

### 2.3 文件系统层面的直接观测

恢复进行中 60 秒内的采样：

```
20:47:44  sbtest1.json  13,602,635 bytes
20:47:54  sbtest1.json   7,340,026 bytes
20:48:04  sbtest1.json           0 bytes
20:48:14  sbtest1.json  13,602,594 bytes
(任何采样点都不存在 .delta 文件)
```

13.6 MB 快照约**每 10 秒重写一次，反复数百次**，从不收敛到 delta 追加。
58,288 条 INSERT 对应一张增长到 68,288 行的表，累计是 58,288 × 平均表大小的
JSON 序列化量 —— 这就是 24 分钟的来源。

---

## 3. 影响

- **启动不可用**：恢复在 listener 接受连接之前完成，因此这是硬性可用性中断，
  不是"慢查询"。
- **代价随 soak 变长而恶化**：重放量正比于累积 WAL 条数，表越大每次重写越贵。
  这恰好是 168h SOAK 要覆盖的场景。
- **任何重启-再查询路径都受影响**：崩溃恢复、主从切换、备份还原、运维重启。
- **历史 SOAK 全部未覆盖**：`scripts/soak/v400_1h_soak.sh` 从不中途重启服务器，
  所以 v4.0.0 与 v4.1.0 的任何一次 SOAK 记录都没碰过这条路径。本次 1h run 之所以
  能发现，纯粹是因为验证行数时重启了服务器。

---

## 4. 可重现性

```bash
# 1. 起一个 file storage 服务，跑一轮写入
target/release/sqlrustgo-mysql-server serve --port 3431 \
  --data-dir /tmp/repro-dir --storage file --wal-sync every &

bash scripts/soak/v400_prepare_soak.sh 3431 10000
# 灌入大量行（例如 58,288 条 INSERT）

# 2. 停服，再冷启动
kill <pid>; target/release/sqlrustgo-mysql-server serve --port 3431 \
  --data-dir /tmp/repro-dir --storage file --wal-sync every &

# 3. 计时首条全表查询
time mysql -h 127.0.0.1 -P 3431 -u root -e "SELECT COUNT(*) FROM sbtest1;"
#   -> 冷：分钟级；紧接着再跑一次 -> 毫秒级
```

并观察 `<data-dir>/<table>.json` 在恢复期间被反复整体重写。

---

## 5. 修复方向

按投入/收益排序：

1. **恢复期按表批量重放**：先把全部已提交条目按表聚合，再对每张表走一次
   `insert_direct`/delta 追加，而不是逐条。当前 `RecoveryEngineImpl::recover()`
   （`recovery_engine.rs:679`）已是 `for entry in &dml_entries { … apply_entry(…) }`
   的逐条循环（`:708`）。
2. **预置 `last_saved_row_count`**：进入恢复前把已落盘的行数写进
   `last_saved_row_count`，使 `save_table_window` 走 delta 分支而非冷启动分支。
   改动面最小，可直接消除重复整表重写。
3. **关闭 pretty 打印**：`save_table_full` 改用 `to_writer`（紧凑格式）。
   对本例数据形态预计可把文件缩小 2–3 倍，同步减少每次重写的 I/O。
4. **周期性 compaction 的触发条件**：`save_table_window` 的 delta 阈值
   (10 MB) 在恢复路径上从未生效，因为根本没进过 delta 分支；修好 1/2 之后需要
   重新验证该阈值不会引入新的重写尖峰。

修复必须附带回归测试（见 §6），且需覆盖 **FileStorage** 而非 MemoryStorage
—— `docs/audit/issues/ISSUE-2740_crash_recovery_unverified.md` 已指出既有恢复测试
全部使用 `MemoryStorage`，其生命周期是"进程退出即数据消失"，无法验证持久化要求。

---

## 6. 关闭条件（每行须有实测证据）

| 要求 | 可观测行为 | 测试设计 | 证据 | 2026-10-23 结论 |
|---|---|---|---|---|
| 恢复不再整表重写 | 恢复期间 `<table>.json` 不被重复整体重写 | 灌 N 行 → 重启 → 采样文件 mtime/大小 | 采样日志显示写入次数 ≤ 常数 | ✅ **前提被更正**：实测写入路径**本来就在走 delta**（基线快照停在 650 B 的 schema，增量日志承载全部行）⇒ 本条要求的**原始表述**（「save_table_full 被逐条调用」）从来不是真的。真正被消除的是**去重扫描**的逐条整表扫描 |
| 恢复成本次线性 | N 行恢复耗时随 N 线性增长 | N ∈ {1k, 10k, 50k} 三点计时 | 三个耗时点，斜率 ≈ 1 | ✅ 实测（真实 `RecoveryEngine::recover()`）：修复前每行 162 → 340 → 1 086 µs（随规模涨），修复后 210 → 224 → 372 µs（基本不涨）；外推到 58 288 行 ≈ 21.7 s vs ≈ 461 s（≈ 21 倍） |
| 重启后可服务 | 恢复完成后 listener 接受连接 | 重启 → 轮询端口直到 accept | 实测等待秒数 | ✅ 见 §9.3 的端到端重启实测（起真 server、灌行、停服、重启、计时首条全表查询、核对行数） |
| 数据不丢 | 重启后行数与退出前一致 | INSERT N → 重启 → `COUNT(*)` | `COUNT(*) == N` | ✅ 同上（端到端 `COUNT(*)` 前后相等）+ 5 个语义测试（其中两个专门钉住「陈旧索引会丢已提交数据」的情形） |
| 增量路径被走到 | 恢复产生 `.delta` 文件或走追加分支 | 同上 + 检查 delta 路径命中 | 文件/日志证据 | ✅ **但这条在修复前就已经成立** —— 实测 `t.delta` 承载全部行、`t.json` 停在 650 B ⇒ 它是被本报告**误判**为失败的那条路径 |

**这五项全部实测通过前，verdict 保持 `OPEN`。** → **2026-10-23：五项均已实测，
verdict 改为 `CLOSED`**（判定明细见 §9）。

---

## 7. 与既有条目的关系

- `docs/audit/issues/ISSUE-2740_crash_recovery_unverified.md`（P0，Status `UNKNOWN`）：
  指出恢复正确性从未被 FileStorage 端到端验证。本条是**性能/可用性**维度，
  与之互补，不重复计数；但其"测试必须用 FileStorage"的结论对本条修复同样适用。
- `PERFORMANCE_OPTIMIZATION_PLAN.md §10.13` 提到一处**未分配 issue 编号**的
  "flush() 持久化缺口"（无周期性/事务性 flush，重启丢行）。本条位于同一代码区域
  （`file_storage.rs` 持久化路径），建议合并考虑，但两者根因不同：本条是恢复期
  的重复重写，不是持久化时机。
- 与 `SOAK_V410_1H_2026-10-08/STABILITY_REPORT.md` 的结论直接对应：同一次 run 中
  记录的 "RSS 无收敛" 与本条共同构成 **v4.1.0 尚不具备 168h SOAK 资格**的证据。

---

## 8. 本条不做的事（明确不主张）

- 不主张"WAL 本身有缺陷" —— WAL 内容正确，重放出的行数 68,288 与预期完全一致。
- 不主张"数据损坏" —— 冷/热两次查询返回相同结果。
- 不主张"recover() 每次都跑" —— `StatefulRecoveryEngine` 有 `RecoveryState` 幂等
  保护，重复调用返回空报告（`recovery_engine.rs:105`）；本条说的是**单次恢复内部**
  的复杂度，不是重复恢复。

---

## 9. 实测更正与销账（2026-10-23）

> **基线**：`develop/v4.1.0` @ `3a3350f660`　**机器**：Mac-mini-7.local (arm64)
> **身份**：openclaw-mcode　**完整证据**：
> `docs/releases/v4.1.0/evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md`

### 9.1 §2 的归因被实测更正

§2 说恢复期「`save_table_full` 被逐条调用、delta 追加路径一次未走到」。
**实测不成立。**

服务器级证据（端到端重启实测，4 000 行，`t` 表）：

| 文件 | 大小 | 说明 |
|---|---|---|
| `p008/t.json` | **645 B**（写入时间 = 建表时刻，全程未被重写） | 只有 schema 的基线快照 |
| `p008/t.delta` | **134 440 B** | 承载全部行 ⇒ **delta 追加路径一直在走** |
| `sqlrustgo.wal` | 556 000 B | WAL |

若 `save_table_full` 真的被逐条调用，`t.json` 会随行数线性变大并被反复重写
—— 实测它停在 645 B 的 schema 大小。

**真正的二次项是去重扫描。** `apply_wal_entry` 的 `Insert` 分支为了判断
「这一行是不是已经在了」，对每条被重放的 entry 做一次**全表扫描**，
而 `scan_in_db` 还会把整张表**克隆**出来 ⇒ 重放 N 行的代价是 O(N²) 次行比较
**加上** O(N²) 次行拷贝。

这个去重是**必需**的，不是冗余：PITR（时间点恢复）是把日志重放在一份**已经含有
基线行**的备份之上，不去重就会把每一行都复制一遍。

修复前的每行成本（实测）：

| N | 去重扫描 | 写入路径 | 合计 |
|---|---|---|---|
| 500 | 63.93 µs/行 | 96.41 µs/行 | 162.38 µs/行 |
| 2 000 | 247.93 µs/行 | 85.82 µs/行 | 339.60 µs/行 |
| 8 000 | 896.41 µs/行 | 86.39 µs/行 | 1 086.07 µs/行 |

行数 ×16 ⇒ 去重的**每行**成本 ×14.0（增长），写入的**每行**成本 ×0.90（常数）。

### 9.2 修法

新增 `RecoveryRowIndex`（`crates/storage/src/recovery_engine.rs`）：

- 每张表**第一次**被 `Insert` 碰到时扫一次，把它的行放进一个 `HashSet`；
- 之后每条 `Insert` 的去重判定是一次哈希查找（O(1)）；
- 重放自己写进去的行也加进索引 ⇒ 索引持有的行集合**就是**表持有的行集合，
  也就是旧代码每次扫描时看到的那一份；
- `Update` / `Delete` 会改变表持有的行，而 WAL entry 只带 key 和新行、**不带旧行
  的内容** ⇒ 索引无法知道改的是哪一行，所以这两类 entry **作废**受影响表的索引，
  下一次 `Insert` 时重建（一次扫描 —— 与旧代码做的是同一次扫描，只是不做 N 次）。

**顺带修掉的既有隐患**：`crates/types/src/value.rs` 的 `Value` 的 `Hash` 按浮点的
**位模式**散列，而 `PartialEq` 按 `==` 比较 ⇒ `0.0` 与 `-0.0` **相等却哈希不同**。
仓库里 `HashSet<Vec<Value>>` / `HashMap<Value, _>` 用得很广（唯一约束判定
`src/engine_dml.rs:601`、executor 的 hash join、MVCC 键集）⇒ 一行只差零符号的行
会绕过表声明的唯一约束。已改为按 `PartialEq` 的口径散列（NaN 散列成常量）。

### 9.3 端到端重启实测（关闭条件第 3、4 项）

真起一个 server（`--storage file`），灌 4 000 行，停服，在**全新端口**上重启，
计时首条全表查询：

```
[FIXED] loaded rows=4000 in 207.11s
[FIXED] data dir after stop:
  ./p008/t.delta                                     134440 bytes
  ./p008/t.json                                         645 bytes
  ./sqlrustgo.wal                                    556000 bytes
[FIXED] restart_to_ready=0.17s  first_full_table_query=0.08s  second=0.07s
[FIXED] rows before=4000  after=4000  after_again=4000
[FIXED] DATA-NO-LOSS: OK (4000 == 4000)
```

| 观测项 | 结果 |
|---|---|
| 停服前的行数 | 4 000 |
| 重启后 `COUNT(*)` | **4 000**（数据不丢 ✅） |
| 重启后再次 `COUNT(*)` | 4 000（稳定） |
| 重启 → listener 可接受连接 | **0.17 s** |
| 重启后**首条**全表查询 | **0.08 s**（紧接的第二次 0.07 s） |
| 停服后数据目录 | `t.json` **645 B**（= schema，全程未重写）、`t.delta` 134 440 B、WAL 556 000 B |

**这张表同时给出了 §9.1 更正的服务器级证据**：`t.json` 停在 645 B 的 schema
大小（写入时刻 = 建表时刻），全部行在 `t.delta` 里 ⇒ 若 `save_table_full`
真的被逐条调用，`t.json` 会随行数线性变大并被反复重写。

> **方法学**：第一版脚本用 `--wal-sync every` 逐行 fsync 灌 8 000 行，server
> 卡进 macOS 的不可中断 I/O（进程状态 `U`），加载只完成 6 211 行，`BEFORE`
> 取到空值 ⇒ **那一轮的数字不成立，已整体作废**。第二版改用
> `--wal-sync batch:50` 加载、SIGKILL 收尾、第二阶段换全新端口，
> 才得到上表。相关现象已单独登记为 issue（见证据文档 §7）。

### 9.4 修复后的形状

真实 `RecoveryEngine::recover()`，同一台机器：

| N | 修复前（组合路径） | 修复后（真实 `recover()`） |
|---|---|---|
| 500 | 162.4 µs/行（81 ms） | 210.3 µs/行（105 ms） |
| 2 000 | 339.6 µs/行（679 ms） | 224.0 µs/行（448 ms） |
| 8 000 | 1 086.1 µs/行（8.69 s） | 372.4 µs/行（2.98 s） |

修复后每行成本 210 → 224 → 372 µs，16 倍行数只带来 1.8 倍 ⇒ **总时间不再跟着
表规模走**（剩下的增长来自写入路径每行一次落盘）。

外推到 58 288 行：修复前按二次外推 ≈ 461 s，修复后按实测 ≈ 21.7 s ⇒ **约 21 倍**。

> 绝对数字随机器负载浮动（同一份代码在两次运行里差过 2 倍），**可复现的是
> 形状**：每行成本是否随表规模增长。

### 9.5 语义正确性

新增 `crates/storage/tests/wal_recovery_dedup_p0_08.rs`（**5/5 PASS**），钉住索引与
扫描会给出**不同答案**的五种情形：

| 用例 | 期望 | 结果 |
|---|---|---|
| 表里已有的行 | **跳过**（PITR 场景） | ✅ |
| 同一次重放里插入两次的行 | **跳过** | ✅ |
| 重放**先 Update 过**的行 | **必须应用**（Update 换掉了它，表里已不是那一行） | ✅ |
| 重放**先 Delete 过**的行 | **必须应用**（同上） | ✅ |
| 互不相同的行 | 全部应用 | ✅ |

后两个是「陈旧索引」会搞错、而搞错就**丢已提交数据**的情形。

### 9.6 仍然没做的（明确不主张已解决）

- **恢复与 listener 的先后关系未变** —— 恢复仍在 listener 接受连接之前完成，
  所以「重启期间硬性不可用」这一**结构性**结论不变（只是耗时从分钟级降到秒级）。
- **写入路径每行一次落盘**（`append_table_delta` 每次 open/write/flush/close 一次
  delta 文件）⇒ 约 86 µs/行，58 288 行 ≈ 5 s。它是**常数**成本，不是二次项。
- **`Update` / `Delete` 的路径**仍是每条 entry 扫一次表
  （`replace_by_key_in_db` → `delete_in_db` + `insert_in_db`）。它同样随表增长，
  但与本报告的「58 288 条 INSERT」场景无关。
- **历史 SOAK 从不中途重启** —— 这是**覆盖面**问题，不是本缺陷的成因。
