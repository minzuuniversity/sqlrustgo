# v1.3.0 用户手册

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`
> **阶段**: Beta

---

## 一、快速开始

### 1.1 安装

```bash
git clone http://192.168.0.252:3000/openclaw/GMP-Platform.git
cd GMP-Platform
git checkout develop/v1.3.0
cargo build --release --workspace
```

### 1.2 初始化

```bash
# 初始化数据库（HNSW 索引自动构建）
./target/release/gmp-cli init

# 验证
./target/release/gmp-cli --help
```

---

## 二、CLI 命令参考

### 2.1 检索命令

```bash
# 全文检索
./target/release/gmp-cli search "洁净区级别划分"

# 指定通道
./target/release/gmp-cli search "洁净区级别划分" --channels vector,fts --top-k 10

# 四通道融合
./target/release/gmp-cli search "洁净区级别划分" --channels all --top-k 20
```

### 2.2 审计命令

```bash
# 查询审计链
./target/release/gmp-cli audit --actor "user@example.com" --limit 100

# 查询特定 trace
./target/release/gmp-cli audit --trace "trace-id-here"
```

### 2.3 索引命令

```bash
# 重建索引
./target/release/gmp-cli init

# 验证向量索引
./target/release/gmp-cli verify-embeddings
```

---

## 三、配置说明

### 3.1 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `GMP_DB_PATH` | `./gmp.db` | 数据库路径 |
| `GMP_OLLAMA_URL` | `http://localhost:11434` | Ollama 地址 |
| `GMP_MQTT_BROKER` | `tcp://localhost:1883` | MQTT Broker |

### 3.2 配置文件

（v1.3.0 配置格式待最终确定）

---

## 四、MQTT 设备集成

### 4.1 连接设备

```bash
# 设置 MQTT Broker
export GMP_MQTT_BROKER="tcp://localhost:1883"

# 启动 MQTT 服务
./target/release/gmp-cli mqtt-server
```

### 4.2 设备事件格式

称重设备上报：
```json
{
  "device": "scale-01",
  "type": "weight",
  "value": 1250.5,
  "unit": "g",
  "timestamp": "2026-05-24T10:00:00Z"
}
```

温湿度传感器：
```json
{
  "device": "sensor-temp-01",
  "type": "temperature",
  "value": 22.5,
  "unit": "C",
  "timestamp": "2026-05-24T10:00:00Z"
}
```

---

## 五、FAQ

### Q: HNSW 索引重建需要多长时间？
A: 13809 embeddings 预计 < 2h（使用 Ollama bge-m3）

### Q: 向量索引填充需要什么环境？
A: 需要 Ollama 服务运行，并加载 bge-m3 模型

### Q: Event Bus 有什么用？
A: Event Bus 统一管理所有审计事件，MQTT 设备数据会自动进入审计链

### Q: MQTT 设备如何上报数据？
A: 通过 MQTT Broker 上报到 `gmp/events/#` 主题

---

## 六、常见问题

| 问题 | 解决方案 |
|------|---------|
| `cargo build` 失败 | 确保 Rust ≥ 1.80，`cargo clean && cargo build` |
| Ollama 连接失败 | `ollama serve` 确保运行，`curl localhost:11434` 验证 |
| 向量查询无结果 | 确认 13809 embeddings 已生成（`gmp-cli verify-embeddings`）|
| MQTT 连接失败 | 确认 Mosquitto 运行：`docker logs mqtt-broker` |
