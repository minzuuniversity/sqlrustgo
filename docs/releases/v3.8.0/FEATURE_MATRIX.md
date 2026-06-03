# SQLRustGo v3.8.0 功能矩阵

> **版本**: v3.8.0
> **评估日期**: 2026-06-04
> **基准**: MySQL 5.7

---

## 符号说明

| 符号 | 含义 |
|------|------|
| ✅ | 完全支持 |
| ⚠️ | 部分支持 / 有已知问题 |
| ❌ | 不支持 |

---

## 1. DDL（数据定义）

| 功能 | 状态 | 备注 |
|------|------|------|
| CREATE TABLE | ✅ | |
| DROP TABLE | ✅ | |
| ALTER TABLE ADD COLUMN | ⚠️ | |
| ALTER TABLE DROP COLUMN | ⚠️ | |
| CREATE INDEX | ✅ | |
| DROP INDEX | ✅ | |
| PRIMARY KEY | ✅ | |
| CREATE VIEW | ⚠️ | 仅简单视图 |

---

## 2. DML（数据操作）

| 功能 | 状态 | 备注 |
|------|------|------|
| INSERT | ✅ | |
| INSERT ... ON DUPLICATE KEY UPDATE | ⚠️ | |
| UPDATE | ⚠️ | UPDATE replay bug (F-09) |
| DELETE | ✅ | |
| SELECT | ✅ | |
| SELECT JOIN (INNER) | ⚠️ | |
| SELECT JOIN (LEFT/RIGHT) | ❌ | TPC-H Q10~Q22 缺失 |
| SELECT UNION | ⚠️ | |

---

## 3. Transaction

| 功能 | 状态 | 备注 |
|------|------|------|
| BEGIN | ✅ | |
| COMMIT | ✅ | |
| ROLLBACK | ✅ | |
| Isolation Level | ⚠️ | READ COMMITTED ✅ |

---

## 4. SQL 语法

| 功能 | 状态 | 备注 |
|------|------|------|
| WHERE / AND / OR | ✅ | |
| LIKE | ✅ | |
| CASE WHEN | ⚠️ | 仅简单 CASE |
| Window functions | ⚠️ | |
| CTE (WITH) | ✅ | |
| LIKE pattern | ✅ | |

---

## 5. 协议与工具

| 功能 | 状态 | 备注 |
|------|------|------|
| MySQL Wire Protocol | ✅ | |
| Prepared Statement | ✅ | |
| mysqladmin | ❌ | F-32 SPEC 存在 |
| mysqldump | ⚠️ | 工具存在，未测试 |

---

## 6. TPC-H 支持

| Q# | 状态 | Q# | 状态 |
|----|------|----|------|
| Q1 | ✅ | Q12 | ❌ |
| Q2 | ✅ | Q13 | ❌ |
| Q3 | ✅ | Q14 | ❌ |
| Q4 | ✅ | Q15 | ❌ |
| Q5 | ✅ | Q16 | ❌ |
| Q6 | ✅ | Q17 | ❌ |
| Q7 | ✅ | Q18 | ❌ |
| Q8 | ✅ | Q19 | ❌ |
| Q9 | ✅ | Q20 | ❌ |
| Q10 | ❌ | Q21 | ❌ |
| Q11 | ❌ | Q22 | ❌ |

**支持: 9/22 (41%)**

---

## 7. 覆盖率汇总

| 类别 | 支持项 | 总项 | 覆盖率 |
|------|--------|------|--------|
| DDL | 9 | 13 | 69% |
| DML | 10 | 14 | 71% |
| Transaction | 5 | 8 | 63% |
| SQL 语法 | 20 | 28 | 71% |
| 存储引擎 | 7 | 10 | 70% |
| 协议 | 4 | 6 | 67% |
| Admin 工具 | 0 | 4 | 0% |
| 安全 | 1 | 4 | 25% |
| **总计** | **56** | **87** | **64%** |
