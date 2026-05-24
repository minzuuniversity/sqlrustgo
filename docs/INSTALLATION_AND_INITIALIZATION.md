# GMP-Platform 安装与初始化指南

> **版本**: v1.3.0
> **日期**: 2026-05-24
> **目标**: 从源码编译、系统安装、基于原始 GMP MD 文档重建完整检索系统
> **前置条件**: GMP MD 文档目录 `gmp-md/` 已存在于源码目录

---

## 一、五层架构概览

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Layer 5: LLM 推理与问答                                                 │
│   qwen2.5:14b (Ollama) + 意图分类 + 答案生成 + 引用溯源               │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 4: Reranker 重排                                                 │
│   BGE-Reranker-v2-m3 (Cross-Encoder) + RRF 融合                       │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 3: 检索引擎层                                                     │
│   四通道: 规则/向量/全文/图谱 → Top-30 → RRF(k=60) → Top-20          │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 2: 知识图谱层                                                     │
│   GMP 实体节点 + 语义关系边 + Cypher 查询 + 路径推理                     │
│   节点: SOP/Deviation/CAPA/Validation/Device/Material 等                │
│   关系: TRIGGERS/BELONGS_TO/MANAGED_BY/REQUIRES/MANDATES 等           │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 1: 统一数据存储层 — SQLRustGo                                     │
│   RDBMS: gmp_documents + gmp_document_chunks                            │
│   Vector: gmp_embeddings_bin (HNSW, bge-m3 1024d, binary BLOB)         │
│   FTS: gmp_chunks_fts (InvertedIndex + jieba 分词)                     │
│   Graph: gmp_graph_nodes + gmp_graph_edges (DiskGraphStore + WAL)       │
├─────────────────────────────────────────────────────────────────────────┤
│ Layer 0: 离线知识抽取管线 ★ 初始化时运行 ★                              │
│   Step 0: GMP 术语库构建 (gmp_terms + gmp_synonyms)                    │
│   Step 1: NER 实体识别 → doc_entities → Layer 2 节点                   │
│   Step 2: 关系抽取 (RE) → doc_relations → Layer 2 边                   │
│   Step 3: 多维分类 → doc_taxonomy (topic/doc_type/risk/department)      │
│   Step 4: 相关性评分 → doc_relevance (term_density/reg_citation 等)     │
│   Step 5: 图谱写入 → Layer 2 更新 (gmp_graph_nodes + gmp_graph_edges)    │
└─────────────────────────────────────────────────────────────────────────┘
```

### 层级关系

```
Layer 0 输出 → Layer 2 (知识图谱节点/边补充)
Layer 0 输出 → Layer 1 (补充字段: entity_tags, taxonomy_path, relevance_score)
Layer 0 输出 → Layer 3 (检索时可选: 分类过滤/相关性增强)
Layer 0 输出 → Layer 4 (答案引用可溯源到图谱实体路径)
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
ollama pull bge-reranker-v2-m3             # 重排模型
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
│   ├── gmp-cli/              # CLI 工具 (index/search/chat/evaluate)
│   ├── gmp-server/           # HTTP API 服务器
│   ├── gmp-storage/          # 存储引擎 (Layer 1: SQLRustGo + InvertedIndex + HNSW)
│   ├── gmp-retrieval/        # 检索引擎 (Layer 3: 四通道 + RRF 融合)
│   ├── gmp-reranker/         # 重排模型 (Layer 4: BGE-Reranker-v2-m3)
│   ├── gmp-llm/              # LLM 问答 (Layer 5: Ollama)
│   ├── gmp-graph/            # 知识图谱 (Cypher 查询引擎)
│   ├── gmp-kg/               # 知识图谱存储 (Layer 2: 实体/关系/评分)
│   ├── gmp-re/               # 关系抽取 (Layer 0 Step 2)
│   ├── gmp-classifier/       # 多维分类 (Layer 0 Step 3)
│   ├── gmp-scorer/           # 相关性评分 (Layer 0 Step 4)
│   └── gmp-event-bus/         # 事件总线
├── gmp-md/                   # ★ GMP 原始文档 (1696 篇 .md)
├── scripts/
│   └── run_layer0_pipeline.py # ★ Layer 0 管线 (NER/RE/分类/评分/图谱)
├── data/                     # 数据目录 (索引后生成)
│   └── gmp.db               # SQLRustGo FileStorage 数据库
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

### 5.1 Step 1: 文档索引 (Layer 1 建立)

将 1696 篇原始 MD 文档解析、分块并构建全文索引：

