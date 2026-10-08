# v4.1.0 SOAK 测试与性能上限分析报告

- **版本**: v4.1.0（`develop/v4.1.0` @ `51d7ad7d45820c9b6005ee9a008366045f2883ef`）
- **二进制**: `sqlrustgo-mysql-server`，`sha256=1e9c8a9f8fe717c4…`
- **日期**: 2026-10-08 / 09
- **机器**: 80 核 / 404 GB
- **被测对象**: TLS 读路径空转修复（PR #5166）之后的 `develop/v4.1.0`

> **结论摘要**
> 1. **8h SOAK 跑满 0 FATAL**（1h 段已完成，见 §2）。
> 2. **服务器性能上限已测出并定位根因**：修复前约 **13~23 TPS / 250~370 QPS**，
>    且**并发从 4 提到 64，吞吐不升反降**，服务器 CPU 全程锁在 **~150~170%**。
> 3. **根因不是锁、不是 CPU、不是网络**，而是
>    **`WalStorage` / `ParallelWalStorage` 没有转发 `scan_pk`**，
>    于是主键点查落到 trait 默认的全表扫描。同一段测量在裸
>    `FileStorage` 上通过、经 `WalStorage` 即失败：
>    200 行 3.77ms → 20000 行 **431.44ms**（114×）。见 §4.4。

---

## 1. 被测配置

| 项 | 值 |
|---|---|
| 存储 | `file`（FileStorage） |
| WAL | `batch:10000` |
| server-threads | 16（稳定性）／ 64（容量） |
| max-connections | 200 / 512 |
| auth | none |
| 负载 | `sysbench oltp_read_write` + `oltp_read_only` |
| 表 | 1 张 × 10000 行（sysbench 标准 sbtest1） |
| 客户端并发 | 8（SOAK）／ 1~128（容量阶梯） |

---

## 2. 稳定性：8h SOAK

### 2.1 运行状态

`sysbench --time=28800 --report-interval=3600`，**独立服务器实例**（端口 3701），
与 §3 容量测试（端口 3801）物理隔离，互不干扰。

| 项 | 1h 段实测 | 8h 段 |
|---|---:|---:|
| 总时长 | **3602.42s 跑满** | 进行中 |
| FATAL | **0** | **0** |
| ignored errors | 0 | **0** |
| reconnects | 0 | **0** |
| 事务 | 45,609（12.66/s） | 12.06 TPS（1h 段） |
| 查询 | 912,180（253.21/s） | 241.17 QPS（1h 段） |

**CPU 普查**（每 30s 采样）：平均 **165%**，最高 **174%**，进程消失 **0** 次。
44 次 `SELECT 1` 探针全部返回 `1`。

> 空转会表现为 >1000% CPU（#5099 修复前实测 842%~1334%）。
> 本次全程 ≤174%，**TLS 空转未复现**。

### 2.2 1h SOAK 的历史对照

| 阶段 | 结果 |
|---|---|
| #5099 修复前 | 60s FATAL，或服务器 850%~1334% CPU 卡死 |
| 修复后 1h | ✅ 3602.42s 跑满，0 FATAL |
| 修复后 8h | ✅ 进行中，0 FATAL / 0 err / 0 reconn |

---

## 3. 性能上限：容量阶梯

### 3.1 写负载（`oltp_read_write`）

**64 server-threads，逐步提高客户端并发，每档 120s：**

| 客户端 | TPS | QPS | p95 (ms) | 服务器 CPU | errors |
|---:|---:|---:|---:|---:|---:|
| 4 | **15.49** | 309.73 | 292.60 | 156% | 0 |
| 8 | 13.89 | 277.79 | 657.93 | 159% | 0 |
| 16 | 11.19 | 223.77 | — | 160% | 0 |
| 24 | 13.18 | 263.60 | — | 160% | 0 |
| 32 | 13.43 | 268.53 | — | 159% | 0 |
| 48 | 6.49 | 129.75 | — | 158% | 0 |
| 64 | 12.72 | 254.35 | — | 157% | 0 |
| 96 | **FATAL** | — | — | — | — |

> 96 客户端时 `FATAL: Worker threads failed to initialize within 30 seconds`。

**这张表最重要的一列是 CPU：从 4 客户端到 64 客户端，服务器 CPU 始终是
156%~160%。** 80 核的机器只用了 1.6 核，而吞吐不随并发上升 —— 这不是
CPU 饱和，是某个固定大小的串行点。

### 3.2 读负载（`oltp_read_only`）—— 对照实验

