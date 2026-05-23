# GMP 内审检索系统 v1.1.0 部署指南

> **版本**: 1.1.0
> **更新日期**: 2026-05-22
> **维护人**: Hermes Agent + AI Pipeline

---

## 1. 环境要求

### 1.1 硬件

| 组件 | 最低要求 | 推荐配置 |
|------|----------|----------|
| CPU | 8 核 | 16 核 (x86_64) |
| RAM | 16 GB | 32 GB |
| 磁盘 | 10 GB SSD | 20 GB SSD |
| GPU | 不需要 | NVIDIA RTX 5080 (可选加速) |

### 1.2 软件

| 软件 | 版本 | 说明 |
|------|------|------|
| 操作系统 | macOS 14+ / Ubuntu 22.04+ | 仅 x86_64 (当前) |
| Rust | >= 1.80 | curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh |
| Ollama | >= 0.1.0 | 本地 LLM 推理服务 |

---

## 2. 安装步骤

### 2.1 克隆项目

```bash
git clone http://192.168.0.252:3000/openclaw/GMP-Platform.git
cd GMP-Platform
git checkout v1.1.0
```

### 2.2 编译

```bash
cargo build --release
./target/release/gmp-retrieve --version
# 输出: gmp-retrieve v1.1.0
```

### 2.3 安装 Ollama + 模型

```bash
brew install ollama
ollama serve
ollama pull bge-m3:latest
ollama pull bge-reranker-v2-m3:latest
ollama pull qwen2.5:14b
curl http://localhost:11434/api/tags
```

---

## 3. 数据准备

### 3.1 数据文件

```bash
ls gmp-md/*.md | wc -l          # 应为 1696
ls agent-knowledge/*.yaml | wc -l  # 应为 >= 55
ls gmp_kg_build/test_results_final/408_scenarios.json
```

### 3.2 索引构建

```bash
# Layer 0 知识抽取管线 (v1.1.0 新增)
gmp-retrieve index --source gmp-md/ --source agent-knowledge/ --layer 0

# 全量索引构建 (含 HNSW 向量索引)
gmp-retrieve index --source gmp-md/ --source agent-knowledge/

# 验证索引
gmp-retrieve index --validate
```

---

## 4. 配置

### 4.1 环境变量

```bash
export OLLAMA_HOST="http://localhost:11434"
export GMP_DB_PATH="./gmp.db"
```

### 4.2 HNSW 配置 (v1.1.0)

HNSW 参数已自动配置: M=16, ef_construction=200, ef_search=100

---

## 5. 验证部署

### 5.1 基本功能验证

```bash
gmp-retrieve search "SOP" --channel fts | head -5
gmp-retrieve search "偏差管理" --channel vector | head -5
gmp-retrieve search "CAPA" --channel graph | head -5
gmp-retrieve search "偏差分级" --channel rule
gmp-retrieve search "验证主计划" --all
```

### 5.2 LLM 功能验证

```bash
echo "偏差分级标准是什么？" | gmp-retrieve shell
```

### 5.3 性能验证

```bash
time gmp-retrieve search "偏差与CAPA" --all --rerank
gmp-retrieve bench --rounds 20
```

---

## 6. 运维

### 6.1 数据库管理

```bash
cp gmp.db gmp.db.bak.$(date +%Y%m%d)
cp gmp.db.bak.20260522 gmp.db
rm -f gmp.db && gmp-retrieve index --source gmp-md/
```

### 6.2 Ollama 管理

```bash
ollama list
pkill ollama && { ollama serve & }
ollama logs
```

### 6.3 HNSW 索引管理 (v1.1.0)

```bash
gmp-retrieve index --validate vector
gmp-retrieve index --rebuild-vector
```

---

## 7. 故障排查

### 7.1 Ollama 连接失败

```text
错误: Connection refused (os error 61)
```

解决方法:
```bash
ps aux | grep ollama
ollama serve &
```

### 7.2 HNSW 索引加载失败

```text
错误: HNSW index not found
```

解决方法:
```bash
gmp-retrieve index --rebuild-vector
```

### 7.3 向量搜索返回空

```bash
gmp-retrieve index --validate vector
rm -f gmp.db
gmp-retrieve index --source gmp-md/
```

---

## 8. 安全

### 8.1 文件权限

```bash
chmod 600 gmp.db
chmod 755 target/release/gmp-retrieve
```

### 8.2 网络

- Ollama 默认监听 localhost:11434，不对外暴露
- 如需远程访问 Ollama，修改 OLLAMA_HOST=0.0.0.0 并配置防火墙

### 8.3 API Key 检查

```bash
grep -r "sk-" crates/ --include="*.rs"   # 应为空
```