```bash
cd /path/to/GMP-Platform

# ★ 核心命令：索引 GMP 文档 → Layer 1 存储
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

### 5.2 Step 2: Layer 0 知识抽取

执行 NER/RE/分类/评分管线，抽取图谱实体和关系：

```bash
cd /path/to/GMP-Platform

# ★ Layer 0 管线：NER → RE → 分类 → 评分 → 图谱写入
python3 scripts/run_layer0_pipeline.py \
  --db ./data/gmp.db \
  --ollama-url http://localhost:11434
```

**执行内容**:

```
Step 0: 从 gmp_terms.sql 加载 GMP 术语库
Step 1: NER - 正则抽取命名实体 → doc_entities 表
Step 2: RE  - 关系抽取 → doc_relations 表
Step 3: CLF - 多维分类 → doc_taxonomy 表
Step 4: SCR - GMP 相关性评分 → doc_relevance 表
Step 5: 图谱写入 → gmp_graph_nodes + gmp_graph_edges (Layer 2)
```

### 5.3 Step 3: 验证索引

```bash
# 验证 Layer 1 索引完整性
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
department: 质量部           # 责任部门
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

### 7.1 搜索 (Layer 3)

```bash
# 四通道并行检索 (默认)
./target/release/gmp-cli search "偏差调查流程" --db ./data/gmp.db

# 仅 FTS5 全文检索
./target/release/gmp-cli search "偏差调查流程" --channel fts --db ./data/gmp.db

# 仅向量检索
./target/release/gmp-cli search "偏差调查流程" --channel vector --db ./data/gmp.db
```

### 7.2 LLM 问答 (Layer 5)

