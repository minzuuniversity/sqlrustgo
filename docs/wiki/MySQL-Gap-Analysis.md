# MySQL 兼容性差距分析

> MySQL 5.7 兼容性是 v2.8.0 alpha 阶段三大 P0 阻塞项之一。
> 当前失败用例: **57 项** (Issue #41)

## 失败用例分类

| 类别 | 失败数 | 说明 |
|------|--------|------|
| SQL 语法差异 | ~20 | 子查询、JOIN、UNION 等语法不支持 |
| 数据类型 | ~12 | DECIMAL、ENUM、SET、JSON 类型缺失 |
| 函数兼容 | ~10 | 聚合函数、日期函数、字符串函数 |
| 协议差异 | ~8 | MySQL 协议字段编码不一致 |
| 事务语义 | ~7 | SAVEPOINT、ROLLBACK TO 等未实现 |

## 根因分析

1. **Parser 覆盖率不足**: 部分 MySQL 方言 SQL 语法未在 parser 中实现
2. **类型系统不完整**: 缺少 DECIMAL、ENUM 等 MySQL 特有类型
3. **协议实现差异**: 错误码、结果集元数据编码与 MySQL 5.7 不完全一致
4. **函数库缺失**: 约 30% 的 MySQL 内置函数未实现

## 修复路线

| 优先级 | 任务 | 预估工时 | 责任人 |
|--------|------|---------|--------|
| P0 | 修复 SQL 语法差异 (子查询/JOIN) | 3 天 | TRAE AI |
| P0 | 补齐 DECIMAL/ENUM 类型 | 2 天 | TRAE AI |
| P1 | 实现缺失的聚合函数 | 2 天 | TRAE AI |
| P1 | 修复协议编码问题 | 1 天 | TRAE AI |
| P2 | 补齐剩余函数 | 3 天 | TRAE AI |

## 验证标准

- sysbench OLTP 读写测试全部通过
- 57 失败用例降至 <10
- MySQL 5.7 官方测试套件子集通过率 ≥90%

## 相关链接

- [v2.8.0 Overview](../wiki/v2.8.0-Overview) — 版本总览
- [v2.8.0-progress](../wiki/v2.8.0-progress) — 进度与门禁
- [SQLRustGo Architecture](../wiki/SQLRustGo-Architecture) — 架构概览

---

*最后更新: 2026-05-02*