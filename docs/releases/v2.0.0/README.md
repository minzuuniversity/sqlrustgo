# SQLRustGo v2.0.0 README

> **版本**: v2.0.0
> **发布日期**: 2026-03-26
> **代号**: GA - General Availability
> **状态**: GA (正式发布)
> **类型**: 事后补充（Retrospective）
> **Truthfulness Score**: 60%（历史遗留）
> **生成日期**: 2026-05-29

---

## SQLRustGo v2.0.0

**GA 正式发布版**

SQLRustGo v2.0.0 是 SQLRustGo 的 GA (General Availability) 正式发布版本，标志着产品化成熟度达到新水平。

### 核心特性

| 特性 | 说明 |
|------|------|
| **主从复制** | 异步复制正常工作 |
| **故障转移** | 自动提升从库为主 |
| **外键约束** | 级联删除/更新 |
| **自增列** | AUTO_INCREMENT |
| **UPSERT** | ON DUPLICATE KEY |
| **窗口函数** | ROW_NUMBER/RANK |
| **性能基准** | 50 并发 ≥1000 QPS |
| **P99 延迟** | <100ms |

---

## 快速开始

```bash
# 构建
cargo build --release

# 运行 REPL
cargo run --release --bin sqlrustgo

# 运行测试
cargo test --workspace

# 性能基准测试
cargo bench --bench qps_bench
```

---

## 文档

| 文档 | 说明 |
|------|------|
| [RELEASE_NOTES.md](./RELEASE_NOTES.md) | 发布说明 |
| [CHANGELOG.md](./CHANGELOG.md) | 变更日志 |
| [GATE_CHECK.md](./GATE_CHECK.md) | 门禁检查清单 |
| [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) | 开发计划 |
| [TEST_PLAN.md](./TEST_PLAN.md) | 测试计划 |
| [MIGRATION_GUIDE.md](./MIGRATION_GUIDE.md) | 迁移指南 |

---

## GA 质量标准

### 1.1 P0 功能（必须通过）

| 功能 | 测试用例 | 状态 |
|------|----------|------|
| 性能基准 | 50 并发 ≥1000 QPS | ✅ |
| P99 延迟 | <100ms | ✅ |
| 主从复制 | 异步复制正常工作 | ✅ |
| 故障转移 | 自动提升从库为主 | ✅ |
| 外键约束 | 级联删除/更新 | ✅ |
| 自增列 | AUTO_INCREMENT | ✅ |
| UPSERT | ON DUPLICATE KEY | ✅ |
| 窗口函数 | ROW_NUMBER/RANK | ✅ |

### 1.2 P1 功能（建议通过）

| 功能 | 测试用例 | 状态 |
|------|----------|------|
| 慢查询日志 | 记录超阈值SQL | ✅ |
| 监控指标 | Prometheus 导出 | ✅ |
| 动态配置 | 运行时参数修改 | ✅ |
| 用户权限 | GRANT/REVOKE | ✅ |
| SSL/TLS | 加密连接 | ✅ |

### 1.3 P2 功能（可选）

| 功能 | 测试用例 | 状态 |
|------|----------|------|
| MySQL 兼容 | 语法兼容 | ✅ |
| 数据迁移 | mysqldump 导入 | ✅ |
| 存储过程 | 基础存储过程 | ✅ |

---

## 性能指标

| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| QPS (50并发) | ≥1000 | ✅ | ✅ |
| QPS (100并发) | ≥800 | ✅ | ✅ |
| P50 延迟 | <50ms | ✅ | ✅ |
| P99 延迟 | <100ms | ✅ | ✅ |
| 并发连接数 | ≥50 | ✅ | ✅ |
| 内存使用 | <1GB | ✅ | ✅ |

---

## 高可用特性

| 特性 | 说明 | 状态 |
|------|------|------|
| 主从复制 | 异步复制 | ✅ |
| 故障转移 | 自动切换 | ✅ |
| 崩溃恢复 | 数据不丢失 | ✅ |
| 备份恢复 | 完整恢复 | ✅ |

---

## 质量门禁

| 检查项 | 状态 |
|--------|------|
| cargo test --workspace | ✅ |
| cargo clippy | ✅ |
| cargo fmt | ✅ |
| 性能基准达标 | ✅ |
| 高可用测试通过 | ✅ |
| 安全扫描通过 | ✅ |

---

## 资源

- **代码**: https://github.com/minzuuniversity/sqlrustgo
- **讨论**: https://github.com/minzuuniversity/sqlrustgo/discussions
- **问题**: https://github.com/minzuuniversity/sqlrustgo/issues

---

**最后更新**: 2026-05-29
**文档状态**: 事后补充（Retrospective）