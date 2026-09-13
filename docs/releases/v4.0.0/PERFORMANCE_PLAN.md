# sqlrustgo 性能优化执行计划 (2026-09-13)

> **背景**: v4.1.0 / Phase A PK fast-path SOAK 完成后，定位出**真正的瓶颈**。
> 本计划基于 2026-09-13 的 `.sample` profile 结果 + 8h SOAK + 并行对照，
> 给出有依据的执行序列。**不承诺**每一步都显著涨 TPS — 每一步后必须
> 用并行 SOAK (NEW vs OLD) 验证净收益。

---

## 1. 已完成的诊断 (Phase A + Phase B)

### Profile (sysbench oltp_read_only, 4 threads, 5 sec `sample`)

| 函数 | 样本数 | 含义 |
|------|-------:|------|
| `execute_select` | 10,280 | 实际 SELECT 执行 |
| **`parking_lot lock_shared_slow`** | **10,025** | **reader 等待 writer** |
| `lock_exclusive_slow` | 2,213 | writer 等待 reader |
| `parking_lot wait_for_readers` | 19 | writer 等 reader 排空 |

**关键结论**: 锁竞争来自 engine 的 `begin_transaction` / `commit_transaction`
路径，每次 BEGIN/COMMIT 都会调用 `self.storage.write()`，期间**所有其他
连接的 SELECT 都阻塞**（`storage.read()` 等不到）。

### 存储现状

```rust
// crates/mysql-server/src/lib.rs:5869-5915 (创建一次, 所有连接共享)
let storage: Arc<RwLock<BoxStorageEngine>> = Arc::new(
    parking_lot::RwLock::new(BoxStorageEngine::new(...))
);
```

```rust
// src/execution_engine.rs:1851 (BEGIN 时, 全局写锁!)
let mut storage = self.storage.write();   // <-- 阻塞所有 reader
{
    storage.set_current_tx_id(tx_id.as_u64());   // 普通赋值
    let _ = storage.begin_transaction();           // WAL append (有内部锁)
}
```

### Phase A 已完成 (本会话之前, commit 944df666f 等)

- `scan_pk_eq` trait 方法 + FileStorage/MemoryStorage 实现 + 7 单元测试
- `engine_select_pk::try_extract_pk_eq` helper
- 接线 → 验证 → **回滚** (8h SOAK 测得 2.8x RSS, 6min warmup, 0 TPS 收益)
- 真实瓶颈不是 SELECT alloc 而是全局 write lock

### 已交付的文档 (本会话, commit cfc904544)

- `PHASE_B_INVESTIGATION.md` — 完整 profile 数据 + 各项时间估算
- `PHASE_A_PK_STATUS.md` (v4.1.0-pk-lookup @ 944df666f) — 已禁用接线 + 完整测量

---

## 2. 建议的执行顺序 (按 ROI 排序)

### Step 1 ✅ 已完成: 文档与基线
- 提交 cfc904544: PHASE_B_INVESTIGATION.md (5 远端同步 ✓)
- **基线**: oltp_read_only, 4 threads, table=1000 → **154 TPS** avg (3 次)

### Step 2: 短期高 ROI 优化 (`.250` 已完成 3 个相关修复)

#### 2a. `7bc437f4d` WalStorage::delete O(N)→O(1) (在 .250)
- **路径**: `crates/storage/src/wal_storage.rs`
- **做法**: 把 `Vec::remove` 换成 swap_remove + 维护 id→index 映射
- **验证**: parallel SOAK before/after, table=10000 (size matters)

#### 2b. STMT PREPARE 协议修复 (`.250` 已完成 2 个)
- `5bc22824a` 第三个pass: STMT PREPARE emit trailing lenenc(info)
- `368222945` 第二个pass: STMT PREPARE skip terminator under CLIENT_DEPRECATE_EOF
- **影响**: `mysql --table` 输出, COM_STMT_PREPARE 协议兼容性
- **验证**: 兼容性测试 (./test/integration 跑一遍)

#### Step 2 验证方式
- cherry-pick 这 3 个提交到本地 (`git fetch gitea250 && git cherry-pick 7bc437f4d`)
- rebase 推 5 远端
- parallel SOAK NEW vs OLD (oltp_read_write, 8 threads, 4×10000, 20 min)

### Step 3: 中期 ROI — 消除 `begin_transaction` 的全局写锁

