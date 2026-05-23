# GMP 内审检索系统 v1.2.0 部署指南

> **版本**: 1.2.0
> **更新日期**: 2026-05-23
> **维护人**: TRAE + DeepSeek-V4-Pro
> **分支**: `develop/v1.2.0`

---

## 1. 环境要求

### 1.1 硬件

| 组件 | 最低要求 | 推荐配置 |
|------|----------|----------|
| CPU | 8 核 | 16 核 (x86_64) |
| RAM | 16 GB | 32 GB |
| 磁盘 | 10 GB SSD | 20 GB SSD |
| GPU | 不需要 | NVIDIA RTX 5080 (可选加速 v1.2.0+) |

### 1.2 软件

| 软件 | 版本 | 说明 |
|------|------|------|
| 操作系统 | macOS 14+ / Ubuntu 22.04+ | 仅 x86_64 (当前) |
| Rust | ≥ 1.80 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` |
| Ollama | ≥ 0.1.0 | 本地 LLM 推理服务 |

---

## 2. 从 v1.1.0 升级

### 2.1 升级路径

v1.1.0 → v1.2.0 支持**原地升级**，无需重建索引。

### 2.2 升级步骤

```bash
# 1. 切换到 v1.2.0 分支
git fetch origin
git checkout develop/v1.2.0

# 2. 拉取最新代码
git pull origin develop/v1.2.0

# 3. 编译
cargo build --release

# 4. 验证版本
./target/release/gmp-eval --version
# 输出: gmp-eval v1.2.0

# 5. 运行数据库迁移（如有）
./target/release/gmp-eval --db data/gmp.db --migrate
```

---

## 3. 全新部署

### 3.1 克隆项目

```bash
git clone http://192.168.0.252:3000/openclaw/GMP-Platform.git
cd GMP-Platform
git checkout develop/v1.2.0
```

### 3.2 编译

```bash
# Release 模式 (推荐)
cargo build --release

# 验证
./target/release/gmp-eval --version
# 输出: gmp-eval v1.2.0
```

### 3.3 安装 Ollama + 模型

```bash
# 安装 Ollama (macOS)
brew install ollama

# 启动服务
ollama serve

# 拉取模型
ollama pull bge-m3:latest              # ~2.2 GB, 嵌入模型
ollama pull bge-reranker-v2-m3:latest  # ~1.5 GB, 重排模型
ollama pull qwen2.5:14b                # ~9 GB, LLM 模型

