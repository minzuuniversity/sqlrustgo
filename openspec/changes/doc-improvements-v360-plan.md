# v3.6.0 文档改进计划

> **状态**: completed
> **创建日期**: 2026-05-29
> **规范**: doc-improvements-v360

## 概述

根据 DOC_IMPROVEMENT_PLAN.md 和 OSS_DOC_ANALYSIS.md 的分析，对 SQLRustGo v3.6.0 进行文档体系改进，重点实现 API 文档自动化和 OO 文档规范化。

## 任务列表

### Task 1: 创建 docs/api/ 目录和入口文档

**规范**: doc-improvements-v360/spec.md § Task 1

```markdown
- [ ] 创建 docs/api/ 目录
- [ ] 创建 docs/api/README.md
  - [ ] 包含 API 文档结构说明
  - [ ] 包含 OpenAPI 规范位置
  - [ ] 包含 rustdoc 输出位置
- [ ] 提交到 openspec 变更
```

**验收**: `docs/api/README.md` 存在且包含完整导航

---

### Task 2: 启用 rustdoc 配置

**规范**: doc-improvements-v360/spec.md § Task 2

```markdown
- [ ] 检查现有 .cargo/config.toml
- [ ] 添加 rustdocflags = ["--document-private-items"]
- [ ] 验证 cargo doc --all-features --no-deps 可用
- [ ] 提交到 openspec 变更
```

**验收**: `.cargo/config.toml` 包含 rustdocflags

---

### Task 3: 创建 docs/oo/ 统一索引

**规范**: doc-improvements-v360/spec.md § Task 3

```markdown
- [ ] 扫描 docs/releases/v*/oo/ 目录
- [ ] 创建 docs/oo/README.md
  - [ ] 包含所有 OO 模块列表
  - [ ] 包含版本追溯
  - [ ] 包含每个模块的简要说明
- [ ] 提交到 openspec 变更
```

**验收**: `docs/oo/README.md` 存在且包含所有 OO 模块

---

### Task 4: 创建 rustfmt.toml

**规范**: doc-improvements-v360/spec.md § Task 4

```markdown
- [ ] 参考 tokio/serde 的 rustfmt.toml
- [ ] 创建项目根目录 rustfmt.toml
  - [ ] 设置 max_width = 100
  - [ ] 设置 edition = "2021"
  - [ ] 设置 use_small_heuristics = "Default"
- [ ] 运行 cargo fmt --check 验证
- [ ] 提交到 openspec 变更
```

**验收**: `rustfmt.toml` 存在且 `cargo fmt --check` 通过

---

### Task 5: 更新 docs/README.md

**规范**: doc-improvements-v360/spec.md § Task 5

```markdown
- [ ] 扫描 docs/ 目录结构
- [ ] 更新 docs/README.md
  - [ ] 包含文档结构表格
  - [ ] 包含快速链接
  - [ ] 包含 api/ 和 oo/ 链接
- [ ] 提交到 openspec 变更
```

**验收**: `docs/README.md` 包含完整导航

---

## 执行顺序

```
Task 1 → Task 3 → Task 2 → Task 4 → Task 5
  ↑         ↑         ↑         ↑         ↑
API目录   OO索引   rustdoc   rustfmt   入口
```

## 资源估算

| Task | 工时 | 复杂度 |
|------|------|--------|
| Task 1 | 1h | 低 |
| Task 2 | 0.5h | 低 |
| Task 3 | 2h | 中 |
| Task 4 | 0.5h | 低 |
| Task 5 | 0.5h | 低 |
| **总计** | **4.5h** | - |

## 依赖

- DOC_IMPROVEMENT_PLAN.md（已存在）
- OSS_DOC_ANALYSIS.md（已存在）
- openspec/specs/doc-improvements-v360.spec.md（本文档依赖）

## 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| rustfmt 配置与现有代码冲突 | 中 | 预检现有代码格式化状态 |
| OO 文档版本分散 | 低 | 统一索引支持多版本追溯 |

## 完成标准

所有 Task 验收标准达成后，变更进入 `status: completed`。