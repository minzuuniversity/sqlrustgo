#!/bin/bash
# Gitea Issue Creator for GMP-Platform v1.0.0
TOKEN="07dc6c9f753a10ffbdb779bfc7d87668143e619c"
API="http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/issues"

create_issue() {
  local title="$1" body="$2" milestone="$3" labels="$4"
  local resp=$(curl -s -H "Authorization: token $TOKEN" -H "Content-Type: application/json" -X POST "$API" -d "{
    \"title\": \"$title\",
    \"body\": \"$body\",
    \"milestone\": $milestone,
    \"labels\": [$labels]
  }")
  local num=$(echo "$resp" | python3 -c "import sys,json; print(json.load(sys.stdin).get('number','FAIL'))" 2>/dev/null)
  echo "  #$num: $title"
}

echo "=== P0 ISSUEs (Core Development) ==="

#1
create_issue "[M1] 项目脚手架 + Cargo workspace + CLI 框架" \
  "## 目标\n建立 GMP 内审检索系统项目骨架，包括 Cargo workspace、CLI 框架（clap）、依赖配置。\n\n## 技术栈\n- Rust >= 1.80\n- clap (CLI 框架)\n- SQLRustGo crates 依赖声明\n- ollama-rs, serde, tokio, anyhow\n\n## 验收条件\n- [ ] \`cargo build --release\` 通过\n- [ ] \`gmp-retrieve --help\` 输出帮助信息\n- [ ] \`gmp-retrieve --version\` 输出版本号 v1.0.0-alpha\n- [ ] Cargo.toml 包含所有必要依赖\n\n## 关联文档\n- [开发计划](docs/releases/v1.0.0/DEVELOPMENT_PLAN.md)\n- [四层架构设计](docs/design/gmp-sqlrustgo-four-layer-plan.md)\n\n## 门禁\n- A-Gate: A1 Build, A3 Clippy, A4 Format" \
  16 "\\\"priority/p0\\\",\\\"type/feature\\\""

#2
create_issue "[M2] 数据导入 + 索引基础设施 (FTS/向量/图谱)" \
  "## 目标\n实现 gmp-md/ 文档集导入 SQLRustGo 统一数据底座，建立三种索引。\n\n## 子任务\n### 2a. 文档导入\n- 1696 文档 → \`gmp_documents\` 表\n- 分块 → \`gmp_document_chunks\` 表\n- 预期 ~8700 chunks\n\n### 2b. FTS 索引\n- BM25 评分排序 (依赖 Hermes PR)\n- GMP 词典分词 (FMM)\n\n### 2c. 向量索引\n- bge-m3 嵌入生成 (via Ollama)\n- HNSW 索引构建 (1024d)\n\n### 2d. 图谱构建\n- agent-knowledge 规则 → 实体节点\n- 关系抽取 → 知识图谱边\n- 目标: >=700 节点, >=1500 边\n\n## 验收条件\n- [ ] \`gmp-retrieve index --validate\` 显示 1696/1696\n- [ ] FTS 查询返回 >=5 结果 (channel fts)\n- [ ] 向量查询返回 >=5 结果 (channel vector)\n- [ ] 图谱查询返回 >=3 结果 (channel graph)\n\n## 关联文档\n- [开发计划](docs/releases/v1.0.0/DEVELOPMENT_PLAN.md)\n- [集成状态](docs/releases/v1.0.0/INTEGRATION_STATUS.md)\n\n## 门禁\n- A-Gate: A5 数据完整性, A6 FTS查询, A7 向量查询, A8 图谱查询" \
  17 "\\\"priority/p0\\\",\\\"type/feature\\\""

#3
create_issue "[M3] 四通道检索引擎 (规则/向量/全文/图谱)" \
  "## 目标\n实现四通道并行检索，各通道独立召回 Top-K 结果。\n\n## 子任务\n### 3a. 规则召回 (RuleChannel)\n- 匹配 agent-knowledge YAML 规则\n- 规则库 >= 55 条\n- RRF weight=2.0 (高优先级)\n- 返回: Top-10\n\n### 3b. 向量召回 (VectorChannel)\n- bge-m3 + HNSW 相似度检索\n- 返回: Top-30\n- 指标: recall@20 >= 70%\n\n### 3c. 全文召回 (FtsChannel)\n- BM25 评分排序\n- 返回: Top-30\n- 指标: recall@20 >= 50%\n\n### 3d. 图谱召回 (GraphChannel)\n- Cypher 查询 + BFS 邻居扩展\n- 2-hop 内邻居\n- 返回: Top-10\n\n## 验收条件\n- [ ] 四通道均可独立检索\n- [ ] \`gmp-retrieve search {query} --channel {name}\` 生效\n- [ ] \`gmp-retrieve search {query} --explain\` 显示各通道分数\n\n## 关联文档\n- [架构设计 §Layer 2](docs/design/gmp-sqlrustgo-four-layer-plan.md)\n- [功能矩阵](docs/releases/v1.0.0/FEATURE_MATRIX.md)\n\n## 门禁\n- A-Gate: A6-A9 通道查询验证" \
  17 "\\\"priority/p0\\\",\\\"type/feature\\\""

