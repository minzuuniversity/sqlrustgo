# v3.3.0 安装指南

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
curl -LO https://github.com/minzuuniversity/sqlrustgo/releases/v3.3.0/sqlrustgo-v3.3.0-linux-x64.tar.gz
tar -xzf sqlrustgo-v3.3.0-linux-x64.tar.gz
sudo -S -p '' cp sqlrustgo-v3.3.0/sqlrustgo /usr/local/bin/
sqlrustgo --version
```

### 方式二：源码编译

```bash
git clone https://github.com/minzuuniversity/sqlrustgo.git
cd sqlrustgo
git checkout v3.3.0
cargo build --release
./target/release/sqlrustgo --version
```

### 方式三：Docker

```bash
docker pull ghcr.io/minzuuniversity/sqlrustgo:v3.3.0
docker run -it --rm ghcr.io/minzuuniversity/sqlrustgo:v3.3.0 sqlrustgo --version
```

## 验证安装

```bash
sqlrustgo --version
sqlrustgo health-check
sqlrustgo bench --test tpch
```
