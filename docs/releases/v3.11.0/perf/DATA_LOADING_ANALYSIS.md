# SQLRustGo 数据加载性能分析

**日期**: 2026-07-18
**问题**: SF=1.0 数据加载时间过长

---

## 1. 问题描述

SF=1.0 数据集 (6M lineitem rows) 加载时间预估 **超过 10 小时**，而 PostgreSQL 相同数据仅需 **31 秒**。

---

## 2. 实测数据对比

### 2.1 加载速度对比

| 数据库 | 方法 | 6M lineitem 加载时间 | 吞吐量 |
|--------|------|----------------------|--------|
| **PostgreSQL** | COPY FROM | **31 秒** | ~200,000 rows/s |
| **SQLite** | .import | ~60 秒 | ~100,000 rows/s |
| **SQLRustGo** | INSERT via wire | **>10 小时** (预估) | ~200 rows/s |

### 2.2 INSERT 延迟实测

| 操作 | SQLRustGo | PostgreSQL | 差距 |
|------|-----------|-----------|------|
| 单条 INSERT | ~3,000 ms | <1 ms | 3000x |
| 100行批量 | 258 ms | <1 ms | 258x |
| 1000行批量 | 6,023 ms | <10 ms | 600x |

---

## 3. 瓶颈分析

### 3.1 主要瓶颈

```
┌─────────────────────────────────────────────────────────────┐
│                    当前加载流程                              │
│                                                             │
│  CLI → MySQL Wire → Parse → Plan → Execute → WAL → Storage   │
│    │           │        │       │         │        │          │
│   0ms        0ms     0ms    ~0ms     ~0ms    ~3000ms      │
│                                              ↑              │
│                                       每次 INSERT 3秒      │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 根因分析

**问题 1**: 无批量加载命令
- PostgreSQL: `COPY FROM file`
- MySQL: `LOAD DATA INFILE`
- SQLRustGo: ❌ 不支持

**问题 2**: INSERT via wire protocol 开销
- 每次 INSERT 约 3 秒延迟
- 包含: 协议解析、SQL 解析、执行、WAL 写入、存储

**问题 3**: 批处理效率低
- 虽然批量 INSERT (1000行) 比单条快
- 但服务器仍逐行处理每批数据

**问题 4**: WAL 同步策略
- 默认 `wal_sync = every` (每次写入都 sync)
- PostgreSQL 使用 group commit 优化

---

## 4. 优化建议

### 4.1 高优先级 (立即实现)

#### 方案 A: 增加批量 INSERT 优化
```rust
// 在 executor 中识别批量 INSERT
// 批量插入时跳过逐行 WAL，每 1000 行写入一次
INSERT INTO t VALUES (...), (...), (...)  // 1000行
```

#### 方案 B: 添加 COPY/FROM 命令
```sql
COPY table FROM 'path/to/data.tbl' WITH (FORMAT tbl);
```

#### 方案 C: 调整 WAL 策略
```bash
sqlrustgo serve --wal-sync batch  # 每秒同步一次
```

### 4.2 中优先级 (短期内实现)

#### 方案 D: 内存表批量写入
```rust
// 先写入内存表，最后一次性刷入磁盘
INSERT INTO memory_table SELECT * FROM source;
FLUSH memory_table TO disk;
```

#### 方案 E: 并行加载
```bash
# 使用多个并发连接
sqlrustgo import --parallel 4 --file data.tbl
```

### 4.3 长期优化

#### 方案 F: 二进制格式支持
- 使用 `sf1_binary_import.rs` 的二进制格式
- 直接内存映射，避免解析开销

---

## 5. 预期改进

| 优化方案 | 预估提升 | 实现难度 |
|---------|---------|---------|
| 批量 INSERT 优化 | 5-10x | 低 |
| COPY FROM 命令 | 50-100x | 中 |
| WAL batch 模式 | 3-5x | 低 |
| 并行加载 | 2-4x | 中 |
| **综合** | **100-500x** | - |

**目标**: 将 6M 行加载时间从 **10+ 小时** 降至 **1-5 分钟**

---

## 6. 参考实现

### PostgreSQL COPY 优化技术

1. **批量元组插入**: 一次处理多行
2. **延迟 WAL**: batch commit
3. **跳过索引更新**: 批量构建
4. **内存映射文件**: mmap

### 相关代码

- `crates/storage/examples/sf1_binary_import.rs` - 二进制格式导入示例
- `crates/mysql-server/src/load_data.rs` - 现有 LOAD DATA 实现

---

## 7. 下一步

1. [ ] 在 `sqlrustgo-mysql-server` 中实现批量 INSERT 优化
2. [ ] 添加 `COPY FROM` 命令支持
3. [ ] 添加 `--wal-sync batch` 参数
4. [ ] 添加 `--parallel-import` 参数

---

**Report Generated**: 2026-07-18
