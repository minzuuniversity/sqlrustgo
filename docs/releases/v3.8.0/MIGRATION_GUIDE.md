# SQLRustGo v3.8.0 升级指南

> **版本**: v3.8.0
> **前版本**: v3.7.0
> **发布日期**: 2026-06-04

---

## 二进制入口变更

### 已废弃

| 废弃命令 | 替代方案 |
|---------|---------|
| `sqlrustgo-sql-cli` | `sqlrustgo-mysql-server repl` |
| `sqlrustgo-tools` | `sqlrustgo-mysql-server diag` |

### 统一入口

v3.8.0 所有功能通过 `sqlrustgo-mysql-server` 统一提供：

```bash
sqlrustgo-mysql-server serve --host 127.0.0.1 --port 3306
sqlrustgo-mysql-server exec "CREATE TABLE t (id INT PRIMARY KEY)"
sqlrustgo-mysql-server repl
sqlrustgo-mysql-server bench
```

---

## 行为变更

### 事务语义

| 场景 | v3.7.0 | v3.8.0 |
|------|---------|---------|
| BEGIN 后 DML | 可能不写 WAL | 写 WAL |
| COMMIT | 只刷 page | 先写 WAL log_commit 再刷 |
| ROLLBACK | 内存回滚 | 丢弃 write_buffer |

---

## 数据目录兼容性

v3.8.0 的 FileStorage 数据文件格式与 v3.7.0 兼容，无需迁移。

---

## 升级步骤

```bash
# 1. 停止现有实例
pkill sqlrustgo

# 2. 备份数据
cp -r /var/lib/sqlrustgo /var/lib/sqlrustgo.v3.7.0.bak

# 3. 部署 v3.8.0
cargo build --release -p sqlrustgo-mysql-server
sudo cp target/release/sqlrustgo-mysql-server /usr/local/bin/

# 4. 启动
sqlrustgo-mysql-server serve --data-dir /var/lib/sqlrustgo
```
