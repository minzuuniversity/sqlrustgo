# SQLRustGo API 文档

> **版本**: v3.6.0
> **更新日期**: 2026-05-29

---

## 文档结构

```
docs/api/
├── README.md              # 本文件 — API 文档入口
├── openapi.yaml           # OpenAPI 3.0 规范（规划中）
└── rustdoc/               # rustdoc HTML 文档
    └── [crate]/           # 各 crate 的 API 文档
```

---

## API 文档类型

### 1. Rust API 文档（rustdoc）

SQLRustGo workspace 包含以下 crate，每个 crate 都有独立的 rustdoc 文档：

| Crate | 说明 | 文档状态 |
|-------|------|----------|
| `sqlrustgo-parser` | SQL 解析器 | rustdoc 生成中 |
| `sqlrustgo-planner` | 逻辑计划 | rustdoc 生成中 |
| `sqlrustgo-optimizer` | 查询优化器 | rustdoc 生成中 |
| `sqlrustgo-executor` | 执行引擎 | rustdoc 生成中 |
| `sqlrustgo-storage` | 存储引擎 | rustdoc 生成中 |
| `sqlrustgo-transaction` | 事务管理 | rustdoc 生成中 |
| `sqlrustgo-catalog` | 目录服务 | rustdoc 生成中 |
| `sqlrustgo-network` | 网络协议 | rustdoc 生成中 |
| `sqlrustgo-types` | 类型系统 | rustdoc 生成中 |

**生成 rustdoc**:
```bash
cargo doc --all-features --no-deps
```

**查看 rustdoc**:
```bash
# 在浏览器中打开
cargo doc --open
```

### 2. REST API 规范（OpenAPI）

> **状态**: 规划中

SQLRustGo 支持 MySQL 协议和可选的 REST API。OpenAPI 规范正在开发中。

**目标规范位置**: `docs/api/openapi.yaml`

### 3. CLI/REPL 文档

REPL 交互式 SQL 查询工具文档位于项目根目录的 `README.md`。

**启动 REPL**:
```bash
cargo run --bin sqlrustgo
```

---

## 快速链接

| 内容 | 位置 |
|------|------|
| 项目主 README | `/home/ai/sqlrustgo/README.md` |
| 版本文档索引 | [`docs/releases/`](releases/) |
| 治理文档 | [`docs/governance/`](governance/) |
| OO 文档 | [`docs/oo/`](oo/) |
| 架构文档 | [`docs/architecture/`](architecture/) |

---

## 贡献指南

### 添加 API 文档

1. 在代码中添加 rustdoc 注释：
   ```rust
   /// 执行 SQL 查询
   ///
   /// # Arguments
   /// * `sql` - SQL 语句字符串
   ///
   /// # Returns
   /// 查询结果集
   pub fn execute(&self, sql: &str) -> Result<Vec<Row>> { ... }
   ```

2. 运行 `cargo doc --all-features --no-deps` 生成文档

3. 提交到对应 crate 的 `docs/api/rustdoc/` 目录

### 更新 OpenAPI 规范

OpenAPI 规范使用 `utoipa` 注解自动生成。参见 `DOC_IMPROVEMENT_PLAN.md`。

---

## 参考

- [Rustdoc 官方文档](https://doc.rust-lang.org/rustdoc/)
- [OpenAPI 3.0 规范](https://spec.openapis.org/oas/v3.0.3)
- [utoipa - Rust OpenAPI 注解](https://github.com/juhaku/utoipa)
- [docs.rs - Rust 库文档托管](https://docs.rs)