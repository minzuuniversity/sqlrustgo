# SQLRustGo 文档改进方案

> **日期**: 2026-05-29
> **版本**: v1.0
> **基于**: OSS_DOC_ANALYSIS.md (开源文档对标分析)

---

## 一、改进目标

1. **API 文档自动化** — 优先级 P0
2. **OO 文档规范化** — 优先级 P0
3. **文档结构优化** — 优先级 P1
4. **rustdoc 输出整合** — 优先级 P1

---

## 二、API 文档自动化方案

### 2.1 现状问题

- `docs/api/` 目录不存在
- 无 OpenAPI/Swagger 规范文件
- 无 REST API 文档
- 无 CLI/REPL 文档

### 2.2 解决方案

#### Phase 1: rustdoc 输出到 docs/

```toml
# Cargo.toml 添加
[package]
rustdoc-flags = "--蓋特色 --no-edition"

# 或在 .cargo/config.toml 中配置 doc 输出
[build]
rustdocflags = ["--output-format", "html"]
```

#### Phase 2: OpenAPI 规范（REST API）

对于 `sqlrustgo-network` crate 的 REST API endpoints：

```yaml
# docs/api/openapi.yaml
openapi: 3.0.3
info:
  title: SQLRustGo REST API
  version: 3.5.0
paths:
  /api/v1/query:
    post:
      summary: Execute SQL Query
      tags: [query]
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                sql:
                  type: string
                params:
                  type: array
      responses:
        '200':
          description: Query result
```

#### Phase 3: utoipa 集成（推荐）

在 `Cargo.toml` 添加：

```toml
[dependencies]
utoipa = "4"
utoipa-swagger-ui = "6"
```

在代码中添加 OpenAPI annotations：

```rust
#[utoipa::path(
    post,
    path = "/api/v1/query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query result", body = QueryResponse)
    )
)]
```

### 2.3 实现计划

| 阶段 | 任务 | 产出 |
|------|------|------|
| P0-1 | 创建 `docs/api/` 目录结构 | docs/api/ 目录 |
| P0-2 | 启用 `cargo doc --document-private-items` | docs/rustdoc/ HTML |
| P0-3 | 添加 utoipa 到 network crate | OpenAPI annotations |
| P0-4 | 生成 openapi.yaml | docs/api/openapi.yaml |
| P0-5 | 集成 Swagger UI | docs/api/swagger-ui/ |

---

## 三、OO 文档规范化方案

### 3.1 现状问题

- OO 文档分散在多个版本目录
- 无统一索引
- 无自动化生成

### 3.2 解决方案

#### Phase 1: 统一 OO 文档结构

```
docs/
└── oo/                          # 统一 OO 文档目录
    ├── index.md                 # 索引（自动生成）
    ├── digital-signature-chain/ # OO-1
    ├── electronic-signature/    # OO-2 (21 CFR Part 11)
    ├── immutable-record/        # OO-3
    ├── correction-chain/        # OO-4
    ├── provenance-tracking/     # OO-5
    ├── trusted-timestamp/       # OO-6
    ├── hsm-kms-integration/    # OO-6
    ├── gmp-workflow-engine/     # OO-9
    └── README.md                # 入口
```

#### Phase 2: OO 文档模板标准化

每个 OO 模块包含：
1. `SPEC.md` — 功能规格（来自 Issue）
2. `IMPLEMENTATION.md` — 实现细节
3. `TEST_REPORT.md` — 测试报告
4. `CHANGELOG.md` — 变更记录

#### Phase 3: 自动化 OO 文档生成

在 `scripts/doc/gen_oo_doc.py` 中：

```python
#!/usr/bin/env python3
"""
自动生成 OO 文档索引
扫描 docs/releases/v*/oo/ 目录
生成统一的 docs/oo/index.md
"""
```

### 3.3 实现计划

