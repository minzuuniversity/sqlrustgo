# V312-06 SQL-backed Graph Projection — Verification Report

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=1bc0066b459ff59a751788e9405ad5aacdfccc4b, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

## Issue & PR

| 字段 | 值 |
|------|-----|
| Issue | #3893 |
| PR | #3922 (merged) |
| Merge Commit SHA | `1bc0066b459ff59a751788e9405ad5aacdfccc4b` |
| 报告日期 | 2026-08-09 |

## 模块变更

| 文件 | Evidence Hash (SHA-256 前 16 字符) | 行数 | 说明 |
|------|-----------------------------------|------|------|
| `gmp/src/graph.rs` | `22452ab591ade929` | 446 | GraphNode、GraphEdge、GraphProjection、BFS、EvidenceBundle |

## 功能验证

- `GraphNode` / `GraphEdge`: 节点和边结构，`From<PathEdge>` / `From<Relation>` 转换
- `GraphProjection`: 从 SQL 表 (`gmp_relations`) 构建内存图投影
- `project_subgraph()`: BFS 从种子节点展开子图
- `EvidenceItem` / `EvidenceBundle`: 证据结构，`Serialize`/`Deserialize`
- `generate_evidence_bundle()`: 生成证据包，关联 SOP/CLAUSE/CAPA
- `get_graph_stats()`: 图统计（节点数、边数、类型分布）

## 测试命令

```bash
cargo test -p sqlrustgo-gmp --lib
```

## 测试结果

```
running 154 tests
  graph::tests::test_graph_node_creation ... ok
  graph::tests::test_graph_edge_from_relation ... ok
  graph::tests::test_project_subgraph ... ok          ← BFS 子图投影
  graph::tests::test_evidence_bundle_serialization ... ok
  graph::tests::test_generate_evidence_bundle ... ok
  graph::tests::test_get_graph_stats ... ok
  [... 130+ tests ...]
test result: ok. 154 passed; 0 failed; 0 ignored
```

**PASS — 154 tests passed, 0 failed**

## Evidence Hash (merge commit)

```
1bc0066b459ff59a751788e9405ad5aacdfccc4b
```

## OpenSpec

`openspec/changes/v312-06-graph-projection/` — proposal + design + specs + tasks 齐全

## 关闭边界

- [x] PR #3922 merged，merge commit 在 `develop/v3.12.0` 可达
- [x] graph.rs 代码存在且编译通过
- [x] 154 tests passed, 0 failed
- [x] BFS 子图投影测试存在
- [x] EvidenceBundle 序列化测试存在
- [x] OpenSpec 文档齐全

**状态: PASS — 满足关闭条件**
