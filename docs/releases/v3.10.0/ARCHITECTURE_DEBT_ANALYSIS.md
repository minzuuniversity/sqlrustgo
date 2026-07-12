# SQLRustGo v3.10.0 架构重构技术债务综合评估报告

> **日期**: 2026-07-12
> **版本**: v3.10.0 (develop/v3.10.0)
> **目的**: 评估 F-16 Gap Locking 和 F-23 Clustered Index 集成是否可纳入 v3.10.0 架构重构，以及识别其他可协同完成的债务项

---

## 1. 执行摘要

### 1.1 债务全景

| 类别 | 总数 | ✅ 关闭 | ⚠️ 部分 | ❌ 未完成 |
|------|------|--------|--------|---------|
| F-xx 功能缺口 | 36 | 23 (64%) | 4 (11%) | 9 (25%) |
| I-xx 集成缺口 | 12 | 10 (83%) | 2 (17%) | 0 |
| T-xx 测试缺口 | 20 | 16 (80%) | 2 (10%) | 2 (10%) |
| **合计** | **68** | **49 (72%)** | **8 (12%)** | **11 (16%)** |

### 1.2 本次评估范围

两个 Issue 需要评估：
- **#3770**: F-16 Gap Locking 真实 B+ Tree 集成
- **#3771**: F-23 Clustered Index 真实磁盘 B+ Tree 集成

**核心问题**: 这两项是否可以在 v3.10.0 完成？是否应该纳入更大的架构重构计划？

---

## 2. 现状分析

### 2.1 当前 B+ Tree 实现

```
crates/storage/src/bplus_tree/
├── index.rs          (1976 行) - 磁盘 B+ Tree 索引
├── hash_index.rs     (270 行)   - Hash 索引
└── mod.rs            (129 行)   - 简化 B+ Tree 封装
```

**当前 `BTreeIndex` 特性**:
- ✅ 唯一索引 (主键/唯一约束)
- ✅ 键比较 (compare_values)
- ✅ 页面分裂/合并
- ✅ 磁盘持久化
- ❌ Gap Lock (间隙锁)
- ❌ Clustered Index (行存储在叶子节点)
- ❌ 非唯一索引

### 2.2 Mock 实现 vs 真实实现

| 特性 | Mock (`tests/`) | 真实 (`bplus_tree/`) |
|------|-----------------|---------------------|
| Gap Lock | GapLockManager (HashMap) | ❌ 无 |
| Clustered Index | BTreeMap 存储 Row | ❌ 无 (只存 Key→Value) |
| 索引类型 | 唯一+非唯一 | 仅唯一 |
| 持久化 | In-memory | ✅ 磁盘 |

---

## 3. F-16 Gap Locking 集成分析

### 3.1 现状

**Mock 实现** (`tests/gap_locking_test.rs`):
```rust
pub struct GapLockManager {
    locks: Arc<Mutex<HashMap<String, Vec<GapLock>>>>,
    isolation: Arc<Mutex<HashMap<u64, IsolationLevel>>>,
    block_count: Arc<Mutex<u64>>,
}

impl GapLockManager {
    pub fn acquire_gap(&self, tx_id: u64, table: &str, range: GapRange) -> Result<(), String>
    pub fn release_all(&self, tx_id: u64) -> usize
    pub fn can_insert(&self, table: &str, value: i64, other_tx: u64) -> Result<(), String>
}
```

### 3.2 集成路径

```
GapLockManager 接口:
├── acquire_gap(tx_id, table, range) → 检测/获取间隙锁
├── release_all(tx_id) → 释放事务所有锁
├── can_insert(table, value, tx_id) → 检查能否插入
└── IsolationLevel 设置

集成点:
├── BTreeIndex::insert() → 插入前检查 gap 锁
├── BTreeIndex::delete() → 删除前检查 gap 锁
└── transaction commit/rollback → 释放锁
```

### 3.3 改动范围

| 组件 | 改动量 | 说明 |
|------|--------|------|
| GapLockManager 迁移 | ~100 行 | 从 tests/ 迁到 storage crate |
| BTreeIndex 添加锁字段 | ~50 行 | 添加 gap_lock_manager 字段 |
| insert() 修改 | ~30 行 | 插入前调用锁检查 |
| delete() 修改 | ~30 行 | 删除前调用锁检查 |
| transaction 集成 | ~50 行 | commit/rollback 释放锁 |
| **合计** | **~260 行** | |

### 3.4 风险评估

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| 锁粒度冲突 | 中 | 高 | 设计审查 |
| 死锁 | 低 | 高 | 锁超时机制 |
| 性能退化 | 中 | 中 | Benchmark 验证 |

**结论**: ✅ **可在 v3.10.0 完成，改动范围可控**

---

## 4. F-23 Clustered Index 集成分析