| 阶段 | 任务 | 产出 |
|------|------|------|
| P0-1 | 创建 `docs/oo/` 统一目录 | docs/oo/index.md |
| P0-2 | 标准化 OO 文档模板 | docs/oo/templates/ |
| P0-3 | 编写 OO 文档生成脚本 | scripts/doc/gen_oo_doc.py |
| P0-4 | 迁移现有 OO 文档 | docs/oo/ 完整内容 |

---

## 四、rustdoc 输出整合方案

### 4.1 rustdoc 配置

创建 `rustfmt.toml`（当前缺失）：

```toml
# rustfmt.toml
edition = "2021"
max_width = 100
use_small_heuristics = "Default"
```

创建 `.cargo/config.toml`：

```toml
[build]
rustdocflags = [
    "--no-edition",
    "--output-format", "html",
    "--document-private-items",
]
```

### 4.2 CI 集成

在 `scripts/ci/doc_check.sh` 中：

```bash
#!/bin/bash
# rustdoc CI 检查
cargo doc --all-features --no-deps
# 检查是否有 warnings
cargo doc --all-features --no-deps -- --document-private-items 2>&1 | grep -q "warning" && exit 1
```

---

## 五、文档目录结构优化

### 5.1 目标结构

```
docs/
├── api/                      # API 文档（新增）
│   ├── openapi.yaml
│   ├── swagger-ui/
│   └── rustdoc/             # rustdoc 输出
├── oo/                       # OO 文档（整合）
│   ├── index.md
│   └── [module]/
├── releases/                 # 版本文档（已有）
├── governance/               # 治理文档（已有）
├── architecture/             # 架构文档（已有）
├── tutorials/                # 教程（已有）
└── README.md                 # 文档入口
```

### 5.2 README.md 入口

更新 `docs/README.md`：

```markdown
# SQLRustGo 文档

## 文档结构

| 目录 | 说明 |
|------|------|
| [api/](api/) | API 文档（OpenAPI + rustdoc） |
| [oo/](oo/) | OO 操作文档（GMP合规） |
| [releases/](releases/) | 版本发布文档 |
| [governance/](governance/) | 治理文档 |
| [architecture/](architecture/) | 架构文档 |

## 快速链接

- [API Documentation](api/)
- [OO Documentation](oo/)
- [版本发布](releases/)
```

---

## 六、测试文档现状与改进

### 6.1 现状

| 指标 | 数值 |
|------|------|
| 测试文件 | 65 个 .rs |
| e2e 测试 | 7 个 |
| CI 测试 | 3 个 |
| 门禁脚本 | 63 个 |

### 6.2 改进计划

1. **测试覆盖报告自动化**
   - 已有: `scripts/gate/coverage_threshold.toml`
   - 改进: `cargo llvm-cov report --html` 生成可视化报告

2. **TPC-H 测试文档**
   - 当前: 无预生成数据
   - 改进: 添加 `scripts/tpch/TPC-H-TEST-GUIDE.md`

3. **门禁测试脚本文档**
   - 当前: 63 个脚本，无统一索引
   - 改进: 创建 `scripts/gate/README.md`

---

## 七、优先级汇总

| 优先级 | 任务 | 预计工时 |
|--------|------|----------|
| P0 | 创建 docs/api/ 目录结构 | 2h |
| P0 | 启用 rustdoc 输出 | 1h |
| P0 | 创建 docs/oo/ 统一索引 | 2h |
| P0 | 编写 OO 文档生成脚本 | 3h |
| P1 | 添加 utoipa 依赖到 network crate | 4h |
| P1 | 生成 openapi.yaml | 4h |
| P1 | 更新 docs/README.md 入口 | 1h |
| P2 | CI 集成 rustdoc 检查 | 2h |
| P2 | TPC-H 测试文档 | 2h |

---

## 八、参考项目

| 项目 | 参考内容 |
|------|---------|
| tokio | rustdoc + docs.rs 集成 |
| sqlx | README Badges + FAQ |
| duckdb | 独立文档站点 + Sphinx |
| swagger-ui | OpenAPI → UI 自动化 |

---

*本文档基于 OSS_DOC_ANALYSIS.md 分析结果生成*