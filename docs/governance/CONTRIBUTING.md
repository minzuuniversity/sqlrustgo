# CONTRIBUTING — 贡献指南

> **版本**: 1.0
> **创建日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro

---

## 一、开发环境

| 组件 | 要求 | 说明 |
|------|------|------|
| Rust | ≥ 1.80 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Ollama | latest | `brew install ollama` (macOS) |
| Gitea CLI | optional | 用于 ISSUE 管理 |

---

## 二、分支策略

```text
main                    # 稳定版本
  └── develop/v1.0.0    # 当前版本开发分支
       └── feature/xxx  # 功能分支 (Gitea ISSUE #N)
       └── bugfix/xxx   # 修复分支
```

### 分支命名规范

| 类型 | 格式 | 示例 |
|------|------|------|
| 开发 | `develop/vX.Y.Z` | `develop/v1.0.0` |
| 功能 | `feature/<描述>` | `feature/llm-chat-integration` |
| 修复 | `bugfix/<描述>` | `bugfix/rrf-score-normalization` |
| 发布 | `release/vX.Y.Z` | `release/v1.0.0` |

---

## 三、Commit Message 规范

```text
<type>: <简短描述>

[可选的详细说明]
[可选的 ISSUE 引用]

类型:
  feat      新功能
  fix       Bug 修复
  docs      文档变更
  test      测试相关
  refactor  重构
  chore     构建/工具
```

示例:
```
feat: 实现 BM25 全文检索评分

基于 storage::fts::InvertedIndex 增加 search_with_scores() 接口，
支持 k1/b 参数配置。关联 ISSUE #1.

Closes #1
```

---

## 四、代码审查要求

### 4.1 PR 提交前自检

- [ ] `cargo build --release` 通过
- [ ] `cargo test` 全部通过
- [ ] `cargo clippy -- -D warnings` 零警告
- [ ] `cargo fmt -- --check` 通过

### 4.2 PR 描述模板

```markdown
## 变更描述
{简短描述}

## 关联 ISSUE
Closes #{N}

## 变更类型
- [ ] 新功能
- [ ] Bug 修复
- [ ] 文档
- [ ] 重构

## 测试
- [ ] 单元测试通过
- [ ] 集成测试通过（如适用）
- [ ] 手动验证（如适用）

## 检查清单
- [ ] 代码遵循项目规范
- [ ] 添加了必要的测试
- [ ] 更新了相关文档
```

---

## 五、项目结构

```
GMP-Platform/
├── Cargo.toml              # Rust workspace
├── crates/
│   └── gmp-retrieve/       # 主项目 crate
│       ├── src/
│       │   ├── main.rs     # CLI 入口
│       │   ├── pipeline.rs # 四层管线
│       │   ├── channels/   # 检索引擎通道
│       │   ├── reranker/   # Reranker 集成
│       │   ├── llm/        # LLM 问答
│       │   └── cli/        # CLI 命令
│       └── tests/
├── gmp-md/                 # GMP 文档集 (1696 份)
├── agent-knowledge/        # 结构化规则 (55+ YAML)
├── gmp_kg_build/           # 391 测试场景
├── docs/
│   ├── design/             # 设计方案
│   ├── governance/         # 治理文档
│   ├── releases/           # 版本文档
│   └── user/               # 用户文档
└── scripts/
    └── gate/               # 门禁脚本
```

---

## 六、开发流程

### 6.1 功能开发

```
1. 从 Gitea ISSUE 认领任务
2. 创建 feature 分支: git checkout -b feature/xxx develop/v1.0.0
3. 实现功能 + 测试
4. 推送: git push origin feature/xxx
5. 创建 PR 到 develop/v1.0.0
6. 代码审查 + CI 通过
7. 合并
```

### 6.2 阶段推进

```
Alpha → Beta → RC → GA
  ↑        ↑      ↑     ↑
  A-Gate   B-Gate R-Gate G-Gate

每个阶段必须通过对应门禁检查清单
所有 FAIL 项必须创建 ISSUE 追踪
```

---

## 七、AI Agent 开发约束

AI Agent 在开发本系统时需遵循以下规则：

```text
✅ 所有代码变更前需阅读相关文件
✅ 门禁检查必须使用 GATE_SPEC.md 中的标准
✅ 开发计划必须遵循 DEVELOPMENT_PLAN_TEMPLATE.md 格式
✅ 测试计划必须遵循 TEST_PLAN_TEMPLATE.md 格式
❌ 禁止跳过门禁检查直接推进阶段
❌ 禁止在没有 PR 证据的情况下关闭 ISSUE
```