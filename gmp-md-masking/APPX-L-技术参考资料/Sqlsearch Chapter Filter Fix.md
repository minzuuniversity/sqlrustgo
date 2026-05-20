---
entity_type: concept
confidence: high
domains: [sqlrustgo, gmp, search]
created: 2025-04-17
updated: 2025-04-17
---

# Sqlsearch Chapter Filter Fix

## 问题

`SqlSearch` 命令的 `chapter` 过滤参数被忽略，所有查询返回相同结果。

## 根因

`Request::SqlSearch` 定义了 4 个过滤参数：
```rust
SqlSearch {
    query: String,
    department: Option<String>,
    category: Option<String>,
    chapter: Option<String>,  // 定义了但没处理！
    top_k: usize,
}
```

handler 代码只处理了 `department` 和 `category`：
```rust
if let Some(ref dept) = department { ... }
if let Some(ref cat) = category { ... }
// 缺少 chapter 的处理！
```

## 修复

在 handler 中增加 chapter 过滤：

```rust
if let Some(ref chap) = chapter {
    let props_chap = doc
        .properties
        .get("chapter")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !props_chap.contains(chap) {
        continue;
    }
}
```

## properties 中的 chapter 字段

```json
{
  "category": "SOP",
  "chapter": "02",
  "code": "GMP-02-SOP-001_001",
  "department": ""
}
```

由 `import_sops.py` 从文件路径解析：
```
2025GMP-MD/04_/SOP/GMP-04-SOP-001_053.md
                ^^^^ → chapter = "04"
```

## 验证

| 查询 | 返回数 | DB 实际 |
|------|--------|---------|
| 验证 (无过滤) | 20 | 109 |
| 验证, ch01 | 3 | 6 |
| 验证, ch04 | 11 | 24 |

过滤生效，ch01=3 < 无过滤=20，ch04=11 < 无过滤=20。

## 相关

- [[SQLRustGo GMP CLI]]
- [[Persist Sqlite Backend]]
