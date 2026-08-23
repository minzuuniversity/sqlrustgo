# TPC-H SF=1 Data Loading — 设计规范

> **实现模式:** superpowers:writing-plans → superpowers:subagent-driven-development
>
> **目标:** 解决 SF=1 TPC-H 数据载入耗时痛点（lineitem 6M 行从 ~10 小时加速到 < 60 秒），
> 通过将生产默认存储从 JSON（`FileStorage`）切换到 BINT v3 二进制格式
> （`BinaryTableStorage` 的流式 append 版本），同时保留 JSON 读取兼容路径。
>
> **范围:** 1) 新增 BINT v3 段文件格式与流式 row writer；2) 多段合并 compactor；
> 3) 写时迁移 JSON → BIN（`.json.bak` 永久归档）；4) 性能验证。**不做**在线热迁移
> 服务、不动 WAL 与事务模型、不动 query planner。
>
> **关联背景:**
> - `docs/releases/v3.11.0/perf/DATA_LOADING_ANALYSIS.md`（已识别的瓶颈分析）
> - `crates/storage/src/file_storage.rs:368-388`（JSON 全表序列化瓶颈）
> - `crates/storage/src/binary_storage.rs`（已有 BINT v2 实现可借鉴）
> - `crates/storage/src/append_only_storage.rs`（可借鉴的 LSM 思路）

## 设计决策摘要

| 维度 | 选择 | 理由 |
|---|---|---|
| 存储格式 | **BINT v3**（16 KB 页对齐段文件 + 行级 CRC32C） | 复用 BufferPool page size；流式 append 无全表 clone；已有 v2 实现可演进 |
| 默认存储 | **BIN 替代 JSON** 作为生产默认 | 10-30× 加速主要来源 |
| 迁移触发 | **写时 lazy migration**（on-first-INSERT） | 零停机；启动期不做迁移避免崩溃窗口 |
| JSON 兼容 | **保留 JSON 读，写时迁移 + `.json.bak` 归档** | 用户已选"保留 JSON 读取" |
| 性能目标 | **lineitem SF=1 < 60s**（对齐 SQLite） | 用户已选 SQLite 对齐 |
| WAL 模式 | **LOAD DATA 强制 Batch 模式** | 600 次 fsync → 1 次 fsync ≈ 480× 加速 |
| 锁模型 | **段级 `append_lock` + `read_lock`** | 读写可并发；不同段之间无锁竞争 |
| 实施方式 | **feature flag `bin_storage_default`** | 默认 OFF 跑 1 周再切 ON；保留回滚能力 |

## 1. 架构总览

### 当前路径（瓶颈）

```
LOAD DATA → bulk_insert_records → FileStorage::insert_buffered
   → buffered.len() ≥ 10_000? → FileStorage::flush_buffer
   → FileStorage::save_table
       → 整表 .clone() + serde_json::to_string_pretty + 一次性 write_all
       → 690 MB JSON × 600 次
       → 总计 ~200 GB 序列化 + ~200 GB 磁盘写入
```

### 目标路径

```
LOAD DATA → bulk_insert_records → BinaryTableStorage::insert_streaming
   → append_rows_to_segment → 写到 .bin 段文件（page-aligned, ~64 MB/段）
   → 段 ≥ 阈值? → seal, 拉起新段
   → 加载结束 → 提交 root.index + WAL Batch fsync（一次）
```

### 新增模块

| 模块 | 路径 | 作用 |
|---|---|---|
| `bin_segment` | `crates/storage/src/bin_segment.rs` | 单段文件读写（streaming row writer, 16 KB 页对齐，行级 + 段级 CRC32C） |
| `bin_compactor` | `crates/storage/src/bin_compactor.rs` | 多段合并为单段快照（后台线程触发；`BinCompactor::run()` trait） |
| `bin_index` | `crates/storage/src/bin_index.rs` | 表级 `root.bin` 索引（段文件列表 + 列元数据 + 行数） |
| `binary_storage_v2` | `crates/storage/src/binary_storage_v2.rs` | 公开 API：`insert_streaming` / `scan` / `batch_get`，与 `FileStorage` 同接口 |
| `bin_migration` | `crates/storage/src/bin_migration.rs` | JSON → BIN 写时迁移；`.json.bak` 归档 |