```bash
./target/release/gmp-cli chat \
  "偏差调查的基本流程是什么？" \
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

## 八、Layer 3: 四通道检索架构

### 8.1 通道定义

| 通道 | 数据来源 | 技术 | Top-K | Weight |
|------|----------|------|--------|--------|
| **RuleChannel** | `agent-knowledge/*.yaml` | 关键词匹配规则 | 10 | 2.0 |
| **VectorChannel** | `gmp_embeddings_bin` | bge-m3 1024d + HNSW | 30 | 1.0 |
| **FtsChannel** | `gmp_chunks_fts` | jieba 分词 + OR 连接 | 30 | 1.0 |
| **GraphChannel** | `gmp_graph_nodes/edges` | Cypher 图查询 | 10 | 1.0 |

### 8.2 FTS5 关键设计

```rust
// jieba 中文分词 → OR 连接 (关键: 不是 AND)
let tokens = jieba.cut("偏差调查流程", false);
// → ["偏差", "调查", "流程"]
let fts_query = tokens.join(" OR ");
// → "偏差 OR 调查 OR 流程"
```

### 8.3 RRF 融合

```
四通道各取 Top-K → 最多 80 docs
    ↓
RRF 融合 (k=60): score = Σ weight_i / (k + rank_i(d))
    ↓
Top-20
    ↓
Layer 4: Reranker 重排 → Top-10
    ↓
Layer 5: LLM 生成答案
```

---

## 九、Layer 0: 离线知识抽取管线详解

### 9.1 Layer 0 输出表

```sql
-- GMP 术语库 (Step 0)
CREATE TABLE gmp_terms (
    term_id       INTEGER PRIMARY KEY,
    canonical_term TEXT NOT NULL,
    term_type     TEXT NOT NULL,
    synonyms      TEXT,
    hypernyms     TEXT,
    gmp_chapter   TEXT,
    created_at    INTEGER
);

-- NER 实体识别结果 (Step 1)
CREATE TABLE doc_entities (
    id            INTEGER PRIMARY KEY,
    doc_id        TEXT NOT NULL,
    entity_type   TEXT NOT NULL,
    surface_form  TEXT NOT NULL,
    canonical_term TEXT NOT NULL,
    start_pos     INTEGER,
    end_pos       INTEGER,
    confidence    REAL DEFAULT 1.0,
    created_at    INTEGER
);

-- 关系抽取结果 (Step 2)
CREATE TABLE doc_relations (
    id            INTEGER PRIMARY KEY,
    doc_id        TEXT NOT NULL,
    src_entity    TEXT NOT NULL,
    dst_entity    TEXT NOT NULL,
    rel_type      TEXT NOT NULL,
    rel_direction TEXT,
    created_at    INTEGER
);

-- 多维分类结果 (Step 3)
CREATE TABLE doc_taxonomy (
    doc_id       TEXT PRIMARY KEY,
    topic        TEXT,
    sub_topic    TEXT,
    doc_type     TEXT,
    risk_level   TEXT,
    department   TEXT,
    confidence   REAL DEFAULT 1.0,
    created_at   INTEGER
);

-- 相关性评分结果 (Step 4)
CREATE TABLE doc_relevance (
    doc_id           TEXT PRIMARY KEY,
    overall_score    REAL NOT NULL,
    term_density     REAL,
    reg_citation     REAL,
    process_completeness REAL,
    entity_density   REAL,
    dept_coverage    REAL,
    created_at       INTEGER
);
```

### 9.2 关系类型

| 关系 | 说明 | 示例 |
|------|------|------|
| TRIGGERS | 偏差触发 CAPA | 偏差-001 → CAPA-003 |
| GENERATES | 审计发现生成 CAPA | 审计发现 → CAPA |
| REQUIRES | 设备需要验证 | 混合机 → IQ/OQ/PQ |
| MANDATES | 法规规定 SOP | GMP第84条 → 偏差调查规程 |
| BELONGS_TO | SOP 属于章节 | 清洁验证 → 第7章 |
| MANAGED_BY | SOP 由部门管理 | 偏差调查 → 质量部 |

---

## 十、数据库 Schema (Layer 1: SQLRustGo FileStorage)

### 10.1 核心表

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

-- 知识图谱节点 (Layer 2)
CREATE TABLE gmp_graph_nodes (
    node_id    TEXT PRIMARY KEY,
    node_type  TEXT,
    name       TEXT,
    properties TEXT
);

-- 知识图谱边 (Layer 2)
CREATE TABLE gmp_graph_edges (
    edge_id    TEXT PRIMARY KEY,
    src        TEXT,
    dst        TEXT,
    rel_type   TEXT,
    properties TEXT
);
```

### 10.2 索引

```
gmp_embeddings_bin  → HNSW 索引 (M=16, ef=200)
gmp_document_chunks → InvertedIndex (jieba 中文分词)
```

---

## 十一、Ollama 服务配置

### 11.1 启动

```bash
# macOS/Linux
ollama serve

# 或后台运行
nohup ollama serve > /tmp/ollama.log 2>&1 &
```

### 11.2 验证

```bash
# 测试 Ollama
curl http://localhost:11434/api/tags

# 测试 bge-m3 向量化
curl -X POST http://localhost:11434/api/embeddings \
  -d '{"model":"bge-m3","prompt":"偏差调查流程"}'

# 测试 qwen2.5:14b
curl -X POST http://localhost:11434/api/generate \
  -d '{"model":"qwen2.5:14b","prompt":"你好"}'
```

---

## 十二、完整操作流程 (从零开始)

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
# Step 4: Layer 1 - 文档索引构建
# =============================================
./target/release/gmp-cli index \
  --source ./gmp-md \
  --db ./data/gmp.db

# =============================================
# Step 5: Layer 0 - 知识抽取管线
# =============================================
python3 scripts/run_layer0_pipeline.py \
  --db ./data/gmp.db \
  --ollama-url http://localhost:11434

# =============================================
# Step 6: 验证索引
# =============================================
./target/release/gmp-cli index --validate --db ./data/gmp.db

# =============================================
# Step 7: Layer 3 - 四通道检索测试
# =============================================
./target/release/gmp-cli search "偏差调查流程" --db ./data/gmp.db

# =============================================
# Step 8: Layer 5 - LLM 问答 (Ollama 需运行)
# =============================================
./target/release/gmp-cli chat \
  "偏差调查的基本流程是什么？" \
  --db ./data/gmp.db

# =============================================
# Step 9: 评估模式
# =============================================
./target/release/gmp-cli evaluate --db ./data/gmp.db
```

---

## 十三、常见问题

### Q1: `index` 报 "No such file or directory: gmp-md"

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
./target/release/gmp-cli search "偏差" --db ./data/gmp.db
```

### Q4: Layer 0 脚本报错

```bash
# 确认 Python 依赖
python3 scripts/run_layer0_pipeline.py --help

# 检查数据库路径
ls -lh data/gmp.db
```

---

## 十四、与旧版 (rusqlite) 的差异

| 功能 | 旧版 (rusqlite) | 新版 (SQLRustGo) |
|------|-----------------|-------------------|
| 数据库 | SQLite 单文件 | SQLRustGo FileStorage |
| 向量存储 | JSON 文件 | Binary BLOB (`gmp_embeddings_bin`) |
| FTS5 | SQLite FTS5 虚拟表 | InvertedIndex (sqlrustgo-storage) |
| 依赖 | rusqlite crate + sqlite3 系统库 | 纯 Rust，无系统依赖 |
| 编译 | 需 sqlite3 系统库 | 纯 Rust，无系统依赖 |

---

*最后更新: 2026-05-24*
