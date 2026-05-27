# 安全模块

> 安全模块负责用户认证、权限控制与审计。v2.8.0 重点实现列级权限系统。

## 核心组件

### 1. 认证系统

| 组件 | 功能 |
|------|------|
| Authenticator | 用户名/密码验证 |
| Host-based ACL | 基于来源 IP 的访问控制 |
| Plugin Auth | 可插拔认证插件 (mysql_native_password, caching_sha2) |

### 2. 权限系统 (v2.8.0 新增)

权限层级架构:


### 3. 列级权限 (Issue #37)

v2.8.0 新增特性, P0 优先级:

| 功能 | 状态 |
|------|------|
| GRANT/REVOKE 列级别 | 🟡 开发中 |
| 查询时列权限检查 | 🟡 开发中 |
| DML 列权限检查 | 🟡 开发中 |
| 权限合并与冲突解决 | 🔴 待实现 |

### 4. 审计日志

- SQL 执行审计: 记录所有 DDL/DML 操作
- 登录审计: 成功/失败登录记录
- 权限变更审计: GRANT/REVOKE 操作日志

## 安全配置


## 相关链接

- [SQLRustGo Architecture](../wiki/SQLRustGo-Architecture) — 架构概览
- [v2.8.0 Overview](../wiki/v2.8.0-Overview) — 版本总览
- [governance-system](../wiki/governance-system) — 治理模型

---

*最后更新: 2026-05-02*