#4
create_issue "[M4] RRF 融合 + Reranker 重排引擎" \
  "## 目标\n实现多通道结果融合和精排。\n\n## 子任务\n### 4a. RRF 融合引擎\n- Reciprocal Rank Fusion (k=60)\n- 规则通道 weight=2.0, 其他=1.0\n- 输出: Top-20 candidates\n\n### 4b. Reranker 集成\n- bge-reranker-v2-m3 (via Ollama)\n- Cross-Encoder 精排\n- CPU 优先，支持 GPU 可选\n- 回退策略: Reranker 不可用时直接输出 RRF 结果\n\n## 验收条件\n- [ ] \`gmp-retrieve search {query} --all\` 返回 RRF 融合结果\n- [ ] \`gmp-retrieve search {query} --all --rerank\` 输出重排结果\n- [ ] \`gmp-retrieve search --explain\` 显示各通道原始排名\n- [ ] RRF 权重配置可调整\n\n## 关联文档\n- [架构设计 §Layer 3](docs/design/gmp-sqlrustgo-four-layer-plan.md)\n- [ADR-003: RRF 融合](docs/releases/v1.0.0/ARCHITECTURE_DECISIONS.md)\n\n## 门禁\n- A-Gate: A9 RRF 融合验证\n- B-Gate: B6 Reranker 可用" \
  17 "\\\"priority/p0\\\",\\\"type/feature\\\""

#5
create_issue "[M4] LLM 问答集成 + 意图分类器" \
  "## 目标\n基于检索结果生成带引用的答案，并自动识别用户查询意图。\n\n## 子任务\n### 5a. LLM 答案生成\n- Ollama + qwen2.5:14b\n- RAG prompt 模板 (4 context slots)\n- 引用溯源: 标记来源文档\n- 温度=0.3, max_tokens=1024\n- 失败重试: 3 次 + 30s 超时\n\n### 5b. 意图分类器\n- 7 种意图: RetrieveSop, CheckCompliance, CompareRegulation, TraceProcess, ClassifyIncident, GenerateChecklist, AnalyzeRootCause\n- 基于 prompt 分类，无需训练\n- 目标: 准确率 >= 80%\n\n## 验收条件\n- [ ] \`gmp-retrieve shell\` 交互问答可用\n- [ ] 答案包含 [来源: xxx.md] 引用\n- [ ] 意图分类在 explain 模式可见\n- [ ] 超时/失败可优雅降级\n\n## 关联文档\n- [架构设计 §Layer 4](docs/design/gmp-sqlrustgo-four-layer-plan.md)\n- [用户手册 §LLM 问答](docs/user/USER_MANUAL.md)\n\n## 门禁\n- B-Gate: B7 LLM 问答可用" \
  18 "\\\"priority/p0\\\",\\\"type/feature\\\""

#6
create_issue "[M3] CLI 全命令 (index/search/evaluate/shell/bench)" \
  "## 目标\n实现完整的 CLI 命令集，包括索引、检索、评估、交互式问答和性能基准。\n\n## 子命令\n| 命令 | 功能 | 参数 |\n|------|------|------|\n| \`index\` | 文档导入+索引 | --source, --validate, --append |\n| \`search\` | 多通道检索 | --channel, --all, --rerank, --explain |\n| \`evaluate\` | 391 场景评估 | --test-set, --by-category, --limit |\n| \`shell\` | 交互式问答 | — |\n| \`bench\` | 性能基准 | --rounds, --concurrent |\n\n## 验收条件\n- [ ] 所有子命令 --help 输出完整\n- [ ] search 支持单通道 + 全通道 + rerank 模式\n- [ ] evaluate 输出 JSON 格式结果\n- [ ] bench 输出 P50/P95/P99\n\n## 关联文档\n- [用户手册](docs/user/USER_MANUAL.md)\n- [部署指南](docs/releases/v1.0.0/DEPLOYMENT_GUIDE.md)\n\n## 门禁\n- R-Gate: 所有 CLI 命令可用" \
  18 "\\\"priority/p0\\\",\\\"type/feature\\\""

#7
create_issue "[M5] 391 场景测试 + 通过率报告" \
  "## 目标\n基于 gmp_kg_build 391 场景集，执行系统化测试并生成通过率报告。\n\n## 测试计划\n| 阶段 | 目标 | 通过率 |\n|------|------|--------|\n| Alpha | Layer 2 only | >= 65% |\n| Beta | Layer 2+3+4 | >= 78% |\n| RC #1 | 全量回归 | 记录基线 |\n| RC #2 | 修复后回归 | >= 80% |\n| GA | 最终验证 | >= 80% (无退化) |\n\n## 失败分类\n- 知识缺口 (缺中国GMP) ~15%\n- 语义偏差 ~10%\n- 图谱不足 ~5%\n- LLM 幻觉 ~5%\n- 数据不完整 ~5%\n\n## 验收条件\n- [ ] \`gmp-retrieve evaluate --test-set 391_scenarios.json\` 可用\n- [ ] 支持 --by-category 分类评估\n- [ ] 输出 JSON 报告 (通过/部分/失败/不可回答)\n\n## 关联文档\n- [测试计划](docs/releases/v1.0.0/TEST_PLAN.md)\n- [性能目标](docs/releases/v1.0.0/PERFORMANCE_TARGETS.md)\n\n## 门禁\n- R-Gate: R5 391 >= 80%" \
  18 "\\\"priority/p0\\\",\\\"type/test\\\""

