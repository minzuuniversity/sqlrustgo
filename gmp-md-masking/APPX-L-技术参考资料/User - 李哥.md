---
entity_type: person
confidence: high
domains: [team, gmp-project]
created: 2025-04-17
updated: 2025-04-17
---

# 李哥 (User Profile)

## 背景

- 身份：高小厂（一家制药公司？）的助理
- 主要工作：开发、测试、部署软件
- 语言：中文

## 偏好与风格

- 偏好**三路检索架构**：向量搜索 + 图搜索 + 精确 SQL，RRF 融合
- 能独立诊断 Ollama embedding 问题根因（T5 tokenizer 不支持）
- 说话直接，不绕弯子
- 喜欢用实际数据验证，而非理论推演

## 项目上下文

- **SQLRustGo**：Rust 实现的关系型数据库，带图存储和向量存储
- **GMP 知识库**：药品生产质量管理规范知识管理系统，557 SOP 文档
- **opencode**：AI 编程工具，已安装并配置 MiniMax-CN-Coding-Plan 模型
- **qmd**：另一个向量搜索工具，已集成

## 已知环境

- Ollama qwen2.5（3584维）在 M6000 GPU 上运行正常
- Ollama embedding 方案：推荐 `nomic-embed-text`，备选 sentence-transformers
- CLI 必须用 `stdbuf -oL -eL` 包裹避免 SQLite journal 竞态

## 相关

- [[SQLRustGo GMP CLI]]
- [[Dashboard]]
