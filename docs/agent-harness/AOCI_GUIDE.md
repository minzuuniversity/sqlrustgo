# AOCI Guide

> **基线**: HEAD `583050bc63`
> **索引状态**: `aoci.code.txt` 1.4MB / 7080 行, `aoci.meta.txt` v2 protocol FRAS-2
> **Baseline**: `.aoci/baseline.json` 追踪 5926 文件 (5011 index + 915 observe)
> **最近更新**: 2026-10-09T17:22:16Z — 落后 HEAD ~1 周

## AOCI 是什么

AOCI (Repository Cognition Object, v2 protocol) 是仓库级语义索引, 给每个托管对象 (代码文件、数据库表等) 分配独立 Entry + 5 元组标签:

```
path[ABCD-Tag]: F:<核心职责> | R:<重要关系> | A:<架构层> | S:<非显然约束/历史>
```

| 标签 | 含义 |
|---|---|
| A (Layer) | C-SharedFoundation / E-EntryBoundary / A-ApplicationOrchestration / D-DomainLogic / K-AlgorithmComputation / M-Middleware / P-Persistence / I-IntegrationAdapter / R-RuntimeFoundation / L-LibrarySDK / F-DeclarativeConfiguration / O-OperationsDelivery / T-TestValidation / S-DocumentationSpecification / X-DevelopmentTooling / Z-Other |
| B (Module) | G-CrossDomain / U-UserInteraction / B-CoreBusiness / D-DataState / I-IdentityAccess / N-NetworkProtocol / M-MessageEvent / S-SecurityPrivacy / C-ConfigurationPolicy / O-Observability / R-ReliabilityRecovery / P-PerformanceResource / W-WorkflowScheduling / A-AnalyticsIntelligence / H-HardwareDevice / L-Localization / V-BuildRelease / Q-QualityAssurance / E-ExtensionPlugin / Z-Other |
| C (Importance) | 9-highest ~ 1-lowest |
| E (Scale) | L-large>400 / M-medium200-400 / S-small100-200 / T-tiny<100 行 |

## AOCI 知识分工 (与 GitNexus 互补)

| 知识类型 | 推荐权威来源 |
|---|---|
| 当前函数实现 | Git HEAD 源码 |
| 类型/Trait/符号引用 | rust-analyzer / 源码 |
| 调用与依赖图 | **GitNexus**, 辅以源码验证 |
| 模块职责 + 设计解释 | **AOCI** (F/R/A/S) |
| 长期语义认知 | **AOCI** |
| 测试是否通过 | 实际测试报告 (cargo test output) |
| 架构规范与决策 | ADR / 设计文档 |
| 当前开发状态 | Git / Issue / PR / CI |

## AOCI 工作流 (简版)

> 完整协议见根 `AGENTS.md` 第 253-355 行 (AOCI Repository Cognition 章节, 由 model 维护)
> **不要**从文件/正则/AST 推导语义 — 必须 model 阅读实际证据

### 启动 (Begin)
1. 仓库已有 `aoci.code.txt`? — **是** (1.4MB, 7080 行)
2. 检查 baseline.json `updated_at` — 2026-10-09 (落后 HEAD 6 天)
3. 调用 `aoci_rules` 拿当前 session contract
4. 调用 `aoci_overview` (或 `check_only=true` 查 checkpoint)
5. **context_compaction 强制重载** (mandatory reload rule)

### 任务中
- 查询单个对象: `aoci_query_entry(path)` 或读 `aoci.code.txt`
- 任务完成 → `aoci_maintain` 检查是否需更新

### 任务结束 (Close)
- 变更 AOCI 托管对象 → `aoci_maintain` 一次 (不在中间每次编辑后)
- 拿到 candidates → 由 model 基于证据独立 author F/R/A/S 更新
- 一批 → `aoci_update_entry` (保留 source_sha256, candidate_id, batch identity)
- 证据不足 → `aoci_report` (不要猜)

## AOCI 关键约束 (不要违反)

- ❌ 用 paths / filenames / extensions / AST / regex / template 生成语义
- ❌ 写 `origin=host_model`
- ❌ 伪造 receipt
- ❌ context_compaction 后用历史 Memory 补 (必须 fresh `aoci_overview`)
- ❌ shrink Index coverage or slice batch 满足 transport 限制
- ❌ 单个 object 不一致 retry — 重新拿 batch
- ✅ Model 必须独立 author 完整 tag + F/R/A/S
- ✅ 模型 cognition + 治理 alignment 分开报告

## 当前 AOCI 索引观察

来自 `aoci.code.txt` 第 1-100 行抽样:

- `.gitattributes[EG4T]`: F=Routes Git LFS filters | S=Dropping LFS line silently breaks TPC-H/SOAK fixtures
- `AGENTS.md[SB4M]`: F=Documents AGENTS.md | S=Cites issues #3600 whose state may have moved
- `ARCHITECTURE_RULES.md[SB4M]`: F=Documents AI 协作安全规则 | S=Written against v1.2.0; version claims inside age with the branch
- `BRANCH_GOVERNANCE.md[SB4S]`: S=Written against v1.2.0; version claims age
- `CHANGELOG.md[SB4L]`: S=Cites issues #100, #101, #102, #103, #1166 whose state may have moved
- `README.md[SB4M]`: S=Cites issues #4550, #4846, #4847, #4848 whose state may have moved
- `SQLRUSTGO_210_ANALYSIS_REPORT.md[SB4M]`: S=Cites issues #1022, #1023, ...
- `TPCH_BENCHMARK_REPORT.md[SB4S]`: S=Written against v1.6.0; **OUTDATED**
- `TPC-H-v3.0.md[SB4M]`: S=Cites issues #3431, #3550, ...
- `VERSION[DB4T]`: S=Repository asset enforced by git/CI

→ **结论**: AOCI 索引自身标注了大量"过时"风险, 必须以 commit + 文档复核为准。

## AOCI 失效降级

AOCI 不可用时:
1. 读 `aoci.code.txt` 直接 (file-based fallback)
2. 用 grep + docs/architecture/ 推导模块职责
3. 不要重写整个索引 — 只更新差异