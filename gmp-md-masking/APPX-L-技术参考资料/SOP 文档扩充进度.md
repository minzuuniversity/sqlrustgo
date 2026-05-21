# SOP 文档扩充进度

## 任务目标
将 145 个截断 SOP（500-1000 字符）扩充为完整 SOP（每个 ≥2000 中文字符）

## 背景
[COMPANY-NAME-PHARMA] GMP 知识库（~/gmp-qa/gmp.db）原有 562 个 SOP 文档，
其中 145 个只有框架标题，内容截断在 500-1000 字符。

## 工厂上下文
- 工厂: [COMPANY-NAME-PHARMA]
- 产品: [PRODUCT-A-CAPSULE]（0.4g 硬胶囊剂）
- 工艺流程: 提取（10 倍水，100°C，60min）→ 浓缩 → 醇沉 → 干燥 → 粉碎 → 制粒（75% 乙醇）→ 胶囊填充（NJP1200B）→ 铝塑板包装
- GMP 参考: 2010 版 GMP，ICH Q9
- 洁净区: D 级洁净区
- 工艺用水: 纯化水（电导 ≤1.3μS/cm@25°C，TOC ≤0.5mg/L）

## 技术方案
- **SOP 生成**: MiniMax-M2.7 云端模型（MiniMax API）
  - API: `https://api.minimaxi.com/v1/chat/completions`
  - Key: 从 `~/.local/share/opencode/auth.json` 读取
- **Embedding**: Ollama bge-m3（1024 维，/api/embeddings 接口）
- **数据库**: SQLite `~/gmp-qa/gmp.db`
  - 表 `gmp_documents`: id, title, content, chapter, department, category, updated_at
  - 表 `gmp_embeddings`: doc_id, embedding(BLOB 4096 bytes), updated_at

## SOP 生成脚本
- `/tmp/gen_sop.py` — 原始并行版本（5 batch，触发了 429）
- `/tmp/sop_retry81.py` — 修复后的串行版本，逐一生成

## 当前进度

### 后台进程
- PID: 2368485（`setsid python3 /tmp/sop_retry81.py &`）
- 启动时间: 05:31
- 日志: `/tmp/sop_retry81_output.txt`
- 查询: `grep -c 'chars OK' /tmp/sop_retry81_output.txt`

### 进度（每条 ~2 分钟）
- [x] Batch 1-5 初始并行生成（81 个失败，429 限流）
- [x] 串行重试启动（/tmp/sop_retry81.py，75 个 SOP）
- [x] 24/75 已完成（约 05:09 时）
- [ ] 剩余约 51 个，约 60 分钟完成

### 完成样例
- doc55: 10650 chars
- doc56: 9055 chars
- doc58: 7091 chars
- doc60: 9167 chars
- doc64: 10905 chars
- doc65: 8531 chars
- doc66: 8594 chars
- doc68: 8913 chars
- doc71: 8362 chars
- doc72: 10091 chars
- doc73: 11253 chars
- doc77: 8458 chars
- doc81: 9325 chars
- doc84: 11153 chars
- doc86: 9909 chars

## 完成后步骤

### 1. 验证所有 SOP 已填充
```python
python3 -c "
import sqlite3
db = sqlite3.connect('/home/ai/gmp-qa/gmp.db')
bad = db.execute(\"SELECT id FROM gmp_documents WHERE LENGTH(content) < 100\").fetchall()
print(f'Bad docs: {len(bad)}')
"
```

### 2. 批量更新 embedding
87 个短内容 SOP 的 embedding 需要重新生成 bge-m3 向量并写入 gmp_embeddings 表。

### 3. 400 场景测试
```bash
cd /home/ai/sqlrustgo && cargo test -p sqlrustgo-gmp --test integration
```

## 技术细节

### MiniMax API 调用
```python
def call_minimax(prompt: str) -> str:
    payload = {
        "model": "MiniMax-M2.7",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 4096,
        "temperature": 0.3,
        "stream": True
    }
    # stream=True → data: {...} 逐行
```

### bge-m3 Embedding
```python
def embed_bge_m3(text: str) -> bytes:
    req = urllib.request.Request(
        "http://localhost:11434/api/embeddings",
        data=json.dumps({"model": "bge-m3:latest", "prompt": text[:8000]}).encode(),
        headers={"Content-Type": "application/json"}, method="POST"
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        emb = json.loads(resp.read())["embedding"]
    return struct.pack(f"<{len(emb)}f", *emb)  # 1024 floats → 4096 bytes
```

### SQLite BLOB 写入
```python
cur.execute("UPDATE gmp_embeddings SET embedding=?, updated_at=? WHERE doc_id=?", (embedding, now, doc_id))
```
