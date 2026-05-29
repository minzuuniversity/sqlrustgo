# v3.3.0 升级指南

> 本文说明如何从 v3.2.0 升级到 v3.3.0。

## 支持的升级路径

- v3.2.0 → v3.3.0 ✅
- v3.2.x → v3.3.0 ✅

## 升级前检查

```bash
sqlrustgo --version  # 应输出: v3.2.x
sqlrustgo health-check
sqlrustgo backup create /backup/pre-v330-$(date +%Y%m%d)
```

## 升级步骤

### 方式一：二进制升级

```bash
curl -LO https://github.com/minzuuniversity/sqlrustgo/releases/v3.3.0/sqlrustgo-v3.3.0-linux-x64.tar.gz
tar -xzf sqlrustgo-v3.3.0-linux-x64.tar.gz
sudo -S -p '' systemctl stop sqlrustgo
sudo -S -p '' cp sqlrustgo-v3.3.0/sqlrustgo /usr/local/bin/
sudo -S -p '' systemctl start sqlrustgo
sqlrustgo --version
```

### 方式二：Docker

```bash
docker pull ghcr.io/minzuuniversity/sqlrustgo:v3.3.0
docker-compose down && docker-compose up -d
```

## 升级后检查

```bash
sqlrustgo --version  # 输出应为: v3.3.0
sqlrustgo health-check
```

## 新功能验证

```bash
# Workflow V2
curl http://localhost:8080/api/v1/workflow/v2/list

# Provenance
curl http://localhost:8080/api/v1/provenance/graph

# Evidence
curl http://localhost:8080/api/v1/evidence/list
```

## 回滚

```bash
sudo -S -p '' systemctl stop sqlrustgo
sqlrustgo backup restore /backup/pre-v330-20260517
git checkout v3.2.0 && cargo build --release
sudo -S -p '' systemctl start sqlrustgo
```