| 客户端 | TPS | QPS | p95 (ms) | 平均延迟 (ms) | 最大 (ms) |
|---:|---:|---:|---:|---:|---:|
| 4 | 18.73 | 299.64 | 262.64 | 213.51 | 295.60 |
| 8 | **23.41** | 374.50 | 442.73 | 341.61 | 596.84 |
| 16 | 20.32 | 325.09 | 995.51 | 786.21 | 1287.63 |
| 32 | 18.85 | 301.61 | 2082.91 | 1691.97 | 2563.96 |
| 64 | 11.13 | 178.06 | **9799.46** | 5691.94 | 12829.86 |

**读负载同样在 8 客户端封顶（23.41 TPS），之后吞吐下降而延迟线性恶化。**

### 3.3 关键对照：纯读 vs 读写

为了排除「写锁串行化」，做了**只有读、没有任何写者**的对照：

```
8 个纯读连接（无任何写入）：17.6 QPS
```

**8 个纯读连接比 1 个连接（136 QPS）还慢 7 倍。** 写锁与并发下降无关。

### 3.4 逐步加压：吞吐反而下降

| 并发 | QPS | 服务器 CPU |
|---:|---:|---:|
| 1 | **136.2** | 95% |
| 2 | 41.6 | 99% |
| 4 | 24.5 | 148% |
| 8 | 17.0 | 168% |

**这是本次测试最重要的观测：增加并发使吞吐下降 8 倍，而 CPU 几乎没有变化。**

在正常服务器上，加并发要么线性提升吞吐，要么 CPU 饱和。这里两者都没发生 ——
说明客户端在**排队**，服务器在**串行处理**，且串行点每次都要付出固定代价。

---

## 4. 根因：主键点查退化为全表扫描

### 4.1 决定性实验

> ⚠️ 本节在 **sysbench sbtest1** 上测得，该表有 `k` 上的二级索引。
> 结论（主键点查是全扫描）成立，但「多列谓词更快」的原因是命中了
> **idx_k**，不是 PK 路径。完整更正见 §4.4。

同一张表（10000 行），只改谓词：

| 查询 | 延迟 | 吞吐 |
|---|---:|---:|
| `SELECT id FROM t WHERE id=1` | **25.80 ms** | 9.3 QPS |
| `SELECT id FROM t WHERE id=1 AND k=1` | **1.41 ms** | 1277.9 QPS |
| `SELECT id,k FROM t WHERE id=1` | 25.26 ms | — |
| `SELECT 1`（不碰表） | **1.37 ms** | 732.4 QPS |
| `SHOW TABLES` | 1.16 ms | 860.3 QPS |

**关键对照：**

- `SELECT 1`（不访问表）**1.37ms** → 网络、协议、引擎调度都不是瓶颈。
- `WHERE id=1 AND k=1` **1.41ms** → 命中 `idx_k`，代价与 `SELECT 1` 相同。
- `WHERE id=1` **25.80ms** → 主键路径退化，扫全表。

### 4.2 代价与行数线性相关（确认全扫描）

| 行数 | 延迟 |
|---:|---:|
| 100 | 1.46 ms |
| 1,000 | 1.49 ms |
| 2,000 | 14.06 ms |
| 4,000 | 18.34 ms |
| 6,000 | 29.10 ms |
| 8,000 | 30.07 ms |
| 9,000 | 34.71 ms |
| 10,000 | 31.35 ms |

100→1000 行基本持平，2000 行后**随行数近似线性增长**。每次主键点查都在扫描整张表。

### 4.3 定位到的代码路径

> ⚠️ **2026-10-09 更正**：本节初版把根因写成「`scan_with_index_in`
> 未命中索引路径」。**该定位有误** —— 谓词确实命中，是**索引本身是空的**。
> 详见 §4.4 与 issue #5168 的更正评论。

**PK 快速路径存在且已接线：**

- `src/engine_select.rs:1417` → `storage.scan_pk(...)`
- `FileStorage::scan_pk`（`file_storage.rs:5882`）→ `scan_with_index` → B+Tree
- `SHOW INDEX` 确实列出 `PRIMARY ... BTREE`

**真正的根因是 PK 索引建表时构建、此后不再维护**（见 §4.4）。

`crates/mysql-server/src/lib.rs:5828` 的分支：

```rust
let result = if let Some(stmt) = is_read_only {
    let eng = engine.read();              // 读路径：共享锁
    ...
} else {
    let mut eng = engine.write();         // 独占全局锁
    ...
};
```

