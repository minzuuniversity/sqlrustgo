# OO-CE3: Provenance Knowledge Graph

> **版本**: v1.0
> **日期**: 2026-05-18
> **基于**: v3.3.0
> **维护人**: hermes-agent
> **Issue**: #1240
> **状态**: 新建设计

---

## 一、概述

### 1.1 目标

从审计日志升级为 GMP Knowledge Graph，支持血缘推导、影响分析和合规查询。

### 1.2 核心理念

```
Provenance Graph = Graph DB + Cypher Query + Lineage Derivation + Impact Analysis
```

### 1.3 图结构

```
GMP Knowledge Graph Schema:
═══════════════════════════════════════════════════════════════════

    Operator ──→ SOP ──→ Batch ──→ Device
        │           │          │
        ↓           ↓          ↓
    Signature   Training   Calibration
        │
        ↓
    Deviation ──→ CAPA ──→ Closure
```

---

## 二、架构设计

### 2.1 组件关系

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   Provenance Knowledge Graph                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────────┐   │
│  │   Graph     │───▶│   Cypher    │───▶│   Query Engine             │   │
│  │   Loader   │    │   Parser    │    │                             │   │
│  └─────────────┘    └─────────────┘    └─────────────────────────────┘   │
│         │                                      │                          │
│         ▼                                      ▼                          │
│  ┌─────────────┐                       ┌─────────────────────────────┐   │
│  │  Lineage    │                       │   Visualization API          │   │
│  │  Derivation │                       │   (GraphSON/JSON)            │   │
│  └─────────────┘                       └─────────────────────────────┘   │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────┐   │
│  │                         Graph Storage                               │   │
│  │  - Nodes: Operator, SOP, Batch, Device, Signature, Deviation       │   │
│  │  - Edges: PERFORMED, SIGNED, FOLLOWED, CAUSED, RESOLVED           │   │
│  │  - Properties: timestamp, role, status, hash                      │   │
│  └───────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 组件说明

| 组件 | 说明 | 位置 |
|------|------|------|
| Graph DB | 图数据库 | `crates/provenance-graph/` |
| Cypher Parser | 查询解析器 | `crates/cypher-parser/` |
| Lineage Derivation | 血缘推导 | `crates/lineage-engine/` |
| Visualization API | 可视化 API | `crates/graph-viz-api/` |

---

## 三、图模型

### 3.1 节点类型

```cypher
// Node Types

(:Operator {
  id: ID,
  name: String,
  role: String,
  department: String,
  certifications: [String],
  hire_date: Date
})

(:SOP {
  id: ID,
  title: String,
  version: String,
  effective_date: Date,
  category: String  // manufacturing, quality_control, storage
})

(:Batch {
  id: ID,
  product_name: String,
  batch_size: Integer,
  manufacturing_date: Date,
  expiry_date: Date,
  status: String  // in_progress, released, rejected
})

(:Device {
  id: ID,
  name: String,
  type: String,  // reactor, mixer, fill_line, etc.
  serial_number: String,
  last_calibration: Date
})

(:Signature {
  id: ID,
  type: String,  // approval, review, release
  meaning: String,
  timestamp: DateTime,
  hash: String
})

(:Deviation {
  id: ID,
  type: String,
  severity: String,  // critical, major, minor
  status: String,  // open, investigation, closed
  created_at: DateTime
})

(:CAPA {
  id: ID,
  type: String,  // corrective, preventive
  effectiveness: String,
  status: String,
  created_at: DateTime
})
```

### 3.2 边类型

```cypher
// Edge Types

// Operator performs Batch operations
(Operator)-[:PERFORMED {
  role: String,
  timestamp: DateTime,
  task: String
}]->(Batch)

// Operator signed the record
(Operator)-[:SIGNED {
  meaning: String,
  timestamp: DateTime,
  hash: String
}]->(Signature)

// Operator followed SOP
(Operator)-[:FOLLOWED {
  timestamp: DateTime,
  completion_status: String
}]->(SOP)

// Batch manufactured according to SOP
(Batch)-[:MANUFACTURED_ACCORDING_TO {
  version_used: String
}]->(SOP)

// Device used in Batch production
(Device)-[:USED_IN {
  timestamp: DateTime,
  operation: String
}]->(Batch)

// Device calibrated
(Device)-[:CALIBRATED {
  timestamp: DateTime,
  technician: ID,
  result: String
}]->(:Calibration)

// Signature applies to Batch
(Signature)-[:APPLIES_TO {
  context: String
}]->(Batch)

// Deviation caused by Batch
(Batch)-[:CAUSED {
  deviation_id: ID
}]->(Deviation)

// Deviation requires CAPA
(Deviation)-[:REQUIRES {
  urgency: String
}]->(CAPA)

// CAPA resolves Deviation
(CAPA)-[:RESOLVES {
  resolution_date: DateTime
}]->(Deviation)
```

---

## 四、查询示例

### 4.1 批次血缘查询

```cypher
// Query: Batch 2026-001 的完整血缘
MATCH path = (b:Batch {id: '2026-001'})-[:MANUFACTURED_ACCORDING_TO*1..5]-()
RETURN path

// 结果:
// Batch → SOP (manufacturing process)
// Batch → Device (equipment used)
// Batch → Operator (who operated)
// Batch → Signature (who signed)
// Batch → Deviation (any issues)
```