### 兼容层（分层共存）

- **`FileStorage`**：保留并继续维护，承担 JSON 格式的读取与回滚路径
- **`BinaryTableStorage`**（v2）：新生产写入路径
- 启动期**不主动迁移**，仅在第一次 INSERT 时由 `bin_migration` 触发
- 两个后端共存于同一 `BoxStorageEngine` trait 下，由 `bin_index` 决定运行时路由
- `.json.bak` 永久归档，回滚时由 `sqlrustgo-admin storage rollback` 重建 JSON 路径

## 2. BINT v3 二进制格式规范

### 段文件布局（`.bin`，16 KB 页对齐）

```
Offset    Content
─────────────────────────────────────────────────────
0x0000    Magic "BINTv3" (8 bytes)
0x0008    Format version = 3 (u32 LE)
0x000C    Flags (u32 LE): bit0=compressed, bit1=indexed
0x0010    Creation timestamp (u64 LE, nanos since epoch)
0x0018    Column count (u16 LE)
0x001A    Row count (u32 LE, segment 内行数)
0x001E    Reserved (12 bytes)
0x0020    Schema offset (u16 LE, 相对段首)
0x0022    Data start offset (u32 LE, 默认 0x4000 = 16 KB)
0x0026    Reserved (6 bytes)
0x0028    CRC32C of header (u32 LE)
0x002C..0x3FFF    Reserved / future flags
0x4000    ┐ Row 0
          │ Row 1
          │ ...
          │ Row N-1
          │ Unused (padding to next page)
          │
last-16KB ┘ Page footer: row_count, segment_size, next_offset, CRC32C
```

### 行编码（变长/定长混合）

```
┌────────────────────────────────────────────────┐
│ RowHeader (16 bytes)                           │
│   u32: row_size │
│   u32: var_field_offset (相对行首)             │
│   u64: txn_id / row_id (LSN)                    │
│   u16: NULL bitmap (128 columns max)           │
│   u16: reserved                                │
├────────────────────────────────────────────────┤
│ Fixed-length fields (按 schema 列顺序)         │
│   INT64      → 8 bytes (raw LE)                │
│   INT32      → 4 bytes                         │
│   SMALLINT   → 2 bytes                         │
│   FLOAT      → 4 bytes                         │
│   DOUBLE     → 8 bytes (IEEE 754)              │
│   BOOL       → 1 byte                          │
│   DATE       → 4 bytes (days since 1970-01-01) │
│   CHAR(N)    → N bytes (定长)                  │
│   DECIMAL    → 16 bytes (i128 LE)              │
│   TIMESTAMP  → 8 bytes                         │
├────────────────────────────────────────────────┤
│ Variable-length fields (顺序写出)              │
│   VARCHAR(n) / TEXT / BLOB → u32 length + UTF-8 bytes │
├────────────────────────────────────────────────┤
│ Footer (4 bytes): CRC32C of row bytes          │
└────────────────────────────────────────────────┘
```

### 关键设计决策

| 决策 | 理由 |
|---|---|
| 16 KB 页对齐 | 复用 BufferPool（已有 `page_size = 16384`）；scan 时 mmap + 走 BufferPool 缓存零拷贝 |
| 定长字段直接 raw LE | 无需 deserialize；scan 时按 schema 偏移表直接 `ptr::read_unaligned` |
| 变长字段用 offset 表 | O(1) 跳到目标列，避免变长字段扫描 |
| NULL bitmap 128 位 | 一次处理 128 列（TPC-H 最多 16 列，远低于上限） |
| 行尾 CRC32C | 校验单行完整性，避免一个字节坏掉导致整段重读 |
| 段尾页 footer CRC32C | 校验段文件尾部，主要防止写入时断电导致的 torn write |

