# v3.0.0 升级指南

> 本文说明如何从 v2.6.0 升级到 v3.0.0。

## 支持的升级路径

- v2.6.0 → v3.0.0 ✅
- v2.6.x → v3.0.0 ✅

## 升级前检查

```bash
sqlrustgo --version  # 应输出: v2.6.x
sqlrustgo health-check
sqlrustgo backup create /backup/pre-v300-$(date +%Y%m%d)
```

## 升级步骤

### 方式一：二进制升级

```bash
curl -LO https://github.com/minzuuniversity/sqlrustgo/releases/v3.0.0/sqlrustgo-v3.0.0-linux-x64.tar.gz
tar -xzf sqlrustgo-v3.0.0-linux-x64.tar.gz
sudo -S -p '' systemctl stop sqlrustgo
sudo -S -p '' cp sqlrustgo-v3.0.0/sqlrustgo /usr/local/bin/
sudo -S -p '' systemctl start sqlrustgo
sqlrustgo --version
```

### 方式二：Docker

```bash
docker pull ghcr.io/minzuuniversity/sqlrustgo:v3.0.0
docker-compose down && docker-compose up -d
```

## 升级后检查

```bash
sqlrustgo --version  # 输出应为: v3.0.0
sqlrustgo health-check
```

## 回滚

```bash
sudo -S -p '' systemctl stop sqlrustgo
sqlrustgo backup restore /backup/pre-v300-20260501
git checkout v2.6.0 && cargo build --release
sudo -S -p '' systemctl start sqlrustgo
```
