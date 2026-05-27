# v3.3.0 Changelog

> **状态**: GA
> **分支**: `origin/develop/v3.3.0`
> **GA Tag**: `a1ddb44c`
> **日期**: 2026-05-20

---

## [3.3.0] - 2026-05-20 (GA)

### 目标

Industrial Trust Platform（工业级可信闭环平台），建立 GMP 工业级可信闭环。

### 战略演进

```
v3.2.0: Trust Convergence（可信收敛）✅ GA
    ↓
v3.3.0: Industrial Trust Platform（可信内核）✅ GA
    ↓
v3.4.0: GMP Management Suite（管理套件）✅ GA
    ↓
v3.5.0: AI Native GMP Platform
```

### 核心功能

**Trust Infrastructure 四层架构**：

| 层级 | 组件 | PR |
|------|------|-----|
| Trust Infra Kernel | Performance Governance | #1259 |
| Trust Infra Kernel | Crash Simulation | #1265 |
| Trust Infra Kernel | WAL Formal Verification | #1266 |
| Compliance Evidence | Evidence Engine | #1260 |
| Compliance Evidence | Audit API | #1260 |
| Provenance Graph | Provenance Graph | #1267 |
| Trust Visualization | Trust Viz CLI | #1268 |
| GMP Retrieval v1 | BM25 + RRF + Ollama Reranker | #1257 |

### 已完成功能

| 功能 | PR | 状态 |
|------|-----|------|
| GMP Retrieval v1 (BM25+RRF+Reranker) | #1257 | ✅ |
| Graph Generic Execute Cypher | #1327 | ✅ |
| Executor 覆盖率 76.44% → 82.88% | #1324 | ✅ |
| Trust Infrastructure 四层 | #1259-#1268 | ✅ |

### 性能数据

| 操作 | v3.3.0 实测 | v3.2.0 基线 | 状态 |
|------|-----------|------------|------|
| Simple SELECT | ~20,000 QPS | 11,435 QPS | ✅ |
| UPDATE | ~12,000 QPS | 11,807 QPS | ✅ |
| DELETE | ~20,000 QPS | 14,383 QPS | ✅ |
| TPC-H SF=0.1 | 22/22 PASS | — | ✅ |

### 门禁状态

| Gate | 结果 | 日期 |
|------|------|------|
| Alpha (16/16) | ✅ PASS | 2026-05-17 |
| Beta (14/14) | ✅ PASS | 2026-05-18 |
| RC | ✅ PASS | 2026-05-20 |
| GA | ✅ PASS | 2026-05-20 |

---

## [3.2.0] - 2026-05-18 (GA)

*(参见 docs/releases/v3.2.0/CHANGELOG.md)*
