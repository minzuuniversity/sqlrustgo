---
name: Parser Module
description: SQLRustGo Parser 模块详解
created: 2026-04-24
source: 本地分析
tags: [sqlrustgo, parser, lexer, ast]
---

# Parser 模块详解

## 概述

Parser 模块 (5,261 LOC) 是 SQLRustGo 的词法和语法分析核心。

## 目录结构

```
sqlrustgo/
└── parser/
    ├── lib.rs
    ├── lexer.rs      # 词法分析
    ├── parser.rs     # 语法分析
    ├── token.rs      # Token 定义
    └── ast/          # AST 节点定义
```

## 核心组件

### Token (token.rs)

Token 是词法分析的最小单元。

**Token 类型**:
| 类型 | 示例 | 说明 |
|------|------|------|
| Keyword | SELECT, FROM, WHERE | SQL 关键字 |
| Identifier | table_name, column1 | 标识符 |
| Literal | 'hello', 123, 45.6 | 字面量 |
| Operator | +, -, *, /, = | 运算符 |
| Punctuation | (, ), ,, ; | 标点符号 |

**实现要点** (关键坑):
> Lexer 关键字添加: 既要在 `token.rs` 添加 `Token` 枚举, 也要在 `lexer.rs` 的 `match` 语句中添加 `"KEYWORD" => Token::Keyword`

### Lexer (lexer.rs)

词法分析器，将 SQL 字符串转换为 Token 流。

**关键函数**:
```rust
pub fn tokenize(&mut self, input: &str) -> Vec<Token>
```

**处理流程**:
1. 逐字符扫描输入
2. 识别 Token 类型
3. 生成 Token 序列

### Parser (parser.rs)

语法分析器，将 Token 流转换为 AST。

**关键函数**:
| 函数 | 功能 |
|------|------|
| `parse()` | 解析 SQL 语句入口 |
| `parse_create()` | 解析 CREATE 语句 |
| `parse_select()` | 解析 SELECT 语句 |
| `parse_insert()` | 解析 INSERT 语句 |
| `parse_update()` | 解析 UPDATE 语句 |
| `parse_delete()` | 解析 DELETE 语句 |

**实现要点** (关键坑):
> `parse_insert` 处理 REPLACE: 必须先检查 `Token::Replace`，存在则消耗；不存在才 `expect Token::Insert`

## AST 节点

### 语句类型

| 节点类型 | 说明 |
|----------|------|
| CreateTable | 建表语句 |
| Select | 查询语句 |
| Insert | 插入语句 |
| Update | 更新语句 |
| Delete | 删除语句 |
| DropTable | 删表语句 |

### 表达式类型

| 表达式类型 | 说明 |
|------------|------|
| BinaryExpr | 二元表达式 (a + b) |
| UnaryExpr | 一元表达式 (-a) |
| FunctionCall | 函数调用 |
| ColumnRef | 列引用 |
| Literal | 字面量 |

## 测试

```bash
cd sqlrustgo/parser
cargo test
```

**测试数量**: 72 个

## 依赖关系

```
parser
├── token.rs
├── lexer.rs
└── ast/
```

## 相关链接

- SQLRustGo 架构概览
- Executor 执行引擎

---

*最后更新: 2026-04-24*