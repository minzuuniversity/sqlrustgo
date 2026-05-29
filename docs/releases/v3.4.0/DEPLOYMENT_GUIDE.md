# v3.4.0 部署指南

## 部署模式

### 1. 单机部署（开发/测试）

```bash
./target/release/sqlrustgo serve --port 5432 --data /var/lib/sqlrustgo/data
```

### 2. Systemd 服务部署（生产）

```bash
sudo -S -p '' tee /etc/systemd/system/sqlrustgo.service << 'SERVICE'
[Unit]
Description=SQLRustGo Database
After=network.target

[Service]
Type=simple
User=sqlrustgo
Group=sqlrustgo
ExecStart=/usr/local/bin/sqlrustgo serve --port 5432
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=multi-user.target
SERVICE

sudo -S -p '' systemctl daemon-reload
sudo -S -p '' systemctl enable sqlrustgo
sudo -S -p '' systemctl start sqlrustgo
```

### 3. Docker Compose 部署

```yaml
version: '3.8'
services:
  sqlrustgo:
    image: ghcr.io/minzuuniversity/sqlrustgo:v3.4.0
    ports:
      - "5432:5432"
    volumes:
      - ./data:/var/lib/sqlrustgo/data
    restart: unless-stopped
```

```bash
docker-compose up -d
```

### 4. Kubernetes 部署

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
        resources:
          requests:
            memory: "4Gi"
            cpu: "1000m"
```

## 性能调优

```toml
[performance]
max_connections = 1000
buffer_pool_size = "8GB"
```

## 备份与恢复

```bash
# 备份
sqlrustgo backup create /backup/$(date +%Y%m%d)

# 恢复
sqlrustgo backup restore /backup/20260525
```
