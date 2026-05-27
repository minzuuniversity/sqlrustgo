## 1. Project Setup

- [x] 1.1 Create new crate `sqlrustgo-compliance-judge` in `crates/`
- [x] 1.2 Add dependencies: `reqwest`, `serde`, `tokio`, `tracing`
- [x] 1.3 Create module structure: `agent/`, `rules/`, `audit/`, `ollama/`, `confidence/`

## 2. Ollama Integration (`ollama-integration`)

- [x] 2.1 Define `OllamaClient` trait with `chat()` and `chat_stream()` methods
- [x] 2.2 Implement `OllamaClientImpl` using reqwest HTTP client
- [x] 2.3 Add error types: `OllamaConnectionError`, `ModelNotFoundError`
- [x] 2.4 Implement SSE streaming parser for chat_stream
- [x] 2.5 Write unit tests with mock OllamaClient

## 3. Compliance Rule Parser (`compliance-rule-parser`)

- [x] 3.1 Define `Rule` struct with fields: id, text, severity, language, category
- [x] 3.2 Implement `RuleParser` with YAML/JSON parsing support
- [x] 3.3 Implement `RuleValidator` for completeness checking
- [x] 3.4 Write unit tests for parsing edge cases

## 4. Confidence Scoring (`confidence-scoring`)

- [x] 4.1 Define `ConfidenceScore` struct with score (0.0-1.0) and explanation
- [x] 4.2 Implement `ConfidenceCalculator` using logprob analysis
- [x] 4.3 Set threshold constant (0.7) for NEEDS_REVIEW status
- [x] 4.4 Write unit tests for confidence calculation

## 5. Compliance Judge Agent (`compliance-judge-agent`)

- [x] 5.1 Define `JudgmentRequest` and `JudgmentResult` structs
- [x] 5.2 Define `JudgmentStatus` enum: COMPLIANT, NON_COMPLIANT, NEEDS_REVIEW, TIMEOUT
- [x] 5.3 Implement `ComplianceJudge::judge()` method with 500ms timeout
- [x] 5.4 Implement prompt construction for LLM
- [x] 5.5 Implement response parsing into JudgmentResult
- [x] 5.6 Write integration tests with mock Ollama

## 6. Audit Log (`compliance-audit-log`)

- [x] 6.1 Define `AuditLogEntry` struct with all required fields
- [x] 6.2 Implement `AuditLogWriter` using WAL persistence
- [x] 6.3 Implement hash chain verification for integrity
- [x] 6.4 Implement `AuditLogReader` for querying by operation_id and time range
- [x] 6.5 Write integration tests for crash recovery

## 7. API Integration

- [x] 7.1 Add ComplianceJudge service to main application
- [x] 7.2 Create HTTP endpoint `POST /api/v1/compliance/judge`
- [x] 7.3 Create HTTP endpoint `GET /api/v1/compliance/audit/{operation_id}`
- [x] 7.4 Write API integration tests

## 8. Integration with GMP Module

- [x] 8.1 Integrate ComplianceJudge with existing `sqlrustgo-gmp` module
- [x] 8.2 Add GMP event hooks for automatic compliance checking
- [x] 8.3 Write end-to-end tests with real Ollama (if available)

## 9. Documentation

- [x] 9.1 Add module-level documentation
- [x] 9.2 Update project README with new capability
- [x] 9.3 Add API documentation for endpoints