### 列类型 → 字节宽度

```rust
// crates/storage/src/bin_segment.rs
fn column_width(col: &ColumnDef) -> Option<usize> {
    use DataType::*;
    match &col.data_type {
        BigInt      => Some(8),
        Int         => Some(4),
        SmallInt    => Some(2),
        Float       => Some(4),
        Double      => Some(8),
        Bool        => Some(1),
        Date        => Some(4),
        Char(n)     => Some(*n as usize),     // 定长 CHAR(N)
        VarChar(_) | Text | Blob => None,     // 变长
        Decimal(..) => Some(16),             // i128 LE
        Timestamp   => Some(8),
    }
}
```

## 3. 数据流与并发模型

### LOAD DATA → insert_streaming 调用链

```
┌──────────────────────────┐
│ MySQL client (tcp port)  │  LOAD DATA LOCAL INFILE 'lineitem.tbl'
└──────────┬───────────────┘
           │  wire packet 0xFB
           ▼
┌─────────────────────────────────────────────┐
│ handle_load_local_infile (lib.rs:3970)      │
│   - whitelist check                          │
│   - 循环 read content packets                │
│   - parse_tbl_line() → Vec<Record>           │
│   - rows_per_flush=10_000 时 chunked batch   │
└──────────┬───────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────┐
│ ExecutionEngine::bulk_insert_records (530)  │
│   - 持有 Mutex<BinaryTableStorage>           │
│   - 跳过 SQL parser (Round-21 fix)           │
│   - 调用 storage.insert_streaming(rows)      │
└──────────┬───────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────┐
│ BinaryTableStorage::insert_streaming        │
│   - 拿段锁 (Mutex<Segment::append_lock>)     │
│   - 追加 rows 到当前活跃段                   │
│   - 段 ≥ 64 MB → seal, 拉起新段             │
│   - 返回 (无磁盘 I/O，内存 append)           │
└──────────┬───────────────────────────────────┘
           │
           ▼ (加载结束, 调用 engine.flush())
┌─────────────────────────────────────────────┐
│ BinaryTableStorage::flush                   │
│   - seal 所有活跃段                          │
│   - 写 root.index (mmap 文件 < 4 KB)         │
│   - WAL fsync (WalSyncMode::Batch 时聚合)   │
└─────────────────────────────────────────────┘
```

### 锁模型

| 锁 | 粒度 | 持锁时间 | 用途 |
|---|---|---|---|
| `BinaryTableStorage::root` | `RwLock<RootIndex>` | 极短（更新 root.index 指针） | 表级元数据 |
| `Segment::append_lock` | `Mutex<()>` | 单批写入时间（~1ms） | 单段追加 |
| `Segment::read_lock` | `RwLock<()>` | 长（scan 期间） | 读取 |

**关键属性**：
- LOAD DATA 持段 `append_lock` 不阻塞读段：读段用 `read_lock`，读写可并发
- 不同段之间完全独立：lineitem LOAD 不会阻塞 orders SELECT
- 同表 SELECT vs LOAD：仅在切换段瞬间竞争 root.index（<10μs）

### WAL 集成

```
WalSyncMode::Every → 每次 batch_insert 一次 fsync（默认）
WalSyncMode::Batch → flush() 时一次 fsync（推荐 LOAD DATA）
WalSyncMode::Off   → 仅 root.index fsync（最快，不推荐）
```

LOAD DATA 强制使用 `WalSyncMode::Batch`：
- 调用栈：`ExecutionEngine::bulk_insert_records` 检测到调用方上下文是 LOAD DATA 路径（由 `handle_load_local_infile` 通过 `EphemeralConfig.wal_sync_mode_override` 显式传入），临时将 `WalSyncMode` 切到 `Batch`
- 加载结束后 `engine.flush()` 统一 fsync 一次，然后恢复原 `WalSyncMode`
- SF=1 lineitem 6M 行：原 Every 模式 600 次 fsync × 8ms = 4.8s
- Batch 模式：1 次 fsync ≈ 10ms（**~480× 加速**）