#### 3a. 让 `WalStorage::begin_transaction` / `commit_transaction` / `rollback_transaction` 用内部锁 (`Mutex<HashMap>` + `AtomicU64`)
- **路径**: `crates/storage/src/wal_storage.rs`
- **改动**: `current_tx_id: u64` → `AtomicU64`, `active_txs: HashMap` → `Mutex<HashMap>`, `next_lsn: u64` → `AtomicU64`
- **trait**: 添加 `begin_transaction_lockfree(&self, tx_id)` / `commit_transaction_lockfree(&self)` / `rollback_transaction_lockfree(&self)`, 默认实现返回 Err
- **引擎**: `src/execution_engine.rs:1851` 用 `storage.begin_transaction_lockfree()` 替换 `storage.write()`
- **预计收益**: 消除 `10,025` 个 `lock_shared_slow` 样本 → 直接放大 reader 并行度

#### 3b. 同样改造 `FileStorage::begin_transaction` / `commit_transaction` (如果仍然在路径上)
- 同上, atomic + Mutex

#### Step 3 验证方式
- 必须 ALL 138 lib tests + AHI tests + 51 executor tests 通过
- parallel SOAK NEW vs OLD (oltp_read_only, 4 threads, table=1000, 20 min)
- 期望: NEW TPS ≥ 250 (reader 完全并行, 无 write lock 阻塞)
- profile 再次 `sample`, 确认 `lock_shared_slow` 显著下降

### Step 4: 长期 ROI — MVCC for reads (Phase B.2)

#### 4a. 给 `FileStorage::tables: HashMap<String, TableData>` 加版本号
- 每行加 `version: u64`, begin_tx 时取当前版本, commit 时 merge
- SELECT 不需要写锁 (snapshot isolation)
- 风险: undo log / savepoint / recovery 兼容

#### 4b. 对应 `crates/storage/src/wal_storage.rs` 的 active_txs 也要支持快照读
- 复杂度: 高
- 预计工作量: 8-16 小时
- **不要在 Phase B 推进时合并 — 单独 session**

### Step 5: 缓冲池 (Phase B.3, 仅当 Step 3-4 完成)

#### 5a. `crates/storage/src/buffer_pool.rs` 已存在 (page-level)
- 需要 wrapper 让 FileStorage 的 `scan()` 先查 buffer_pool
- 预计工作量: 4-8 小时

---

## 3. 不做的事 (避免重复 Phase A 的坑)

| 优化 | 不做的原因 |
|------|----------|
| AHI `access_count` 用 `AtomicU64` | 上一轮 SOAK: -0.9% (噪声内), 锁计数几乎不变 |
| Engine pool (`Phase A.1 ExecutorPool`) | 已经存在 (v4.1.0-mvp), 但**不在 .252 hot path** |
| 增加 `--executor-parallelism` | profile 已证明对 sysbench 0 影响 (170→170 TPS) |
| 改 `RwLock<ExecutionEngine>` 为 per-conn | **每个连接已经有自己的** engine (L5576) |
| BufferPool 集成到 FileStorage | 需要重设计 load 路径, 单独 session |
| MVCC for reads | 需要 redo undo/savepoint, 单独 session |

---

## 4. 风险与回滚

- **每个 Step 完成后必须**:
  1. `cargo test --lib` (138 通过)
  2. parallel SOAK NEW vs OLD (10-20 min)
  3. `.sample` profile 再跑一次, 量化变化
- **如果 TPS 改善 < 5%** → 回滚 + 文档化 + 下次重看
- **如果测试失败** → 立即 `git revert` 该 commit

---

## 5. 提交约定

- 每个 Step 一个 commit: `perf(storage): ...` 或 `perf(executor): ...`
- commit message 必须包含:
  - baseline TPS (前后对比)
  - profile 数据 (锁/内存变化)
  - 回滚方案

## 6. 已同步状态 (2026-09-13)

| 远端 | SHA |
|------|-----|
| gitea250 | `7bc437f4d` ⚠️ 已自动前进 3 commit (待同步到本地+其他远端) |
| github | `cfc904544` |
| gitcode | `cfc904544` |
| gitea252 | 暂时无法连接 (待重试) |
| origin | `cfc904544` (如果 .252 同一个 HTTP 服务, 也需要更新) |

**下一步**:
1. fetch `.250` 的 3 个新 commit
2. cherry-pick / merge 到本地 develop/v4.0.0
3. 推到 github / gitcode / .252 (恢复时)
4. 验证 + SOAK