# V312-11-Fix: SET Statement Support

## 问题
Parser不支持SET语句来设置会话变量。

### 错误
```
Parse error: Expected Transaction, got Identifier("debug_force_external")
```

### 失败测试
- quantile_fun.test
- sql__quantile_fun.test
- aggregate__quantile_fun.test

### 涉及SQL
```sql
SET debug_force_external=true;
```

## 根因分析
Parser不识别SET关键字作为语句起始。

## 解决方案
在parser中添加SET语句解析：
1. 识别`SET`关键字
2. 解析`SET variable = value`形式
3. 存储在会话上下文中供后续使用（可选）

## 验收标准
- quantile_fun.test等SET相关测试能运行（即使SET本身被忽略）
- 至少不报parse错误

## 影响范围
- parser: 添加SET语句解析