### 崩溃恢复语义

| 场景 | 恢复策略 |
|---|---|
| LOAD DATA 中途崩溃 | 启动时检查 root.index 是否完整：未提交 → 整个 segment 删除 |
| 部分写段（断电） | 段文件 header CRC32C 失败 → 丢弃整个段 |
| 行级 CRC32C 失败 | log warning，跳过该行，scan 继续 |
| root.index 损坏 | 从 `.json.bak` 回退到 FileStorage 路径 |

### 与并发 LOAD 的关系

- **单连接 LOAD**：走当前路径，无变化
- **多连接 LOAD DATA**：v1 不支持（需要 segment-level lock 重构），v2 路线图
- **LOAD 与 SELECT 并发**：完全支持（同段 append_lock 不阻塞 read_lock）

## 4. 错误处理 / 失败模式

### 故障矩阵

| # | 故障模式 | 触发场景 | 检测 | 恢复行为 | 测试要求 |
|---|---|---|---|---|---|
| F1 | 段文件 torn write | 段写到一半断电 | 段头 `header_crc32c` 校验失败 | 启动扫描时丢弃该段；root.index 未提交则回滚 | 单元测试：手动截断段文件 → 加载应跳过 |
| F2 | 行级 CRC32C 失败 | 磁盘位翻转 / 部分页覆盖 | 行尾 `row_crc32c` 不匹配 | log warning，跳过该行，scan 返回剩余行 | 单元测试：位翻转单行 → 跳过验证 |
| F3 | 磁盘满 | LOAD DATA 中途 ENOSPC | write() 返回 Err | seal 当前段，停止后续 batch；已写入段保留 | 集成测试：模拟 ENOSPC → 部分行保留 |
| F4 | Schema 不兼容 | 旧 BIN 文件 vs 新代码 | root.index.version vs 代码期望版本 | 自动触发 JSON 回退路径（读 `.json.bak`） | 单元测试：旧 version 段 → 回退测试 |
| F5 | root.index 损坏 | 写入 root.index 时崩溃 | 启动时 root_crc32c 失败 | 从 `.json.bak` 重建索引；若都失败 → panic + 提示手动恢复 | 集成测试：手动损坏 root.index |
| F6 | WAL replay 后段丢失 | flush() 后断电，segment 未持久化 | root.index 中无该段 | WAL replay 期间检查段文件存在性；缺失则丢弃对应 WAL entry | 集成测试：删段 + 重启 |
| F7 | 权限拒绝 | 数据目录不可写 | 首次 append 返回 PermissionDenied | panic with actionable error；不静默回退到 JSON | 单元测试：chmod 000 数据目录 |
| F8 | OOM during compaction | 段合并时内存峰值超限 | Vec capacity 溢出 → catch_unwind | 中止 compaction，保留旧段；下次启动重试 | 压力测试：1 GB 段合并 |
| F9 | 并发读写竞争 | LOAD DATA 时同表 SELECT | segment append_lock 排队 | SELECT 自动等待 <10ms；LOAD 不阻塞读 | 集成测试：50 并发 SELECT + LOAD |

### 错误传播规则

```rust
// 关键原则：fail-loud for unrecoverable, fail-soft for recoverable
match err.kind() {
    // F1, F5: 数据完整性 → fail-loud（panic + 可恢复提示）
    DataCorruption | IndexCorruption => panic!("...recoverable via: ..."),
    // F2: 单行 CRC 失败 → fail-soft（skip + log）
    RowChecksumMismatch => { log::warn!("skip row {}: {:?}", row_id, err); continue; }
    // F3, F7: I/O → propagate to caller
    IoError => return Err(SqlError::Storage(err.to_string())),
    // F8: 内部资源 → fail-soft（重试 + 跳过本次）
    OutOfMemory => { log::error!("OOM, skipping compaction"); return Ok(()); }
}
```

### 监控指标（新增）

