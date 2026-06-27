# v3.4.0 部署指南

## 部署模式

### 1. 单机部署（开发/测试）

```bash
# 直接运行
./target/release/sqlrustgo serve --port 5432 --data /var/lib/sqlrustgo/data
```

### 2. Systemd 服务部署（生产）

```bash
# 创建服务文件
sudo -S -p '' tee /etc/systemd/system/sqlrustgo.service << 'SERVICE'
[Unit]
Description=SQLRustGo Database
After=network.target

[Service]
Type=simple
User=sqlrustgo
Group=sqlrustgo
ExecStart=/usr/local/bin/sqlrustgo serve --port 5432 --config /etc/sqlrustgo/sqlrustgo.toml
Restart=on-failure
RestartSec=10s
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
SERVICE

# 启用服务
sudo -S -p '' systemctl daemon-reload
sudo -S -p '' systemctl enable sqlrustgo
sudo -S -p '' systemctl start sqlrustgo

# 检查状态
sudo -S -p '' systemctl status sqlrustgo
```

### 3. Docker Compose 部署（开发环境）

```yaml
version: '3.8'
services:
  sqlrustgo:
    image: ghcr.io/minzuuniversity/sqlrustgo:v3.4.0
    ports:
      - "5432:5432"
    volumes:
      - ./data:/var/lib/sqlrustgo/data
      - ./config:/etc/sqlrustgo
    environment:
      - RUST_LOG=info
    restart: unless-stopped
```

```bash
docker-compose up -d
```

### 4. Kubernetes 部署（生产）

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: sqlrustgo
spec:
  selector:
    matchLabels:
      app: sqlrustgo
  serviceName: sqlrustgo
  replicas: 1
  template:
    metadata:
      labels:
        app: sqlrustgo
    spec:
      containers:
      - name: sqlrustgo
        image: ghcr.io/minzuuniversity/sqlrustgo:v3.4.0
        ports:
        - containerPort: 5432
        volumeMounts:
        - name: data
          mountPath: /var/lib/sqlrustgo
        resources:
          requests:
            memory: "4Gi"
            cpu: "1000m"
          limits:
            memory: "16Gi"
            cpu: "4000m"
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 100Gi
```

## 部署配置

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `RUST_LOG` | 日志级别 | `info` |
| `SQLRUSTGO_PORT` | 端口 | `5432` |
| `SQLRUSTGO_DATA` | 数据目录 | `/var/lib/sqlrustgo` |
| `SQLRUSTGO_CONFIG` | 配置文件路径 | `/etc/sqlrustgo/sqlrustgo.toml` |

### 性能调优

```toml
[performance]
max_connections = 1000
buffer_pool_size = "8GB"
wal_buffer_size = "256MB"

[performance.query]
cache_size = "1GB"
max_statement_age = 3600
```

## 监控

### Prometheus 指标

```bash
# 启用指标端点
sqlrustgo serve --metrics-port 9090

# Prometheus 抓取配置
scrape_configs:
  - job_name: 'sqlrustgo'
    static_configs:
      - targets: ['localhost:9090']
```

### 日志

```bash
# 查看日志
journalctl -u sqlrustgo -f

# 查看错误
journalctl -u sqlrustgo -p err
```

## 备份与恢复

### 备份

```bash
# 在线备份
sqlrustgo backup create /backup/$(date +%Y%m%d)

# 检查备份
sqlrustgo backup list
```

### 恢复

```bash
# 停止服务
sudo -S -p '' systemctl stop sqlrustgo

# 恢复数据
sqlrustgo backup restore /backup/20260524

# 重启服务
sudo -S -p '' systemctl start sqlrustgo
```

## 高可用

### 主从复制

```toml
[replication]
mode = "async"
primary = "sqlrustgo-primary:5432"
replica = "sqlrustgo-replica:5432"
```

### 故障转移

使用 keepalived 或 kube-vip 实现 VIP 漂移。

## 安全

### TLS 配置

```toml
[tls]
enabled = true
cert_file = "/etc/sqlrustgo/tls/server.crt"
key_file = "/etc/sqlrustgo/tls/server.key"
```

### 认证

```toml
[auth]
method = "password"
password_file = "/etc/sqlrustgo/users.pw"
```

## 升级

```bash
# 1. 下载新版本
curl -LO https://github.com/minzuuniversity/sqlrustgo/releases/v3.4.0/sqlrustgo-v3.4.0-linux-x64.tar.gz

# 2. 备份
sqlrustgo backup create /backup/pre-upgrade

# 3. 停止服务
sudo -S -p '' systemctl stop sqlrustgo

# 4. 替换二进制
sudo -S -p '' cp sqlrustgo-v3.4.0/sqlrustgo /usr/local/bin/

# 5. 启动服务
sudo -S -p '' systemctl start sqlrustgo

# 6. 验证
sqlrustgo --version
```