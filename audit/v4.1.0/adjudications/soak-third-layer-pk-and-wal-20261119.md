# 第三层阻塞 —— 主键等值点查返空 + 写事务 O(WAL) 二次方：定性与修复闭合

**日期**：2026-11-19（UTC）　**性质**：**事实登记 + 判决性复现 + 因果归因 + 修复闭合**。
**对象**：168h SOAK 的 r3/r4 起跑前排查；被测树 `680418e9f9…`（r3）与修复后的
`93ac1d427ec0694f613da8470a88003db08b9f53`（PR #5865 合并头）。

---

## 0. 一句话结论

在 `file` 后端上，**主键等值点查 `WHERE id = N` 约 99% 返回 0 行**，
而范围查与聚合 100% 正常；**每个写事务都把整个 WAL 读一遍再重写**，
写代价随 WAL 线性增长 ⇒ 整轮二次方。

两者**都已被本地判决性复现**，**因果归因已用对照实验锁定到单一触发条件**，
**修复已落地并用同一探针做了修复前后对比**（点查命中率 8.03% → 100.00%，
WAL 1,516,127 B → 90 B）。⇒ **r4 轮按修复后的树起跑**。

---

## 1. 现象（逐条可复核）

### 1.1 252 上的实测读数（r3 轮）

| 项 | 值 |
|---|---|
| `point_select` 查询数 | 72,449 |
| **命中（返回行数 > 0）** | **653 ⇒ 命中率 0.90%** |
| `range_scan` 查询数 | 29,264（**29,137 次返回 100 行**、127 次返回 99 行 ⇒ 100% 命中） |
| `count_agg` 查询数 | 14,393（100% 正常） |
| 落盘 `sbtest1.json` | 4,759,024 B、**24,399 行**，id 1..10000 **齐全** |
| 落盘 `sbtest1_idx_id.json` | **15 B，恒为 `{"map": {}}`** |

**数据不缺、范围查能看到、只有等值查看不到** ⇒ 缺陷在**取行层**，不在数据层。

### 1.2 命中率的形状（判决性细节）

`point_select` 命中率**随时间衰减**：起跑首个千行内 7.30% → 稳定在 0.3%–0.5%。
这个形状本身就是线索：说明「早先能查到、后来查不到」，指向**某个随写操作
单调恶化的状态**。

---

## 2. 判决性复现（本机，同一驱动、同一台架）

复刻 `scripts/soak/v400_prepare_soak.sh` 的建表与**单条 10 000 行 INSERT**，
再用**真实驱动** `scripts/soak/v400_soak_driver.py`（16 线程混合负载）施压。

| 轮次 | 负载 | `point_select` 命中率 |
|---|---|---|
| 简单复现（无并发写） | 单线程点查 | **100%** |
| 混合轮（修复前） | read 80 / insert 10 | **5.67%**（3,142 次里 318 次命中） |

⇒ **缺陷可复现，且只在有并发写的场景出现。**

### 2.1 因果归因：三组对照实验（判别实验）

| 实验 | 负载参数 | `point_select` 命中率 | 判读 |
|---|---|---|---|
| **E1** | read 100 / insert 0（**纯读**，40,150 次） | **100.00%** | 读路径健康 ⇒ 缺陷不在读侧 |
| **E2** | read 50 / insert 50（**读+INSERT**，506 次） | **100.00%** | **INSERT 不是触发条件** |
| **E3** | read 50 / insert 0（**读+UPDATE**，660 次） | **8.03%** | **UPDATE 是触发条件** |

⇒ **触发条件锁定为 UPDATE**。

---

## 3. 根因（代码级，四环，缺一不可）

### 3.1 环 1：引擎把 UPDATE 实现成 delete + insert

`src/engine_dml.rs`（UPDATE 执行体）：

```rust
storage.delete_in_db(&stmt_db, &table_name, &delete_key)?;
storage.insert_in_db(&stmt_db, &table_name, vec![new_row.clone()])?;
```

### 3.2 环 2：删除会压缩行向量

`crates/storage/src/file_storage.rs` 的 `remove_matching` 用 `Vec::retain`
**就地压缩** ⇒ 被删行之后的每一行**物理下标都前移一位**。

### 3.3 环 3：主键 B+Tree 存的是物理行号，且删除路径从不维护索引