| 指标 | 阈值告警 |
|---|---|
| `bin_segment_crc_failures_total` | > 0 → 立即告警 |
| `bin_load_rows_skipped_total` | > 100/s → 磁盘健康问题 |
| `bin_root_index_rebuild_total` | > 0 → 启动失败告警 |
| `bin_segment_sealed_total` | 趋势监控（识别快速段切换） |

### 与现有错误体系的关系

所有存储错误统一映射到 `SqlError::Storage`，复用现有错误返回链路。
**不引入新的错误枚举**，仅在 `StorageErrorKind` 增加 3 个变体：
- `RowChecksumMismatch`
- `DataCorruption`
- `IndexCorruption`

## 5. 迁移策略

### 迁移触发模型

**写时迁移**（lazy, on-first-write）

```
Storage::open(table_name):
1. 检查 root.bin 存在？ → 是 → 用 BinaryTableStorage 读
2. 检查 .json 存在？ → 是 → 走 FileStorage 读路径
3. 都不存在？ → 新表, 空目录

Storage::insert_streaming(table_name, rows):
1. 当前存储 = BinaryTableStorage? → 是 → 直接 append
2. 当前存储 = FileStorage (legacy JSON)? → 是 → 触发自动迁移:
   a. 读 .json → 在内存中构建 TableData
   b. 写 .bin 段文件
   c. 写 root.bin 索引
   d. 归档 .json → .json.bak（不删除, 保留回滚能力）
   e. 后续读写走 BIN 路径
3. 否（全新表）→ 创建 BIN 段 + root.bin
```

### 关键设计原则

| 原则 | 体现 |
|---|---|
| 零停机迁移 | 迁移过程不阻塞 SELECT；可在 LOAD DATA 期间完成 |
| 原子性 | root.bin + 所有 .bin 段写入完成后才归档 .json（崩溃则 JSON 仍是 source of truth） |
| 可回滚 | `.json.bak` 永久保留直到下次手工清理 |
| 读不感知 | SELECT 接口签名不变；自动选最快路径 |

### 启动顺序保证（崩溃安全）

```
启动流程：
1. 读 root.bin → 成功 → 用 BIN 路径
2. 读 root.bin → 失败/不存在 →
   a. 读 .json → 成功 → 用 JSON 路径（兼容旧数据）
   b. 都不存在 → 新表
3. 绝不在启动时主动迁移（避免启动期间崩 = 数据丢失）
```

迁移**只在 INSERT 路径上**触发，且走原子协议。

### 回滚路径（运维）

```bash
# 紧急回滚到 JSON 存储（仅在 BIN 出现严重 bug 时）
$ sqlrustgo-admin storage rollback --table lineitem
  → 检测 .json.bak 存在？
    ├─ 是 → 删 .bin + root.bin，恢复 .json.bak → .json，重启
    └─ 否 → 拒绝（提示先从备份恢复 .json）

# 永久清理 .json.bak（迁移已稳定后）
$ sqlrustgo-admin storage cleanup-bak --older-than 30d
```

### 跨版本兼容

| 当前代码 | 旧 BIN (v2) | 新 BIN (v3) | JSON |
|---|---|---|---|
| 读 v2 | ✅（降级到 v2 reader） | n/a | ✅ |
| 读 v3 | ❌（拒绝, 需升级代码） | ✅ | ✅ |
| 写 | 永远写 v3 | n/a | ✅（不写 JSON, 除非启动期迁移） |

→ 升级部署时，先滚动升级代码（只读 v3），再让流量触发旧段写入路径。

### 迁移期间性能影响

| 阶段 | 性能 |
|---|---|
| 首次 INSERT 触发迁移 | **单次**多 ~200ms（一次性写 BIN + 归档 JSON） |
| 迁移后 | 与直接创建 BIN 表**完全相同**（~10-30× 加速生效） |
| SELECT（迁移前） | 不变（仍走 JSON 路径） |
| SELECT（迁移后） | 不变（走 BIN 路径，但接口签名一致） |

### 兼容性测试矩阵

