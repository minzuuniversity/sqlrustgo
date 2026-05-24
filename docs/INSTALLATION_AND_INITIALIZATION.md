# GMP-Platform 安装与初始化指南

> **版本**: v1.3.0
> **日期**: 2026-05-24
> **目标**: 从源码编译、系统安装、基于原始 GMP MD 文档重建完整检索系统
> **前置条件**: GMP MD 文档目录 `gmp-md/` 已存在于源码目录

---

## 一、系统架构概览

```
┌─────────────────────────────────────────────────────────────────┐
│                    GMP 合规内审检索系统                           │
├─────────────────────────────────────────────────────────────────┤
│  Layer 4: LLM 问答层    → qwen2.5:14b (Ollama) + RAG + 引用溯源 │
│  Layer 3: 图谱通道      → sqlrustgo-graph (Cypher) 知识图谱       │
│  Layer 2: 检索引擎层    → 规则/向量/FTS5/图谱 四通道并行召回        │
│  Layer 1: 数据存储层    → SQLRustGo (RDBMS + HNSW向量 + 倒排索引)  │
└─────────────────────────────────────────────────────────────────┘

原始 GMP MD 文档 (gmp-md/, 1696 篇)
    ↓
gmp-cli index         → 文档解析/分块/FTS5 索引
scripts/run_layer0_pipeline.py  → NER/RE/分类/评分 + 图谱抽取
    ↓
SQLRustGo FileStorage (./data/gmp.db)
    ├── gmp_documents          (文档元数据, 1696 条)
    ├── gmp_document_chunks    (分块内容, ~8700 chunks)
    ├── gmp_chunks_fts        (FTS5 倒排索引, InvertedIndex)
    ├── gmp_embeddings_bin    (bge-m3 1024d binary BLOB 向量)
    ├── gmp_graph_nodes        (图谱节点, ~700 entities)
    └── gmp_graph_edges        (图谱边, ~1500 relations)
    ↓
四通道检索 + RRF融合 + Reranker → LLM 问答
```

---

## 二、硬件与软件依赖

### 2.1 硬件要求

| 组件 | 最低要求 | 推荐配置 |
|------|----------|----------|
| 内存 | 16 GB | 32 GB+ |
| 存储 | 50 GB | 100 GB SSD |
| GPU | — | NVIDIA GPU (CUDA) 用于 Ollama 加速 |

### 2.2 软件依赖

| 依赖 | 版本 | 安装方式 |
|------|------|----------|
| Rust | 1.75+ | `rustup install stable` |
| Cargo | latest | `rustup update` |
| Python | 3.10+ | 系统自带 |
| Ollama | latest | ollama.com |

### 2.3 Ollama 模型准备

```bash
# 安装 Ollama (macOS/Linux)
curl -fsSL https://ollama.com/install.sh | sh

# 下载必需模型
ollama pull bge-m3                          # 向量嵌入 (1024d)
ollama pull bge-reranker-v2-m3              # 重排模型
ollama pull qwen2.5:14b                    # LLM 问答

# 验证
ollama list
```

---

## 三、源码获取

```bash
git clone http://192.168.0.252:3000/openclaw/GMP-Platform.git
cd GMP-Platform
git checkout develop/v1.3.0
```

### 3.1 目录结构

```
GMP-Platform/
├── crates/
│   ├── gmp-cli/           # CLI 工具 (index/search/chat/evaluate)
│   ├── gmp-server/        # HTTP API 服务器
│   ├── gmp-storage/       # 存储引擎 (SQLRustGo + InvertedIndex + HNSW)
│   ├── gmp-retrieval/     # 四通道检索 + RRF 融合
│   ├── gmp-reranker/      # 重排模型 (bge-reranker-v2-m3)
│   ├── gmp-llm/           # LLM 问答 (Ollama)
│   ├── gmp-graph/         # 知识图谱 (Cypher)
│   ├── gmp-event-bus/      # 事件总线
│   └── gmp-api/           # API 类型定义
├── gmp-md/                # ★ GMP 原始文档 (1696 篇 .md)
├── scripts/
│   └── run_layer0_pipeline.py  # NER/RE/分类/评分 + 图谱抽取
├── data/                  # 数据目录 (索引后生成)
│   └── gmp.db             # SQLRustGo FileStorage 数据库
└── docs/releases/v1.3.0/
```

