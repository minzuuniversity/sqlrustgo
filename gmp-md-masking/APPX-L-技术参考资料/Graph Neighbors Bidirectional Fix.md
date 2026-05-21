---
entity_type: pattern
confidence: high
domains: [sqlrustgo, gmp, graph]
created: 2025-04-17
updated: 2025-04-17
---

# Graph Neighbors Bidirectional Fix

## 问题

`GraphNeighbors` 查询 QA 部门，返回 `neighbors: []`。

## 根因分析

GMP 知识图谱的边是**单向的**：
- SOP 节点 → 部门节点（`edge_type = 归口管理部门`）
- 即 QA 部门的边全部是**入边**（incoming），没有出边（outgoing）

原代码：
```rust
let neighbor_ids: Vec<NodeId> = match rel_type {
    Some(rt) => graph.neighbors_by_edge_label(nid, rt),  // 只查 outgoing
    None => graph.outgoing_neighbors(nid),               // 只查 outgoing
};
```

`neighbors_by_edge_label` 在 InMemoryGraphStore 的实现：
```rust
fn neighbors_by_edge_label(&self, node: NodeId, edge_label: &str) -> Vec<NodeId> {
    self.labels.get(edge_label)
        .map(|label_id| self.adjacency.get_neighbors(node, label_id))
        .unwrap_or_default()
}
```

`adjacency.get_neighbors` 只查 outgoing 边，所以 QA 永远返回空。

## 解决方案

### 1. 新增 trait 方法

`graph_store.rs` trait 定义加：
```rust
fn incoming_neighbors_by_edge_label(&self, node: NodeId, edge_label: &str) -> Vec<NodeId>;
```

### 2. 实现（InMemoryGraphStore）

```rust
fn incoming_neighbors_by_edge_label(&self, node: NodeId, edge_label: &str) -> Vec<NodeId> {
    let label_id = match self.labels.get(edge_label) {
        Some(id) => id,
        None => return vec![],
    };
    self.edges
        .get_incoming(node)
        .iter()
        .filter_map(|&edge_id| self.edges.get(edge_id))
        .filter(|edge| edge.label == label_id)
        .map(|edge| edge.from)
        .collect()
}
```

### 3. 三处都要实现

| 实现 | 路径 |
|------|------|
| InMemoryGraphStore | `graph_store.rs` |
| MultiShardGraphStore | `sharded_graph.rs` |
| DiskGraphStore | `disk_graph_store.rs` |

### 4. 修改 CLI 遍历逻辑

```rust
let neighbor_ids: Vec<NodeId> = match rel_type {
    Some(_rt) => {
        let out = graph.neighbors_by_edge_label(nid, rel_type.unwrap());
        let incoming = graph.incoming_neighbors_by_edge_label(nid, rel_type.unwrap());
        out.into_iter().chain(incoming).collect()
    }
    None => {
        let out = graph.outgoing_neighbors(nid);
        let incoming = graph.incoming_neighbors(nid);
        out.into_iter().chain(incoming).collect()
    }
};
```

## 验证结果

| 查询 | 修复前 | 修复后 |
|------|--------|--------|
| GraphNeighbors QA | 0 | 24 |
| GraphNeighbors 生产部 | 0 | 57 |
| GraphNeighbors 第02章 | 0 | 124 |

## 相关

- [[SQLRustGo GMP CLI]]
- [[Graph Store Bidirectional]]
