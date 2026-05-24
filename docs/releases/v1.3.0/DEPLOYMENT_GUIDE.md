# v1.3.0 部署指南

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`
> **阶段**: Beta

---

## 一、系统要求

### 1.1 硬件要求

| 组件 | 最低要求 | 推荐 |
|------|----------|------|
| CPU | 8 核 | 16 核 |
| RAM | 16 GB | 32 GB |
| 磁盘 | 10 GB | 20 GB |
| GPU | 不需要 | RTX 5080 (可选，向量生成加速) |

### 1.2 软件依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| Rust | ≥ 1.80 | 编译 |
| SQLRustGo | latest | 数据底座 |
| Ollama | ≥ 0.1.0 | LLM 推理 |
| bge-m3 | latest | 嵌入模型 (1024d) |
| qwen2.5 | 14b | LLM 答案生成 |
| bge-reranker-v2-m3 | latest | Reranker |
| MQTT Broker (可选) | Mosquitto | IoT 设备集成 |

---

## 二、快速部署

### 2.1 二进制构建

```bash
git clone http://192.168.0.252:3000/openclaw/GMP-Platform.git
cd GMP-Platform
git checkout develop/v1.3.0

# 编译（推荐 Release）
cargo build --release --workspace

# 验证编译成功
./target/release/gmp-cli --version
```

### 2.2 数据库初始化

```bash
# 初始化数据库（自动构建 HNSW 索引）
./target/release/gmp-cli init

# 导入文档（如已有数据可跳过）
./target/release/gmp-cli index --path ./gmp-md
```

### 2.3 Ollama 服务

```bash
# 启动 Ollama
ollama serve

# 验证模型
curl -s http://localhost:11434/api/tags | jq '.models[].name'
# 确保 bge-m3, qwen2.5:14b, bge-reranker-v2-m3 存在
```

---

## 三、MQTT 集成部署（可选）

### 3.1 MQTT Broker

```bash
# 使用 Docker 启动 Mosquitto
docker run -d \
  --name mqtt-broker \
  -p 1883:1883 \
  eclipse-mosquitto:2 \
  mosquitto -c /mosquitto-no-auth.conf

# 验证
docker logs mqtt-broker
```

### 3.2 MQTT 环境变量

```bash
export GMP_MQTT_BROKER="tcp://localhost:1883"
export GMP_MQTT_CLIENT_ID="gmp-platform-$(hostname)"

# 运行 MQTT 服务
./target/release/gmp-cli mqtt-server
```

---

## 四、配置说明

### 4.1 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `GMP_DB_PATH` | `./gmp.db` | 数据库路径 |
| `GMP_OLLAMA_URL` | `http://localhost:11434` | Ollama 服务地址 |
| `GMP_MQTT_BROKER` | `tcp://localhost:1883` | MQTT Broker 地址 |
| `RUST_LOG` | `info` | 日志级别 |

### 4.2 gmp-server (MySQL 协议)

```bash
# 启动 MySQL 协议服务（端口 3306）
./target/release/gmp-server --port 3306
```

---

## 五、验证部署

```bash
# 1. 编译验证
cargo build --release --workspace && echo "✅ BUILD OK"

# 2. Lint 验证
cargo clippy --all-targets --all-features -- -D warnings && echo "✅ CLIPPY OK"

# 3. 安全审计
cargo audit && echo "✅ AUDIT OK"

# 4. 单元测试
cargo test --lib && echo "✅ TESTS OK"

# 5. CLI 验证
./target/release/gmp-cli --help

# 6. 向量索引验证（待 P0-1 完成）
./target/release/gmp-cli verify-embeddings
```

---

## 六、回滚方案

### 6.1 二进制回滚

```bash
# 停止当前服务
pkill -f gmp-cli
pkill -f gmp-server

# 回滚到 v1.2.0
cd GMP-Platform
git checkout develop/v1.2.0
cargo build --release --workspace

# 重启服务
./target/release/gmp-cli server
```

### 6.2 数据回滚

v1.3.0 schema 完全兼容 v1.2.0，无需数据迁移。直接切换二进制即可。

---

## 七、Docker 部署（可选）

```dockerfile
FROM rust:1.80-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release --workspace

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/gmp-cli /usr/local/bin/
ENTRYPOINT ["gmp-cli"]
```