---

## 四、从源码编译

### 4.1 编译

```bash
# Release 编译 (完整 workspace)
cargo build --release --workspace

# 仅编译 gmp-cli (日常使用)
cargo build --release -p gmp-cli
```

### 4.2 验证编译

```bash
./target/release/gmp-cli --version
./target/release/gmp-cli --help
```

---

## 五、数据初始化流程

### 5.1 Step 1: 文档索引 (gmp-cli index)

将 1696 篇原始 MD 文档解析、分块并构建全文索引：

```bash
cd /path/to/GMP-Platform

# ★ 核心命令：索引 GMP 文档
./target/release/gmp-cli index \
  --source ./gmp-md \
  --db ./data/gmp.db
```

**执行内容**:
```
Step 1: 遍历 gmp-md/**/*.md (1696 个文件)
Step 2: 解析 YAML frontmatter (code, title, version, department)
Step 3: 按 ## 二级标题分块 → ~8700 chunks
Step 4: 写入 gmp_documents 表 (元数据)
Step 5: 写入 gmp_document_chunks 表 (分块内容)
Step 6: 构建 FTS5 倒排索引 (InvertedIndex, jieba 分词)
Step 7: 迁移已有向量 → gmp_embeddings_bin (binary BLOB, HNSW 索引)
```

**输出示例**:
```
Indexing from: ./gmp-md...
Indexing complete:
  Documents imported: 1696
  Chunks indexed: 8734
```

### 5.2 Step 2: Layer 0 知识抽取 (Python 脚本)

执行 NER/RE/分类/评分管线，抽取图谱实体和关系：

```bash
cd /path/to/GMP-Platform

# 完整管线 (NER → RE → 分类 → 评分 → 图谱写入)
python3 scripts/run_layer0_pipeline.py \
  --db ./data/gmp.db \
  --ollama-url http://localhost:11434
```

**执行内容**:
```
Step 1: NER - 正则抽取命名实体 (SOP/Deviation/CAPA/Equipment/Regulation 等)
Step 2: RE  - 关系抽取 (TRIGGERS/GENERATES/REQUIRES/MANDATES 等)
Step 3: CLF - 文档分类 (topic/doc_type/risk_level/department)
Step 4: SCR - GMP 相关性评分
Step 5: 写入 gmp_graph_nodes 和 gmp_graph_edges
```

### 5.3 Step 3: 验证索引

```bash
# 验证索引完整性
./target/release/gmp-cli index --validate --db ./data/gmp.db
```

**输出示例**:
```
Index Validation:
  Documents: 1696
  Chunks: 8734
  Embeddings: 8734
  FTS Tokens: 52410
Status: VALID
```

---

## 六、GMP MD 文档格式

### 6.1 文档结构

原始 `gmp-md/` 中的文档命名规范：

```
GMP-{CH}-{TYPE}-{SEQ}_{SUBSEQ}_{TITLE}.md

示例:
GMP-01-GUI-001_001_公司质量方针和质量目标管理规程.md
GMP-02-SOP-001_001_文件标准管理规程.md
GMP-07-BPR-001_批生产记录表.md
```

| 字段 | 说明 | 示例 |
|------|------|------|
| CH | 章节号 01-09, APPX | 01, 02, ..., APPX |
| TYPE | 文档类型 | GUI/SOP/REC/DEV/CAP/POL/AUD/OOS/VAL |
| SEQ | 顺序号 | 001, 002, ... |
| SUBSEQ | 子序号 | 001, 002, ... |
| TITLE | 中文标题 | 文件标准管理规程 |

### 6.2 YAML Frontmatter

```yaml
---
document_type: SOP           # 文档类型 (SOP/GUI/REC/DEV/CAP/POL 等)
code: GMP-01-SOP-001_001   # 文档编号
title: 文件标准管理规程     # 文档标题
version: V1.0              # 版本号
effective_date: 2025-01-01  # 生效日期
department: 质量部          # 责任部门
---
```

### 6.3 章节目录

