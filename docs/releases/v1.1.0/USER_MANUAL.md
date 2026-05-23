# GMP 内审检索系统 v1.1.0 用户手册

> **版本**: 1.1.0
> **更新日期**: 2026-05-22
> **维护人**: Hermes Agent + AI Pipeline

---

## 1. 快速开始

### 1.1 安装

```bash
# 克隆项目
git clone http://192.168.0.252:3000/openclaw/GMP-Platform.git
cd GMP-Platform
git checkout v1.1.0

# 编译
cargo build --release
# 二进制: target/release/gmp-retrieve

# 验证
./target/release/gmp-retrieve --version
```

### 1.2 前置要求

```bash
# 1. 确保 Ollama 运行并加载模型
ollama pull bge-m3:latest
ollama pull bge-reranker-v2-m3:latest
ollama pull qwen2.5:14b

# 2. 验证模型加载
curl http://localhost:11434/api/tags | grep -E "bge-m3|bge-reranker|qwen2.5"
```

---

## 2. 数据索引构建

### 2.1 首次索引

```bash
# 全量导入 (文档 + 向量化 + 图谱 + Layer 0)
gmp-retrieve index --source gmp-md/ --source agent-knowledge/

# 预计耗时: 2-4 小时 (1696 文档, CPU)
# 产出: gmp.db (SQLRustGo 数据库文件)
```

### 2.2 Layer 0 索引 (v1.1.0 新增)

```bash
# 单独构建 Layer 0 知识抽取管线
gmp-retrieve index --source gmp-md/ --layer 0

# 验证 Layer 0 结果
gmp-retrieve index --validate
```

### 2.3 验证索引

```bash
# 检查完整性
gmp-retrieve index --validate

# 输出示例:
# Documents:  1696/1696
# Chunks:     8742
# Vectors:    8742 (HNSW)
# Nodes:      723
# Edges:      1581
# Terms:      508+
```

---

## 3. 检索查询

### 3.1 基本查询

```bash
# 全通道检索 (默认)
gmp-retrieve search "偏差分级标准"

# 指定通道
gmp-retrieve search "SOP起草流程" --channel fts      # 全文检索
gmp-retrieve search "CAPA实施" --channel vector       # 语义检索 (HNSW)
gmp-retrieve search "质量标准" --channel rule          # 规则检索
gmp-retrieve search "混合机操作" --channel graph       # 图谱检索

# 启用 Reranker 重排
gmp-retrieve search "验证主计划" --rerank
```

### 3.2 调试模式

```bash
# 查看各通道分数
gmp-retrieve search "偏差与CAPA的关系" --all --explain
```

---

## 4. LLM 问答

### 4.1 交互式 Shell

```bash
# 启动交互式 Shell
gmp-retrieve shell

# 在 Shell 中提问
偏差分级标准是什么？

# 退出
/exit
```

### 4.2 单次问答

```bash
# 直接提问
gmp-retrieve chat "GMP原料药管理要求是什么"

# 带意图分类
gmp-retrieve chat "偏差处理流程" --route
```

---

## 5. 评估与测试

### 5.1 批量场景评估 (v1.1.0: 408 场景)

```bash
# 运行评估
gmp-retrieve evaluate --test-set gmp_kg_build/test_results_final/408_scenarios.json

# 输出: 407/408 PASS (99.8%)
```

### 5.2 性能基准测试

```bash
# 性能测试
gmp-retrieve bench --rounds 20

# v1.1.0 预期结果:
# QPS: 154.0
# P95: 6.5ms
```

---

## 6. 增量更新

### 6.1 新增文档

```bash
# 增量索引 (自动跳过未变化文件)
gmp-retrieve index --source new_doc.md --append

# 增量 Layer 0 处理
gmp-retrieve index --layer 0 --incremental
```

### 6.2 重建索引

```bash
# 重建向量索引
gmp-retrieve index --rebuild-vector

# 重建 Layer 0
gmp-retrieve index --layer 0 --rebuild
```

---

## 7. 附录

### 7.1 CLI 帮助

```bash
gmp-retrieve --help
gmp-retrieve index --help
gmp-retrieve search --help
gmp-retrieve evaluate --help
gmp-retrieve shell --help
gmp-retrieve bench --help
```

### 7.2 配置文件

默认配置位于: `~/.config/gmp-retrieve/config.yaml`

### 7.3 数据目录

| 目录 | 说明 |
|------|------|
| gmp-md/ | GMP 源文档 |
| agent-knowledge/ | 规则库 YAML |
| gmp_kg_build/ | 测试场景 |

---

**文档版本**: v1.1.0
**最后更新**: 2026-05-22
