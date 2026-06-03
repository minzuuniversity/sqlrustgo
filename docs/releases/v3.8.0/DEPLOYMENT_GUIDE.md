# SQLRustGo v3.8.0 部署指南

> **版本**: v3.8.0
> **发布日期**: 2026-06-04

---

## 系统要求

| 组件 | 最低要求 | 推荐 |
|------|---------|------|
| CPU | 2 cores | 4+ cores |
| 内存 | 4 GB | 16 GB |
| 磁盘 | 10 GB | 50 GB SSD |
| OS | macOS 12+ / Linux | macOS 14+ / Ubuntu 22.04+ |
| Rust | 1.75+ | 1.80+ |

---

## 编译

```bash
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout develop/v3.8.0
cargo build --release -p sqlrustgo-mysql-server
```

---

## 部署模式

### 直接运行（开发/测试）

```bash
./target/release/sqlrustgo-mysql-server serve --host 127.0.0.1 --port 3306
```

### systemd 服务（生产）

```bash
sudo tee /etc/systemd/system/sqlrustgo.service <<EOF
[Unit]
Description=SQLRustGo MySQL-compatible Database
After=network.target
[Service]
Type=simple
User=sqlrustgo
ExecStart=/usr/local/bin/sqlrustgo-mysql-server serve --host 0.0.0.0 --port 3306 --data-dir /var/lib/sqlrustgo
Restart=always
[Install]
WantedBy=multi-user.target
EOF
sudo systemctl daemon-reload
sudo systemctl enable sqlrustgo
sudo systemctl start sqlrustgo
```

---

## 连接

```bash
mysql -h 127.0.0.1 -P 3306 -u root
```

---

## 备份与恢复

```bash
# 备份
sudo systemctl stop sqlrustgo
sudo tar -czf /backup/sqlrustgo-$(date +%Y%m%d).tar.gz /var/lib/sqlrustgo
sudo systemctl start sqlrustgo

# 恢复
sudo systemctl stop sqlrustgo
sudo tar -xzf /backup/sqlrustgo-YYYYMMDD.tar.gz -C /
sudo systemctl start sqlrustgo
```