```
gmp-md/
├── CH01-文件管理与数据完整性/     # GUI/SOP/REC
├── CH02-质量管理体系/            # GUI/SOP/REC/DEV/CAP/POL
├── CH03-机构与人员/              # GUI/SOP/REC/TRA
├── CH04-厂房与设施/              # GUI/SOP/REC
├── CH05-设备管理/                # GUI/SOP/REC/VAL
├── CH06-物料与产品/              # GUI/SOP/REC
├── CH07-确认与验证/              # GUI/SOP/REC/BPR/VAL
├── CH08-生产管理/                # GUI/SOP/REC/BPR/AUD/OOS
├── CH09-委托生产与委托检验/       # GUI/SOP/REC
└── APPX-A~L/                    # 附录 (~380 篇)
```

---

## 七、检索命令

### 7.1 搜索

```bash
# 四通道并行检索 (默认)
./target/release/gmp-cli search 偏差调查流程 --db ./data/gmp.db

# 仅 FTS5 全文检索
./target/release/gmp-cli search 偏差调查流程 --channel fts --db ./data/gmp.db

# 仅向量检索
./target/release/gmp-cli search 偏差调查流程 --channel vector --db ./data/gmp.db
```

### 7.2 LLM 问答

```bash
./target/release/gmp-cli chat \
  偏差调查的基本流程是什么？ \
  --ollama-url http://localhost:11434 \
  --model qwen2.5:14b \
  --db ./data/gmp.db
```

### 7.3 评估模式

```bash
./target/release/gmp-cli evaluate \
  --db ./data/gmp.db \
  --ollama-url http://localhost:11434
```

---

## 八、四通道检索架构

### 8.1 通道定义

| 通道 | 数据来源 | 技术 | Top-K |
|------|----------|------|-------|
| **RuleChannel** | `agent-knowledge/*.yaml` | 关键词匹配规则 | 10 |
| **VectorChannel** | `gmp_embeddings_bin` | bge-m3 1024d + HNSW | 30 |
| **FtsChannel** | `gmp_chunks_fts` | jieba 分词 + OR 连接 | 30 |
| **GraphChannel** | `gmp_graph_nodes/edges` | Cypher 图查询 | 10 |

### 8.2 FTS5 关键设计

```rust
// jieba 中文分词 → OR 连接 (关键: 不是 AND)
let tokens = jieba.cut(偏差调查流程, false);
// → [偏差, 调查, 流程]
let fts_query = tokens.join( OR );
// → 偏差 OR 调查 OR 流程
```

### 8.3 RRF 融合

```rust
// Reciprocal Rank Fusion, k=60
RRF_score(d) = Σ 1/(k + rank_i(d))

四通道各取 Top-5 → 最多 20 docs
    ↓
RRF 融合 (k=60) → Top-20
    ↓
Reranker 重排 → Top-10
    ↓
LLM 生成答案
```

---

## 九、数据库 Schema (SQLRustGo FileStorage)

### 9.1 核心表

```sql
-- 文档元数据
CREATE TABLE gmp_documents (
    doc_id        TEXT PRIMARY KEY,
    code          TEXT,
    title         TEXT,
    chapter       TEXT,
    doc_type      TEXT,
    department    TEXT,
    status        TEXT,
    source_path   TEXT,
    mtime         INTEGER,
    version       TEXT,
    effective_date TEXT,
    created_at    INTEGER,
    updated_at    INTEGER
);

-- 分块内容
CREATE TABLE gmp_document_chunks (
    chunk_id    INTEGER PRIMARY KEY,
    doc_id      TEXT,
    chunk_idx   INTEGER,
    content     TEXT,
    token_count INTEGER
);

-- 向量嵌入 (bge-m3, 1024d, binary BLOB)
CREATE TABLE gmp_embeddings_bin (
    chunk_id   INTEGER PRIMARY KEY,
    doc_id     TEXT,
    embedding  BLOB,       -- binary float32[1024]
    model      TEXT
);

-- 知识图谱节点
CREATE TABLE gmp_graph_nodes (
    node_id    TEXT PRIMARY KEY,
    node_type  TEXT,       -- Batch/Device/SOP/Deviation/CAPA/Regulation
    name       TEXT,
    properties TEXT
);

-- 知识图谱边
CREATE TABLE gmp_graph_edges (
    edge_id    TEXT PRIMARY KEY,
    src        TEXT,
    dst        TEXT,
    rel_type   TEXT,       -- TRIGGERS/GENERATES/REQUIRES/MANDATES
    properties TEXT
);
```

