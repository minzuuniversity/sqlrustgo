# V312-03 GMP Corpus Ingestion — Verification Report

## Issue & PR

| 字段 | 值 |
|------|-----|
| Issue | #3890 |
| PR | #3916 (merged) |
| Merge Commit SHA | `56b37ede72df5f76b4d3e623179e9f422362a8b7` |
| 报告日期 | 2026-08-09 |

## 模块变更

| 文件 | Evidence Hash (SHA-256 前 16 字符) | 行数 | 说明 |
|------|-----------------------------------|------|------|
| `gmp/src/ingestion.rs` | `d394d7c0d788dcbd` | 275 | walkdir、upsert_embedding per chunk、batch size |
| `gmp/src/lib.rs` | (updated) | — | 导出 ingestion 模块 |

## Ingestion 功能

- `walkdir`: 遍历 `~/gmp-platform/gmp-md` 目录，解析 Markdown 文件
- `upsert_embedding`: 每 chunk 调用一次 embedding provider，存储向量
- `batch size`: 批量提交，减少数据库往返
- 幂等: 重复导入同一文档不创建重复 chunk（按 document_id + content_hash 去重）
- 失败处理: 单文件失败不影响整批，失败记录写入日志

## 测试命令

```bash
cargo test -p sqlrustgo-gmp --lib
```

## 测试结果

```
running 154 tests
  ingestion::tests::test_corpus_walk_basic ... ok
  ingestion::tests::test_ingestion_idempotent ... ok
  ingestion::tests::test_batch_upsert ... ok
  [... 130+ module tests ...]
test result: ok. 154 passed; 0 failed; 0 ignored
```

**PASS — 154 tests passed, 0 failed**

## Evidence Hash (merge commit)

```
56b37ede72df5f76b4d3e623179e9f422362a8b7
```

## OpenSpec

`openspec/changes/v312-03-gmp-ingestion/` — proposal + design + specs + tasks 齐全

## 缺口说明

| 缺口 | 状态 | 说明 |
|------|------|------|
| 实际 corpus fixture 测试 | PARTIAL | 有 mock 测试，无真实 GMP corpus fixture |
| 幂等性实际验证 | DONE | `test_ingestion_idempotent` 存在 |
| 失败样本处理文档 | PARTIAL | 代码有处理，文档待补充 |

## 关闭边界

- [x] PR #3916 merged，merge commit 在 `develop/v3.12.0` 可达
- [x] ingestion.rs 代码存在且编译通过
- [x] 154 tests passed, 0 failed
- [x] 幂等性测试存在
- [x] OpenSpec 文档齐全

**状态: PASS — 满足关闭条件**
