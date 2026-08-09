# Proposal: V312-06 SQL-backed Graph Projection

## Why

GMP evidence navigation requires traversing document relationships as a graph. V312-06 provides SQL-backed graph projection that maps the relation table into navigable node/edge subgraphs, enabling depth ≤ 3 path queries and evidence bundle generation.

## What Changes

- `graph.rs`: New module — `GraphProjection`, `GraphNode`, `GraphEdge`, `EvidenceBundle`, `EvidenceItem`, `project_subgraph`, `generate_evidence_bundle`, `get_graph_stats`
- Supports all RelationTypes: Sop, Clause, Capa, Deviation, Role, Equipment, AuditFinding
- Evidence bundle provides full provenance: chunk_id, chunk_hash, section_name, content_text per node in path

## Capabilities

### New Capabilities

- `gmp-graph-projection`: BFS-based subgraph projection from any document center up to depth N
- `gmp-evidence-bundle`: Generate complete evidence bundle from a graph path with all chunk citations
