# Issue #1365: AI 流式输出 SSE 实现规范

> **Issue**: #1365  
> **版本**: v3.5.0 P0  
> **创建日期**: 2026-05-26  
> **状态**: OPEN（待实现）  
> **验收标准**: 首字节延迟 < 100ms，支持中断取消  

---

## 一、目标

在 GMP API Server 中新增 SSE（Server-Sent Events）端点，支持 LLM 流式输出，使 AI 推理过程可实时展示给用户。

## 二、架构设计

### 2.1 端点设计

```
POST /api/v1/ai/chat/stream
```

**请求体**:
```json
{
  "messages": [
    {"role": "system", "content": "..."},
    {"role": "user", "content": "请分析这批次的偏差情况"}
  ],
  "model": "qwen2.5:14b",
  "stream": true,
  "contexts": [
    {"doc_id": "doc-001", "title": "偏差报告", "content": "...", "score": 0.95, "source": "vector"}
  ]
}
```

**响应**: `text/event-stream` 类型，每个事件格式：
```
data: {"content": "正在", "done": false}
data: {"content": "分析", "done": false}
data: {"content": "...", "done": true, "final": true}
```

**Error 事件**:
```
event: error
data: {"code": "LLM_UNAVAILABLE", "message": "Ollama 服务未启动"}
```

### 2.2 核心组件

| 组件 | 职责 | 文件 |
|------|------|------|
| `StreamChatRequest/Response` | 流式请求/响应 DTO | `crates/llm/src/stream.rs` |
| `LlmClient::chat_stream()` | 流式 chat trait 方法 | `crates/llm/src/lib.rs` |
| `OllamaLlmClient::chat_stream()` | Ollama 流式实现 | `crates/llm/src/ollama_stream.rs` |
| `chat_stream_endpoint()` | SSE HTTP 处理器 | `crates/server/src/ai_endpoints.rs` (新建) |
| `CancellationToken` | 中断取消支持 | `crates/llm/src/cancellation.rs` |

### 2.3 依赖 Ollama 流式 API

Ollama `/api/chat` 支持 `stream: true`，返回 NDJSON 流：
```json
{"model":"qwen2.5:14b","created_at":"...","message":{"role":"assistant","content":"正在"},"done":false}
{"model":"qwen2.5:14b","created_at":"...","message":{"role":"assistant","content":"分析"},"done":false}
{"model":"qwen2.5:14b","created_at":"...","message":{"role":"assistant","content":"完成","done":true,"done_reason":"stop","context":{...},"total_duration":...}}
```

### 2.4 中断取消机制

使用 `CancellationToken`（Rust `tokio_util` 或自定义）：
- 前端发送 `DELETE /api/v1/ai/chat/stream/{session_id}` 或 HTTP 连接断开
- 后端检测取消信号，终止 LLM 调用

---

## 三、实现任务

### Task 1: 创建流式 DTO 和 CancellationToken

**Objective**: 定义流式请求/响应结构和取消令牌

**Files**:
- Create: `crates/llm/src/stream.rs`
- Modify: `crates/llm/src/lib.rs` (添加 `chat_stream` trait 方法)

**Step 1: 创建 `crates/llm/src/stream.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

/// 流式聊天请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChatRequest {
    pub messages: Vec<super::ChatMessage>,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    pub contexts: Vec<super::RetrievedDocument>,
    pub instructions: Option<String>,
}

fn default_model() -> String { "qwen2.5:14b".to_string() }
fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> usize { 2048 }

/// 流式响应事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum StreamEvent {
    #[serde(rename = "content")]
    Content { content: String, done: bool },
    #[serde(rename = "error")]
    Error { code: String, message: String },
    #[serde(rename = "final")]
    Final { answer: String, citations: Vec<super::Citation> },
}

/// 取消令牌
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: std::sync::Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self { cancelled: std::sync::Arc::new(AtomicBool::new(false)) }
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}
```

**Step 2: 验证编译**
```
cargo check -p sqlrustgo-llm
```

---

### Task 2: 扩展 LlmClient trait 添加 chat_stream 方法

**Objective**: 在 trait 中添加流式 chat 方法签名

**Files**:
- Modify: `crates/llm/src/lib.rs`

**Step 1: 添加 trait 方法**

在 `LlmClient` trait 中添加：
```rust
use super::stream::{CancellationToken, StreamEvent};

/// Stream chat completion, yields events
async fn chat_stream(
    &self,
    messages: &[ChatMessage],
    temperature: f32,
    max_tokens: usize,
    cancellation: CancellationToken,
) -> LlmResult<GeneratedAnswer>;
```

**Step 2: 在 `OllamaLlmClient` 中添加空实现（后续完成）**