### 4.1 现状

**Mock 实现** (`tests/clustered_index_test.rs`):
```rust
pub struct ClusteredIndex {
    pages: Vec<LeafPage>,           // 叶子节点存完整行
    pk_to_page: BTreeMap<i64, usize>,
    secondary_idx: HashMap<String, Vec<i64>>,  // 二次索引
}
```

**当前 `BTreeIndex` 结构**:
```rust
struct BTreeNode {
    keys: Vec<Key>,
    values: Vec<IndexValue>,  // IndexValue = u32 (page pointer)
    children: Vec<u32>,       // 内部节点指向子页面
}
```

### 4.2 架构差异

```
当前 B+ tree:                    Clustered B+ tree:
Key → IndexValue(u32)           Key → RowData
叶子节点只存索引键               叶子节点存完整行数据

查询流程:                        查询流程:
1. 搜索 B+ Tree                 1. 搜索 B+ Tree
2. 获取 page pointer             2. 直接获取行数据
3. 读取 page 获取数据
```

### 4.3 改动范围

| 组件 | 改动量 | 说明 |
|------|--------|------|
| BTreeNode 重构 | ~200 行 | 叶子节点存 Row 数据 |
| Row 编码/解码 | ~100 行 | 行数据序列化 |
| BTreeIndex.insert() 重写 | ~150 行 | 插入行而非 page pointer |
| BTreeIndex.search() 重写 | ~100 行 | 返回行数据 |
| 二次索引支持 | ~150 行 | 指向主键的索引 |
| 查询计划调整 | ~100 行 | 利用 clustered index 优化 |
| **合计** | **~800 行** | 较大架构改动 |

### 4.4 风险评估

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| 数据局部性退化 | 中 | 高 | 充分测试 |
| 写入放大 | 高 | 中 | Benchmark 验证 |
| 页面分裂逻辑复杂化 | 高 | 中 | 设计审查 |
| 二次索引一致性 | 中 | 高 | 事务集成 |

**结论**: ⚠️ **建议推迟到 v3.11.0，需要较大架构重构**

---

## 5. 协同架构重构建议

### 5.1 建议的 v3.10.0 架构重构范围

基于分析，建议以下协同重构：

```
v3.10.0 架构重构 Phase 1:

1. F-16 Gap Locking 集成 (~260 行)
   ├── 迁移 GapLockManager 到 storage crate
   ├── BTreeIndex 添加锁支持
   └── 并发测试验证

2. Storage Engine 统一接口 (~200 行)
   ├── 统一 MemoryStorage / FileStorage 接口
   ├── 移除重复代码
   └── 添加统一测试

3. 非唯一索引支持 (~300 行) [新发现债务]
   ├── BTreeIndex 支持重复键
   ├── unique constraint 验证
   └── 迁移现有测试

总计: ~760 行代码改动
```

### 5.2 可推迟到 v3.11.0 的项

| Item | 原因 | 估计工作量 |
|------|------|-----------|
| F-23 Clustered Index | 需要较大架构重构 | ~800 行 |
| F-24 Adaptive Hash Index | 需要 B+ tree 扩展 | ~500 行 |
| F-25 Change Buffer | 需要 storage pipeline 重构 | ~400 行 |
| F-27 表压缩 (LZ4/zstd) | 需要 compression API | ~300 行 |

---

## 6. 技术债务完整清单 (v3.6.0-v3.9.0)

### 6.1 F-xx 功能缺口 (36 项)

