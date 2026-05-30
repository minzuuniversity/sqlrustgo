# Governance v4.1 Runtime Specification
## Evidence Graph Enforcement System

> Status: ACTIVE
> Model: Runtime Evidence Graph Governance
> Replaces: ALL previous policy-based governance assumptions
> Version: 4.1.0
> Effective: 2026-05-30

---

## 0. Core Principle (Non-Negotiable)

The system of record is:

```
TRUTH := Evidence Graph
```

**Any state not represented in the Evidence Graph is considered non-existent.**

Documents are projections of graph state, not sources of truth.

---

## 1. System Architecture

### 1.1 Governance Runtime Stack (4 Layers)

```
Layer 1: Event Ingestion Layer
├── Git events       → CommitNode emission
├── CI events        → CiRunNode emission
├── Test events      → TestNode + ArtifactNode emission
└── Gate events      → PolicyEvaluationNode emission

Layer 2: Evidence Graph Builder
├── Converts events into immutable graph nodes
├── Maintains edge relationships (append-only)
└── No node modification, no edge rewrites

Layer 3: Graph Validation Engine
├── Verifies reachability constraints
├── Detects orphan nodes (degree=0 → INVALID)
├── Validates Task→Commit→CI→Artifact path completeness
└── Enforces evidence immutability

Layer 4: Gate Evaluation Engine
└── Computes system state via graph traversal ONLY
```

---

## 2. System of Truth

### 2.1 Single Source of Truth

```
TRUTH := Evidence Graph (SQLite-backed graph store)
```

### 2.2 NON-TRUTH SOURCES (Explicitly Deprecated)

| Deprecated Source | Reason |
|-------------------|--------|
| Documents (Markdown/Reports/Plans) | Projection only |
| AI-generated summaries | Unverified interpretation |
| Human-written GA reports | Not graph-bound |
| Shell script outputs (check_*.sh) | Advisory only |
| Logs without graph binding | Isolated assertions |

### 2.3 Document Role

Documents are **Projection Layer** (read-only interpretation of graph state).
They MUST NOT influence system state or gate decisions.

---

## 3. Evidence Graph Model

### 3.1 Node Types

| NodeType | Authority | Description |
|----------|-----------|-------------|
| TaskNode | System (plan ingestion) | Intent/scope declaration |
| CommitNode | Git (immutable) | Code change event |
| CiRunNode | CI system (immutable) | Test execution record |
| TestNode | CI system (immutable) | Individual test result |
| ArtifactNode | CI system (immutable) | Binary/report artifact |
| PolicyEvaluationNode | Gate engine (immutable) | Gate decision record |

### 3.2 Edge Types

```
TaskNode ──IMPLEMENTED_BY──→ CommitNode
CommitNode ──VERIFIED_BY──→ CiRunNode
CiRunNode ──PRODUCES──→ ArtifactNode
ArtifactNode ──VALIDATES──→ TaskNode
TaskNode ──AFFECTS──→ TaskNode (dependency)
```

### 3.3 Immutability Rule

**Once created, nodes and edges are IMMUTABLE:**
- No updates to existing nodes
- No edge rewrites
- No deletions
- Evidence locking enforced at storage layer

---

## 4. Runtime Constraints (Enforceable Rules)

### 4.1 Evidence Authority Rule

ONLY system components may create evidence:

| Authority | May Create |
|-----------|-----------|
| CI system | CiRunNode, TestNode, ArtifactNode |
| Git system | CommitNode |
| Gate engine | PolicyEvaluationNode |
| Plan ingestion | TaskNode |

**AI systems are EXPLICITLY PROHIBITED from generating:**
- CI run IDs
- commit hashes
- artifact hashes
- policy evaluation results
- any node type

### 4.2 AI Capability Restriction

AI is limited to:
- Querying Evidence Graph (read-only)
- Interpreting graph state (explanation)
- Producing derived documents (projections)

AI MUST NOT:
- Declare completion (PASS/FAIL)
- Generate evidence IDs
- Mark tasks as done
- Modify any graph state
- Create nodes or edges

### 4.3 Orphan Node Rule

**Any node with degree = 0 is INVALID.**
```
orphan_node → OUTSIDE GOVERNANCE
```