```rust
async fn chat_stream(
    &self,
    messages: &[ChatMessage],
    temperature: f32,
    max_tokens: usize,
    _cancellation: CancellationToken,
) -> LlmResult<GeneratedAnswer> {
    // TODO: 实现流式调用
    self.chat(messages, temperature, max_tokens).await.map(|s| GeneratedAnswer {
        answer: s,
        citations: vec![],
        confidence: None,
        needs_human_review: false,
    })
}
```

**Step 3: 验证编译**
```
cargo check -p sqlrustgo-llm
```

---

### Task 3: 实现 Ollama 流式调用

**Objective**: 实现 `OllamaLlmClient::chat_stream()` 使用 Ollama `/api/chat` 流式接口

**Files**:
- Create: `crates/llm/src/ollama_stream.rs`

**Step 1: 创建 `crates/llm/src/ollama_stream.rs`**

```rust
use super::{ChatMessage, LlmError, LlmResult};
use crate::stream::{CancellationToken, StreamEvent};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::pin::Pin;
use tokio::sync::mpsc;

type EventStream = Pin<Box<dyn tokio_stream::Stream<Item = LlmResult<StreamEvent>> + Send>>;

/// Ollama 流式 chat 实现
pub async fn ollama_chat_stream(
    base_url: &str,
    model: &str,
    messages: &[ChatMessage],
    temperature: f32,
    max_tokens: usize,
    cancellation: CancellationToken,
) -> LlmResult<(String, EventStream)> {
    #[derive(Serialize)]
    struct ChatRequest<'a> {
        model: &'a str,
        messages: &'a [ChatMessage],
        stream: bool,
        options: ChatOptions,
    }
    #[derive(Serialize)]
    struct ChatOptions {
        temperature: f32,
        num_predict: usize,
    }

    let client = Client::new();
    let request = ChatRequest {
        model,
        messages,
        stream: true,
        options: ChatOptions { temperature, num_predict: max_tokens },
    };

    let resp = client
        .post(format!("{}/api/chat", base_url))
        .json(&request)
        .send()
        .await?;

    let stream = resp.bytes_stream();
    let model_clone = model.to_string();

    // 使用 mpsc channel 转换
    let (tx, rx) = mpsc::channel(100);
    let cancellation = cancellation;

    // 在后台任务中处理流
    tokio::spawn(async move {
        let mut full_content = String::new();
        futures_util::pin_mut!(stream);

        while let Some(chunk_result) = stream.next().await {
            if cancellation.is_cancelled() {
                let _ = tx.send(Ok(StreamEvent::Error {
                    code: "CANCELLED".to_string(),
                    message: "Request cancelled".to_string(),
                })).await;
                return;
            }

            match chunk_result {
                Ok(bytes) => {
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        for line in text.lines() {
                            if let Ok(response) = serde_json::from_str::<OllamaStreamResponse>(&line) {
                                let content = response.message.content.clone();
                                if !content.is_empty() {
                                    full_content.push_str(&content);
                                    let _ = tx.send(Ok(StreamEvent::Content {
                                        content,
                                        done: response.done,
                                    })).await;
                                }
                                if response.done {
                                    let _ = tx.send(Ok(StreamEvent::Final {
                                        answer: full_content.clone(),
                                        citations: vec![],
                                    })).await;
                                    return;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Ok(StreamEvent::Error {
                        code: "STREAM_ERROR".to_string(),
                        message: e.to_string(),
                    })).await;
                    return;
                }
            }
        }
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    Ok((full_content, Box::pin(stream)))
}

#[derive(Deserialize)]
struct OllamaStreamResponse {
    message: OllamaMessage,
    done: bool,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}
```

**Step 2: 验证编译**
```
cargo check -p sqlrustgo-llm
```

---

### Task 4: 创建 AI endpoints 文件并实现 SSE 端点

**Objective**: 创建 `crates/server/src/ai_endpoints.rs`，实现 `/api/v1/ai/chat/stream` 端点

**Files**:
- Create: `crates/server/src/ai_endpoints.rs`
- Modify: `crates/server/src/lib.rs` (注册模块)

**Step 1: 创建 `crates/server/src/ai_endpoints.rs`**