| 测试 | 预期 |
|---|---|
| `test_json_to_bin_migration_atomic` | 迁移中段崩溃 → JSON 仍是 source of truth |
| `test_bin_to_json_rollback` | 手动 rollback → 数据与 BIN 写入前完全一致 |
| `test_concurrent_select_during_migration` | 迁移期间 SELECT 返回正确数据 |
| `test_multi_table_independent_migration` | 各表独立迁移，lineitem 迁移不影响 orders |
| `test_migration_idempotent` | 已迁移表再次 INSERT 不重复迁移 |

## 6. 测试策略

### 5 层测试金字塔

```
                  ┌──────────────────────────────┐
                  │ L5: E2E TPC-H SF=1 22/22 PASS │  ← 回归保证
                  ├──────────────────────────────┤
                  │ L4: 性能基准 (lineitem < 60s) │  ← 性能对标
                  ├──────────────────────────────┤
                  │ L3: 故障注入 (F1-F9 覆盖)     │  ← 鲁棒性
                  ├──────────────────────────────┤
                  │ L2: 集成 (迁移 / 并发 / WAL)  │  ← 模块协作
                  ├──────────────────────────────┤
                  │ L1: 单元 (bin_segment 等)     │  ← 基础正确性
                  └──────────────────────────────┘
```

### L1: 单元测试（`bin_segment.rs` 模块内）

```rust
#[test] fn roundtrip_single_row_int64() { ... }            // 写+读+比较
#[test] fn roundtrip_row_with_nulls() { ... }
#[test] fn roundtrip_variable_length_text() { ... }
#[test] fn row_header_size_constant() {
    assert_eq!(std::mem::size_of::<RowHeader>(), 16);
}
#[test] fn segment_max_size_enforced() { ... }            // 段超 64MB 自动 seal
#[test] fn crc32c_detects_corruption() { ... }             // 位翻转 1 字节
#[test] fn segment_append_is_atomic_at_page_boundary() { ... }
```

覆盖率门槛：**L1 单元测试行覆盖 ≥ 90%**，新增代码 100%。

### L2: 集成测试（`tests/integration/`）

| 测试 | 覆盖 |
|---|---|
| `bin_storage_basic_io.rs` | 写 N 行 → 读 N 行 → 顺序匹配 |
| `bin_storage_concurrent_select.rs` | 50 线程 SELECT + 1 线程 LOAD |
| `bin_storage_migration_atomic.rs` | 写时迁移中断不破坏 JSON |
| `bin_storage_wal_integration.rs` | 崩溃恢复后段重建 |
| `bin_storage_compaction_roundtrip.rs` | 多段合并后读出数据一致 |

### L3: 故障注入（`tests/integration/fault_injection/`）

| F# | 测试 | 方法 |
|---|---|---|
| F1 | `torn_write_recovery.rs` | `truncate -s 50% segment.bin` → 启动应跳过 |
| F2 | `row_crc_skip.rs` | `dd seek=...` 翻转 1 字节 → scan 应跳过该行 |
| F3 | `disk_full_simulation.rs` | 用 `fault-injection` crate mock ENOSPC |
| F5 | `corrupted_root_recovery.rs` | 手动写脏 root.bin → JSON 回退 |
| F8 | `compaction_oom_recovery.rs` | 设小内存限制 → 合并失败保留旧段 |

### L4: 性能基准（`benches/tpch_load_bench.rs`）

| 基准 | 命令 | 目标 | 报告 |
|---|---|---|---|
| `lineitem_load_sf1` | `cargo bench --bench tpch_load_bench` | **< 60 秒** | 行/秒, P99 延迟, 磁盘 MB/s |
| `orders_load_sf1` | 同上 | < 15 秒 | 同上 |
| `customer_load_sf1` | 同上 | < 2 秒 | 同上 |
| `mixed_read_write` | 同上 | 读 P99 < 50ms | SELECT 延迟直方图 |

阈值断言（在 CI 中）：
```rust
#[ignore] #[test]
fn lineitem_load_under_60_seconds() {
    let duration = bench_load_lineitem_sf1();
    assert!(duration < Duration::from_secs(60),
            "lineitem load regressed: {:?}", duration);
}
```

