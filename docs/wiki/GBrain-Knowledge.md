# GBrain Knowledge Base

GBrain 是一个结构化知识库系统，位于 `~/wiki/gbrain/`，通过 `~/wiki/sync_to_github_wiki.py` 脚本同步到 GitHub Wiki。

## 架构

GBrain 是三层知识体系的一部分:
- **第一层 (QMD)**: 原始文档，本地 BM25+向量搜索
- **第二层 (GBrain)**: 结构化知识库，通过 LLM 处理生成，同步到 GitHub Wiki
- **第三层 (LLM-Wiki)**: 概念/实体/比较页面，人机协作的知识积累层

## 技术堆栈

- **存储**: SQLite + 本地文件
- **索引**: BM25 + 向量嵌入
- **重排序**: LLM 精排
- **同步**: GitHub Wiki API
- **查询路由**: `~/wiki/query_router.py`

## 相关页面

- [qmd-search](../wiki/qmd-search) — 第一层搜索引擎
- [hermes-agent](../wiki/hermes-agent) — 代理调用 GBrain 查询路由