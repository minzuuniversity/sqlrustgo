# ADR-012: SQLRustGo vs GMP-Platform Scope Boundary

> **Version**: v3.8.0
> **Date**: 2026-06-03
> **Author**: Hermes Agent
> **Status**: ACCEPTED
> **Related**: v3.5.0 DEV_PLAN.md (AI Native GMP Platform)

## 1. Context

v3.5.0 DEV_PLAN.md defines 10 P0/P1/P2 features under the theme "AI Native GMP
Platform" (AI 偏差调查助手, LLM 合规判断, GMP Retrieval v3, 本地 LLM, SSE, 预测性
维护, NL 报表, 审计摘要, 规则推荐, 多模态/语音/培训助手).

When auditing v3.8.0 against v3.5.0 feature list, the audit reveals **0/10
implemented in v3.8.0 codebase**. This ADR clarifies whether this is a gap or
a scope boundary issue.

## 2. Decision

**The v3.5.0 AI Native GMP features belong to the GMP-Platform repository,
NOT the SQLRustGo database repository.**

### 2.1 Repository Architecture

```
┌────────────────────────────────────────────────────────────┐
│  GMP-Platform Repository (gitea/openclaw/gmp-platform)      │
│  ├─ GMP native applications                                │
│  ├─ AI agents (LLM, retrieval, recommendation)             │
│  ├─ GMP workflows, EBR, signatures                          │
│  └─ HTTP API (REST + SSE)                                  │
│           ↑↑↑ (consumes)                                    │
└──────────┼──────────────────────────────────────────────────┘
           │
┌──────────┼──────────────────────────────────────────────────┐
│  SQLRustGo Repository (gitea/openclaw/sqlrustgo)           │
│  ├─ MySQL-compatible SQL engine                             │
│  ├─ Storage + WAL + MVCC                                    │
│  ├─ Network protocol server (MySQL wire)                    │
│  └─ Exposes SQL API                                          │
└────────────────────────────────────────────────────────────┘
```

### 2.2 Feature Classification

| Feature Category | Repository | v3.5.0 Example |
|------------------|-----------|----------------|
| **SQL Engine** | SQLRustGo | Parser, executor, optimizer, storage |
| **WAL/Transaction** | SQLRustGo | WAL manager, MVCC, recovery |
| **MySQL Protocol** | SQLRustGo | mysql-server crate |
| **AI Agents** | GMP-Platform | LLM judge, retrieval, summarization |
| **GMP Workflows** | GMP-Platform | EBR, signatures, batch release |
| **GMP API** | GMP-Platform | REST endpoints, SSE |

## 3. v3.5.0 Feature Classification

| ID | Feature | Correct Repository | v3.5.0 DEV_PLAN location |
|----|---------|--------------------|-----------------------------|
| v350-01 | AI 偏差调查助手 | GMP-Platform | TODO (in DEV_PLAN.md) |
| v350-02 | LLM 合规判断引擎 | GMP-Platform | TODO |
| v350-03 | GMP Retrieval v3 集成 | GMP-Platform | TODO |
| v350-04 | 本地 LLM 推理支持 | GMP-Platform | TODO |
| v350-05 | AI 流式输出 SSE | GMP-Platform | TODO |
| v350-06 | 预测性设备维护 | GMP-Platform | TODO |
| v350-07 | 自然语言报表生成 | GMP-Platform | TODO |
| v350-08 | 审计链 AI 摘要 | GMP-Platform | TODO |
| v350-09 | 规则自动推荐 | GMP-Platform | TODO |
| v350-10 | 多模态/语音/培训助手 | GMP-Platform | TODO |

**0/10 belong in SQLRustGo. 10/10 belong in GMP-Platform.**

## 4. Implications for v3.8.0 Audit

### 4.1 v3.5.0 audit result for SQLRustGo

**Correct interpretation**: 0/10 missing, NOT 0/10 implemented. The 0%
implementation rate in SQLRustGo is the **correct** state because the features
belong to GMP-Platform.

### 4.2 v3.5.0 audit (if performed on GMP-Platform) would show:

- ❌ All 10 features still TODO in v3.5.0 GA
- ❌ None of the 10 features have SPEC/TEST_PLAN/TEST_DESIGN in v3.5.0
- ❌ None of the 10 features have gate integration

This is the **actual** AI Native GMP delivery status, but it lives in
GMP-Platform, not SQLRustGo.

## 5. What SQLRustGo v3.8.0 Should Audit

For v3.5.0, SQLRustGo v3.8.0 should only audit features that touch the SQL
engine or storage:

| v3.5.0 SQLRustGo-relevant feature | v3.8.0 status |
|-----------------------------------|----------------|
| (None - v3.5.0 was a GMP feature release, not a SQL engine release) | N/A |

**v3.5.0 did not introduce new SQL engine features** (per v3.5.0 DEV_PLAN.md
which is 100% GMP application layer).

## 6. Consequences

### 6.1 Positive

- v3.8.0 audit matrix (HISTORICAL_FEATURE_COVERAGE_MATRIX.md) correctly
  excludes GMP-Platform features
- No false positive gap reports for v3.5.0

### 6.2 Negative

- v3.5.0 actual AI Native delivery status is **unknown** (would need to
  audit GMP-Platform separately)
- Cross-repo integration tests are limited to API contract verification

## 7. References

- v3.5.0 DEV_PLAN.md (AI Native GMP Platform strategy)
- v3.2.0 DEVELOPMENT_PLAN.md (GMP Native Data Platform predecessor)
- gitea/openclaw/gmp-platform repository (separate scope)
- ADR-007 (WAL architecture clarification)
- ADR-008 (Cross-version debt governance)
- ADR-010 (Cross-version debt governance)
- ADR-011 (v3.8.0+1 TX-WAL repair)
