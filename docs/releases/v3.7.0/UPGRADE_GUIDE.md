# SQLRustGo v3.7.0 Upgrade Guide

> **版本**: v3.7.0  
> **日期**: 2026-05-30

---

## 1. 从 v3.6.0 升级

### 1.1 变更摘要

| 功能 | v3.6.0 | v3.7.0 | 说明 |
|------|--------|--------|------|
| Session-level Transactions | ❌ | ✅ | COMMIT 持久化 |
| Auth enforcement | bypass | enforced | SKIP_AUTH=false |
| Engine cache | shared | per-session | Session-level cache |

### 1.2 破坏性变更

**认证变更**: v3.7.0 强制认证，不再允许空密码 bypass。

旧连接方式（v3.6.0）:
```bash
mysql -u root  # 允许空密码
```

v3.7.0 连接方式:
```bash
mysql -u root -p  # 需要输入密码（空密码直接回车）
```

### 1.3 数据迁移

v3.6.0 的内存数据在升级后不可用。建议：
1. 导出 v3.6.0 数据
2. 升级到 v3.7.0
3. 重新导入数据

---

## 2. 配置变更

### 2.1 环境变量

| 变量 | v3.6.0 | v3.7.0 |
|------|--------|--------|
| SQLRUSTGO_SESSION_CACHE | 不支持 | 默认启用 |
| SKIP_AUTH | true | false |

### 2.2 端口

默认端口不变：`3306`

---

## 3. 已知问题

### 3.1 不支持 ROLLBACK

v3.7.0 不支持 ROLLBACK。事务一旦 COMMIT，无法回滚。

**workaround**: 使用应用层事务管理。

### 3.2 无 MVCC

v3.7.0 使用会话级隔离，不是 MVCC。并发写入可能有冲突。

---

## 4. 回滚

### 4.1 回滚到 v3.6.0

```bash
git checkout v3.6.0
cargo build --release
```

### 4.2 数据兼容性

v3.7.0 的存储格式与 v3.6.0 兼容。数据可以迁移。

---

## 5. v3.8.0 预览

v3.8.0 将包含：
- WAL / crash recovery
- Full MVCC / ROLLBACK
- ParallelVolcanoExecutor
- Single DML execution path