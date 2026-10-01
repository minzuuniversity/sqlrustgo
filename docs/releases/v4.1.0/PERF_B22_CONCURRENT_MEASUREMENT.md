# Phase B2.2 / #4915 — 并发证据补测报告

> **provenance (ADR-014 5 evidence fields):**
> - `source_agent`: deepseek-flash (DSH harness)
> - `source_run`: b22-concurrent-2026-10-01
> - `timestamp`: 2026-10-01T11:35Z
> - `evidence_hash`: local-git:`3d503b5ab09263be3eb068d8d0d47df43e119dad` (develop/v4.1.0)
> - `conflict_resolution`: N/A — 本报告**补充**而非否定 `PERF_B2_4915_AB_MEASUREMENT.md` §4 B2.2 的自我声明
>
> **政策**: Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-008 + ADR-014
> **状态**: MEASURED — 结论为**机制性否定 + 性能中性**，不是收益证明
> **关联**: #4915；`PERF_B2_4915_AB_MEASUREMENT.md` §6「本次没有测到的」第 1 条

---

## 1. 目的

`PERF_B2_4915_AB_MEASUREMENT.md` §6 明确写道：

> **并发 / TPS**：sysbench `oltp_read_write` 8 threads 未跑。B2.2 的全部意义
> 在于并发读延迟，**缺这一项等于 B2.2 未被证明**。

本报告即补测该项。做法不是 sysbench，而是直接针对存储层的并发读延迟 A/B，
因为 B2.2 修改的是 `FileStorage::flush` 的锁持有范围，只有存储层测量能归因。

---

## 2. 被测差异

两侧二进制唯一差异为 `crates/storage/src/file_storage.rs` 中
**inherent** `FileStorage::flush()` 的实现：

| 侧 | 实现 | 锁持有范围 |
|---|---|---|
| **before** (`be665d6bc1`) | 整个 dirty 循环在 `with_write_lock` 内，`save_table` 逐表序列化+`write()` | 序列化 + 磁盘 I/O **全部在锁内** |
| **after** (`e2355c0680` + `ff34478830`) | 短临界区取 `TableData::snapshot_from(last_saved)` 窗口 → 释放锁 → `save_table_window` | 仅窗口拷贝在锁内 |

---

## 3. 决定性发现：B2.2 修改的方法**没有生产调用者**

`FileStorage` 有两个 `flush`：

| | 位置 | 内容 |
|---|---|---|
| **inherent** `pub fn flush(&self)` | `crates/storage/src/file_storage.rs:946` | **B2.2 修改的就是这个** |
| **trait 覆写** `fn flush(&mut self)` | `crates/storage/src/file_storage.rs:4293` | **实际被调用的那个** |

trait 覆写（`StorageEngine::flush`）已经是「锁内只 drain dirty 名字、I/O 在锁外」：

```rust
fn flush(&mut self) -> SqlResult<()> {
    let dirty: Vec<String> = Self::with_write_lock(self.as_mut_self(), |s| {
        std::mem::take(&mut s.dirty_tables).into_iter().collect()
    });
    self.flush_all_buffers()?;
    for name in dirty {
        if let Some(table_data) = self.tables.get(&name).cloned() {   // ← 整表 clone
            self.save_table(&name, &table_data)?;                      // ← 锁外 I/O
        }
    }
    Ok(())
}
```

**证据（本次实测）**：在 inherent `flush()` 入口插入标记后运行并发/墙钟两个
测量，标记**一次都没有打印**，而在 `MvccStorage::flush` 插桩则打印了 21 次：

```
B22_MVCC_FLUSH    : 21
B22_FLUSH_ENTERED : 0     # inherent FileStorage::flush 入口标记
```

即经 `MvccStorage`（服务器实际使用的包装）调用时，走的是 trait 覆写，
inherent 版本被完全绕过。（插桩已全部移除，未进入提交。）

**推论（须与事实区分）**：
- **事实**：B2.2 修改的路径在生产调用链上不可达。
- **事实**：inherent `flush()` 在仓库内唯一的调用点是
  `file_storage.rs:4842`，位于 `flush_parallel` 内部（3+ dirty 表分支）。
- **事实**：trait 覆写**已经**把 I/O 放在锁外，所以 B2.2 想消除的
  「持锁期间做磁盘 I/O」在**活路径上本来就不存在**。
- **推论**：因此 B2.2 **不可能**带来 B2 文档所期待的那种并发读延迟收益。
  下面的测量与这一推论一致。

> **注**：trait 覆写里仍留着 `self.tables.get(&name).cloned()` 的**整表 clone**——
> 正是 `PERF_B2_4915_AB_MEASUREMENT.md` §4 把 B2.2 首版判为 0.86x 回归的同一个
> 模式。该 clone 在活路径上仍然存在，只是发生在锁外。

---

## 4. 测量与结果

