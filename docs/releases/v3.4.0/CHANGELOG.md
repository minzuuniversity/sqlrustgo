# v3.4.0 Changelog

> **Version**: 3.4.0
> **Date**: 2026-05-24
> **Status**: GA (正式发布)
> **Branch**: `main`
> **HEAD**: `d934228b` (RC coverage threshold fix)
> **GA from**: `d934228b` (executor stored proc coverage tests)

---

## v3.4.0 (2026-05-24) - GA

### Added

#### GMP Management API (P0) — `sqlrustgo-gmp-api`

**EBR Core — Electronic Batch Record**
- `BatchManager` — Batch lifecycle management (Created → InProgress → PendingQA → Released → Archived)
- Batch CRUD operations with audit trail
- Deviation management (OnHold / Rejected)
- PR: #1260

**Electronic Signature Service**
- `SignatureService` — 21 CFR Part 11 compliant electronic signatures
- `ApprovalChain` — Multi-signature approval workflows
- Signature verification and audit trail
- PR: #1261

**Audit API**
- `AuditService` — Audit chain management
- `AuditChain` — SHA-256 hash chain for audit records
- Electronic signature integration
- PR: #1260

**Device API**
- `DeviceService` — OPC UA device integration
- `OpcUaClient` — OPC UA client for device data acquisition
- Real-time device monitoring
- PR: #1262

**Rule Engine Visual Editor**
- `RuleEditorService` — Visual compliance rule configuration
- `RuleEvaluator` — Rule evaluation engine
- Pre-built GMP compliance rules
- PR: #1263

**Dashboard API**
- `DashboardService` — Compliance score, audit chains, batches overview
- Trust metrics visualization
- Real-time alert push via WebSocket

**Export API**
- JSON/PDF audit package export
- Evidence chain export

#### GMP Retrieval v2 (P0) — BM25 + RRF + Reranker + LLM Chat

**BM25 Full-Text Search**
- `Bm25Searcher` — Okapi BM25 ranking algorithm
- Keyword and phrase search
- Relevance scoring

**RRF Fusion**
- `RrfFusion` — Reciprocal Rank Fusion for multi-channel result merging
- Combines Rule + Vector + FTS + Graph channels
- Configurable fusion parameters

**Ollama Reranker**
- `OllamaReranker` — LLM-based result reranking
- Integration with Ollama `qwen3-reranker` model
- Contextual relevance scoring

**Ollama LLM Chat**
- `OllamaChat` — LLM-powered GMP compliance Q&A
- RAG (Retrieval-Augmented Generation) pipeline
- Integration with Ollama `qwen3:8b` model

**GMP Retrieval CLI**
- `gmp search` — Hybrid search command
- `gmp chat` — LLM-powered Q&A command
- `gmp index` — Document indexing command

PR: #1297

#### Trust Visualization CLI

**Trust Visualization Module**
- `TrustStatusCli` — Trust status monitoring tool
- `ComplianceDashboard` — Compliance score dashboard
- `EvidenceChainViz` — Audit chain visualization
- Builder pattern implementation

---

### Changed

#### `sqlrustgo-gmp`

**Workflow V2**
- Extended `WorkflowState` types with `#[derive(Default)]`
- Enhanced `map_identity` removal in workflow state management
- Integration tests expanded to 470 test cases

**Trust Visualization**
- Added `TrustMetrics` struct with builder pattern
- `ComplianceScore` calculation improvements

#### `sqlrustgo-mysql-server`

**SELECT Built-in Functions**
- Extended function support for SELECT statements
- Enhanced SQL compatibility

---

### Fixed

**Parser OOM — StringLiteral/BooleanLiteral token consumption**
- `parse_select_statement`: StringLiteral branch missing `self.next()` caused infinite loop accumulating Expression objects until OOM (80GB+)
- `parse_select_statement`: BooleanLiteral branch missing `self.next()` (same issue)
- Added `MAX_RECURSION_DEPTH=64` guard to `parse_or_expression` and `parse_and_expression`
- PR: #1329

**Workflow State**
- `#[derive(Default)]` fix for `WorkflowState`
- Removed `map_identity` in workflow state handling
- PR: #1303

---

### Dependencies

| Crate | Change | Version |
|-------|--------|---------|
| `sqlrustgo-gmp-api` | New | 0.1.0 |
| `sqlrustgo-gmp-retrieval` | New | — |

---

## v3.3.0 (2026-05-20) — Industrial Trust Platform GA

See [v3.3.0 Changelog](./v3.3.0/CHANGELOG.md) for complete history.

---

## v3.2.0 (2026-05-16) — GMP Framework Core

See [v3.2.0 Changelog](./v3.2.0/CHANGELOG.md) for complete history.
