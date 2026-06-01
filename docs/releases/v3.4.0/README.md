# v3.4.0 Release — GMP Management Suite

> **状态**: GA (正式发布)
> **起点**: develop/v3.4.0 (commit `b72c640a` from v3.3.0 GA)
> **分支**: `main` (commit `d934228b`)
> **Milestone**: v3.4.0 (id=TBD)
> **发布完成**: 2026-05-24

---

## 一、版本目标

**战略定位**: GMP Management Suite（管理套件），在 v3.3.0 可信内核基础上构建完整的管理套件。

### 核心价值

- **可信内核 → 可信产品**：将 v3.3.0 的可信闭环能力产品化
- **合规人员友好**：通过 Web API 完成合规操作
- **审计即服务**：一键导出审计报告，降低第三方审计成本

### 战略演进

```
v3.2.0: Trust Convergence（可信收敛）✅ GA
v3.3.0: Industrial Trust Platform（可信内核）✅ GA
v3.4.0: GMP Management Suite（管理套件）← 当前 RC
v3.5.0: AI Native GMP Platform
```

---

## 二、门禁状态

### Alpha Gate ✅ 通过 (2026-05-22)

| # | 检查项 | 结果 |
|---|--------|------|
| A1 | Build | ✅ PASS |
| A2 | Unit Tests | ✅ PASS |
| A3 | Clippy | ✅ PASS |
| A4 | Format | ✅ PASS |
| A5 | Coverage (≥70%) | ✅ PASS (76.44%) |
| A6 | MySQL Protocol | ✅ PASS |
| A7 | TPC-H SF=0.1 | ✅ PASS (22/22) |
| TI-1~5 | Trust Infrastructure | ✅ PASS |

### Beta Gate ✅ 通过 (2026-05-22)

| # | 检查项 | 结果 |
|---|--------|------|
| B1 | Build | ✅ PASS |
| B2 | Unit Tests | ✅ PASS |
| B3 | Clippy | ✅ PASS |
| B4 | Format | ✅ PASS |
| B5~B14 | GMP API / Retrieval / Trust | ✅ PASS |

### RC Gate ✅ 进行中

| # | 检查项 | 状态 |
|---|--------|------|
| R1 | Build | ✅ PASS |
| R2 | Test --all-features --workspace | ⏳ Gitea Actions |
| R3 | Clippy | ✅ PASS |
| R4 | Format | ✅ PASS |
| R5 | Coverage ≥85% (L1) | ⚠️ 73.36% < 85% |
| R6 | Security (cargo audit) | ✅ PASS |
| R7 | SQL Compat | ✅ PASS |
| R8 | TPC-H SF=1 | ✅ PASS (22/22) |
| R-S1~S4 | Stability (16h/24h) | ⏳ Z6G4 |

详见: [RC_GATE_CHECKLIST.md](RC_GATE_CHECKLIST.md)

---

## 三、核心功能

### GMP Management API (`sqlrustgo-gmp-api`)

| 模块 | 功能 | PR |
|------|------|-----|
| EBR Core | Batch Lifecycle Management | #1260 |
| Signature | 21 CFR Part 11 Electronic Signature | #1261 |
| Audit | Audit Chain + Electronic Signature | #1260 |
| Device | OPC UA Device Integration | #1262 |
| Rule Editor | Visual Compliance Rule Configuration | #1263 |
| Dashboard | Compliance Score + WebSocket Alerts | - |
| Export | JSON/PDF Audit Package Export | - |

### GMP Retrieval v2 (`sqlrustgo-gmp-retrieval`)

| 模块 | 功能 | 说明 |
|------|------|------|
| BM25 | Okapi BM25 ranking | 关键词/短语搜索 |
| RRF Fusion | Reciprocal Rank Fusion | 四通道检索合并 |
| Ollama Reranker | LLM-based reranking | qwen3-reranker |
| Ollama Chat | RAG pipeline | qwen3:8b |
| GMP CLI | `gmp search/chat/index` | 命令行工具 |

### Trust Visualization

| 模块 | 功能 |
|------|------|
| TrustStatusCli | Trust status monitoring |
| ComplianceDashboard | Compliance score dashboard |
| EvidenceChainViz | Audit chain visualization |

---

## 四、Executor 覆盖率

| 阶段 | 目标 | 实际 |
|------|------|------|
| Alpha | ≥70% | 76.44% ✅ |
| Beta | ≥80% | - |
| GA | ≥85% | - |

详见: [docs/governance/gate_spec_v340.md](../governance/gate_spec_v340.md)

---

## 五、文档索引

| 文档 | 说明 |
|------|------|
| [CHANGELOG.md](CHANGELOG.md) | v3.4.0 详细变更 |
| [DEV_PLAN.md](DEV_PLAN.md) | 开发测试计划 |
| [ALPHA_GATE_CHECKLIST.md](ALPHA_GATE_CHECKLIST.md) | Alpha 门禁清单 |
| [BETA_GATE_CHECKLIST.md](BETA_GATE_CHECKLIST.md) | Beta 门禁清单 |
| [RC_GATE_CHECKLIST.md](RC_GATE_CHECKLIST.md) | RC 门禁清单 |
| [GA_GATE_CHECKLIST.md](GA_GATE_CHECKLIST.md) | GA 门禁清单 |
| [GA_GATE_REPORT.md](GA_GATE_REPORT.md) | GA 门禁报告 |
| [LEGACY_ISSUES.md](LEGACY_ISSUES.md) | 遗留问题追踪 |
| [oo/OO_ROADMAP.md](oo/OO_ROADMAP.md) | OO 文档路线图 |
| [TEST_SYSTEM_ANALYSIS.md](TEST_SYSTEM_ANALYSIS.md) | 测试系统分析 |

---

## 六、历史版本

- [v3.3.0 GA](../v3.3.0/README.md) — Industrial Trust Platform
- [v3.2.0 GA](../v3.2.0/README.md) — Trust Convergence
- [v3.1.0 Beta](../v3.1.0/README.md) — GMP Framework Core
