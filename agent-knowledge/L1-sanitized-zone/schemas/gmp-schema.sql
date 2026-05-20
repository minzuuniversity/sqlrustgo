# 数据库 Schema（脱敏版）

> 来源: gmp-qa-package/data/gmp.db + graph.db
> 隔离层级: L1-sanitized-zone（Schema 保留，值删除，可出云）
> 提取日期: 2026-05-20

---

## gmp_documents 表

```sql
CREATE TABLE gmp_documents (
    id          INTEGER PRIMARY KEY,
    title       TEXT,           -- 文档标题
    content     TEXT,           -- 文档内容（脱敏后仅保留结构）
    chapter     TEXT,           -- 章节编码
    department  TEXT,           -- 归口部门
    category    TEXT,           -- 文档类别
    updated_at  TIMESTAMP
);
```

## gmp_embeddings 表

```sql
CREATE TABLE gmp_embeddings (
    doc_id      INTEGER REFERENCES gmp_documents(id),
    embedding   BLOB,           -- bge-m3 1024维 f32 向量
    updated_at  TIMESTAMP
);
```

## kg_nodes 表（知识图谱节点）

```sql
CREATE TABLE kg_nodes (
    id          SERIAL PRIMARY KEY,
    node_type   TEXT NOT NULL,  -- sop/guide/form/dept/role/device/material/deviation/capa/clause
    name        TEXT NOT NULL,
    code        TEXT,
    version     TEXT,
    date        TEXT,
    dept        TEXT,
    properties  JSONB,
    created_at  TIMESTAMP DEFAULT NOW()
);
```

## kg_relations 表（知识图谱关系）

```sql
CREATE TABLE kg_relations (
    id          SERIAL PRIMARY KEY,
    from_node   INTEGER REFERENCES kg_nodes(id),
    to_node     INTEGER REFERENCES kg_nodes(id),
    rel_type    TEXT NOT NULL,  -- BELONGS/FOLLOWS/CREATES/RELATED/PROCED/PART_OF
    weight      FLOAT DEFAULT 1.0,
    properties  JSONB,
    created_at  TIMESTAMP DEFAULT NOW(),
    UNIQUE(from_node, to_node, rel_type)
);
```

## Agent 规则引擎表（新增）

```sql
CREATE TABLE agent_rules (
    id          SERIAL PRIMARY KEY,
    rule_id     TEXT NOT NULL UNIQUE,
    description TEXT,
    applies_to  TEXT,
    trigger     TEXT,
    conditions  JSONB,          -- 规则条件（JSON）
    sop_ref     TEXT,
    severity    TEXT,
    enabled     BOOLEAN DEFAULT TRUE,
    created_at  TIMESTAMP DEFAULT NOW()
);

CREATE TABLE agent_workflows (
    id              SERIAL PRIMARY KEY,
    workflow_id     TEXT NOT NULL UNIQUE,
    name            TEXT,
    description     TEXT,
    steps           JSONB,      -- 流程步骤（JSON）
    version         TEXT,
    enabled         BOOLEAN DEFAULT TRUE,
    created_at      TIMESTAMP DEFAULT NOW()
);

CREATE TABLE evidence_records (
    id              SERIAL PRIMARY KEY,
    record_type     TEXT NOT NULL,   -- weighing/deviation/capa/inspection
    workflow_id     TEXT,
    step_id         TEXT,
    operator_code   TEXT,
    evidence_data   JSONB,          -- 证据数据（脱敏后）
    audit_chain_ref TEXT,           -- SQLRustGo 审计链引用
    created_at      TIMESTAMP DEFAULT NOW()
);
```
