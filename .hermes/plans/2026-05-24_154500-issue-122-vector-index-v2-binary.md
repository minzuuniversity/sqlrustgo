# SPEC: Issue #122 — 向量索引生成流水线 (纯 sqlrustgo)

## 背景

- `migrate_from_rusqlite_if_needed()` 是唯一使用 rusqlite 的地方（一次性迁移）
- 迁移后所有数据在 sqlrustgo FileStorage 中，rusqlite 仅用于读取源 gmp.db
- 当前 embedding 存储为 JSON TEXT（低效），需要改为二进制 BLOB

## 目标

1. 用二进制 BLOB 存储 embedding（摆脱 JSON）
2. 在 StorageEngine::new() 中自动构建 HNSW 索引
3. 向量检索 Recall@10 ≥ 0.80，延迟 P95 < 50ms

## 二进制 BLOB 格式

每个 embedding 存储为：

```
[4 bytes: dimension u32 BE][4×dimension bytes: f32 BE]
例：dim=1024 → 4 + 4096 = 4100 bytes
```

SQLite BLOB 列存储，无 JSON parse 开销。

## 实施步骤

### Step 1: 确认 sqlrustgo Blob 支持

检查 `sqlrustgo_types::Value::Blob` 是否可用，以及 storage.insert() 是否支持 BLOB 列类型。

### Step 2: 修改 init_schema — 添加 gmp_embeddings_bin 表

在 `init_schema()` 中添加（与 gmp_embeddings 并存，渐进迁移）：

```rust
// Binary BLOB embeddings table
if !storage.has_table("gmp_embeddings_bin") {
    let emb_table = TableInfo {
        name: "gmp_embeddings_bin".to_string(),
        columns: vec![
            ColumnDefinition::new("chunk_id", "INTEGER"),
            ColumnDefinition::new("dimension", "INTEGER"),
            ColumnDefinition::new("embedding", "BLOB"),
        ],
        primary_key: Some("chunk_id".to_string()),
    };
    storage.create_table(&emb_table)?;
}
```

### Step 3: 修改 migrate_from_rusqlite_if_needed — 写入 BLOB

源数据（gmp_embeddings 表）仍是 JSON，需要：
1. 用 rusqlite 读取 `chunk_id, embedding_json`
2. JSON decode → `Vec<f32>`
3. 编码为二进制 BLOB
4. 写入 `gmp_embeddings_bin` 表（sqlrustgo）

```rust
// In migrate_from_rusqlite_if_needed():
let mut stmt = conn.prepare("SELECT chunk_id, embedding_json FROM gmp_embeddings")?;
let mut rows = stmt.query([])?;

while let Some(row) = rows.next()? {
    let chunk_id: i64 = row.get(0)?;
    let json_str: String = row.get(1)?;

    // JSON → Vec<f32>
    let embedding: Vec<f32> = serde_json::from_str(&json_str)?;

    // Encode as binary BLOB: [dim u32][f32 × dim]
    let mut blob = Vec::with_capacity(4 + embedding.len() * 4);
    blob.extend_from_slice(&(embedding.len() as u32).to_be_bytes());
    for v in &embedding {
        blob.extend_from_slice(&v.to_be_bytes());
    }

    // Insert into sqlrustgo (NOT rusqlite)
    let record = vec![
        sqlrustgo_types::Value::Integer(chunk_id),
        sqlrustgo_types::Value::Integer(embedding.len() as i64),
        sqlrustgo_types::Value::Blob(blob),
    ];
    storage.insert("gmp_embeddings_bin", vec![record])?;
}
```

### Step 4: 实现 rebuild_hnsw_from_db() — 从 BLOB 读取

```rust
pub fn rebuild_hnsw_from_db(&self) -> Result<usize> {
    if !self.hnsw_index.is_empty() {
        return Ok(self.hnsw_index.len());
    }

    let storage = self.storage.lock().unwrap();
    let embeddings = storage.scan("gmp_embeddings_bin").unwrap_or_default();
    drop(storage);

    let mut count = 0;
    for emb in &embeddings {
        if emb.len() >= 3 {
            let chunk_id = doc_value_int(emb, 0);
            let blob = doc_value_blob(emb, 2); // &[u8]

            if blob.len() < 4 { continue; }
            let dim = u32::from_be_bytes([blob[0], blob[1], blob[2], blob[3]]) as usize;
            if blob.len() != 4 + dim * 4 { continue; }

            let mut vec = vec![0.0f32; dim];
            for i in 0..dim {
                vec[i] = f32::from_be_bytes([
                    blob[4 + i*4], blob[5 + i*4], blob[6 + i*4], blob[7 + i*4]
                ]);
            }

            let _ = self.hnsw_index.insert(chunk_id as u64, &vec);
            count += 1;
        }
    }

    if count > 0 {
        self.hnsw_index.build_index()?;
    }
    eprintln!("[init] HNSW indexed {} vectors (dim={})", count, self.hnsw_index.dimension());
    Ok(count)
}
```

### Step 5: 在 new() 末尾调用 rebuild_hnsw_from_db()

在 `StorageEngine::new()` 末尾（migrate_all() 之后）追加：

```rust
// Build HNSW index from binary embeddings
self.rebuild_hnsw_from_db()?;
```

### Step 6: 更新所有 embedding_dim = 1024

- gmp-cli/main.rs: 所有 `StorageConfig { embedding_dim: 768 }` → `1024`
- gmp-eval/main.rs: 同上

### Step 7: 添加 CLI 命令 `rebuild-hnsw`

用于手动重建索引：
```rust
Commands::RebuildHnsw { db } => {
    let config = StorageConfig {
        embedding_dim: 1024,
        use_sqlrustgo_storage: true,
        db_path: db,
        data_dir: Path::new(&db).parent().unwrap().to_string_lossy().to_string(),
    };
    let storage = StorageEngine::new(config)?;
    let n = storage.rebuild_hnsw_from_db()?;
    println!("HNSW rebuilt: {} vectors", n);
}
```

## 验收标准

- [ ] rusqlite 仅用于读取源 gmp.db（一次性），之后操作全走 sqlrustgo
- [ ] gmp_embeddings_bin 表存在且记录数 = gmp_embeddings 表记录数
- [ ] HNSW index 非空（`hnsw_index.len() > 0`）after init
- [ ] `search_vectors` 返回非空结果
- [ ] 向量 Recall@10 ≥ 0.80（通过 gmp-eval）
- [ ] 向量搜索 P95 延迟 < 50ms

## 文件变更

| File | Change |
|------|--------|
| `crates/gmp-storage/src/storage.rs` | init_schema (添加 gmp_embeddings_bin), migrate_from_rusqlite_if_needed (写入BLOB), rebuild_hnsw_from_db (读取BLOB), new() 末尾调用 rebuild |
| `crates/gmp-cli/src/main.rs` | embedding_dim: 1024, 新增 RebuildHnsw 命令 |
| `crates/gmp-eval/src/main.rs` | embedding_dim: 1024 |
| `crates/gmp-storage/src/traits.rs` | rebuild_hnsw_from_db trait 签名（已存在） |

## 备选方案

若 `sqlrustgo_types::Value::Blob` 不支持二进制数据，或 BLOB 插入失败：
→ 退回到 JSON TEXT 存储，只修复 HNSW 构建逻辑（简化版）