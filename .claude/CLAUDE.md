# CLAUDE.md

此文件为 Claude Code (claude.ai/code) 提供使用此存储库中的代码时的指导。

## 项目概述

SQLRustGo 是支持 SQL-92 子集的关系数据库系统的 Rust 实现。采用现代分层架构从头开始构建。

## 常用命令

```bash
# Build
cargo build --all-features

# Run tests
cargo test --all-features

# Run a single test
cargo test test_name --all-features

# Lint with clippy
cargo clippy --all-features -- -D warnings

# Format check
cargo fmt --check --all

# Doc tests
cargo test --doc

# Run REPL
cargo run --bin sqlrustgo
```

＃＃ 建筑学

> 本节按当前代码结构核对过（2026-09-29）。实际布局是 `crates/` 下的
> workspace crate + 根 `src/` 的执行引擎，不是早期文档里的单层 `src/` 分层。

```
┌─────────────────────────────────────┐
│      crates/cli (REPL / sqlrustgo)   │
├─────────────────────────────────────┤
│ crates/mysql-server, crates/server  │  ← MySQL 风格协议接入
├─────────────────────────────────────┤
│ src/execution_engine.rs + src/engine_*.rs │  ← Query execution
├─────────────────────────────────────┤
│ crates/executor                     │  ← 表达式求值 (eval_*)
├─────────────────────────────────────┤
│ crates/planner, crates/optimizer    │  ← 计划与优化
├─────────────────────────────────────┤
│ crates/parser                       │  ← SQL → AST
│   (lexer.rs, parser.rs, token.rs)   │     lexer 在此 crate 内，非独立模块
├─────────────────────────────────────┤
│ crates/storage                      │  ← Page, BufferPool, B+ Tree
├─────────────────────────────────────┤
│ crates/transaction                  │  ← WAL, TxManager
├─────────────────────────────────────┤
│ crates/network                      │  ← TCP server/client
├─────────────────────────────────────┤
│ crates/types                        │  ← Value, SqlError
└─────────────────────────────────────┘
```

## 关键模块

|模块|目的|
|--------|---------|
| `crates/parser` |对 SQL 输入进行标记（`lexer.rs`）并解析为语句 AST（`parser.rs`）|
| `crates/storage` |页面管理、BufferPool (LRU)、B+ Tree 索引|
| `src/engine_*.rs` |执行 SQL 语句|
| `crates/transaction` |预写日志，开始/提交/回滚|
| `crates/mysql-server` |采用 MySQL 风格协议的 TCP 服务器/客户端|

## 铁锈版

将 Rust 版本 2024 与 Tokio 异步运行时结合使用。
