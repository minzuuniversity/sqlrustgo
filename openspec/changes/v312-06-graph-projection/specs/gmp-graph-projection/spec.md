# GMP Graph Projection

## ADDED Requirements

### Requirement: Subgraph projection returns nodes and edges

`project_subgraph` returns `GraphProjection` with `nodes: Vec<GraphNode>` and `edges: Vec<GraphEdge>`.

#### Scenario: Project 2-hop subgraph
- **WHEN** `project_subgraph(storage, 1, 2, None)` is called on a graph with docs 1→2→3
- **THEN** the projection contains nodes for docs 1,2,3 and edges for 1→2 and 2→3

### Requirement: GraphNode contains full document metadata

Each `GraphNode` has: doc_id, node_type (doc_type), title, status, effective_date, version.

#### Scenario: Node metadata
- **WHEN** a document "SOP-001" (active, effective 2025) is in the projected subgraph
- **THEN** node.node_type = "SOP-001", node.status = "ACTIVE", node.effective_date = 2025..., node.version = latest

### Requirement: Evidence bundle aggregates chunks from all path nodes

`generate_evidence_bundle` returns an `EvidenceBundle` where `evidence` contains all chunks from all documents in the path.

#### Scenario: Evidence bundle from 3-node path
- **WHEN** path has nodes A→B→C with chunks [c1,c2], [c3], [c4,c5]
- **THEN** bundle.evidence = [c1, c2, c3, c4, c5], total_chunks = 5

### Requirement: Graph statistics

`get_graph_stats` returns total_nodes, total_edges, node_types breakdown, edge_types breakdown, avg_degree.

#### Scenario: Graph stats
- **WHEN** `get_graph_stats` is called on a corpus with 100 docs and 150 relations
- **THEN** stats.avg_degree = 150*2/100 = 3.0
