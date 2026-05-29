# v3.5.0 安装指南

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
curl -LO https://github.com/minzuuniversity/sqlrustgo/releases/v3.5.0/sqlrustgo-v3.5.0-linux-x64.tar.gz

# 解压
tar -xzf sqlrustgo-v3.5.0-linux-x64.tar.gz

# 安装
sudo cp sqlrustgo-v3.5.0/sqlrustgo /usr/local/bin/

# 验证安装
sqlrustgo --version
```

### 方式二：源码编译

```bash
# 克隆仓库
git clone https://github.com/minzuuniversity/sqlrustgo.git
cd sqlrustgo

# 切换到 v3.5.0
git checkout v3.5.0

# 编译
cargo build --release

# 验证
./target/release/sqlrustgo --version
```

### 方式三：Docker

```bash
# 拉取镜像
docker pull ghcr.io/minzuuniversity/sqlrustgo:v3.5.0

# 运行
docker run -it --rm ghcr.io/minzuuniversity/sqlrustgo:v3.5.0 sqlrustgo --version
```

## 验证安装

```bash
# 1. 检查版本
sqlrustgo --version
# 输出: sqlrustgo v3.5.0

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

### 配置文件

```bash
mkdir -p /etc/sqlrustgo
cp config/default.toml /etc/sqlrustgo/sqlrustgo.toml
```

## 卸载

```bash
# 二进制安装
sudo rm /usr/local/bin/sqlrustgo
rm -rf /var/lib/sqlrustgo

# Docker
docker rmi ghcr.io/minzuuniversity/sqlrustgo:v3.5.0
```

## 常见问题

### Q: 编译失败？

A: 确保 Rust 版本 ≥ 1.75：
```bash
rustc --version
cargo --version
```

### Q: 运行时提示缺少动态库？

A: Ubuntu/Debian:
```bash
sudo apt install libssl-dev libudev-dev pkg-config
```

### Q: Docker 容器无法启动？

A: 检查 Docker 版本 ≥ 20.10：
```bash
docker --version
```
