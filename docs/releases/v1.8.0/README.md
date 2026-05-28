# SQLRustGo v1.8.0 README

> **版本**: v1.8.0
> **发布日期**: 2026-03-25
> **代号**: SQL-92 Compliance Enhancement
> **状态**: GA (正式发布)
> **类型**: 事后补充（Retrospective）
> **Truthfulness Score**: 60%（历史遗留）
> **生成日期**: 2026-05-29

---

## SQLRustGo v1.8.0

**SQL-92 符合性增强版**

SQLRustGo v1.8.0 是 SQL-92 符合性增强版本，聚焦于解析器功能补全和测试套件建设。

### 核心特性

| 特性 | 说明 |
|------|------|
| **ALTER TABLE** | 表结构修改支持 |
| **DECIMAL 类型** | 精确数值类型 |
| **JSON 类型** | JSON 数据类型 |
| **CREATE/DROP INDEX** | 索引管理 |
| **LIMIT/OFFSET** | 分页查询 |
| **字符串函数** | 完整字符串函数支持 |
| **SQL 注释** | /* */ 和 -- 注释 |

---

## 快速开始

```bash
# 构建
cargo build --all-features

# 运行 REPL
cargo run --bin sqlrustgo

# 运行测试
cargo test --all-features

# SQL-92 合规测试
bash scripts/test/sql92_compliance.sh
```

---

## 文档

| 文档 | 说明 |
|------|------|
| [RELEASE_NOTES.md](./RELEASE_NOTES.md) | 发布说明 |
| [CHANGELOG.md](./CHANGELOG.md) | 变更日志 |
| [COMPREHENSIVE_ASSESSMENT_REPORT.md](./COMPREHENSIVE_ASSESSMENT_REPORT.md) | 综合评估报告 |
| [TEST_PLAN.md](./TEST_PLAN.md) | 测试计划 |

---

## SQL-92 功能完成度

| 功能 | Issue | 状态 | PR |
|------|-------|------|-----|
| ALTER TABLE 支持 | #761 | ✅ | #772 |
| DECIMAL 数据类型 | #762 | ✅ | #773 |
| JSON 数据类型 | #763 | ✅ | #774 |
| 存储过程 Token | #764 | ✅ | #777 |
| 触发器 Token | #765 | ✅ | #778 |
| CREATE/DROP INDEX | #766 | ✅ | #775 |
| 字符串函数 | #767 | ✅ | #776 |
| LIMIT/OFFSET 分页 | #760 | ✅ | #760 |
| INSERT SET 语法 | #768 | ✅ | 已实现 |
| SQL 注释支持 | - | ✅ | #783 |

**完成率**: 10/10 (100%)

---

## 新增 Token

| Token | 用途 |
|-------|------|
| DECIMAL | 精确数值类型 |
| JSON | JSON 数据类型 |
| PROCEDURE | 存储过程关键字 |
| TRIGGER | 触发器关键字 |
| FUNCTION | 函数关键字 |
| INDEX | 索引关键字 |

---

## 测试结果

| 测试套件 | 结果 |
|----------|------|
| cargo test (lib) | ✅ 13 passed |
| cargo test (parser) | ✅ 137 passed |
| cargo test (workspace) | ✅ 全部通过 |

### SQL-92 合规测试

```
============================
Summary:
  Passed: 18
  Failed: 0
  Pass rate: 100.00%
============================
```

| 类别 | 测试数 | 通过 |
|------|--------|------|
| DDL | 6 | ✅ |
| DML | 4 | ✅ |
| Queries | 4 | ✅ |
| Types | 4 | ✅ |
| **总计** | **18** | **100%** |

---

## 质量门禁

| 检查项 | 状态 |
|--------|------|
| 编译通过 | ✅ |
| 测试通过 | ✅ |
| Clippy 检查 | ✅ (4 warnings) |
| 格式检查 | ✅ |
| SQL-92 合规 | ✅ (100%) |

---

## 资源

- **代码**: https://github.com/minzuuniversity/sqlrustgo
- **讨论**: https://github.com/minzuuniversity/sqlrustgo/discussions
- **问题**: https://github.com/minzuuniversity/sqlrustgo/issues

---

**最后更新**: 2026-05-29
**文档状态**: 事后补充（Retrospective）