### 4.4 Missing Path Rule

**Any TaskNode without complete path is INVALID:**
```
Task → Commit → CI → Artifact
```
Missing any segment → INVALID state.

### 4.5 Evidence Locking Rule

Once evidence is created:
```
forbid(modify_node(id))
forbid(delete_node(id))
forbid(modify_edge(from, to))
```

---

## 5. Gate System (Replaces ALL Shell Scripts)

### 5.1 Gate Definition

```
Gate Result = Graph Reachability Function
```

NOT: script output, document declaration, AI assessment.

### 5.2 Formal Gate Rule

A task is considered VALID ONLY IF:

```
∃ path: Task → Commit → CI(status=PASS) → Artifact
```

### 5.3 Gate Computation

```rust
fn gate_validate(task_id: &str, graph: &GraphStore) -> GateResult {
    let (reachable, path) = graph.check_task_completion(task_id)?;
    if reachable && path.len() >= 3 {
        GateResult::PASS
    } else {
        GateResult::FAIL
    }
}
```

### 5.4 Deprecated Systems

| Deprecated | Replacement |
|------------|-------------|
| check_plan_integrity.sh | Graph plan-node binding check |
| check_evidence_binding.sh | Graph reachability computation |
| GA score (numeric) | Graph path existence (boolean) |
| Checklist-based gates | Graph traversal gates |

These scripts may exist as **debugging utilities only**, not as authority.

---

## 6. Plan System (Critical Redesign)

### 6.1 Plan as Root Node

A plan document creates a **TaskNode** as graph root.
The plan itself is immutable base; only append-only logs are permitted.

### 6.2 Forbidden Operations

- Rewriting plan to match outcomes (Type D fraud)
- Changing historical intent
- Overwriting completion status
- Backdating evidence

### 6.3 Plan Truth Rule

```
Plan ⊆ Intent (immutable)
Execution = Evidence Graph (not plan)
```

Plan documents do NOT determine execution status.

---

## 7. Fraud Detection Model

### 7.1 Fraud Type Definitions

| Type | Definition | Detection |
|------|------------|-----------|
| Type A | Execution fabrication — no CI path | Orphan check |
| Type B | Gate fabrication — no graph evidence | Path validation |
| Type C | Evidence fabrication — fake IDs | Authority check |
| Type D | Task completion fraud — no commit linkage | Reachability |

### 7.2 Detection Rule

```
fraud_exists := ∃ declared_state s.t. NOT reachable_in_graph(s)
```

---

## 8. Migration Policy (v3 → v4.1)

### 8.1 Deprecated Concepts

- GA Score (numeric) — REMOVED
- Document-based approval — REMOVED
- Script-based gating as authority — REMOVED

### 8.2 Migration Rule

```
All system state MUST be reconstructed from event history.
```

### 8.3 Replay Requirement

Any historical version validation MUST use:
1. Event replay from authoritative sources
2. Graph reconstruction from events
3. Reachability evaluation (NOT documentation review)

---

## 9. System Enforcement

This specification is **NOT advisory**.

It is a **runtime constraint contract**.

Any system component violating this spec is considered **OUTSIDE GOVERNANCE BOUNDARY**.

---

## 10. Implementation Reference

- **Evidence Graph Core**: `crates/evidence-graph/`
  - `GraphStore`: SQLite-backed graph storage
  - `check_task_completion()`: reachability gate computation
  - `get_orphan_nodes()`: fraud detection
  - `EvidenceIngestor`: authoritative event ingestion
- **Node Types**: Task, Commit, CiRun, Test, Artifact, PolicyEval
- **Edge Types**: IMPLEMENTED_BY, VERIFIED_BY, PRODUCES, VALIDATES, AFFECTS
- **Tests**: 5/5 PASS (commit `08b15fd6`)

---

## 11. Related Documents

- `crates/evidence-graph/src/lib.rs` — Core implementation
- `docs/governance/ANTI_FABRICATION_POLICY.md` — Fraud type definitions
- `docs/governance/AI_GENERATOR_AUDIT_CHECKLIST.md` — AI constraints
- `scripts/gate/check_evidence_binding.sh` — Evidence binding checker
- `scripts/gate/check_plan_integrity.sh` — Plan integrity checker (debug only)