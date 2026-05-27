# QMD Search

QMD (Q Mind) 是本地文档搜索引擎，采用 BM25 + 向量搜索 + LLM 重排序的三级架构。

## 配置

- **系统命令**: `qmd`
- **已知问题 (2026-04-24)**: 新文件不自动索引，需 `qmd collection remove <name>` + `qmd collection add` 重新添加
- **不存在命令**: `qmd collection update`（设计如此）
- **验证**: 添加后用 `qmd ls <name>` 检查文件是否被识别

## 相关页面

- [gbrain-knowledge](../wiki/gbrain-knowledge) — 三层知识体系中的第二层
- [hermes-agent](../wiki/hermes-agent) — agent 调用 QMD 进行文档搜索