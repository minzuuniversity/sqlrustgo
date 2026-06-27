# v3.4.0 安装指南

## 系统要求

| 项目 | 要求 |
|------|------|
| CPU | x86_64 / ARM64 |
| 内存 | 最低 4GB，建议 8GB+ |
| 磁盘 | 10GB+ 可用空间 |
| OS | Linux (Ubuntu 20.04+) / macOS 12+ |
| Rust | 1.75+ |

## 安装方式

### 方式一：二进制下载（推荐）

```bash
# 下载预编译二进制
curl -LO https://github.com/minzuuniversity/sqlrustgo/releases/v3.4.0/sqlrustgo-v3.4.0-linux-x64.tar.gz

# 解压
tar -xzf sqlrustgo-v3.4.0-linux-x64.tar.gz

# 安装
sudo cp sqlrustgo-v3.4.0/sqlrustgo /usr/local/bin/

# 验证安装
sqlrustgo --version
```

### 方式二：源码编译

```bash
# 克隆仓库
git clone https://github.com/minzuuniversity/sqlrustgo.git
cd sqlrustgo

# 切换到 v3.4.0
git checkout v3.4.0

# 编译
cargo build --release

# 验证
./target/release/sqlrustgo --version
```

### 方式三：Docker

```bash
# 拉取镜像
docker pull ghcr.io/minzuuniversity/sqlrustgo:v3.4.0

# 运行
docker run -it --rm ghcr.io/minzuuniversity/sqlrustgo:v3.4.0 sqlrustgo --version
```

## 验证安装

```bash
# 1. 检查版本
sqlrustgo --version
# 输出: sqlrustgo v3.4.0

# 2. 运行健康检查
sqlrustgo health-check

# 3. 运行基准测试
sqlrustgo bench --test tpch
```

## 安装后配置

### 创建数据目录

```bash
mkdir -p /var/lib/sqlrustgo/{data,wal,backup}
chown -R sqlrustgo:sqlrustgo /var/lib/sqlrustgo
```

### 配置 GMP API（可选）

```bash
# 启用 GMP REST API 服务
sqlrustgo serve --gmp-api --port 8080
```