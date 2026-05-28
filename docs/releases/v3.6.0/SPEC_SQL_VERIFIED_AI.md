# SPEC: Evidence-based SQL-Verified AI

## Issue #1466

## 1. Overview

This specification defines the Evidence-based SQL-Verified AI framework for SQLRustGo. The framework provides a structured approach to trace, verify, and validate AI-generated SQL queries through evidence chains, enabling auditable and trustworthy AI-assisted database operations.

## 2. Architecture

### 2.1 Core Components

- **Evidence Enum**: Represents different types of evidence in the verification chain
- **EvidenceChain**: A collection of evidence items with an associated confidence score
- **EvidenceTrace**: A complete trace record containing trace_id, steps, and verdict
- **Verdict Enum**: The verification result (Proven, Partial, Unverifiable, Contradicted)

### 2.2 Evidence Types

| Type | Description |
|------|-------------|
| `SQLQuery` | SQL query string with optional parameters |
| `ToolCall` | External tool invocation evidence |
| `Result` | Query execution result evidence |
| `Document` | Documentation or schema reference |
| `Heuristic` | AI heuristic/rule-based evidence |

## 3. Data Structures

### 3.1 Evidence Enum

```rust
pub enum Evidence {
    SQLQuery { query: String, params: Option<Vec<String>> },
    ToolCall { tool: String, input: String, output: Option<String> },
    Result { rows: Vec<Vec<String>>, row_count: usize },
    Document { doc_type: String, content: String },
    Heuristic { rule: String, confidence: f32 },
}
```

### 3.2 EvidenceChain

```rust
pub struct EvidenceChain {
    pub evidences: Vec<Evidence>,
    pub confidence_score: f32,
}
```

### 3.3 EvidenceTrace

```rust
pub struct EvidenceTrace {
    pub trace_id: String,
    pub steps: Vec<Evidence>,
    pub verdict: Verdict,
}
```

### 3.4 Verdict Enum

```rust
pub enum Verdict {
    Proven,          // Query is fully verified
    Partial,         // Query is partially verified
    Unverifiable,   // Cannot verify the query
    Contradicted,    // Evidence contradicts the query
}
```

## 4. Core Functions

### 4.1 sql_verify

```rust
pub fn sql_verify(query: &str, evidence: &EvidenceChain) -> Verdict
```

Verifies a SQL query against an evidence chain and returns a Verdict.

**Verification Logic:**
- **Proven**: Query matches evidence chain with confidence >= 0.9
- **Partial**: Query has partial evidence match, confidence 0.5-0.9
- **Unverifiable**: No relevant evidence found for the query
- **Contradicted**: Evidence contradicts the query (e.g., result set empty but query expects data)

## 5. Acceptance Criteria

1. ✅ New crate `crates/evidence/` created at workspace root
2. ✅ `Evidence` enum with all 5 variants (SQLQuery, ToolCall, Result, Document, Heuristic)
3. ✅ `EvidenceChain` struct with Vec<Evidence> and confidence_score
4. ✅ `EvidenceTrace` struct with trace_id, steps, and verdict
5. ✅ `Verdict` enum with 4 variants (Proven, Partial, Unverifiable, Contradicted)
6. ✅ `sql_verify(query: &str, evidence: &EvidenceChain) -> Verdict` function
7. ✅ Added to Cargo workspace members
8. ✅ Compiles without errors
9. ✅ Tests pass
10. ✅ SPEC document created at `docs/releases/v3.6.0/SPEC_SQL_VERIFIED_AI.md`

## 6. File Structure

```
crates/evidence/
├── Cargo.toml
└── src/
    ├── lib.rs         # Evidence, EvidenceChain, Verdict, sql_verify
    └── trace.rs       # EvidenceTrace struct
```

## 7. Dependencies

- `serde`: Serialization support
- `thiserror`: Error handling
- `uuid`: Trace ID generation
- `chrono`: Timestamp support