```rust
//! AI Chat SSE Endpoints

use crate::llm::{OllamaLlmClient, LlmClient, StreamChatRequest, StreamEvent, CancellationToken};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::codec::LinesCodec;
use std::pin::Pin;
use tokio::time::{timeout, Duration};

type EventStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<String, std::io::Error>> + Send>>;

/// POST /api/v1/ai/chat/stream - SSE 流式聊天
pub async fn chat_stream_handler(
    req: StreamChatRequest,
) -> Result<EventStream, String> {
    let llm = get_global_llm_client()?;
    
    let cancellation = CancellationToken::new();
    let cancellation_clone = cancellation.clone();

    let answer = llm.chat_stream(
        &req.messages,
        req.temperature,
        req.max_tokens,
        cancellation,
    ).await.map_err(|e| e.to_string())?;

    let (tx, rx) = mpsc::channel(100);
    
    tokio::spawn(async move {
        let mut answer_stream = answer;
        while let Some(event_result) = timeout(Duration::from_secs(30), answer_stream.next()).await {
            match event_result {
                Ok(Ok(event)) => {
                    let data = serde_json::to_string(&event).unwrap();
                    let sse_data = format!("data: {}\n\n", data);
                    if tx.send(Ok(sse_data)).await.is_err() {
                        break; // 客户端断开
                    }
                    if matches!(event, StreamEvent::Content { done: true, .. }) 
                       || matches!(event, StreamEvent::Final { .. }) {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    let _ = tx.send(Ok(format!("event: error\ndata: {{\"code\":\"LLM_ERROR\",\"message\":\"{}\"}}\n\n", e))).await;
                    break;
                }
                Err(_) => {
                    // 超时
                    let _ = tx.send(Ok("event: timeout\ndata: {\"message\":\"timeout\"}\n\n".to_string())).await;
                    break;
                }
            }
        }
    });

    Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
}

fn get_global_llm_client() -> Result<Arc<dyn LlmClient>, String> {
    // 从全局注册表获取，暂用 placeholder
    Err("LLM client not initialized".to_string())
}
```

**Step 2: 在 `crates/server/src/lib.rs` 添加模块**

```rust
pub mod ai_endpoints;
```

**Step 3: 验证编译**
```
cargo check -p sqlrustgo-server
```

---

### Task 5: 集成到 main server 并添加路由

**Objective**: 将 SSE 端点集成到现有 HTTP 服务器

**Files**:
- Modify: `crates/server/src/http_server.rs`

**Step 1: 添加 SSE 路由处理**

在 `http_server.rs` 中添加：
```rust
if path == "/api/v1/ai/chat/stream" && method == "POST" {
    let body = read_body(&mut stream);
    let req: StreamChatRequest = serde_json::from_str(&body)
        .map_err(|e| format!("Invalid request: {}", e))?;
    
    let stream = ai_endpoints::chat_stream_handler(req).await
        .map_err(|e| format!("Stream error: {}", e))?;
    
    // 发送 SSE 响应头
    let response = "HTTP/1.1 200 OK\r\n\
        Content-Type: text/event-stream\r\n\
        Cache-Control: no-cache\r\n\
        Connection: keep-alive\r\n\
        Transfer-Encoding: chunked\r\n\r\n";
    stream.write_all(response.as_bytes())?;
    
    // 流式发送事件
    tokio::io::AsyncWriteExt::write_all(&mut stream, &response_bytes).await?;
    return Ok(());
}
```

---

### Task 6: 端到端测试

**Step 1: 启动 Ollama（如未启动）**
```
ollama serve &
ollama pull qwen2.5:14b
```

**Step 2: 启动 server**
```
cargo run -p sqlrustgo-server --release
```

**Step 3: 测试 SSE 流式输出**
```
curl -X POST http://localhost:8080/api/v1/ai/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"Hello"}],"model":"qwen2.5:14b"}'
```

**Expected**: 看到分块的 `data: {...}` SSE 事件

**Step 4: 测试中断取消**
```
# 在另一个终端，或使用 JS EventSource + AbortController
```

---

### Task 7: 性能验证

**Step 1: 测量首字节延迟**
```bash
time curl -o /dev/null -s -w "%{time_starttransfer}\n" \
  -X POST http://localhost:8080/api/v1/ai/chat/stream \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"简短回答：1+1=?"}]}'
```

**Expected**: 首字节延迟 < 100ms（网络 + LLM 首 token）

---

## 四、验收标准

| 验收项 | 标准 | 状态 |
|--------|------|------|
| SSE 端点 | POST `/api/v1/ai/chat/stream` 返回 `text/event-stream` | TODO |
| 流式输出 | 每个 token 实时推送，`data: {"t":"content","content":"...","done":false}` | TODO |
| 中断取消 | 客户端断开时服务器停止生成 | TODO |
| 首字节延迟 | < 100ms（本地模型） | TODO |
| 错误处理 | LLM 不可用时发送 `event: error` | TODO |

---

## 五、变更记录

| 版本 | 日期 | 变更说明 |
|------|------|----------|
| v1.0 | 2026-05-26 | 初始规范 |