#8
create_issue "[M5] 用户手册 + 部署指南 + 发布文档" \
  "## 目标\n完成 v1.0.0 所有用户和管理文档。\n\n## 文档清单\n- [x] 用户手册 (USER_MANUAL.md)\n- [x] 部署指南 (DEPLOYMENT_GUIDE.md)\n- [x] 发布说明 (RELEASE_NOTES.md)\n- [x] CHANGELOG.md\n- [ ] 更新 00-release-summary.md (GA 填写)\n- [ ] 更新 RELEASE_GATE_CHECKLIST.md (执行结果)\n\n## 验收条件\n- [ ] 用户可通过手册独立完成安装+检索\n- [ ] 部署指南覆盖 macOS + Linux\n- [ ] 发布说明列出所有核心特性\n\n## 关联文档\n- [用户手册](docs/user/USER_MANUAL.md)\n- [部署指南](docs/releases/v1.0.0/DEPLOYMENT_GUIDE.md)\n\n## 门禁\n- G-Gate: G5 文档清单" \
  18 "\\\"priority/p0\\\",\\\"type/docs\\\""

echo ""
echo "=== P1 ISSUEs (Enhancement) ==="

#9
create_issue "[P1] 索引增量更新支持" \
  "## 目标\n支持部分文档的增量导入和索引更新。\n\n## 需求\n- \`gmp-retrieve index --source {doc} --append\`\n- \`gmp-retrieve index --rebuild-vector\` 仅重建向量索引\n- 不影响已有数据\n\n## 验收条件\n- [ ] 单文档追加不重建全库\n- [ ] 增量向量更新可用\n\n## 关联文档\n- [开发计划 §P1](docs/releases/v1.0.0/DEVELOPMENT_PLAN.md)" \
  19 "\\\"priority/p1\\\",\\\"type/feature\\\""

#10
create_issue "[P1] 性能压测 + 基准报告" \
  "## 目标\n执行系统性能压测并输出基准报告。\n\n## 压测场景\n- 单查询: P50, P95, P99 延迟\n- 并发: 10 并发 x 100 轮\n- 稳定性: 50 轮无崩溃\n\n## 性能目标\n| 指标 | 目标 |\n|------|------|\n| P50 | < 5s |\n| P95 | < 15s |\n| P99 | < 30s |\n| 稳定性 | 0 crash |\n\n## 验收条件\n- [ ] \`gmp-retrieve bench\` 输出完整统计\n- [ ] 稳定性通过 50 轮\n\n## 关联文档\n- [性能目标](docs/releases/v1.0.0/PERFORMANCE_TARGETS.md)" \
  19 "\\\"priority/p1\\\",\\\"type/test\\\""

echo ""
echo "=== P2 ISSUEs (Future) ==="

#11
create_issue "[P2] 中国 GMP 条文补充到知识库" \
  "## 目标\n补充中国 GMP 2010 版条文，提升中文法规场景覆盖。\n\n## 需求\n- 中国 GMP 2010 核心条文\n- 与现有知识库融合\n- 改善知识缺口类失败\n\n## 验收条件\n- [ ] 391 场景 '知识缺口' 类失败减少 >= 5%\n\n## Milestone\nv1.1.0" \
  20 "\\\"priority/p2\\\",\\\"type/feature\\\""

#12
create_issue "[P2] 设备手册数据集成" \
  "## 目标\n集成设备操作和维护数据，改善设备类场景检索。\n\n## 需求\n- 设备 ID/SOP 文档导入\n- 与图表/操作细节关联\n\n## 验收条件\n- [ ] 设备类 391 场景通过率提升 >= 10%\n\n## Milestone\nv1.1.0" \
  20 "\\\"priority/p2\\\",\\\"type/feature\\\""

#13 - Web API
create_issue "[P2] Web API 模式 (RESTful)" \
  "## 目标\n提供 RESTful API 替代纯 CLI 访问。\n\n## 需求\n- POST /api/v1/search\n- POST /api/v1/evaluate\n- GET /api/v1/health\n- JSON 请求/响应\n\n## 验收条件\n- [ ] curl 可完成检索请求\n- [ ] 返回格式与 CLI 一致\n\n## Milestone\nv1.1.0" \
  20 "\\\"priority/p2\\\",\\\"type/feature\\\""

echo ""
echo "=== Done ==="