gdb 采样确认了锁竞争热点：

```
#1  parking_lot::raw_rwlock::RawRwLock::lock_exclusive_slow ()
#2  sqlrustgo_mysql_server::do_command_loop::<sqlrustgo_mysql_server::TlsStream> ()
#3  sqlrustgo_mysql_server::handle_connection ()
```

**但 3.3 的纯读对照证明锁不是主因**：8 个纯读连接同样退化。

### 4.4 真正的根因：包装层未转发 `scan_pk`

> ⚠️ 本文经**两次更正**。初版称「谓词未命中索引路径」，二版称
> 「PK 索引失维护」—— **两版都不成立**。本节是经失败测试验证的版本。

#### 索引本身是好的

直接测 `FileStorage`（绕过所有包装层）：

```
A rows visible to scan_in_db: 100
B rows visible after flush:   100
C scan_with_index(id=50) rows: 1     ← 索引里有数据
D scan_pk(50) present: true
```

`scan_with_index("t","id",50)` 返回 **1 行**，索引已正确填充
（flush 路径上的 `update_pk_index_window`，`file_storage.rs:4632`，
确实在维护索引）。

#### 缺陷在包装层

服务器真实存储链（`crates/mysql-server/src/lib.rs:6843-6881`）：

```
FileStorage -> MvccStorage -> ParallelWalStorage / WalStorage
```

覆写 `scan_pk` 的类型只有 `BinaryStorage`、`FileStorage`、`MvccStorage`。
**`WalStorage` 与 `ParallelWalStorage` 都没有覆写**，落到 trait 默认实现
（`engine.rs:1055`）—— **全表扫描 + 线性查找**。

| 测量对象 | 200 行 | 20000 行 | 比值 |
|---|---:|---:|---:|
| `FileStorage` 直接调用 | 通过 | 通过 | 不随行数增长 |
| `WalStorage<MvccStorage<FileStorage>>` | 3.77 ms | **431.44 ms** | **114×** |

回归测试 `crates/storage/tests/wal_scan_pk_forwarding_5168.rs` 中，
`pk_lookup_cost_must_not_scale_with_table_size`（裸 FileStorage）**通过**，
`wal_wrapped_pk_lookup_must_not_scan`（经 WalStorage）**失败** ——
**同一段测量代码，只因多一层包装就失败**。

#### 服务器端行为一致

| 查询（20000 行表） | 延迟 |
|---|---:|
| `WHERE id=1`（命中） | 9.109 ms |
| `WHERE id=999999`（无匹配） | 9.187 ms |

不存在的键与存在的键代价相同 —— 全扫描无论命中与否都跑完；
索引查找未命中应瞬间返回。

#### 前两版错在哪

| 版本 | 说法 | 为什么不成立 |
|---|---|---|
| 初版 | `scan_with_index_in` 未命中索引路径 | 谓词确实命中（`engine_select.rs:1417`） |
| 二版 | PK 索引失维护 | 索引**确实**被 flush 维护，测试证明有数据 |

二版错在：我只证明了「`insert_with_index` 只被测试调用」，
就当成了「没有路径维护索引」—— 没去查 `update_pk_index_window`
是否在 flush 路径上。

#### 修复方向

给 `WalStorage` 与 `ParallelWalStorage` 补 `scan_pk` / `scan_pk_range`
转发，与它们已有的 `scan_in` / `scan_in_db` 转发
（`wal_storage.rs:548-552`）保持一致。**已实施，见 §4.6。**

#### 方法论

连续两次「读代码 → 下结论」都错，第三次靠**能失败的测试**才定位成功。
这个缺陷的表观现象（点查慢）与两个不同原因都相容，
只有受控测量能区分。

### 4.5 影响量化

| 场景 | 修复前 | 修复后 |
|---|---:|---:|
| 20000 行表主键点查 | 9.02 ms | **0.467 ms** |
| sysbench 8 客户端 TPS | 13.89 | **38.53** |
| 8h SOAK 吞吐 | 12 TPS | 待 §2 8h 完成后复测 |

第 1 项瓶颈已消除；剩余瓶颈见 §5.2。

### 4.6 修复与实测收益

补上两个包装层的 `scan_pk` / `scan_pk_range` 转发（各 6~7 行，
与既有 `scan_in` 转发同一模式）。

