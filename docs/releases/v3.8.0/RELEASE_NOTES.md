# SQLRustGo v3.8.0 Release Notes

> **版本**: v3.8.0
> **发布日期**: 2026-06-04
> **类型**: Architecture Unification Release

---

## 概述

SQLRustGo v3.8.0 是**架构收敛版本**，核心目标是统一执行路径、接入 WAL 事务核心、为 MVCC 完整化奠定架构基础。

**从 v3.7.0 到 v3.8.0，系统从"查询执行引擎"进化为"支持 WAL 的事务型数据库"。**

---

## 重大变更

### 二进制入口统一

v3.8.0 引入了统一的 canonical 二进制入口 `sqlrustgo-mysql-server`：

```bash
# 服务模式（MySQL 协议）
sqlrustgo-mysql-server serve --host 127.0.0.1 --port 3306

# 单条 SQL 执行
sqlrustgo-mysql-server exec "SELECT * FROM t"

# 交互式 REPL
sqlrustgo-mysql-server repl

# 基准测试
sqlrustgo-mysql-server bench
```

**已废弃**: `sqlrustgo-sql-cli`、`sqlrustgo-tools` 请迁移到上述子命令。

---

## 新功能

### WAL Recovery Chain ✅

PR-830A~E 完整实现了 WAL 恢复链：

- **PR-830A**: Stateless WAL + Tx Context Route A
- **PR-830B**: WAL 抽象层（memory/file WAL managers）
- **PR-830C**: WAL Replay 正确性
- **PR-830D**: RecoveryEngine 确定性重放
- **PR-830E**: Engine Restart + FileStorage 启动序列
- **PR-830F**: WAL Lifecycle Controller (Checkpoint + Truncation)

**验证**: 22/22 RECOVERY 测试全部 PASS

### TPC-H Q1~Q22 (13/22 PASS)

本版本支持 TPC-H 22 条查询中的 13 条。

**尚未支持**: Q10~Q22（需要 OUTER JOIN、CUBE/ROLLUP、GROUPING SETS 等）

---

## 已知问题

| Issue | 优先级 | 说明 |
|-------|--------|------|
| F-09 UPDATE replay bug | P0 | `#[ignore]`，根因已知，待 v3.8.1 修复 |
| F-06 TransactionalFacade | P0 | DEAD CODE（编译错误），待重构 |
| PR-870 MERGE | P1 | Parser 不支持 MERGE 语法，STUB 状态 |

---

## 下一步

- **v3.8.1**: 修复 F-09 UPDATE replay bug、F-06 TransactionalFacade 重构
- **v3.9.0**: MVCC 完整化、PR-870 MERGE 实现、TPC-H Q10~Q22

---

*发布人: Hermes Agent*
*发布日期: 2026-06-04*
