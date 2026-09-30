# #4912 A1 性能对照测量报告

> **provenance (ADR-014 5 evidence fields):**
> - `source_agent`: deepseek-flash (DSH harness)
> - `source_run`: a1-ab-measure-2026-09-30
> - `timestamp`: 2026-09-30T21:40Z
> - `evidence_hash`: local-git:`99198a51512ab397d367570ed68b58fbed168fd5` + binaries `md5:0a132fd963e516abb9527ef86ab05e5a` (after) / `md5:ae1f7fa99cd53378498136b654cb9186` (before)
> - `conflict_resolution`: N/A — single-agent measurement
>
> **政策**: Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-008 + ADR-014
> **Status**: MEASURED
> **关联**: Issue #4912 / #4913；`PERFORMANCE_AUDIT_REPORT_2026-09-30.md` F-01

---

## 1. 测量目的

`PERFORMANCE_AUDIT_REPORT_2026-09-30.md` 的 F-01 断言：`BoxStorageEngine`
未转发 `*_lockfree`，导致每次 BEGIN/COMMIT 取全局 `Arc<RwLock<storage>>`
写锁，阻塞所有并发 SELECT。本测量用于**验证该断言是否成立**，并量化修复收益。

**这是一个 A/B 实验**：两个 release 二进制的唯一差异是
`crates/storage/src/binary_storage.rs` 中 `BoxStorageEngine` 是否覆写
`begin/commit/rollback_transaction_lockfree` 三个方法（共 1243 字符）。
其余源码、编译 profile（`[profile.release]` LTO fat / codegen-units=1）、
运行时参数完全相同。

| 二进制 | md5 | 说明 |
|---|---|---|
| `before` | `ae1f7fa99cd53378498136b654cb9186` | 回退 3 个转发方法（等价修复前状态） |
| `after` | `0a132fd963e516abb9527ef86ab05e5a` | 含 #4912 修复 |

> 可复现性证据：恢复修复源码后重新构建，产物 md5 与 `after` **逐字节一致**。

---

## 2. 工作负载

复现 `docs/releases/v4.0.0/V400_04_20MIN_SOAK.md` §3 / Issue #4910 记录的争用形态。

| 参数 | 值 |
|---|---|
| 服务器 | `target/release/sqlrustgo-mysql-server serve` |
| 存储 | `--storage file`（默认；走 `WalStorage`，与基线 profile 同一路径） |
| WAL 同步 | `--wal-sync every`（默认，每事务 fsync） |
| 认证 | `--auth-mode none` |
| 连接线程池 | `--server-threads 32` |
| 并发连接 | **8** |
| 每连接事务数 | 150 |
| 事务形态 | `BEGIN` → `SELECT id FROM a1_bench WHERE id=1` → `COMMIT` |
| 总事务数 | 1,200 / 轮 |
| 每二进制轮次 | 1 轮 profiled + 3 轮计时 |
| 错误数 | before 0 / after 0 |

**驱动**: `/tmp/a1bench/measure_a1.py`（自写最小 MySQL 协议客户端，
按 phase 分别记录 BEGIN / SELECT / COMMIT 的墙钟延迟）。

**采样**: 运行期间用 macOS `sample <pid> 3` 抓取服务器 CPU 栈
（与 #4910 相同方法）。

---

## 3. 结果

### 3.1 吞吐（3 轮中位数）

| 指标 | before | after | 提升 |
|---|---:|---:|---:|
| **TPS** | **402.0** | **6,846.2** | **17.0x** |
| 每轮 TPS | 372.5 / 418.1 / 402.0 | 6944.2 / 6846.2 / 6736.2 | — |
| 墙钟（1200 txns） | 2.87–3.22 s | 0.17–0.18 s | — |

### 3.2 分阶段延迟（3 轮中位数，毫秒）

| 阶段 | 分位 | before | after | 提升 |
|---|---|---:|---:|---:|
| BEGIN | p50 | 4.203 | 0.231 | **18.2x** |
| BEGIN | p95 | 18.987 | 0.716 | **26.5x** |
| BEGIN | p99 | 24.242 | 1.017 | **23.8x** |
| SELECT | p50 | 4.256 | 0.240 | **17.8x** |
| SELECT | p95 | 18.553 | 0.732 | **25.3x** |
| SELECT | p99 | 23.977 | 0.977 | **24.5x** |
| COMMIT | p50 | 4.040 | 0.238 | **16.9x** |
| COMMIT | p95 | 19.201 | 0.694 | **27.7x** |
| COMMIT | p99 | 24.132 | 0.945 | **25.5x** |

### 3.3 CPU 栈证据（`sample`）— 最直接的一项