### L5: TPC-H SF=1 22 查询全过（`tests/integration/oracle/tpch_sf1_22.rs`）

```rust
#[test] fn tpch_sf1_22_vs_3engines() {
    for query in 1..=22 {
        run_query_and_compare_against_postgres_mysql(query);
    }
}
```

**执行流程**：
1. 启动 PostgreSQL + MySQL + SQLRustGo
2. 各库导入 SF=1 TPC-H 数据
3. 依次执行 Q1-Q22，记录每个引擎的执行结果
4. 三方结果比较，任一不一致 → fail

> **新增强制项**：SQLRustGo 加载阶段耗时必须 < 60s（lineitem）+ 15s（orders），否则 fail-loud。

### 性能监控基线（回归保护）

| 表 | 当前耗时 | 目标 | 改进幅度 |
|---|---|---|---|
| lineitem SF=1 | ~10+ 小时（仓库 Issue 报） | < 60 秒 | **600×** |
| orders SF=1 | ~2+ 小时（推算） | < 15 秒 | **480×** |
| customer SF=1 | ~5 分钟 | < 2 秒 | **150×** |

### 测试门禁（合并前必过）

```
L1 单元 → L2 集成 → L3 故障注入 → cargo bench (基线不恶化)
  → L4 性能基准 (断言 <60s) → L5 TPC-H 22/22
```

任何一个失败 → 阻塞 PR 合并。

## 7. 实施卷轴与风险回滚

### 4 阶段交付计划（每个阶段独立可回滚）

```
Phase 1              Phase 2                Phase 3                  Phase 4
bin_segment    →   bin_compactor    →   bin_index/migration   →   TPC-H 验证
   ↓                  ↓                    ↓                       ↓
单表读写          多段合并             自动迁移                  完整场景
基础正确          空间回收             线上透明                  性能对标
预计 3 天          预计 3 天           预计 2 天                 预计 2 天
```

### Phase 1: bin_segment（基础 + 单表读写，3 天）

**产出**
- `crates/storage/src/bin_segment.rs`：BINT v3 段文件读写
- `crates/storage/src/binary_storage_v2.rs`：BinaryTableStorage（替代 FileStorage 的新主类）
- 单元测试 L1 完整覆盖

**回滚点**
- BinaryTableStorage 不接 `BoxStorageEngine` 别名 → 完全隔离，旧 FileStorage 不受影响
- 二进制文件 .bin 不存在则回退 JSON

**DoD**
- `cargo test -p storage --lib` 全过
- 单表写 1M 行 → 读 1M 行 → 字节级一致
- 行/段 CRC32C 校验 F1/F2 故障注入测试通过

### Phase 2: bin_compactor（多段管理，3 天）

**产出**
- `crates/storage/src/bin_compactor.rs`：多段合并为单段
- 后台线程触发（每 5 分钟或段数 ≥ 10）
- Compaction OOM 安全网（chunks + spill to disk）

**回滚点**
- compactor 是独立 trait `BinCompactor::run()`，未启用 = 无影响
- 段数量无界增长但功能正常

**DoD**
- 多段 → 单段合并后字节级一致
- F8 OOM 模拟测试通过（保留旧段，崩溃后下次重试）

### Phase 3: bin_index + 自动迁移（2 天）

**产出**
- `crates/storage/src/bin_index.rs`：root.bin 索引
- 写时迁移逻辑（lazy on-first-write）
- `.json.bak` 归档机制
- `sqlrustgo-admin storage rollback` 子命令

**回滚点**
- 迁移通过 `feature flag = bin_storage_default` 控制；OFF 时退回 FileStorage
- `.json.bak` 永远保留直到手工删

**DoD**
- 迁移原子性测试通过（F1/F5 故障注入）
- L2 集成测试全部通过

### Phase 4: TPC-H SF=1 性能验证（2 天）