# 验证
curl http://localhost:11434/api/tags
```

---

## 4. 数据准备

### 4.1 数据文件

确保以下目录存在且数据完整：

```bash
# GMP 文档集
ls gmp-md/*.md | wc -l          # 应为 1696

# 规则库
ls agent-knowledge/*.yaml | wc -l  # 应为 ≥ 55

# 测试场景
ls test-data/408_scenarios.json
```

### 4.2 索引构建

```bash
# 全量索引构建 (耗时 2-4 小时)
./target/release/gmp-eval index --source gmp-md/ --source agent-knowledge/

# 验证索引
./target/release/gmp-eval index --validate

# 预期输出:
# Documents: 1689
# Chunks: ~8700
# Embeddings: ~8700
# Graph nodes: ≥700
# Graph edges: ≥1500
```

---

## 5. 配置

### 5.1 环境变量

```bash
# 可选配置 (导出到 .bashrc 或 .zshrc)
export OLLAMA_HOST="http://192.168.0.252:11434"
export GMP_DB_PATH="./data/gmp.db"
```

### 5.2 检索配置 (可选)

```bash
# 使用默认配置即可，如需自定义:
./target/release/gmp-eval search "偏差" --config config.yaml
```

### 5.3 gmp-api 配置

```bash
# 启动 API 服务器
./target/release/gmp-api --port 8080 --pool-size 10

# 或使用 docker
docker run -p 8080:8080 \
  -v ./data:/data \
  gmp-platform:v1.2.0
```

---

## 6. 验证部署

### 6.1 基本功能验证

```bash
# 1. 全文检索
./target/release/gmp-eval search "SOP" --channel fts | head -5

# 2. 语义检索 (向量通道 - v1.2.0 新增强)
./target/release/gmp-eval search "偏差管理" --channel vector | head -5

# 3. 图谱检索 (NL→Cypher - v1.2.0 新增强)
./target/release/gmp-eval search "偏差调查流程" --graph | head -5

# 4. 规则检索
./target/release/gmp-eval search "偏差分级" --channel rule

# 5. 全通道混合检索
./target/release/gmp-eval search "验证主计划" --all
```

### 6.2 LLM 功能验证

```bash
# 交互式问答
echo "偏差分级标准是什么？" | ./target/release/gmp-eval shell

# LLM-as-Judge 评估模式
./target/release/gmp-eval evaluate --judge-mode
```

### 6.3 性能验证

```bash
# 单查询
time ./target/release/gmp-eval search "偏差与CAPA" --all --rerank

# 批量测试
./target/release/gmp-eval bench --rounds 20

# QPS 基准测试
./target/release/gmp-eval --db data/gmp.db --bench-qps 5
```

---

## 7. 运维

### 7.1 数据库管理

```bash
# 备份
cp data/gmp.db data/gmp.db.bak.$(date +%Y%m%d)

# 恢复
cp data/gmp.db.bak.20260523 data/gmp.db

# 增量备份 (v1.2.0+)
./target/release/gmp-eval backup --output ./backups/

# 查看数据库状态
./target/release/gmp-eval --db data/gmp.db --stats
```

### 7.2 Ollama 管理

```bash
# 查看模型
ollama list

# 重启服务
pkill ollama && ollama serve &

# 查看日志
ollama logs
```

### 7.3 监控

```bash
# 查看 Ollama 资源占用
ps aux | grep ollama

# 查看 gmp.db 大小
ls -lh data/gmp.db

# API 监控 (v1.2.0+)
curl http://localhost:8080/api/v1/stats
```

---

## 8. 故障排查

### 8.1 Ollama 连接失败

```text
错误: Connection refused (os error 61)
```

解决方法:
```bash
# 确认 Ollama 运行
ps aux | grep ollama

# 启动 Ollama
ollama serve &

# 检查 Ollama URL
export OLLAMA_HOST="http://192.168.0.252:11434"
```

### 8.2 索引构建内存不足

```text
错误: Cannot allocate memory
```

解决方法:
```bash
# 减少批处理大小
./target/release/gmp-eval index --source gmp-md/ --batch-size 50
```

### 8.3 向量搜索返回空

```bash
# 验证向量索引
./target/release/gmp-eval index --validate vector

# 如果 embeddings=0，需要重建
./target/release/gmp-eval index --rebuild-embeddings
```

### 8.4 图查询返回空 (v1.2.0 NL→Cypher)

```bash
# 检查图数据
./target/release/gmp-eval --db data/gmp.db --graph-stats

# 手动测试 Cypher
./target/release/gmp-cli shell --cypher "MATCH (n) RETURN count(n)"
```

---

## 9. 安全

### 9.1 文件权限

```bash
chmod 600 data/gmp.db
chmod 755 target/release/gmp-eval
```

### 9.2 网络

- Ollama 默认监听 `localhost:11434`，不对外暴露
- gmp-api 默认监听 `0.0.0.0:8080`，建议配置防火墙
- 如需远程访问 Ollama，修改 `OLLAMA_HOST` 并配置防火墙

### 9.3 API Key 检查

```bash
# 确保无硬编码密钥
grep -r "sk-" crates/ --include="*.rs"   # 应为空
```

---

## 10. Docker 部署 (v1.2.0+)

### 10.1 Dockerfile

```dockerfile
FROM rust:1.80-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/gmp-eval /usr/local/bin/
COPY --from=builder /app/target/release/gmp-api /usr/local/bin/
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
EXPOSE 8080
CMD ["gmp-api", "--port", "8080"]
```

### 10.2 docker-compose.yml

```yaml
version: '3.8'
services:
  gmp-api:
    image: gmp-platform:v1.2.0
    ports:
      - "8080:8080"
    volumes:
      - ./data:/data
    environment:
      - OLLAMA_HOST=http://host.docker.internal:11434
    network_mode: host

  ollama:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
    volumes:
      - ollama:/root/.ollama
    network_mode: host

volumes:
  ollama:
```

### 10.3 启动

```bash
docker-compose up -d
docker-compose logs -f gmp-api
```
