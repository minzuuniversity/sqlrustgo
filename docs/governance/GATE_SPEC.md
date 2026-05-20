# GMP 内审检索系统 门禁规范

> **版本**: 1.0
> **创建日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro

> **SSOT 声明**: 本文档是 GMP 内审检索系统门禁定义的唯一权威来源。所有版本的门禁检查必须基于本文档。

---

## 一、门禁体系概述

### 1.1 四级门禁模型

```
A-Gate → B-Gate → R-Gate → G-Gate
 (α)      (β)      (RC)     (GA)
```

| 门禁 | 名称 | 阶段目标 | 391 场景目标 |
|------|------|----------|-------------|
| A-Gate | Alpha Gate | 核心功能可运行 | — |
| B-Gate | Beta Gate | 功能冻结，全链路可用 | ≥ 65% |
| R-Gate | RC Gate | 发布候选，性能达标 | ≥ 80% |
| G-Gate | GA Gate | 正式发布 | ≥ 80% (无退化) |

---

## 二、A-Gate (Alpha Gate)

### 2.1 入口条件

- 数据底座完成 (1696 文档入库)
- 四通道检索可用 (规则/向量/全文/图谱)
- RRF 融合可用

### 2.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| A1 | 编译 | `cargo build --release` | 无错误 |
| A2 | 单元测试 | `cargo test` | ≥ 80% 通过 |
| A3 | Clippy | `cargo clippy -- -D warnings` | 零警告 |
| A4 | 格式化 | `cargo fmt -- --check` | 无格式错误 |
| A5 | 数据完整性 | `gmp-retrieve index --validate` | 1696/1696 入库 |
| A6 | FTS 查询 | `gmp-retrieve search "SOP" --channel fts` | 返回结果 ≥ 5 |
| A7 | 向量查询 | `gmp-retrieve search "偏差" --channel vector` | 返回结果 ≥ 5 |
| A8 | 图谱查询 | `gmp-retrieve search "CAPA" --channel graph` | 返回结果 ≥ 3 |
| A9 | RRF 融合 | `gmp-retrieve search "偏差分级" --all` | 多通道融合结果 |

---

## 三、B-Gate (Beta Gate)

### 3.1 入口条件

- Reranker 集成完成
- LLM 问答集成完成
- CLI 全命令可用

### 3.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| B1 | 编译 | `cargo build --release` | 无错误 |
| B2 | 单元测试 | `cargo test` | ≥ 90% 通过 |
| B3 | Clippy | `cargo clippy -- -D warnings` | 零警告 |
| B4 | 格式化 | `cargo fmt -- --check` | 无格式错误 |
| B5 | CLI 命令 | `gmp-retrieve --help` + 各子命令 | 全部可用 |
| B6 | Reranker | `gmp-retrieve search "SOP起草" --rerank` | 重排后结果变化 |
| B7 | LLM 问答 | `gmp-retrieve shell` 交互问答 | 返回带引用的答案 |
| B8 | 规则召回 | `gmp-retrieve search "偏差分级" --channel rule` | 匹配到规则 |
| B9 | 391 场景 | `gmp-retrieve evaluate --test-set 391_scenarios.json` | ≥ 65% |

---

## 四、R-Gate (RC Gate)

### 4.1 入口条件

- 391 场景通过率 ≥ 65% (Beta 基线)
- 功能冻结

### 4.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| R1 | 编译 | `cargo build --release` | 无错误 |
| R2 | 单元测试 | `cargo test` | 100% 通过 |
| R3 | Clippy | `cargo clippy -- -D warnings` | 零警告 |
| R4 | 格式化 | `cargo fmt -- --check` | 无格式错误 |
| R5 | 391 场景 | `gmp-retrieve evaluate --test-set 391_scenarios.json` | ≥ 80% |
| R6 | 性能 | `gmp-retrieve bench --concurrent 10 --rounds 100` | P50 < 5s, P95 < 15s |
| R7 | 稳定性 | `gmp-retrieve stress --iterations 50` | 无崩溃 |
| R8 | 安全 | `cargo audit` + API Key 检查 | 无高危, 无泄露 |
| R9 | 文档 | 用户手册 + 部署指南 | 存在且可执行 |
| R10 | 引用溯源 | 抽样 20 个 LLM 答案 | 引用来目可追溯 |

---

## 五、G-Gate (GA Gate)

### 5.1 入口条件

- RC Gate 全部通过
- 无阻塞性 OPEN Issue

### 5.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| G1 | 编译 | `cargo build --release` | 无错误 |
| G2 | 391 场景回归 | `gmp-retrieve evaluate --test-set 391_scenarios.json` | ≥ 80% (无退化) |
| G3 | 文档完整性 | `VERSION_DOCS_SPEC.md` 检查 | 必选文档齐全 |
| G4 | 版本一致性 | 文档头部版本号统一为 v1.0.0 | 无旧版本号 |
| G5 | 安全审计 | `cargo audit` | 无高危 |
| G6 | Git Tag | `git tag v1.0.0` | 正确打标 |
| G7 | 发布分支 | `release/v1.0.0` 创建 | 分支保护 |

---

## 六、391 场景通过率标准

### 6.1 判定方法

对每个场景 ID，自动比较 `gmp-retrieve` 输出与 `expected` 答案的语义一致性：

- **正确**: 回答内容语义匹配，关键信息一致
- **部分正确**: 主要内容匹配，缺细节
- **错误**: 回答与 expected 不一致或明显编造
- **无法回答**: 系统明确表示无法回答

**通过率 = (正确 + 部分正确) / 391 × 100%**

### 6.2 通过率目标

| 阶段 | 最低通过率 | 目标通过率 |
|------|-----------|-----------|
| Beta | 60% | ≥ 65% |
| RC | 75% | ≥ 80% |
| GA | 80% | ≥ 80% (无退化) |

### 6.3 启动 A-Gate: 数据准备

**数据准备（5 项）**

| 检查项 | 方法 | 通过标准 |
|--------|------|----------|
| gmp-md 文档 | `ls gmp-md/` | ≥ 1696 文件 |
| agent-knowledge 规则 | `ls agent-knowledge/` | ≥ 55 YAML |
| 391 测试集 | `ls gmp_kg_build/test_results_final/` | 391 条 JSON |
| Ollama 可用 | `curl http://localhost:11434/api/tags` | bge-m3 + qwen2.5 模型已加载 |
| SQLRustGo 可用 | `sqlrustgo --version` | v3.2.0+ 或已集成 |

**功能验证（6 项）**

| 检查项 | 方法 | 通过标准 |
|--------|------|----------|
| FTS 索引 | `gmp-retrieve index --validate fts` | 1696 文档均已索引 |
| 向量索引 | `gmp-retrieve index --validate vector` | HNSW 索引可用 |
| 图存储 | `gmp-retrieve index --validate graph` | 节点 ≥ 700, 边 ≥ 1500 |
| 规则匹配器 | `gmp-retrieve search "偏差" --channel rule --debug` | 命中 ≥ 1 条规则 |
| RRF 融合 | `gmp-retrieve search "CAPA程序" --all --debug` | 多通道结果融合 |
| 端到端 | `gmp-retrieve evaluate --quick --limit 20` | 至少 10/20 结果可解析 |

### 6.4 豁免机制

如特殊情况导致无法达标，需在 `GATE_EXEMPTIONS.md` 申请豁免：
- 知识缺口类（如缺中国 GMP 条文）可豁免，但需记录于测试报告
- 系统 Bug 类不可豁免，必须修复后重新测试