# V312-11-Fix: Double-Quoted Identifier Handling

## 问题
双引号标识符（如`"MyTable"`）在lexer中未被正确处理。

### 错误
```
Table not found: MyTable
```

### 失败测试
- case_insensitive_alter.test

### 涉及SQL
```sql
CREATE TABLE "MyTable"(i integer, "BigColumn" integer);
ALTER TABLE MyTable ALTER BIGCOLUMN SET DATA TYPE VARCHAR
```

## 根因分析
Lexer的`next_token`函数不处理`"`字符。`"`字符落在`match ch`的默认分支，最终被识别为单个标识符。

## 解决方案
在lexer中添加双引号标识符处理：
1. 添加`read_quoted_identifier`方法
2. 在`match ch`中添加`'"'`分支

## 验收标准
- case_insensitive_alter.test PASS

## 影响范围
- lexer: 添加双引号处理
