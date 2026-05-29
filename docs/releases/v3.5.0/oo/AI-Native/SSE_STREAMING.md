# SSE Streaming

> **Version**: 1.0
> **Date**: 2026-05-26
> **Issue**: #1365
> **Status**: Planning

## 一、功能描述

Server-Sent Events 流式输出，实时显示 AI 推理过程。

## 二、技术方案

### 2.1 SSE 协议

```
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive

event: token
data: {"token": "今"}

event: token
data: {"token": "天"}

event: done
data: {"total_tokens": 150}
```

### 2.2 Rust 实现

```rust
use tokio_stream::StreamExt;

pub async fn stream_chat(
    client: &OllamaClient,
    req: ChatRequest,
) -> impl Stream<Item = SSEEvent> {
    let mut stream = client.chat_stream(req).await?;
    
    stream.map(|chunk| SSEEvent {
        event: "token".into(),
        data: json!({ "token": chunk.token }),
    })
}
```

## 三、验收标准

- 首 token 延迟 < 100ms
- 支持中断取消
- 支持重连

## 四、前端集成

```javascript
const eventSource = new EventSource('/api/v1/gmp/chat/stream', {
    events: ['token', 'done', 'error']
});

eventSource.addEventListener('token', (e) => {
    const data = JSON.parse(e.data);
    appendToken(data.token);
});
```