**产出**
- `tests/integration/oracle/tpch_sf1_22.rs` 强化（加载耗时进入断言）
- `benches/tpch_load_bench.rs` 4 个基准
- 性能报告 `docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md`

**DoD（合并到 develop/v3.13.0 的硬条件）**
- lineitem SF=1 加载 < 60s
- orders SF=1 加载 < 15s
- TPC-H SF=1 22/22 通过
- 无 P0/P1 回归

### 风险登记册

| ID | 风险 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|---|
| R1 | BINT v3 格式设计有缺陷需返工 | 中 | 中 | Phase 1 独立小范围，先跑 7 天小流量再进 Phase 2 |
| R2 | 迁移过程中段文件膨胀占满磁盘 | 低 | 中 | `.json.bak` 仅复制不保留，腾出空间后才完成迁移 |
| R3 | 性能未达 60s 目标 | 中 | 高 | 60s 是软目标；首版未达时改 hardlimit to 120s 并继续迭代 |
| R4 | 二进制格式未来需扩容 | 中 | 中 | 段头预留 16 KB（仅用 48 字节），version 字段已就位 |
| R5 | 与现有 WAL/恢复测试冲突 | 低 | 中 | 复用现有 WAL 接口，回归时跑完整 recovery suite |

### 与 develop/v3.12.0 主线的协调

- 当前在 `develop/v3.12.0`，目标 merge 到 `develop/v3.13.0`（下一开发分支）
- 每个 Phase 独立 commit + PR，便于评审
- 建议新建分支：`feature/v313-bin-storage`，从当前 develop/v3.12.0 HEAD cut

### 工期估算

| Phase | 工期 | 人力 |
|---|---|---|
| Phase 1 | 3 天 | 1 人 |
| Phase 2 | 3 天 | 1 人 |
| Phase 3 | 2 天 | 1 人 |
| Phase 4 | 2 天 | 1 人 + 性能调优 buffer |
| **总计** | **10 工作日** | **1 人 + 5 天 buffer = 3 周** |

## 修改的文件（汇总）

| # | 文件 | 改动 |
|---|---|---|
| 1 | `crates/storage/src/bin_segment.rs` | 新增（~600 行）：BINT v3 段文件读写 + CRC32C |
| 2 | `crates/storage/src/bin_compactor.rs` | 新增（~400 行）：多段合并 |
| 3 | `crates/storage/src/bin_index.rs` | 新增（~200 行）：root.bin 索引 |
| 4 | `crates/storage/src/binary_storage_v2.rs` | 新增（~500 行）：公开 API + insert_streaming |
| 5 | `crates/storage/src/bin_migration.rs` | 新增（~200 行）：JSON → BIN 迁移 |
| 6 | `crates/storage/src/lib.rs` | 新增模块导出 + BoxStorageEngine 默认切换（feature flag 控制） |
| 7 | `crates/storage/src/error.rs` | `StorageErrorKind` 新增 3 个变体 |
| 8 | `src/execution_engine.rs` | `bulk_insert_records` 路径走 BinaryTableStorage |
| 9 | `benches/tpch_load_bench.rs` | 新增（~200 行）：4 个性能基准 |
| 10 | `tests/integration/oracle/tpch_sf1_22.rs` | 强化加载耗时断言 |
| 11 | `tests/integration/bin_storage_*.rs` | 新增（5 个测试文件）：L2 集成测试 |
| 12 | `tests/integration/fault_injection/*.rs` | 新增（5 个测试文件）：L3 故障注入 |
| 13 | `crates/admin/src/storage_commands.rs` | 新增 `rollback` / `cleanup-bak` 子命令 |
| 14 | `docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md` | 新增（~500 行）：性能报告 |

## 开放问题（待后续确认）

- O-1: 行级 CRC32C 计算开销估计占总耗时 ~5%；是否需要在 Phase 1 后做 profile 优化？
- O-2: `bin_storage_default` feature flag 的默认值应在 Phase 4 末尾切到 ON，需要专门一个 PR 触发
- O-3: 多连接 LOAD DATA 并发支持（v2 路线图）不在本次范围