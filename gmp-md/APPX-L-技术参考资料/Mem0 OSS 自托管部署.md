# Mem0 OSS 自托管部署

## 概述
Mem0 OSS v1.0.11 自托管部署，实现中期长期记忆管理。
官方 pip 安装，Qdrant + Ollama bge-m3 + MiniMax 云端 LLM。

## 架构
- **向量库**: Qdrant (localhost:6333, collection=mem0_memory, 1024维)
- **Embedding**: Ollama bge-m3 (localhost:11434, 1024维, T5 tokenizer)
- **LLM**: MiniMax Cloud (MiniMax-M2.7, via api.minimaxi.com)
- **历史DB**: SQLite (~/.mem0/history.db)

## 部署步骤

### 1. Qdrant 容器
```bash
docker run -d --name qdrant -p 6333:6333 -p 6334:6334 qdrant/qdrant:latest
```

### 2. Ollama 服务
```bash
ollama serve  # 已运行 localhost:11434
ollama pull bge-m3:latest
ollama pull qwen2.5:latest
```

### 3. pip 安装 Mem0 OSS
```bash
pip install mem0ai==1.0.11
```

### 4. 配置文件
路径: `/home/ai/mem0_oss_config.py`

关键配置格式（注意：dict 嵌套，不可传 pydantic 实例）:
```python
from mem0.configs.base import MemoryConfig
from mem0.llms.configs import LlmConfig
from mem0.vector_stores.configs import VectorStoreConfig
from mem0.embeddings.configs import EmbedderConfig

mc = MemoryConfig(
    vector_store=VectorStoreConfig(
        provider='qdrant',
        config={'collection_name': 'mem0_memory', 'embedding_model_dims': 1024, 'host': 'localhost', 'port': 6333}
    ),
    llm=LlmConfig(
        provider='minimax',
        config={'model': 'MiniMax-M2.7', 'api_key': KEY, 'minimax_base_url': 'https://api.minimaxi.com/v1', 'temperature': 0.3, 'max_tokens': 2000}
    ),
    embedder=EmbedderConfig(
        provider='ollama',
        config={'model': 'bge-m3:latest', 'ollama_base_url': 'http://localhost:11434', 'embedding_dims': 1024}
    ),
    history_db_path='~/.mem0/history.db',
)
```

### 5. 使用方法
```python
from mem0_oss_config import get_memory_store

store = get_memory_store('li_ge')
store.add('用户信息...', user_id='li_ge', metadata={'source': 'chat'})
results = store.search('查询', user_id='li_ge', limit=5)
all_memories = store.get_all(user_id='li_ge')
```

## 已知问题

### Pydantic v2 验证错误（已解决）
**错误**: `ValidationError: vector_store: Input should be a valid dictionary or instance of VectorStoreConfig`
**原因**: MemoryConfig 接收 pydantic 实例而非 dict 时验证失败
**解决**: 传入 dict 格式的配置（provider + config dict），不要直接传 QdrantConfig/MinimaxConfig 实例

## MiniMax API Key
从 `~/.local/share/opencode/auth.json` 读取:
```bash
cat ~/.local/share/opencode/auth.json | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['minimax-cn-coding-plan']['key'])"
```

## 验证测试
```bash
cd /home/ai && python3 -c "
from mem0_oss_config import get_memory_store
m = get_memory_store('li_ge')
m.add('test memory', user_id='li_ge')
print(m.search('test', user_id='li_ge'))
"
```

## 与 Hermes 集成
Hermes Agent 的 memory 工具现已配置使用 Mem0 OSS。
用户记忆持久化到 Qdrant + SQLite，支持跨会话检索。

## 相关文件
- `/home/ai/mem0_oss_config.py` — Mem0 OSS 配置模块
- `/home/ai/mem0记忆库.py` — 旧版自定义记忆库（ChromaDB 直写）
- `~/.mem0/history.db` — Mem0 历史记录 SQLite