两个 harness 均为 `#[ignore]` 的测量，未纳入常规测试：

- `crates/storage/tests/b22_concurrent_flush_ab.rs` — 4 读者线程 + 1 写者线程，
  共享 `Arc<RwLock<MvccStorage<FileStorage>>>`，读者在 `scan` 期间持读锁。
- `crates/storage/tests/b22_flush_wall.rs` — 单线程，20 轮 × 2000 行增量后
  显式 `flush()`，测墙钟 p50。

两侧各自独立构建：`cargo test -p sqlrustgo-storage --release --test <name> --no-run`。

### 4.1 并发读者延迟（6 次运行，每次约 2800 样本）

| 侧 | p50（6 次） | p95（6 次） | p99（6 次） |
|---|---|---|---|
| before | 4.371–5.156 ms | 9.437–13.618 ms | 10.863–32.985 ms |
| after | 4.457–4.679 ms | 9.426–9.916 ms | 10.697–17.700 ms |

中位对比：p50 4.52 vs 4.66 ms、p95 9.88 vs 9.81 ms。**区间高度重叠**，
p99 在两侧都有一个 17–33 ms 的离群点。

**结论：并发读延迟差异落在噪声内，不构成 B2.2 的收益证据。**

### 4.2 flush 墙钟时间（各 10 次运行）

| 侧 | p50-of-runs | min | max |
|---|---|---|---|
| before | **7.273 ms** | 6.563 | 8.815 |
| after | **7.146 ms** | 6.731 | 8.230 |

**结论：flush 墙钟时间中性（after/before = 0.982x），区间完全重叠。**

> **过程更正（如实记录）**：首轮只跑 3 次，得到 before 7.1 ms vs after 9.3 ms
> （约 0.76x），一度看似回归。把样本提到各 10 次后该差异消失。
> **3 次采样不足以判定本项**——记录此点，以免后续把噪声当回归。

---

## 5. 结论

| 问题 | 结论 |
|---|---|
| B2.2 是否缩短了活路径的持锁时间？ | **否** — 活路径（trait 覆写 `:4293`）本来就已把 I/O 放在锁外 |
| B2.2 是否改善了并发读延迟？ | **未观测到** — 6 次运行两侧区间重叠 |
| B2.2 是否拖慢了 flush？ | **否** — 墙钟中性（0.982x，n=10/侧） |
| B2.2 是否有可测收益？ | **无**（在其当前覆盖面内） |

### 建议（不擅自实施）

1. **在 #4915 中如实标注 B2.2 为「机制性 no-op（针对活路径）」**，而非
   「收益未证明」。二者不同：前者是可达性问题，后者是数据不足问题。
2. **真正的优化点**是 trait 覆写 `:4293` 里的 `self.tables.get(&name).cloned()`
   整表 clone。它与 inherent 版本已被修掉的模式相同，且在活路径上。
   建议改为 `snapshot_from(last_saved)` + `save_table_window`（同一套已验证
   的 API），收益与 B2.1 同族。
3. **建议合并两个 `flush`**（inherent 与 trait 覆写），消除这种「改错方法」
   的类别风险——本次测量揭示的根因是两个同名实现并存且行为不同。
4. **`flush` 的可达性本身值得复核**：本次核查未能找到从 SQL 到
   `StorageEngine::flush` 的正常路径（`commit_transaction_lockfree` 不 flush；
   `commit_transaction_and_flush` 无调用者；`ExecutionEngine::flush` 无调用者）。
   若属实测，则 B2.2 连同建议 2 的收益都只在进程退出时兑现一次。

---

## 6. 未测量 / 边界

- 未测 `--profile bench` 下的 criterion 数据（本次用独立 release 测试二进制）。
- 未测 3+ dirty 表的 `flush_parallel` 分支（inherent `flush()` 唯一调用点）。
- 未测 RSS / 内存峰值——**B2.3 报告已指出其真实收益在内存而非墙钟**，
  本次同样未量化，不能替它下结论。
- 并发 harness 的读者成本被 `scan` 的整表 clone 主导（约 28000 行），
  这抬高了 p50 并可能掩盖锁效应。若需更强结论，应让读者持读锁做
  **固定小查询**而非全表 `scan`。
- 样本量：并发 6 次、墙钟 10 次/侧。未做统计显著性检验。

## 附：原始数据

- 并发 before：`/tmp/b22conc_before.txt`（6 行）
- 并发 after：`/tmp/b22conc_after.txt`（6 行）
- 墙钟 before：`/tmp/b22wall_before.txt`（10 行）
- 墙钟 after：`/tmp/b22wall_after.txt`（10 行）
- 测试二进制：`/tmp/a1bench/b22_test_{before,after}`、`/tmp/a1bench/b22wall_{before,after}`

复现：把 `crates/storage/src/file_storage.rs` 的 inherent `flush()` 换成
`be665d6bc1` 版本即得 before 侧，其余源码不变。
