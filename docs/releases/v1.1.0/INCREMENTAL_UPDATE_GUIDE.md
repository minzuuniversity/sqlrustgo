# GMP-Platform v1.1.0 增量索引更新指南

> 日期: 2026-05-22

---

## 一、概述

增量索引是 GMP-Platform 的核心性能优化特性。当文档目录内容发生变化时，系统仅处理新增或修改的文件，跳过未变化的已有文档，显著提升大规模文档库的索引效率。

### 1.1 问题背景

传统全文检索系统的全量索引存在以下问题：
- **时间成本高**: 每次全量扫描所有文档
- **资源浪费**: 重复处理未变化的稳定文档
- **响应延迟**: 大型文档库（1000+）索引耗时数分钟

### 1.2 解决方案

GMP-Platform 通过 **文件修改时间（mtime）追踪** 实现增量索引：
- 记录每个文档的 file_mtime
- 索引时比对 mtime，跳过未变化文件
- 仅处理新增或内容变更的文档

---

## 二、技术实现

### 2.1 数据库 Schema

```sql
CREATE TABLE IF NOT EXISTS gmp_documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id TEXT UNIQUE NOT NULL,
    code TEXT,
    title TEXT NOT NULL,
    chapter TEXT,
    doc_type TEXT,
    department TEXT,
    status TEXT DEFAULT 'active',
    source_file TEXT,
    file_mtime INTEGER,      -- 文件修改时间戳
    version TEXT,
    effective_date TEXT,
    created_at INTEGER DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER DEFAULT (strftime('%s', 'now'))
);
```

### 2.2 Layer 0 增量管线 (v1.1.0)

v1.1.0 新增 Layer 0 离线知识抽取管线，支持增量处理：

```bash
# 增量 Layer 0 处理
gmp-retrieve index --source gmp-md/ --layer 0 --incremental

# 增量检索
gmp-retrieve search "query" --use-layer-0
```

---

## 三、使用方式

### 3.1 CLI 使用

```bash
# 索引文档目录（自动增量）
gmp-retrieve index ./docs --db ./data/gmp.db

# 输出示例（首次运行）
# Indexing /path/to/docs...
# Stats: imported=150, chunks=892, errors=0

# 输出示例（再次运行，无变化）
# Indexing /path/to/docs...
# Stats: imported=0, skipped=150, chunks=892, errors=0
```

### 3.2 Layer 0 增量处理

```bash
# 仅处理 Layer 0（术语库、NER、RE、分类、评分）
gmp-retrieve index --source gmp-md/ --layer 0 --incremental

# 输出示例
# Layer 0 Processing...
# Terms: 5 new, 0 updated, 503 unchanged
# Entities: 120 new, 15 updated, 0 unchanged
# Relations: 80 new, 10 updated, 0 unchanged
```

---

## 四、增量索引场景

### 4.1 文档更新流程

```
1. 用户修改 docs/GMP-001.md
   └── file_mtime: 1716200000 (新时间戳)

2. 运行 gmp-retrieve index ./docs
   ├── 查询数据库: GMP-001.md mtime = 1716100000 (旧)
   └── 发现不匹配 -> 重新导入
       ├── DELETE 旧记录 (doc_id = "GMP-001")
       └── INSERT 新记录 (新的 mtime + 内容)

3. 输出: imported=1, skipped=149
```

### 4.2 Layer 0 增量更新

```
1. 运行 gmp-retrieve index --layer 0 --incremental
   ├── 检查 doc_entities 中是否有已处理的 doc_id
   └── 对比 mtime 或内容哈希

2. 仅处理:
   ├── 新增文档的 NER/RE/分类/评分
   └── 内容变化文档的 NER/RE/分类/评分

3. 稳定文档: 跳过 Layer 0 处理
```

---

## 五、性能数据

### 5.1 索引性能对比

| 文档规模 | 全量索引 | 增量索引(无变化) | 提升 |
|---------|---------|-----------------|------|
| 50 docs | ~2-3s | ~50ms | 40-60x |
| 100 docs | ~4-5s | ~100ms | 40-50x |
| 500 docs | ~20-30s | ~500ms | 40-60x |
| 1000 docs | ~60s+ | ~1s | 60x+ |

### 5.2 Layer 0 增量性能

| 处理阶段 | 全量 | 增量(无变化) |
|---------|------|-------------|
| 术语库构建 | ~5min | ~100ms |
| NER 实体识别 | ~10min | ~200ms |
| 关系抽取 | ~8min | ~150ms |
| 多维分类 | ~3min | ~50ms |
| 相关性评分 | ~2min | ~30ms |

---

## 六、限制与注意事项

### 6.1 当前限制

1. **文件删除不自动清理**: 已索引的文档删除后，记录仍保留在数据库
2. **跨文件系统移动**: 可能被识别为新文件（旧记录孤立）
3. **网络挂载**: mtime 精度可能受文件系统限制

### 6.2 建议工作流

```bash
# 定期重建索引（清理孤立记录）
gmp-retrieve index --rebuild ./docs

# 查看索引统计
gmp-retrieve validate --db ./data/gmp.db

# 强制重建 Layer 0
gmp-retrieve index --layer 0 --rebuild
```

---

## 七、测试覆盖

| 测试 | 描述 | 状态 |
|------|------|------|
| test_incremental_index_tracks_stats | 增量索引统计正确 | ✅ |
| test_incremental_index_reindex_on_change | 文件变化时重新索引 | ✅ |
| test_incremental_index_new_file_detected | 新文件检测 | ✅ |
| test_layer0_incremental_processing | Layer 0 增量处理 | ✅ |

---

## 八、相关 Issue

- **D-001**: 图索引无持久化 - ✅ 已修复 (SqlrustgoAdapter)
- **D-002**: 16 场景失败 - ✅ 已修复 (Layer 0 术语标准化)
