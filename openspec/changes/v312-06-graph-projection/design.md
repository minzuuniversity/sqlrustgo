# Design: V312-06 SQL-backed Graph Projection

## Context

The `gmp_relations` table stores typed edges between documents. V312-06 projects this into in-memory graph structures for navigation and evidence extraction.

## Decisions

### Decision: BFS for subgraph projection

Simple BFS from center node avoids recursion depth issues and naturally limits to depth ≤ 3 per requirements.

### Decision: EvidenceBundle aggregates all chunks in path

Every node in the path contributes its chunks to the evidence bundle, giving full provenance for downstream RAG (V312-07).

### Decision: GraphEdge derives from both PathEdge and Relation

`PathEdge` from `path_query` output and `Relation` from storage both map to `GraphEdge` — unified representation for rendering.
