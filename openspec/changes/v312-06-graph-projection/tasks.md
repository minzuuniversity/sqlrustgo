# V312-06 SQL-backed Graph Projection — Implementation Tasks

## 1. Implementation

- [x] 1.1 `graph.rs` — `GraphNode`, `GraphEdge`, `GraphProjection`, `EvidenceItem`, `EvidenceBundle`
- [x] 1.2 `graph.rs` — `project_subgraph` with BFS up to depth N, relation type filtering
- [x] 1.3 `graph.rs` — `generate_evidence_bundle` aggregating all chunks from path nodes
- [x] 1.4 `graph.rs` — `get_graph_stats` with node/edge type breakdowns and avg_degree
- [x] 1.5 `graph.rs` — `relation_type_to_node_type` mapping
- [x] 1.6 `lib.rs` — Added `pub mod graph;`

## 2. Tests

- [x] 2.1 `test_graph_node_creation`, `test_graph_edge_from_path_edge`, `test_graph_edge_from_relation`
- [x] 2.2 `test_graph_projection_counts`
- [x] 2.3 `test_evidence_bundle_summary`, `test_evidence_bundle_json`
- [x] 2.4 `test_relation_type_to_node_type` — all 7 types
- [x] 2.5 `test_project_subgraph_empty`, `test_get_graph_stats_empty`
- [x] 2.6 All 114 GMP tests pass

## 3. OpenSpec

- [x] 3.1 Create `v312-06-graph-projection` change
- [x] 3.2 Write `proposal.md`, `design.md`, spec, tasks
