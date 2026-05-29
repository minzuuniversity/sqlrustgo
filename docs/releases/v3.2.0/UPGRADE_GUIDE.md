# v3.2.0 升级指南

> 本文说明如何从 v3.1.0 升级到 v3.2.0。

## 支持的升级路径

- v3.1.0 → v3.2.0 ✅
- v3.1.x → v3.2.0 ✅

## 升级前检查

```bash
sqlrustgo --version  # 应输出: v3.1.x
sqlrustgo health-check
sqlrustgo backup create /backup/pre-v320-$(date +%Y%m%d)
```

## 升级步骤

### 方式一：二进制升级

```bash
curl -LO https://github.com/minzuuniversity/sqlrustgo/releases/v3.2.0/sqlrustgo-v3.2.0-linux-x64.tar.gz
tar -xzf sqlrustgo-v3.2.0-linux-x64.tar.gz
sudo -S -p '' systemctl stop sqlrustgo
sudo -S -p '' cp sqlrustgo-v3.2.0/sqlrustgo /usr/local/bin/
sudo -S -p '' systemctl start sqlrustgo
sqlrustgo --version
```

### 方式二：Docker

```bash
docker pull ghcr.io/minzuuniversity/sqlrustgo:v3.2.0
docker-compose down && docker-compose up -d
```

## 升级后检查

```bash
sqlrustgo --version  # 输出应为: v3.2.0
sqlrustgo health-check
```

## 新功能验证

```bash
# 电子签名验证
curl -X POST http://localhost:8080/api/v1/gmp/signature/verify \
  -H "Content-Type: application/json" \
  -d '{"signature": "...", "reason": "Approved"}'

# 审计链验证
curl -X POST http://localhost:8080/api/v1/gmp/audit/verify

# 证据导出
curl -X POST http://localhost:8080/api/v1/gmp/evidence/export
```

## 回滚

```bash
sudo -S -p '' systemctl stop sqlrustgo
sqlrustgo backup restore /backup/pre-v320-20260524
git checkout v3.1.0 && cargo build --release
sudo -S -p '' systemctl start sqlrustgo
```
