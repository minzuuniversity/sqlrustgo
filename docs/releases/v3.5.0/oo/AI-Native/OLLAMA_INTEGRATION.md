# Ollama Integration

> **Version**: 1.0
> **Date**: 2026-05-26
> **Issue**: #1364
> **Status**: Planning

## 一、功能描述

本地 LLM 推理支持，支持 Ollama/bge-m3 本地推理，延迟 < 2s。

## 二、支持的模型

| 模型 | 用途 | 参数量 |
|------|------|--------|
| qwen2.5:7b | 通用对话、报告生成 | 7B |
| llama3 | 通用对话 | 8B |
| bge-m3 | 文本向量化 | 567M |

## 三、API 接口

### 3.1 Chat

```rust
pub async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
    let response = self.client.chat(req).await?;
    Ok(response)
}
```

### 3.2 Embeddings

```rust
pub async fn embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
    let response = self.client.embeddings(texts).await?;
    Ok(response.embeddings)
}
```

## 四、验收标准

- 支持 chat/embeddings
- 模型自动缓存
- 延迟 < 2s

## 五、部署

```yaml
# docker-compose.yml
ollama:
  image: ollama/ollama:latest
  ports:
    - "11434:11434"
  volumes:
    - ollama_data:/root/.ollama
```
