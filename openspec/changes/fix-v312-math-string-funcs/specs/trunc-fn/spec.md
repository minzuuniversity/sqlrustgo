## ADDED Requirements

### Requirement: TRUNCATE/TRUNC 数值截断函数

TRUNCATE 和 TRUNC 是同义函数，接受两个参数：数值表达式和位数整数。返回将数值截断到指定小数位的结果。

#### Scenario: 正数位数截断
- **WHEN** `SELECT TRUNCATE(3.14159, 2);`
- **THEN** 返回 `3.14`

#### Scenario: 负数位数截断
- **WHEN** `SELECT TRUNCATE(1234.5, -2);`
- **THEN** 返回 `1200.0`

#### Scenario: TRUNC 同义
- **WHEN** `SELECT TRUNC(3.7, 0);`
- **THEN** 返回 `3.0`

#### Scenario: NULL 输入
- **WHEN** `SELECT TRUNCATE(NULL, 2);`
- **THEN** 返回 `NULL`
