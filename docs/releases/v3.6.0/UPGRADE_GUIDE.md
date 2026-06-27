# SQLRustGo v3.6.0 升级指南 (从 v3.5.0)

> **版本**: v3.6.0
> **适用**: 从 v3.5.0 GA 升级
> **HEAD**: 1b2a3c71
> **日期**: 2026-05-30
> **SSOT**: docs/governance/SSOT_CROSS_CHECK.md

---

## 概述

本指南帮助用户从 v3.5.0 GA 升级到 v3.6.0。

**升级概要**: v3.6.0 是知识增强 + 存储可靠性版本。主要新增 WALVerifier、SIMD 集成、Knowledge OS (qmd-bridge) 桥接。与 v3.5.0 相比，**无破坏性 API/配置变更**。

---

## 重大变更

### Breaking Changes

| 变更 | 影响 | 迁移建议 |
|------|------|----------|
| 无 | — | 无迁移操作 |

v3.6.0 不引入破坏性变更。所有 v3.5.0 SQL 语句和 API 无需修改。

### 行为变更

| 操作 | v3.5.0 | v3.6.0 |
|------|--------|--------|
| WAL 验证 | 无专用框架 | WALVerifier 独立 crate |
| SIMD 向量化 | 基础 | 集成到构建流程 |
| 窗口函数 | PercentRank/CumeDist 缺失 | 全部窗口函数就绪 |
| SSI 隔离级别 | 启用 | 降级为 READ COMMITTED |

---

## 升级步骤

### 阶段 1: 准备

1. **备份所有数据库文件**

```bash
mkdir -p /backup/v3.5.0
sqlrustgo-tools backup --database <db> --output-dir /backup/v3.5.0
```

2. **停止所有运行中的 SQLRustGo 实例**

```bash
ps aux | grep sqlrustgo
kill -TERM <pid>
```

### 阶段 2: 升级

3. **切换到 develop/v3.6.0 分支**

```bash
git fetch origin
git checkout develop/v3.6.0
```

4. **重新构建**

```bash
# 标准构建
cargo build --release

# 全特性构建 (推荐)
cargo build --release --all-features

# 仅 WAL 验证
cargo build --release --features wal-verification

# 仅 SIMD
cargo build --release --features simd
```

### 阶段 3: 验证

5. **验证数据完整性**

```bash
# 运行 WAL 验证测试
cargo test -p sqlrustgo-transaction -- wal

# 运行回归测试
cargo test --test regression_test

# 运行崩溃恢复测试
cargo test --test crash_recovery_test
```

6. **启动新版本**

```bash
sqlrustgo --config sqlrustgo.toml
```

7. **验证核心功能**

```sql
-- 测试基本 SQL
SELECT 1;
CREATE TABLE test (id INT);
INSERT INTO test VALUES (1);
SELECT * FROM test;
DROP TABLE test;
```

---

## 新增特性标志

```bash
# v3.6.0 新增 feature flags

# WAL 验证模块
--features wal-verification

# SIMD 向量化加速
--features simd

# qmd-bridge Knowledge OS 桥接
--features qmd-bridge

# 全量
--all-features
```

---

## 配置变更

v3.6.0 **无新增配置项**。现有配置完全兼容 v3.5.0。

---

## 数据迁移

v3.6.0 与 v3.5.0 数据格式完全兼容。无需数据迁移。

---

## 回滚指南

如需回滚到 v3.5.0:

```bash
# 停止 v3.6.0 实例
kill -TERM <pid>

# 切换回旧分支
git checkout develop/v3.5.0
cargo build --release

# 启动旧版本
sqlrustgo --config sqlrustgo.toml
```

> 注意: 如果使用了 v3.6.0 新增的 WALVerifier 特性，回滚后需删除 WAL 验证元数据文件。

---

*SSOT 参考: docs/governance/SSOT_CROSS_CHECK.md*
*更新日期: 2026-05-30*
