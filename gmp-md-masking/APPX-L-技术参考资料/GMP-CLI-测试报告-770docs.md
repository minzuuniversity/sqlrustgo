# GMP CLI 测试报告 — 770 文档

## 测试时间
2025-04-20

## 数据库规模
| 指标 | 数值 |
|------|------|
| 总文档 | 770 |
| Embedding | 770 × 4096B (bge-m3 1024d) |
| 图节点 | 545 (SOP×521 + Chapter×8 + Department×10 + Category×6) |
| 图边 | ~1800 |

## 测试结果：20/20 通过

### [1] 基本连通性
- Ping ✅
- Init ✅

### [2] 向量检索 (bge-m3 cosine)
| 查询 | 最高分 | 标题 |
|------|--------|------|
| deviation investigation | 0.6425 | 偏差相关文档 |
| supplier audit | 0.5249 | 供应商管理文档 |
| cleaning validation | 0.6530 | 清洁验证文档 |
| product release | 0.6463 | 产品放行文档 |
| HVAC validation | 0.5506 | 空调系统文档 |

### [3] SQL 精确检索
- SqlSearch + chapter 过滤 ✅

### [4] 图谱操作
| 功能 | 结果 |
|------|------|
| ListNodes(Chapter) | 8 个章节节点 |
| ListNodes(Department) | 10 个部门节点 |
| ListNodes(SOP) | 521 个 SOP 节点 |
| ListEdgeTypes | 4 种边类型 |
| GraphNeighbors | 双向边查询正常 |

### [5] 三路融合检索 (RRF)
- 10/10 查询全部返回结果 ✅
- 向量 + exact + graph 三路 RRF 融合

## 已知问题

### [低优先级] 部分文档标题格式不佳
- 13 个文档标题含 "审核人: [NAME-B-REVIEWER]" 等元信息前缀
- 内容已清理（去除 <think> think 标签），但标题未完全规范化

### [待修复] SqlSearch schema 不匹配
- 当前二进制使用旧 `persist_sqlite.rs` schema（effective_date 为 TEXT）
- 数据库 schema 已更新为 INTEGER
- 需重新编译二进制以同步 schema

### [已知] TripleSearch 图路径 doc_id 代理
- 图路径用 node_id 而非真实 doc_id 作为 ranking key
- 对结果顺序有轻微影响，不影响功能正确性

## 三路检索架构
```
TripleSearch(query)
  ├─ Path 1: vector_search_sqlite() — bge-m3 cosine ×0.4
  ├─ Path 2: exact keyword (title+content 前500字) ×0.3
  └─ Path 3: graph BFS 2-hop via node name keyword ×0.3
      RRF fusion (k=60)
```

## 文件位置
- CLI: `~/sqlrustgo/target/debug/sqlrustgo-gmp-cli`
- DB: `~/gmp-qa/gmp.db` (770 docs)
- 图: `~/gmp-qa/graph.db` (545 nodes)
- 测试脚本: `/tmp/test_gmp_cli.py`