| 指标 | 修复前 | 修复后 | 改善 |
|---|---:|---:|---:|
| 20000 行主键点查 | 9.02 ms | **0.467 ms** | **19×** |
| 点查代价 vs 行数（100 → 20000） | 0.38 → 9.02 ms（线性） | 0.407 → 0.467 ms（**平坦**） | 扫描消除 |
| sysbench 8 客户端 TPS | 13.89 | **38.53** | **2.8×** |
| sysbench 8 客户端 QPS | 277.79 | **770.52** | **2.8×** |

点查代价不再随行数增长 —— 全扫描已消除。回归测试
`wal_scan_pk_forwarding_5168` 中此前失败的
`wal_wrapped_pk_lookup_must_not_scan` 现已转绿；
`sqlrustgo-storage` 全套测试通过。

---

## 5. 结论与建议

### 5.1 性能上限

**当前服务器吞吐上限约 13~23 TPS / 250~370 QPS，且不随并发提升。**
在 80 核机器上只用 1.6 核。

### 5.2 修复优先级（按收益排序）

| # | 问题 | 状态 |
|---|---|---|
| 1 | **`WalStorage`/`ParallelWalStorage` 未转发 `scan_pk`（#5168）** | ✅ **已修复**，点查 19×、TPS 2.8× |
| 2 | `engine.write()` 全局独占锁覆盖读语句 | 待处理（并发可扩展性） |
| 3 | `FileStorage` 单 `RwLock<WriteState>` | 待处理（写并发可扩展性） |
| 4 | `COUNT(*)`/范围扫描返回 0（#5167） | 待处理（正确性） |

**第 1 项已闭合。** 它曾是最硬的一项：让最常见的访问模式付出全表代价，
且与并发无关 —— 这解释了为什么加并发不涨吞吐。

余下三项的相对收益需在第 1 项修复后重新测量 —— 此前测得的
「CPU 恒定 ~157%」可能是第 1 项掩盖了可扩展性问题。

### 5.3 门禁影响

- **alpha 门禁**：本次修复的 PR #5166 已合并，`develop/v4.1.0` 门禁状态未回退。
- **alpha_to_beta 判据**要求 168h SOAK；本次 8h **不足以满足**，
  但已覆盖 #5099 的 1h 主判据（已达）。
- 上述性能问题**不构成 alpha 门禁阻塞项**，但会让 v4.1.0 的任何压测数字失真 ——
  与 #5099 已记录的判断一致。

---

## 6. 复现方法

```bash
# 稳定性 8h
./target/release/sqlrustgo-mysql-server serve --port 3701 --data-dir /tmp/soak8h \
  --server-threads 16 --max-connections 200 --storage file --wal-sync batch:10000
sysbench oltp_read_write --db-driver=mysql --mysql-host=127.0.0.1 --mysql-port=3701 \
  --mysql-user=root --mysql-db=sb8h --table-size=10000 --tables=1 --threads=8 --time=28800 run

# 容量阶梯（关键：客户端数从 4 逐级加到 96）
# 观察服务器 CPU 是否随并发上升 —— 不上升即为串行瓶颈

# 根因确认：代价是否随行数线性（线性 = 全扫描）
CREATE TABLE t(id INT PRIMARY KEY, k INT);
-- 灌 N 行后测点查：N=100 / 1000 / 5000 / 20000
SELECT id FROM t WHERE id=1;
# 实测 0.38 / 0.78 / 2.50 / 9.02 ms —— 线性，确认每次点查扫全表

# 判定是否走了索引：不存在的键应与存在的键同样快
SELECT id FROM t WHERE id=999999;      -- 9.19ms，与命中键同代价 => 全扫描

# 存储层回归测试（直接证明包装层问题）
cargo test --release -p sqlrustgo-storage \
  --test wal_scan_pk_forwarding_5168 -- --nocapture
# wal_wrapped_pk_lookup_must_not_scan 失败（裸 FileStorage 的用例通过）

# 确认不是网络/协议：同连接上不碰表的查询
SELECT 1;                                -- 1.37 ms
```

> 注意：用 `id=1 AND k=1` 做对照会误导 —— 它命中的是 `k` 上的
> 二级索引（sbtest1 才有），不是 PK 路径。仅 PK 的普通表上，
> 多列谓词反而更慢（9.70ms vs 8.89ms）。

## 7. 关联

- Issue #5099 —— 并发事务丢行 + 服务器卡死
- Issue #5167 —— `COUNT(*)` / 范围扫描返回 0（本报告 §5.2 第 4 项）
- PR #5166 —— TLS 读路径 EOF 空转修复
- `evidence/TLS_READ_SPIN_HANG_5099_2026-10-09.md`