### 4.2 签名链追溯

```cypher
// Query: 追溯签名的完整链
MATCH (s:Signature {id: 'sig_001'})-[:APPLIES_TO]->(b:Batch)
MATCH (op:Operator)-[:SIGNED]->(s)
MATCH (s2:Signature)-[:APPLIES_TO]->(b) WHERE s2.timestamp < s.timestamp
RETURN op, s, b, s2
ORDER BY s2.timestamp
```

### 4.3 影响分析

```cypher
// Query: Operator John 的所有操作影响
MATCH (op:Operator {name: 'John Smith'})-[r]->(n)
WHERE type(r) IN ['PERFORMED', 'SIGNED', 'FOLLOWED']
RETURN n, type(r), r.timestamp
ORDER BY r.timestamp DESC
```

### 4.4 偏差追溯

```cypher
// Query: 偏差的根因追溯
MATCH path = (d:Deviation {id: 'DEV_001'})<-[:CAUSED]-(b:Batch)
MATCH (b)-[:MANUFACTURED_ACCORDING_TO]->(s:SOP)
MATCH (op:Operator)-[:FOLLOWED]->(s)
RETURN path, op, s
```

---

## 五、执行流程

### 5.1 Graph 加载流程

```
┌─────────────────────────────────────────────────────────────────┐
│                  Graph Loading Flow                               │
└─────────────────────────────────────────────────────────────────┘

Audit Log Entry Created
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Parse Entry                                                   │
│     - Extract entity type (Operator, Batch, Device, etc.)       │
│     - Extract properties                                         │
│     - Identify relationships                                     │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. Node Upsert                                                   │
│     - Check if node exists                                       │
│     - If not, create with properties                             │
│     - Update properties if needed                               │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Edge Creation                                                │
│     - Create edge with properties                                │
│     - Link source and target nodes                               │
│     - Add timestamp and metadata                                │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. Index Update                                                 │
│     - Update graph indices                                        │
│     - Rebuild materialized paths if needed                       │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 Lineage Derivation 流程

```
┌─────────────────────────────────────────────────────────────────┐
│               Lineage Derivation Flow                             │
└─────────────────────────────────────────────────────────────────┘

Query: Batch X 的血缘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Find Source Node                                             │
│     - Locate Batch node by ID                                    │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. Traverse Outbound Edges                                      │
│     - Follow :MANUFACTURED_ACCORDING_TO to SOP                  │
│     - Follow :USED_IN to Device                                  │
│     - Follow :CAUSED to Deviation                                │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Traverse Inbound Edges                                       │
│     - Follow :PERFORMED from Operator                            │
│     - Follow :SIGNED from Signature                              │
│     - Follow :APPLIES_TO from Signature                         │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. Build Lineage Graph                                           │
│     - Assemble nodes and edges                                   │
│     - Compute timestamps                                         │
│     - Generate visualization data                                │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
    Return: LineageGraph (JSON/GraphSON)
```

---

## 六、Cypher 支持范围

### 6.1 支持的操作

| 操作类型 | 支持 | 说明 |
|---------|------|------|
| MATCH | ✅ | 节点和边匹配 |
| WHERE | ✅ | 条件过滤 |
| RETURN | ✅ | 结果返回 |
| ORDER BY | ✅ | 排序 |
| LIMIT/SKIP | ✅ | 分页 |
| WITH | ✅ | 链式查询 |
| UNION | ✅ | 结果合并 |
| OPTIONAL MATCH | ✅ | 可选匹配 |
| COLLECT | ✅ | 聚合 |
| COUNT | ✅ | 计数 |

### 6.2 支持的函数

| 函数类型 | 支持 | 说明 |
|---------|------|------|
| String | ✅ | toString, substring, etc. |
| Math | ✅ | length, size, etc. |
| DateTime | ✅ | timestamp, date, etc. |
| Collection | ✅ | head, tail, etc. |
| Graph | ✅ | nodes(), relationships() |

---

## 七、验收标准

### 7.1 功能验收

| 检查项 | 命令 | 标准 |
|--------|------|------|
| 图加载 | `graph-loader import audit_log.json` | 成功加载 |
| Cypher 查询 | `cypher query "MATCH (b:Batch) RETURN b"` | 返回结果 |
| 血缘推导 | `lineage derive --batch-id 2026-001` | 返回完整血缘 |
| 影响分析 | `impact analyze --operator-id op_001` | 返回影响范围 |
| 可视化 | `graph-viz export --batch-id 2026-001` | 生成 GraphSON |

### 7.2 性能验收

| 指标 | 标准 |
|------|------|
| 图加载速度 | > 10,000 节点/秒 |
| 查询延迟 (P99) | < 100ms |
| 血缘推导延迟 | < 500ms (1000 节点) |

---

## 八、相关文档

- `docs/releases/v3.3.0/TRUST_INFRASTRUCTURE_STRATEGY.md` - 战略定位
- `crates/provenance-graph/` - 图数据库实现
- `crates/cypher-parser/` - Cypher 解析器
- `crates/lineage-engine/` - 血缘引擎

---

*本文档由 hermes-agent 生成*
*版本 1.0 - 2026-05-18*
