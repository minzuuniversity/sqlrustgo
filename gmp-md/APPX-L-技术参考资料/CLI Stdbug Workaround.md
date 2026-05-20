---
entity_type: concept
confidence: high
domains: [sqlrustgo, gmp, cli, debugging]
created: 2025-04-17
updated: 2025-04-17
---

# CLI Stdbug Workaround

## 问题现象

单独执行 CLI 命令会**卡死超时**：

```bash
# ❌ 卡死（30秒超时）
echo '{"cmd":"Ping"}' | ./target/debug/sqlrustgo-gmp-cli

# ✅ 正常返回（< 100ms）
echo '{"cmd":"Ping"}' | strace -f ./target/debug/sqlrustgo-gmp-cli 2>&1 | tail -5

# ✅ 正常返回
stdbuf -oL -eL ./target/debug/sqlrustgo-gmp-cli < fifo.in > fifo.out &
```

## 根因分析

**SQLite journal 文件锁竞态 + Rust stdin/stdout 缓冲模式冲突。**

### 第一层：SQLite journal 锁

```
CLI 进程
├── 加载 ~/gmp-qa/gmp.db（557 docs + embeddings BLOB）
├── 加载 ~/gmp-qa/graph.db（559 nodes + 1842 edges）
└── 进入命令循环
    └── 等待 stdin JSON 命令
```

加载数据后，SQLite 内部维护着 `gmp.db-journal` 和 `graph.db-journal` 文件句柄。
当 Rust `read_line()` 从 stdin 读取时，如果 stdin 是**管道模式**（非 TTY），
Rust 标准库使用**全缓冲模式**（fully buffered），而非行缓冲。

### 第二层：Rust 全缓冲 vs SQLite 文件锁

| 缓冲模式 | 行为 | 对 SQLite 的影响 |
|----------|------|------------------|
| 行缓冲（TTY） | 收到 `\n` 才返回 | 正常 |
| 全缓冲（pipe） | 填充 4KB/8KB 缓冲区后才返回 | **SQLite journal 锁超时** |

`strace` 之所以有效：`-f` 参数 `ptrace()` 每个线程，每次系统调用都会触发调试事件，
这**改变了 Rust runtime 和 SQLite 的内部时序**，绕过了竞态条件。

`stdbuf` 之所以有效：`-oL -eL` 强制 **line-buffered** 输出和输入，
使 Rust `read_line()` 能正确处理 `\n` 结尾的命令。

### 验证

```bash
# 无 strace 时卡在 read_line（全缓冲）
# 加 strace 后正常返回
# 加 stdbuf -oL -eL 后正常返回
```

## 正确的 CLI 调用方式

```bash
# 必须用 stdbuf 包裹（生产环境 / 测试环境）
stdbuf -oL -eL env SQLRUSTGO_HOME=~/gmp-qa \
  ./target/debug/sqlrustgo-gmp-cli \
  < fifo.in > fifo.out &

# 或单次命令
( echo '{"cmd":"Ping"}'; sleep 3; ) | stdbuf -oL -eL \
  env SQLRUSTGO_HOME=~/gmp-qa \
  ./target/debug/sqlrustgo-gmp-cli
```

### 单行写法（不稳定，不推荐）

```bash
# 不稳定：可能卡死
echo '{"cmd":"Ping"}' | stdbuf -oL -eL ./target/debug/sqlrustgo-gmp-cli

# 延迟可解决部分问题
( sleep 0.2; echo '{"cmd":"Ping"}'; ) | ./target/debug/sqlrustgo-gmp-cli
```

推荐用 **named pipe + 后台进程 + 延迟读取** 的方式。

## 相关

- [[SQLRustGo GMP CLI]]
- [[Persist Sqlite Backend]]