- 索引的值就是行号（`BPlusTree: BTreeMap<i64, Vec<u32>>`）；
- **全仓没有一处 `index.remove`** ⇒ DELETE 后树里留着陈旧行号；
- 源码注释自己承认：「`WriteState` has no access to the primary-key B+Tree」。

### 3.4 环 4：取行时不校验键值 ⇒ 返回**错行**，随后被 WHERE 复算清空

`scan_with_index_in` 只做越界检查就返回那一行 ⇒ 等值查拿到的是**别的 id 的行**；
查询层随后按 WHERE 复算（`src/engine_select.rs`）把这行过滤掉 ⇒
**用户看到的是「查不到」，而不是「查错了行」**。

> 这一环解释了 §1.2 的衰减形状：陈旧行号只有当它**越界**时才会让索引返回空，
> 从而触发全扫兜底而「侥幸正确」——越靠后的行漂移越大，于是命中率随时间衰减。

### 3.5 伴生缺陷：阈值刷盘分支的索引错位与漏键

`insert_buffered` 的阈值分支用**本次调用的行**去索引**整批**
（缓冲区此前累积的行既漏键、行号又整体平移）；
`flush_buffer` 用的却是正确的窗口法 `update_pk_index_window`。
预置那一步恰好「运气好正确」（单次调用 + 缓冲区为空 + 正好压阈值），
所以平时看不出来。

### 3.6 WAL 的二次方

同一份日志上有**两个 LSN 空间**：`WalStorage` 用自己的计数器盖章，
`FileBackedWalManager::append` 又**覆盖**成它自己的 `next_lsn`
（而它统计的是**所有**经过共享 handle 的条目，含 `FileStorage` 自己的）。
两个计数器速率不同 ⇒ 检查点记录的 LSN 与截断点不在同一空间 ⇒
`truncate_before` 判不出哪些条目可回收 ⇒ **WAL 无界增长**，
而每次提交都要**整份读改写**。

syscall 级证据（隔离探针实例，`strace -f -y`）每提交重复出现：

```
openat(".../sqlrustgo.wal", O_RDONLY)            ← 读整个 WAL
openat(".../sqlrustgo.wal", O_WRONLY|O_TRUNC)    ← 截断到 0
openat(".../sqlrustgo.wal", O_WRONLY|O_CREAT|O_APPEND)
write(...) × N                                   ← 逐条写回 retained
fdatasync(...)
```

实测斜率精确 **1.00**（write_bytes/insert 增量 ÷ WAL 增量）。

---

## 4. 修复（PR #5865，合并头 `93ac1d427e`）

### 4.1 主键等值点查

1. `BPlusTree::adjust_after_removal` —— 删除后**原地修正**树中所有行号
   （O(索引条目数)，与已有的 O(表行数) `retain` 同阶，常数小）；
2. `remove_matching` 回传被删行位置，`delete_at` / `delete_if_at` 据此修复
   本表全部索引；
3. `scan_with_index_in` **校验取回行的键值**，不匹配即丢弃；
4. `scan_pk_in_db` / `scan_pk` 只接受 PK 列**真的等于**查询键的行，
   否则回落到**定点查找**（不整表克隆）；比较位置改用**声明的 PK 列**
   而非 `row.first()`；
5. `insert_buffered` 阈值刷盘分支改用 `update_pk_index_window`。

### 4.2 WAL

1. `WalManager::append` 改为**返回管理器盖的 LSN** —— 日志的 LSN 空间由管理器
   独占并回报，调用方不再自带计数器；
2. `truncate_before` 在**没有任何条目可回收时直接返回**
   （旧代码每次提交都把同一份内容截断重写一遍，这正是 O(WAL) 的来源）；
3. 截断点取 `min(检查点 LSN, 最老在途事务的起始 LSN)` —— 在途事务的条目
   尚未进任何快照，回收它们会丢数据；
4. `next_lsn` 不再回退，日志清空后 LSN 序列不得从 0 重来。

### 4.3 门禁判据补强

`scripts/soak/run_v410_168h_soak.sh` 的臂 A 判据此前**只看 `A_QUERIES > 0`**
与采样覆盖率，**不校验任何一次查询返回了多少行** ⇒
**一轮点查 99% 返空的运行照样会被判 PASS**。
现从 `armA/driver.log` 统计 `point_select` 命中率，要求 **≥ 99%**，
并并入臂 A 的 PASS 合取式与最终报告表格。**判据只增严，不放宽任何既有门槛。**