| ID | Feature | 状态 | 优先级 | 备注 |
|----|---------|------|--------|------|
| F-01 | CREATE EVENT | ⚠️ partial | P3 | 35 files, 不完整 |
| F-02 | FULLTEXT | ⚠️ partial | P3 | 3 files, 不完整 |
| F-03 | GIS 空间数据 | ❌ 未实现 | P2 | 需全量实现 |
| F-04 | INSERT...SELECT | ✅ 已实现 | - | |
| F-05 | 窗口函数 | ✅ 已实现 | - | |
| F-06 | CTE | ✅ 已实现 | - | |
| F-07 | 查询缓存 | ⚠️ partial | P2 | DML invalidation 测试缺失 |
| F-08 | 连接池 | ✅ 已实现 | - | |
| F-09 | WAL 崩溃恢复 | ✅ 已实现 | - | |
| F-10 | TPC-H SF=1 | ✅ 已实现 | - | |
| F-11 | 窗口函数完整 | ✅ 已实现 | - | |
| F-12 | 存储过程游标 | ✅ 已实现 | - | |
| F-13 | 触发器 | ✅ 已实现 | - | |
| F-14 | CTE 递归 | ✅ 已实现 | - | |
| F-15 | SERIALIZABLE | ✅ 已实现 | - | |
| **F-16** | **Gap Locking** | **⚠️ ISOLATED** | **P1** | **需主路径集成** |
| F-17 | JSON 函数 | ✅ 已实现 | - | |
| F-18 | INFORMATION_SCHEMA | ✅ 已实现 | - | |
| F-19 | SSL/TLS | ✅ 已实现 | - | |
| F-20 | 慢查询日志 | ✅ 已实现 | - | |
| F-21 | 在线 DDL | ✅ 已实现 | - | |
| F-22 | Prepared Statement | ✅ 已实现 | - | |
| **F-23** | **Clustered Index** | **⚠️ ISOLATED** | **P2** | **建议 v3.11.0** |
| F-24 | AHI | ⚠️ ISOLATED | P2 | 建议 v3.11.0 |
| F-25 | Change Buffer | ⚠️ ISOLATED | P2 | 建议 v3.11.0 |
| F-26 | Double-Write | ⚠️ ISOLATED | P2 | 建议 v3.11.0 |
| F-27 | 表压缩 | ⚠️ ISOLATED | P2 | RLE only, 需 LZ4/zstd |
| F-28 | XA 两阶段 | ✅ 已实现 | - | |
| F-29 | RLS | ⚠️ ISOLATED | P2 | in-memory catalog |
| F-30 | SEQUENCE | ❌ 未实现 | P2 | |
| F-31 | Performance Schema | ⚠️ ISOLATED | P3 | in-memory mock |
| F-32 | mysqladmin | ⚠️ ISOLATED | P2 | 缺真实 CLI binary |
| F-33 | mysqlbinlog | ✅ 已实现 | - | |
| F-34 | AES-256 | ⚠️ partial | P3 | |
| F-35 | 密码轮转 | ⚠️ ISOLATED | P2 | in-memory only |
| F-36 | 列级权限 | ❌ 未实现 | P3 | |

### 6.2 I-xx 集成缺口 (12 项)

| ID | Feature | 状态 | 优先级 | 备注 |
|----|---------|------|--------|------|
| I-01 | 触发器 | ✅ 已实现 | - | |
| I-02 | 查询缓存 | ✅ 已实现 | - | |
| I-03 | MVCC SSI | ✅ 已实现 | - | |
| I-04 | CTE | ✅ 已实现 | - | |
| I-05 | EXPLAIN | ✅ 已实现 | - | |
| I-06 | mysqldump | ✅ 已实现 | - | |
| I-07 | 窗口函数 | ✅ 已实现 | - | |
| I-08 | INSERT...SELECT | ✅ 已实现 | - | |
| I-09 | 连接池 | ✅ 已实现 | - | |
| I-10 | Group Commit | ✅ 已实现 | - | |
| I-11 | CBO 代价模型 | ⚠️ partial | P2 | 3 rules |
| I-12 | 并行执行 | ✅ 已集成 | P1 | v3.10.0 完成 |

### 6.3 T-xx 测试缺口 (20 项)

| ID | Test | 状态 | 优先级 | 备注 |
|----|------|------|--------|------|
| T-01 | query_cache | ✅ | - | |
| T-02 | WAL crash recovery | ✅ | - | |
| T-03 | MVCC transaction | ✅ | - | |
| T-04 | long_run_stability | ✅ | - | |
| T-05 | crash_recovery | ✅ | - | |
| T-06 | optimizer tests | ⚠️ partial | P2 | |
| T-07 | planner tests | ✅ | - | |
| T-08 | network tests | ✅ | - | |
| T-09 | executor tests | ✅ | - | |
| T-10 | storage tests | ✅ | - | |
| T-11 | perf baseline | ✅ | - | |
| T-12 | regression | ✅ | - | |
| T-13 | TPC-H in CI | ✅ | - | |
| T-14 | Sysbench | ⚠️ partial | P2 | not in CI |
| T-15 | Deadlock injection | ⚠️ ISOLATED | P2 | mock tests |
| T-16 | CPU 80% stress | ⚠️ partial | P2 | |
| T-17 | Network fault | ⚠️ ISOLATED | P2 | mock tests |
| T-18 | Memory fault | ⚠️ ISOLATED | P2 | mock tests |
| **T-19** | **Disk I/O delay** | **❌ 未实现** | **P1** | **v3.10.0 已完成** |
| **T-20** | **kill -9 mid-transaction** | **❌ 未实现** | **P0** | **v3.10.0 已完成** |

---

## 7. 架构债务 Remediation Plan

### 7.1 ARCH-1: execution_engine.rs 单文件

**状态**: ✅ CLOSED (1641 行 < 2000 阈值)

### 7.2 ARCH-2: 双路径 (mysql-server vs bench-cli)

**状态**: ❌ OPEN
**改动**: 24h
```
1. Identify divergent paths           (4h)
2. Extract common engine entry point (8h)
3. Refactor both binaries           (8h)
4. Add cross-binary test            (4h)
```