### 9.2 索引

```
gmp_embeddings_bin  → HNSW 索引 (M=16, ef=200)
gmp_document_chunks → InvertedIndex (jieba 中文分词)
```

---

## 十、Ollama 服务配置

### 10.1 启动

```bash
# macOS/Linux
ollama serve

# 或后台运行
nohup ollama serve > /tmp/ollama.log 2>&1 &
```

### 10.2 验证

```bash
# 测试 Ollama
curl http://localhost:11434/api/tags

# 测试 bge-m3 向量化
curl -X POST http://localhost:11434/api/embeddings \
  -d '{model:bge-m3,prompt:偏差调查流程}'

# 测试 qwen2.5:14b
curl -X POST http://localhost:11434/api/generate \
  -d '{model:qwen2.5:14b,prompt:你好}'
```

---

## 十一、完整操作流程 (从零开始)

```bash
# =============================================
# Step 1: 准备环境
# =============================================
git clone http://192.168.0.252:3000/openclaw/GMP-Platform.git
cd GMP-Platform
git checkout develop/v1.3.0

# =============================================
# Step 2: 编译
# =============================================
cargo build --release --workspace

# =============================================
# Step 3: 安装 Ollama (新终端)
# =============================================
ollama pull bge-m3
ollama pull bge-reranker-v2-m3
ollama pull qwen2.5:14b
ollama serve

# =============================================
# Step 4: 索引构建
# =============================================
./target/release/gmp-cli index \
  --source ./gmp-md \
  --db ./data/gmp.db

# =============================================
# Step 5: Layer 0 知识抽取
# =============================================
python3 scripts/run_layer0_pipeline.py \
  --db ./data/gmp.db \
  --ollama-url http://localhost:11434

# =============================================
# Step 6: 验证
# =============================================
./target/release/gmp-cli index --validate --db ./data/gmp.db

# =============================================
# Step 7: 检索测试
# =============================================
./target/release/gmp-cli search 偏差调查流程 --db ./data/gmp.db

# =============================================
# Step 8: LLM 问答 (Ollama 需运行)
# =============================================
./target/release/gmp-cli chat \
  偏差调查的基本流程是什么？ \
  --db ./data/gmp.db

# =============================================
# Step 9: 评估模式
# =============================================
./target/release/gmp-cli evaluate --db ./data/gmp.db
```

---

## 十二、常见问题

### Q1: `index` 报 No such file or directory: gmp-md

```bash
# 确认目录存在
ls gmp-md/*.md | wc -l    # 应输出 1696

# 使用绝对路径
./target/release/gmp-cli index --source /absolute/path/to/gmp-md
```

### Q2: `index --validate` 显示 embeddings 为 0

向量嵌入从 `data/gmp_data/gmp_embeddings_bin.json` 迁移（首次 `index` 时自动执行）。确保 Ollama 可访问：
```bash
curl http://localhost:11434/api/tags
```

### Q3: `search` 返回空结果

```bash
# 先验证索引状态
./target/release/gmp-cli index --validate --db ./data/gmp.db

# 测试简单查询
./target/release/gmp-cli search 偏差 --db ./data/gmp.db
```

### Q4: Layer 0 脚本报错

```bash
# 确认 Python 依赖
python3 scripts/run_layer0_pipeline.py --help

# 检查数据库路径
ls -lh data/gmp.db
```

---

## 十三、与旧版 (rusqlite) 的差异

| 功能 | 旧版 (rusqlite) | 新版 (SQLRustGo) |
|------|-----------------|-------------------|
| 数据库 | SQLite 单文件 | SQLRustGo FileStorage |
| 向量存储 | JSON 文件 (`gmp_embeddings.json`) | Binary BLOB (`gmp_embeddings_bin`) |
| FTS5 | SQLite FTS5 虚拟表 | InvertedIndex (sqlrustgo-storage) |
| 依赖 | rusqlite crate + sqlite3 系统库 | 纯 Rust，无系统依赖 |
| 编译 | 需 sqlite3 系统库 | 纯 Rust，无系统依赖 |

---

*最后更新: 2026-05-24*
