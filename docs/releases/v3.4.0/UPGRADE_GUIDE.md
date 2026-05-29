# v3.4.0 升级指南

> 本文说明如何从 v3.3.0 升级到 v3.4.0。

## 支持的升级路径

- v3.2.0 → v3.4.0 ✅
- v3.3.0 → v3.4.0 ✅
- v3.3.x → v3.4.0 ✅

## 升级前检查

### 1. 确认当前版本

```bash
sqlrustgo --version
# 应输出: sqlrustgo v3.3.x 或更高
```

### 2. 运行健康检查

```bash
sqlrustgo health-check
# 确保所有检查通过
```

### 3. 备份数据

```bash
sqlrustgo backup create /backup/pre-v340-$(date +%Y%m%d)
```

## 升级步骤

### 方式一：二进制升级（推荐）

```bash
# 1. 下载 v3.4.0
curl -LO https://github.com/minzuuniversity/sqlrustgo/releases/v3.4.0/sqlrustgo-v3.4.0-linux-x64.tar.gz

# 2. 解压
tar -xzf sqlrustgo-v3.4.0-linux-x64.tar.gz

# 3. 停止服务
sudo -S -p '' systemctl stop sqlrustgo

# 4. 替换二进制
sudo -S -p '' cp sqlrustgo-v3.4.0/sqlrustgo /usr/local/bin/

# 5. 启动服务
sudo -S -p '' systemctl start sqlrustgo

# 6. 验证版本
sqlrustgo --version
# 应输出: sqlrustgo v3.4.0
```

### 方式二：源码编译

```bash
git fetch origin
git checkout v3.4.0
cargo build --release
sudo -S -p '' systemctl restart sqlrustgo
```

### 方式三：Docker

```bash
# 1. 拉取新镜像
docker pull ghcr.io/minzuuniversity/sqlrustgo:v3.4.0

# 2. 更新 docker-compose.yaml
sed -i 's/v3.3.0/v3.4.0/g' docker-compose.yaml

# 3. 重启
docker-compose down
docker-compose up -d
```

## 升级后检查

### 1. 版本验证

```bash
sqlrustgo --version
# 输出应为: sqlrustgo v3.4.0
```

### 2. 启动验证

```bash
sqlrustgo health-check
# 应输出: All checks passed
```

### 3. 数据验证

```bash
# 检查数据完整性
sqlrustgo db check --integrity

# 检查复制状态（如配置了复制）
sqlrustgo replication status
```

## 新功能验证

### GMP Management API

```bash
# 启动 GMP 服务器
sqlrustgo serve --gmp-api --port 8080

# 测试批次管理
curl http://localhost:8080/api/v1/gmp/batch

# 测试审计接口
curl http://localhost:8080/api/v1/gmp/audit
```

### GMP Retrieval v2

```bash
# 测试混合检索
curl -X POST http://localhost:8080/api/v1/gmp/retrieve \
  -H "Content-Type: application/json" \
  -d '{"query": "偏差处理", "top_k": 5}'
```

## 回滚

### 回滚到 v3.3.0

```bash
# 1. 停止服务
sudo -S -p '' systemctl stop sqlrustgo

# 2. 恢复备份
sqlrustgo backup restore /backup/pre-v340-20260524

# 3. 降级二进制
git checkout v3.3.0
cargo build --release

# 4. 重启服务
sudo -S -p '' systemctl start sqlrustgo
```

## 常见问题

### Q: 升级后无法启动？

```bash
# 查看日志
journalctl -u sqlrustgo -n 100

# 检查配置文件
sqlrustgo validate-config /etc/sqlrustgo/sqlrustgo.toml
```

### Q: 配置文件不兼容？

A: v3.4.0 完全向后兼容 v3.3.0 配置。新增配置项有默认值，可安全添加。

## 获取帮助

- Issue: https://github.com/minzuuniversity/sqlrustgo/issues
- 讨论: https://github.com/minzuuniversity/sqlrustgo/discussions