| 栈帧 | before | after |
|---|---:|---:|
| `parking_lot` 相关帧 | 54 | **0** |
| `lock_shared_slow`（读者等写者） | 36 | **0** |
| `lock_exclusive_slow`（写者等读者） | 18 | **0** |
| `wait_for_readers` | 0 | 0 |

**after profile 中 parking_lot 锁帧完全消失。** 这正是 F-01 预测的机制：
BEGIN/COMMIT 不再进入全局锁路径后，读者不再等待写者。

---

## 4. 解读与边界

### 4.1 为什么 SELECT 延迟也改善 17.8x

这是本修复的核心机制。修复前 `--storage file` 路径下：

1. `ExecutionEngine::begin_transaction`（`src/execution_engine_methods.rs:1513`）
   探测 `begin_transaction_lockfree` → `BoxStorageEngine` 未转发 → 落到
   trait 默认 `Err`；
2. 于是走 fallback：`self.storage.write()` 取**全局写锁**；
3. 持锁期间其他连接的 SELECT（走 `storage.read()`）全部阻塞。

10 个并发连接里只要有 1 个在做 BEGIN 或 COMMIT，其余 7 个的 SELECT 就排队。
这解释了为什么"读"的 p50 也会涨到 4.2 ms。

### 4.2 本测量**不能**证明的事

- **不代表 `--wal-sync every` 的绝对性能达到生产可用水平。** 无锁路径内部
  仍有 `Mutex<wal>` 串行化所有 WAL append
  （`crates/storage/src/wal_storage.rs:1002`），且每次 COMMIT 仍 `sync_data()`。
  修复消除的是**全局 `RwLock` 争用**，不是 fsync 成本。17x 中相当一部分
  来自"锁等待"被消除，而 fsync 仍是串行的。
- **未测 MVCC / `--storage parallel` 路径。** 本测量固定用默认 `file`。
- **未做 8 小时以上 SOAK。** 这是短时（每轮 0.2–3.5 s）定向测量，
  无内存/长稳结论。
- **未测多核扩展曲线。** 只测了 8 连接一个点。
- **未验证 `--storage binary`。**
- **未做统计显著性检验。** 3 轮样本，但 before/after 差异幅度（17x）
  远超运行间波动（before 372–418，after 6736–6944），方向明确。

### 4.3 与 #4910 的关系

#4910 主张 `ExecutionEngine::execute(&mut self)` 是吞吐上限。本测量表明：
**在 F-01 修复之前，engine 边界问题被存储层全局锁掩盖了**——
17x 的收益完全来自移除一个接线 bug，未改动 `&mut self` 签名。

这不否定 #4910，而是给它一个正确的基线：应在本次修复后再测量，才能
判断 `&mut self` 拆分是否仍有独立收益。

### 4.4 断言与实测的偏差核对

| 审计报告原文 | 实测 | 结论 |
|---|---|---|
| "每次 BEGIN 与 COMMIT 都取全局写锁，持锁期间所有其他连接的 SELECT 阻塞" | SELECT p50 4.256ms → 0.240ms | **成立** |
| "机制上完全解释 #4910 记录的 profile：97.5% 的读路径样本落在锁等待中" | before profile 有 36 个 `lock_shared_slow` 帧；after 为 0 | **成立**（同一锁定机制；帧数绝对值不可直接对比 #4910 的 10,025 样本，因采样时长与负载不同） |
| "零个生产调用者"（F-14 BufferPool） | 未在本测量验证 | 未涉及 |

---

## 5. 结论

F-01 的机制判断**被实测确认**，且收益量级远超审计报告"NOT-MEASURED"时的预期：

- **TPS 17.0x**（402 → 6,846）
- **SELECT p99 24.5x**（23.977 ms → 0.977 ms）
- **`sample` profile 中 parking_lot 锁帧 54 → 0**

同时必须记录边界：该收益来自**消除全局锁争用**，不是消除 fsync；
`Mutex<wal>` 串行化仍在（建议在 #4915 一并评估）。

## 附录 — 原始数据

计时 JSON、`sample` 原始输出、服务器日志位于测量工作目录
（`/tmp/a1results/`，含 `before_run{1,2,3}.json`、`after_run{1,2,3}.json`、
`before_sample.txt`、`after_sample.txt`、`*_server.log`）。
驱动脚本：`/tmp/a1bench/measure_a1.py`、`/tmp/a1bench/run_ab.sh`。

复现命令：

```bash
# 构建 after
cargo build --release -p sqlrustgo-mysql-server
# 构建 before（回退 binary_storage.rs 中 3 个转发方法后）
cargo build --release -p sqlrustgo-mysql-server
# 测量
bash /tmp/a1bench/run_ab.sh before <before-bin> 3402 8 150 /tmp/a1results
bash /tmp/a1bench/run_ab.sh after  <after-bin>  3401 8 150 /tmp/a1results
```
