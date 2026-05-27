# Ollama Vectorizer Specification

## Overview

Ollama integration for generating text embeddings using local LLM models.

## Functionality

### Core Features

1. **Embedding Generation**
   - Support `nomic-embed-text` model (768 dimensions)
   - Support `mxbai-embed-large` model (1024 dimensions)

2. **Batch Embedding**
   - Process multiple texts in single request
   - Max batch size: 100 texts

3. **Caching**
   - LRU cache for recent embeddings
   - Cache size: 10,000 entries

### API

```rust
pub struct EmbeddingRequest {
    pub texts: Vec<String>,
    pub model: String,
}

pub struct EmbeddingResult {
    pub embeddings: Vec<Vec<f32>>,
    pub model: String,
    pub dimension: usize,
}

pub async fn generate_embeddings(
    client: &OllamaClient,
    request: &EmbeddingRequest,
) -> Result<EmbeddingResult, VectorizerError>;
```

### Performance

- Single embedding latency < 50ms
- Batch embedding: ~1ms per text
- Cache hit ratio target: > 80%

### Error Handling

- Model not found → return error with available models
- Ollama unavailable → return error, do not fallback
- Invalid input → return error with details
