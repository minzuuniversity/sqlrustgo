---
id: doc-improvements-v360
title: v3.6.0 Documentation Improvements
version: 1.0
status: draft
created: 2026-05-29
author: hermes-z440
---

# v3.6.0 Documentation Improvements 规范

## 概要

根据 DOC_IMPROVEMENT_PLAN.md 和 OSS_DOC_ANALYSIS.md 的分析结果，改进 v3.6.0 的文档体系，重点实现：
1. **API 文档自动化** — P0 优先级
2. **OO 文档规范化** — P0 优先级
3. **rustdoc 输出整合** — P1 优先级

## 现状分析

### 已完成
- ✅ OSS 开源文档对标分析（OSS_DOC_ANALYSIS.md）
- ✅ 改进方案文档（DOC_IMPROVEMENT_PLAN.md）
- ✅ v3.6.0 基础文档结构

### 待改进

| 项目 | 当前状态 | 目标 |
|------|----------|------|
| docs/api/ 目录 | ❌ 不存在 | ✅ 创建并填充内容 |
| rustdoc 输出 | ❌ 无配置 | ✅ 启用 --document-private-items |
| OO 文档索引 | ⚠️ 分散在多版本 | ✅ 统一索引 |
| docs/README.md | ⚠️ 存在但不完整 | ✅ 增强入口导航 |
| rustfmt.toml | ❌ 不存在 | ✅ 创建标准化配置 |

## 实施范围

### P0 任务（必须完成）

#### 1. 创建 docs/api/ 目录结构

```
docs/api/
├── README.md              # API 文档入口
├── openapi.yaml           # OpenAPI 3.0 规范
└── rustdoc/               # rustdoc HTML 输出
    ├── index.html
    └── [crate]/
```

#### 2. 启用 rustdoc 私有项文档

在 `.cargo/config.toml` 中配置：
```toml
[build]
rustdocflags = ["--document-private-items"]
```

#### 3. 创建 docs/oo/ 统一 OO 文档索引

```
docs/oo/
├── README.md              # OO 文档统一入口
└── [module]/               # 各 OO 模块
```

### P1 任务（计划完成）

#### 4. 创建 rustfmt.toml

标准化 Rust 代码格式化配置。

#### 5. 更新 docs/README.md

增强文档入口导航。

#### 6. 验证 scripts/doc/ 相关脚本

## 任务分解

### Task 1: 创建 docs/api/ 目录和入口文档

**输入**: DOC_IMPROVEMENT_PLAN.md
**输出**: docs/api/README.md

**步骤**:
1. 创建 `docs/api/` 目录
2. 创建 `docs/api/README.md` 入口文档
3. 记录下一步（OpenAPI spec 生成）

### Task 2: 启用 rustdoc 配置

**输入**: 当前 .cargo/config.toml
**输出**: 更新后的 .cargo/config.toml

**步骤**:
1. 检查现有 .cargo/config.toml
2. 添加 rustdocflags 配置
3. 验证 `cargo doc --all-features` 可用

### Task 3: 创建 docs/oo/ 统一索引

**输入**: docs/releases/v*/oo/ 目录
**输出**: docs/oo/README.md

**步骤**:
1. 扫描所有版本的 oo/ 目录
2. 生成统一的 OO 文档索引
3. 包含每个 OO 模块的版本追溯

### Task 4: 创建 rustfmt.toml

**输入**: 最佳实践（tokio/serde 配置）
**输出**: rustfmt.toml

**步骤**:
1. 参考 tokio/serde 的 rustfmt.toml
2. 创建标准化配置
3. 验证 `cargo fmt --check` 通过

### Task 5: 更新 docs/README.md

**输入**: 当前文档结构
**输出**: 更新的 docs/README.md

**步骤**:
1. 扫描 docs/ 目录结构
2. 创建分类导航
3. 添加快速链接

## 验收标准

1. `docs/api/` 目录存在且包含 README.md
2. `.cargo/config.toml` 包含 rustdocflags 配置
3. `docs/oo/README.md` 统一索引存在
4. `rustfmt.toml` 存在且通过 `cargo fmt --check`
5. `docs/README.md` 包含完整导航
6. 所有新增文件遵循项目文档风格

## 技术约束

- 所有新增 .md 文件使用 UTF-8 编码
- 中文优先，必要时英中混排
- 文件路径使用绝对路径（从仓库根目录）
- 遵循现有文档的 YAML frontmatter 格式