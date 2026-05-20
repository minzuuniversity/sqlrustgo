#!/usr/bin/env python3
"""Create Gitea issues for GMP-Platform v1.0.0"""
import json, urllib.request

TOKEN = "07dc6c9f753a10ffbdb779bfc7d87668143e619c"
BASE = "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/issues"

A = 16  # v1.0.0-A (Alpha)
B = 17  # v1.0.0-B (Beta)
RC = 18 # v1.0.0-RC
GA = 19 # v1.0.0-GA
V11 = 20 # v1.1.0

issues = [
    # P0 Core
    {"title": "[M1] 项目脚手架 + Cargo workspace + CLI 框架", "milestone": A, "labels": ["priority/p0", "type/feature"],
     "body": """## 目标
建立 GMP 内审检索系统项目骨架，包括 Cargo workspace、CLI 框架（clap）、依赖配置。

## 技术栈
- Rust >= 1.80
- clap (CLI 框架)
- SQLRustGo crates 依赖声明
- ollama-rs, serde, tokio, anyhow

## 验收条件
- [ ] `cargo build --release` 通过
- [ ] `gmp-retrieve --help` 输出帮助信息
- [ ] `gmp-retrieve --version` 输出版本号 v1.0.0-alpha
- [ ] Cargo.toml 包含所有必要依赖

## 关联文档
- [开发计划](docs/releases/v1.0.0/DEVELOPMENT_PLAN.md)
- [四层架构设计](docs/design/gmp-sqlrustgo-four-layer-plan.md)

## 门禁
- A-Gate: A1 Build, A3 Clippy, A4 Format"""},

    {"title": "[M2] 数据导入 + 索引基础设施 (FTS/向量/图谱)", "milestone": B, "labels": ["priority/p0", "type/feature"],
     "body": """## 目标
实现 gmp-md/ 文档集导入 SQLRustGo 统一数据底座，建立三种索引。

## 子任务
### 2a. 文档导入
- 1696 文档 -> `gmp_documents` 表
- 分块 -> `gmp_document_chunks` 表
- 预期 ~8700 chunks

### 2b. FTS 索引
- BM25 评分排序 (依赖 Hermes PR)
- GMP 词典分词 (FMM)

### 2c. 向量索引
- bge-m3 嵌入生成 (via Ollama)
- HNSW 索引构建 (1024d)

### 2d. 图谱构建
- agent-knowledge 规则 -> 实体节点
- 关系抽取 -> 知识图谱边
- 目标: >=700 节点, >=1500 边

## 验收条件
- [ ] `gmp-retrieve index --validate` 显示 1696/1696
- [ ] FTS 查询返回 >=5 结果
- [ ] 向量查询返回 >=5 结果
- [ ] 图谱查询返回 >=3 结果

## 关联文档
- [集成状态](docs/releases/v1.0.0/INTEGRATION_STATUS.md)

## 门禁
- A-Gate: A5 数据完整性, A6-A8 通道查询"""},

    {"title": "[M3] 四通道检索引擎 (规则/向量/全文/图谱)", "milestone": B, "labels": ["priority/p0", "type/feature"],
     "body": """## 目标
实现四通道并行检索，各通道独立召回 Top-K 结果。

## 子任务
### 3a. 规则召回 (RuleChannel)
- 匹配 agent-knowledge YAML 规则 (>= 55 条)
- RRF weight=2.0 (高优先级), Top-10

### 3b. 向量召回 (VectorChannel)
- bge-m3 + HNSW 相似度检索
- Top-30, recall@20 >= 70%

### 3c. 全文召回 (FtsChannel)
- BM25 评分排序, Top-30
- recall@20 >= 50%

### 3d. 图谱召回 (GraphChannel)
- Cypher 查询 + BFS 邻居扩展
- 2-hop 内邻居, Top-10

## 验收条件
- [ ] 四通道均可独立检索
- [ ] `gmp-retrieve search {query} --channel {name}` 生效
- [ ] `gmp-retrieve search {query} --explain` 显示各通道分数

## 关联文档
- [架构设计 Layer 2](docs/design/gmp-sqlrustgo-four-layer-plan.md)
- [功能矩阵](docs/releases/v1.0.0/FEATURE_MATRIX.md)

## 门禁
- A-Gate: A6-A9 通道查询验证"""},

    {"title": "[M4] RRF 融合 + Reranker 重排引擎", "milestone": B, "labels": ["priority/p0", "type/feature"],
     "body": """## 目标
实现多通道结果融合和精排。

## 子任务
### 4a. RRF 融合引擎
- Reciprocal Rank Fusion (k=60)
- 规则通道 weight=2.0, 其他=1.0
- 输出: Top-20 candidates

### 4b. Reranker 集成
- bge-reranker-v2-m3 (via Ollama)
- Cross-Encoder 精排, CPU 优先
- 回退: Reranker 不可用时直接输出 RRF 结果

## 验收条件
- [ ] `gmp-retrieve search {query} --all` 返回 RRF 融合结果
- [ ] `gmp-retrieve search {query} --rerank` 输出重排结果
- [ ] `gmp-retrieve search --explain` 显示原始排名
- [ ] RRF 权重配置可调整

## 关联文档
- [ADR-003: RRF 融合](docs/releases/v1.0.0/ARCHITECTURE_DECISIONS.md)

## 门禁
- A-Gate: A9 RRF 融合验证
- B-Gate: B6 Reranker 可用"""},

    {"title": "[M4] LLM 问答集成 + 意图分类器", "milestone": RC, "labels": ["priority/p0", "type/feature"],
     "body": """## 目标
基于检索结果生成带引用的答案，并自动识别用户查询意图。

## 子任务
### 5a. LLM 答案生成
- Ollama + qwen2.5:14b
- RAG prompt 模板 (4 context slots)
- 引用溯源: 标记来源文档
- 温度=0.3, max_tokens=1024
- 失败重试: 3 次 + 30s 超时

### 5b. 意图分类器
- 7 种意图
- 基于 prompt 分类, 准确率 >= 80%

## 验收条件
- [ ] `gmp-retrieve shell` 交互问答可用
- [ ] 答案包含 [来源: xxx.md] 引用
- [ ] 超时/失败可优雅降级

## 关联文档
- [架构设计 Layer 4](docs/design/gmp-sqlrustgo-four-layer-plan.md)
- [用户手册 LLM问答](docs/user/USER_MANUAL.md)

## 门禁
- B-Gate: B7 LLM 问答可用"""},

    {"title": "[M3] CLI 全命令 (index/search/evaluate/shell/bench)", "milestone": RC, "labels": ["priority/p0", "type/feature"],
     "body": """## 目标
实现完整的 CLI 命令集。

## 子命令
| 命令 | 功能 |
|------|------|
| `index` | 文档导入+索引 (--source, --validate) |
| `search` | 多通道检索 (--channel, --all, --rerank, --explain) |
| `evaluate` | 391 场景评估 (--test-set, --by-category, --limit) |
| `shell` | 交互式问答 |
| `bench` | 性能基准 (--rounds, --concurrent) |

## 验收条件
- [ ] 所有子命令 --help 输出完整
- [ ] evaluate 输出 JSON 格式结果
- [ ] bench 输出 P50/P95/P99

## 关联文档
- [用户手册](docs/user/USER_MANUAL.md)
- [部署指南](docs/releases/v1.0.0/DEPLOYMENT_GUIDE.md)"""},

    {"title": "[M5] 391 场景测试 + 通过率报告", "milestone": RC, "labels": ["priority/p0", "type/test"],
     "body": """## 目标
基于 391 场景集，执行系统化测试并生成通过率报告。

## 测试阶段
| 阶段 | 目标 |
|------|------|
| Alpha | Layer 2 only, >= 65% |
| Beta | Layer 2+3+4, >= 78% |
| RC #1 | 全量回归, 记录基线 |
| RC #2 | 修复后回归, >= 80% |
| GA | 最终验证, >= 80% (无退化) |

## 失败分类
- 知识缺口 (缺中国GMP) ~15%
- 语义偏差 ~10%
- 图谱不足 ~5%
- LLM 幻觉 ~5%
- 数据不完整 ~5%

## 验收条件
- [ ] `gmp-retrieve evaluate --test-set 391_scenarios.json` 可用
- [ ] 支持 --by-category 分类评估
- [ ] 输出 JSON 报告

## 关联文档
- [测试计划](docs/releases/v1.0.0/TEST_PLAN.md)
- [性能目标](docs/releases/v1.0.0/PERFORMANCE_TARGETS.md)"""},

    {"title": "[M5] 用户手册 + 部署指南 + 发布文档", "milestone": RC, "labels": ["priority/p0", "type/docs"],
     "body": """## 目标
完成 v1.0.0 所有用户和管理文档。

## 文档清单
- [x] 用户手册 (USER_MANUAL.md)
- [x] 部署指南 (DEPLOYMENT_GUIDE.md)
- [x] 发布说明 (RELEASE_NOTES.md)
- [x] CHANGELOG.md
- [ ] 更新 00-release-summary.md (GA 填写)
- [ ] 更新 RELEASE_GATE_CHECKLIST.md (执行结果)

## 验收条件
- [ ] 用户可通过手册独立完成安装+检索
- [ ] 部署指南覆盖 macOS + Linux

## 关联文档
- [用户手册](docs/user/USER_MANUAL.md)
- [部署指南](docs/releases/v1.0.0/DEPLOYMENT_GUIDE.md)"""},

    # P1 Enhancement
    {"title": "[P1] 索引增量更新支持", "milestone": GA, "labels": ["priority/p1", "type/feature"],
     "body": """## 目标
支持部分文档的增量导入和索引更新。

## 需求
- `gmp-retrieve index --source {doc} --append`
- `gmp-retrieve index --rebuild-vector` 仅重建向量索引
- 不影响已有数据

## 验收条件
- [ ] 单文档追加不重建全库
- [ ] 增量向量更新可用"""},

    {"title": "[P1] 性能压测 + 基准报告", "milestone": GA, "labels": ["priority/p1", "type/test"],
     "body": """## 目标
执行系统性能压测并输出基准报告。

## 性能目标
| 指标 | 目标 |
|------|------|
| P50 | < 5s |
| P95 | < 15s |
| P99 | < 30s |
| 稳定性 | 50轮 0 crash |

## 验收条件
- [ ] `gmp-retrieve bench` 输出完整统计
- [ ] 稳定性通过 50 轮

## 关联文档
- [性能目标](docs/releases/v1.0.0/PERFORMANCE_TARGETS.md)"""},

    # P2 Future
    {"title": "[P2] 中国 GMP 条文补充到知识库", "milestone": V11, "labels": ["priority/p2", "type/feature"],
     "body": """## 目标
补充中国 GMP 2010 版条文，提升中文法规场景覆盖。

## 需求
- 中国 GMP 2010 核心条文
- 与现有知识库融合

## 验收条件
- [ ] 391 场景"知识缺口"类失败减少 >= 5%

## Milestone: v1.1.0"""},

    {"title": "[P2] 设备手册数据集成", "milestone": V11, "labels": ["priority/p2", "type/feature"],
     "body": """## 目标
集成设备操作和维护数据，改善设备类场景检索。

## 验收条件
- [ ] 设备类 391 场景通过率提升 >= 10%

## Milestone: v1.1.0"""},

    {"title": "[P2] Web API 模式 (RESTful)", "milestone": V11, "labels": ["priority/p2", "type/feature"],
     "body": """## 目标
提供 RESTful API 替代纯 CLI 访问。

## API 端点
- POST /api/v1/search
- POST /api/v1/evaluate
- GET /api/v1/health

## 验收条件
- [ ] curl 可完成检索请求
- [ ] 返回格式与 CLI 一致

## Milestone: v1.1.0"""},
]

def create_issue(title, body, milestone, _labels):
    data = json.dumps({
        "title": title,
        "body": body,
        "milestone": milestone
    }).encode("utf-8")
    req = urllib.request.Request(BASE, data=data, method="POST")
    req.add_header("Authorization", f"token {TOKEN}")
    req.add_header("Content-Type", "application/json")
    resp = urllib.request.urlopen(req)
    result = json.loads(resp.read())
    return result["number"], result["title"]

for i, iss in enumerate(issues):
    num, title = create_issue(iss["title"], iss["body"], iss["milestone"], iss["labels"])
    print(f"  #{num}: {title}")

print(f"\nDone: {len(issues)} issues created")