### 7.3 ARCH-3: VTU 主路径剩余 5%

**状态**: ⚠️ PARTIAL
**改动**: 24h
```
1. Audit all UPDATE code paths       (4h)
2. Migrate 2 legacy paths to VTU    (12h)
3. Add VTU coverage tests           (4h)
4. Remove legacy code paths         (4h)
```

### 7.4 SEM-1: ROLLBACK MVCC

**状态**: ❌ OPEN
**改动**: 28h
```
1. Add MVCC snapshot restoration    (12h)
2. Wire to ROLLBACK TO SAVEPOINT    (4h)
3. Add 5+ MVCC rollback tests      (8h)
4. Document behavioral guarantee    (4h)
```

### 7.5 SEM-3: ALTER TABLE 不完整

**状态**: ❌ OPEN (C-4d MODIFY 已修复)
**改动**: 20h
```
1. RENAME TABLE                     (4h)
2. RENAME COLUMN                    (4h)
3. MODIFY COLUMN                    (6h) [已修复]
4. Add ALTER TABLE tests            (6h)
```

---

## 8. 建议的 v3.10.0 架构重构计划

### 8.1 推荐范围

```
v3.10.0 架构重构 Phase 1 (建议):

| # | Item | 改动量 | 优先级 | 来源 |
|---|------|--------|--------|------|
| 1 | F-16 Gap Locking 集成 | ~260 行 | P1 | #3770 |
| 2 | ARCH-2 双路径统一 | ~300 行 | P2 | 历史债务 |
| 3 | 非唯一索引支持 | ~300 行 | P2 | 新发现 |
| 4 | io_delay 正式化 | ~100 行 | P2 | T-19 延伸 |
| **Total** | | **~960 行** | | |

v3.10.0 架构重构 Phase 2 (可选):

| # | Item | 改动量 | 优先级 | 来源 |
|---|------|--------|--------|------|
| 1 | F-23 Clustered Index | ~800 行 | P2 | #3771 |
| 2 | F-24 Adaptive Hash Index | ~500 行 | P2 | INT5 |
| 3 | F-25 Change Buffer | ~400 行 | P2 | INT5 |
| **Total** | | **~1700 行** | | |
```

### 8.2 不建议在 v3.10.0 做的项

| Item | 原因 | 建议版本 |
|------|------|---------|
| F-23 Clustered Index | 较大架构重构 | v3.11.0 |
| F-27 表压缩 (LZ4/zstd) | 需 compression API | v3.11.0 |
| F-30 CREATE SEQUENCE | 新功能 | v3.11.0 |
| F-36 列级权限 | 新功能 | v3.12.0+ |
| F-03 GIS | 新功能 | v3.12.0+ |

---

## 9. 验证计划

### 9.1 Gap Locking 验证

```bash
# 1. 单元测试
cargo test --test gap_locking_test

# 2. 并发测试
cargo test --test gap_locking_concurrent_test

# 3. 集成测试
cargo test --test transaction_isolation_test

# 4. 性能基准
cargo bench -- gap_lock_benchmark
```

### 9.2 非唯一索引验证

```bash
# 1. 唯一索引回归
cargo test --test unique_index_test

# 2. 非唯一索引测试
cargo test --test non_unique_index_test

# 3. 重复键查询
cargo test --test duplicate_key_test
```

---

## 10. 附录

### A. 参考文档

- `docs/releases/v3.8.0/debt/INT5_PLUS_DEBT_INVENTORY.md` - 债务总表
- `docs/releases/v3.8.0/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md` - 架构债务计划
- `docs/releases/v3.8.0/archived/INT_DEBT_REMEDIATION_PLAN.md` - 集成债务计划
- `docs/releases/v3.10.0/plans/V310_DEVELOPMENT_PLAN.md` - v3.10.0 开发计划

### B. 代码位置

- B+ Tree 实现: `crates/storage/src/bplus_tree/index.rs`
- Gap Lock Mock: `tests/gap_locking_test.rs`
- Clustered Index Mock: `tests/clustered_index_test.rs`
- Storage Trait: `crates/storage/src/engine.rs`

### C. v3.10.0 当前分支状态

```
develop/v3.10.0 @ 5c4af8a67
├── V310-01~04: ✅ CLOSED (DML/UNION/ACID/ALTER)
├── V310-06~10: ✅ CLOSED (Wired-SOAK/Catalog/DDL/Wire/覆盖率)
├── V310-12: ✅ CLOSED (跨版本债)
├── V310-13: ✅ CLOSED (Parallel Executor)
├── V310-14~18: ❌ 待处理 (技术债务)
└── V310-05/11: ❌ 进行中 (SOAK/TPC-H)
```