---

## 5. 修复前后对比（同一探针、同一台架、同一驱动）

### 5.1 正确性

| 场景 | 修复前 | 修复后 |
|---|---|---|
| 读+UPDATE 点查命中率（E3） | **8.03%** | **100.00%** |
| SOAK 同参混合负载点查命中率 | **5.67%** | **100.00%** |
| 读+INSERT 点查命中率 | 100.00% | 100.00% |
| 纯读点查命中率 | 100.00% | 100.00% |
| 错误数 | 0 | 0 |

### 5.2 性能与资源

| 指标 | 修复前 | 修复后 |
|---|---|---|
| UPDATE 次数 / 180 s（E3） | 1,484 | **6,037**（4.07×） |
| WAL 大小（混合负载） | 1,611,118 B | **135 B** |
| WAL 大小（放大探针，每轮 200 INSERT） | 37,800 → 226,800 B（线性无界） | **恒定 90 B** |

### 5.3 回归测试的有效性

新增 `file_storage::tests::pk_lookup_stays_correct_after_a_delete` 与
两条 `bplus_tree::tests::adjust_after_removal_*`。

**已实测：把「索引修复」与「键值校验」两道防线同时中和后，回归测试失败**，
并精确暴露错行（`id=4` 查到 `Integer(5)`）⇒ 是**真回归测试**而非恒真断言。

存储层单测 **919 passed / 0 failed**。

---

## 6. 对 r4 轮的影响

- r3 轮**已因 7 个错误出局**（见 `soak-r3-io-storm-attribution-20261119.md`），
  其错误窗与宿主同盘 I/O 风暴逐秒对齐；急停开关开启后，
  r3 又跑了 **1.5 小时（elapsed 7168 s）错误数仍为 7、panic 0、零复发** ⇒ 根因闭合的二次印证。
- r4 轮**在修复后的树 `93ac1d427e` 上起跑**，被测二进制重新编译并记录指纹。
- 独占窗口、急停开关、四个守护服务**沿用 r3 的既有机制**，
  **不动判据、不动阈值、不动分母、不换存储基底**。

---

## 7. 尚未闭合 / 留待后续

1. **索引落盘缺口**：`FileStorage::flush` 不调用 `flush_indexes()` ⇒
   落盘 `*_idx_*.json` 运行期恒为空树，**与内存状态不一致**，易误导排查。
   本次未改（属可观测性，不影响正确性）。
2. **`row.first()` 定位 PK 的历史遗留**：`crates/storage/src/engine.rs` 的
   trait 默认实现仍用 `row.first()`；本次改了 `FileStorage` 的两个实现，
   trait 默认实现未动。
3. **`BPlusTree::insert` 对同键是追加而非替换** ⇒ 反复 UPDATE 的键会堆积
   陈旧行号（本次由环 3 的修复覆盖了正确性，但树里的冗余项仍在）。
4. **`insert_buffered` 的索引维护仍有窗口**：缓冲未满阈值时新行不在索引里，
   靠 `scan_with_index_in` 的缓冲分支兜底（该分支的比较逻辑本就正确）。
5. **WAL 的尾部拷贝优化**：本次用「无可回收则直接返回」消除了 O(WAL) 提交代价，
   但真正发生回收时仍是「读全量 + 重写保留段」。WAL 已被截断到 KB 级，
   成本可接受；如需进一步优化可改为尾部拷贝。

---

## 8. 证据清单

| 证据 | 位置 |
|---|---|
| r3 轮 driver.log（点查 0.90% 命中、范围查 100%） | `/home/liying/soak-artifacts/20261119-168h-v410-r3/attempt-20261119T011150Z/armA/driver.log` |
| 本地复现与对照实验脚本 | `/tmp/_pk_repro.sh`、`/tmp/_pk_repro_driver.sh`、`/tmp/_pk_repro_mix.sh`、`/tmp/_wal_amp_probe.py` |
| 修复前后对比读数 | `/tmp/_pkdrv_local_driver.log`（前）、`/tmp/_pkdrv_after_driver.log`（后）、`/tmp/wal_amp_before.json`、`/tmp/wal_amp_after.json` |
| 修复提交 | `757b538046`（PK 索引）、`56587af2ce`（WAL）、`4cc3ab942a`（门禁判据） |
| PR | #5865（已合并） |
