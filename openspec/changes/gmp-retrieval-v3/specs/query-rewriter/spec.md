# Query Rewriter Specification

## Overview

LLM-powered query rewriting to convert natural language queries into structured retrieval queries.

## Functionality

### Core Features

1. **Query Classification**
   - Detect intent: search, filter, comparison, aggregation
   - Extract entities and concepts

2. **Query Expansion**
   - Add synonyms based on domain knowledge
   - Expand abbreviations

3. **Structured Output**
   - Output JSON format with fields: `keywords`, `filters`, `intent`

### API

```rust
pub struct RewrittenQuery {
    pub original: String,
    pub keywords: Vec<String>,
    pub filters: Vec<FilterCondition>,
    pub intent: QueryIntent,
}

pub async fn rewrite_query(
    client: &OllamaClient,
    query: &str,
    model: &str,
) -> Result<RewrittenQuery, RewriterError>;
```

### Example

Input: "找一下最近一周批准的偏差报告"

Output:
```json
{
  "keywords": ["偏差", "报告", "批准"],
  "filters": [
    {"field": "status", "op": "eq", "value": "approved"},
    {"field": "date", "op": "gte", "value": "2026-05-19"}
  ],
  "intent": "search"
}
```

### Performance

- Rewrite latency < 100ms
- Timeout: 500ms with fallback to original query

### Error Handling

- LLM timeout → return original query unchanged
- Invalid JSON → return original query with warning
