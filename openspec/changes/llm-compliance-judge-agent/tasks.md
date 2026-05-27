## 1. Project Setup

- [ ] 1.1 Create new crate `sqlrustgo-compliance-judge` in `crates/`
- [ ] 1.2 Add dependencies: `reqwest`, `serde`, `tokio`, `tracing`
- [ ] 1.3 Create module structure: `agent/`, `rules/`, `audit/`, `ollama/`, `confidence/`

## 2. Ollama Integration (`ollama-integration`)

- [ ] 2.1 Define `OllamaClient` trait with `chat()` and `chat_stream()` methods
- [ ] 2.2 Implement `OllamaClientImpl` using reqwest HTTP client
- [ ] 2.3 Add error types: `OllamaConnectionError`, `ModelNotFoundError`
- [ ] 2.4 Implement SSE streaming parser for chat_stream
- [ ] 2.5 Write unit tests with mock OllamaClient

## 3. Compliance Rule Parser (`compliance-rule-parser`)

- [ ] 3.1 Define `Rule` struct with fields: id, text, severity, language, category
- [ ] 3.2 Implement `RuleParser` with YAML/JSON parsing support
- [ ] 3.3 Implement `RuleValidator` for completeness checking
- [ ] 3.4 Write unit tests for parsing edge cases

## 4. Confidence Scoring (`confidence-scoring`)

- [ ] 4.1 Define `ConfidenceScore` struct with score (0.0-1.0) and explanation
- [ ] 4.2 Implement `ConfidenceCalculator` using logprob analysis
- [ ] 4.3 Set threshold constant (0.7) for NEEDS_REVIEW status
- [ ] 4.4 Write unit tests for confidence calculation

## 5. Compliance Judge Agent (`compliance-judge-agent`)

- [ ] 5.1 Define `JudgmentRequest` and `JudgmentResult` structs
- [ ] 5.2 Define `JudgmentStatus` enum: COMPLIANT, NON_COMPLIANT, NEEDS_REVIEW, TIMEOUT
- [ ] 5.3 Implement `ComplianceJudge::judge()` method with 500ms timeout
- [ ] 5.4 Implement prompt construction for LLM
- [ ] 5.5 Implement response parsing into JudgmentResult
- [ ] 5.6 Write integration tests with mock Ollama

## 6. Audit Log (`compliance-audit-log`)

- [ ] 6.1 Define `AuditLogEntry` struct with all required fields
- [ ] 6.2 Implement `AuditLogWriter` using WAL persistence
- [ ] 6.3 Implement hash chain verification for integrity
- [ ] 6.4 Implement `AuditLogReader` for querying by operation_id and time range
- [ ] 6.5 Write integration tests for crash recovery

## 7. API Integration

- [ ] 7.1 Add ComplianceJudge service to main application
- [ ] 7.2 Create HTTP endpoint `POST /api/v1/compliance/judge`
- [ ] 7.3 Create HTTP endpoint `GET /api/v1/compliance/audit/{operation_id}`
- [ ] 7.4 Write API integration tests

## 8. Integration with GMP Module

- [ ] 8.1 Integrate ComplianceJudge with existing `sqlrustgo-gmp` module
- [ ] 8.2 Add GMP event hooks for automatic compliance checking
- [ ] 8.3 Write end-to-end tests with real Ollama (if available)

## 9. Documentation

- [ ] 9.1 Add module-level documentation
- [ ] 9.2 Update project README with new capability
- [ ] 9.3 Add API documentation for endpoints
