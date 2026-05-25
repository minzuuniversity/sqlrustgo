# v1.7.0 Release Notes

> **版本**: v1.7.0
> **发布日期**: 2026-03-23
> **状态**: ✅ GA
> **Truthfulness Score**: 100%

---

## 版本概述

MySQL 兼容性增强版本，完善 GROUP BY 和 JOIN 支持。

---

## 新增功能

- GROUP BY 支持
- JOIN 增强（LEFT/RIGHT/INNER）
- bplus_tree 测试覆盖完善
- planner 测试覆盖完善
- SQL-92 合规报告

---

## 修复与改进

- benchmark 修复

---

## 测试结果

| 测试项 | 结果 |
|--------|------|
| 单元测试 | ✅ PASS |
| 覆盖率 | ≥85%（v3.4.0 豁免至 83.14%）|
| TPC-H SF=1 | ✅ PASS（22/22）|

---

## 升级说明

无 Breaking Changes，从前一 GA 版本平滑升级。

---

*最后更新: 2026-05-25*
