# SQLRustGo 版本演化计划

> **版本**: v4.0.0
> **更新日期**: 2026-09-12
> **战略定位**: 生产级多模型数据库（SQL + Vector + Graph + GMP）
> **核心原则**: 统一事务、WAL、备份恢复、访问控制、可观测性

---

## 一、战略定位（关键转向）

### 从：单一 SQL 数据库 + GMP 内部能力
### 到：生产级多模型数据库（SQL + Vector + Graph + GMP）

```
核心洞察：

❌ 不要试图成为通用向量数据库或图数据库
✅ 正确策略：统一多模型能力，服务 GMP 合规性内审场景

最终目标：

v3.12.0 = GMP 内审检索系统（内部能力验证）
v4.0.0 = 一等公民多模型数据库（生产级）
```

---

## 二、版本语义化与技术范式

| 主版本 | 技术范式 | 成熟度等级 | 核心特征 | 对标系统 |
|--------|----------|------------|----------|----------|
| 1.x | 单机基础数据库 | L3 | 火山模型、SQL-92、MVCC、B+Tree | early PostgreSQL |
| 2.x | 高性能分析引擎 | L4 | 向量化、列存、CBO、并行执行 | DuckDB |
| 3.x | GMP 合规性内审检索系统 | L3+ | SQL + Vector + Graph 内部能力 | 领域定制 |
| **4.x** | **生产级多模型数据库** | **L4** | **一等公民 SQL/Vector/Graph/GMP** | **统一多模型** |

**演进路径**：3.x（内部验证）→ 4.x（生产级统一多模型）

---

## 三、版本路线（多模型战略）

### 战略总览

```
v3.12.0 ✅      v4.0.0 (开发中)
  GMP 内部验证      多模型生产级
```

### 详细版本规划

| 版本 | 代号 | 核心目标 | 关键功能 | 状态 |
|------|------|----------|----------|------|
| **v3.11.0** | **生产稳定** | MySQL 5.7 替代基础 | TPC-H 22/22、SQLLogicTest、SOAK | ✅ GA (2026-08-09) |
| **v3.12.0** | **GMP 内审检索** | SQL + Vector + Graph 内部能力 | GMP schema、混合检索、图投影、ALCOA+ 审计 | ✅ GA (2026-09-08) |
| **v4.0.0** | **多模型生产级** | 统一多模型数据库 | 一等公民 Vector/Graph、跨模型事务、统一备份恢复 | 🔄 开发中 |

### v4.0.0 开发任务

| 类别 | 功能 | Issue |
|------|------|-------|
| P0 | Vector column 与 vector index SQL syntax | V400-01 |
| P0 | WAL-backed vector storage 与 rebuild | V400-02 |
| P0 | Graph crate revival 或 rewrite | V400-03 |
| P0 | Graph query surface | V400-04 |
| P0 | Cross-model transaction semantics | V400-05 |
| P0 | Unified backup/restore | V400-06 |
| P0 | Unified ACL and audit | V400-07 |
| P1 | Multi-model optimizer 与 metadata filters | V400-08 |
| P0 | Multi-model SOAK | V400-09 |

---

## 四、v3.12.0 核心版本详解（GMP 内审检索系统）

### GA 状态（2026-09-08）

| 指标 | 状态 |
|------|------|
| GA Gate | 72/72 PASS |
| TPC-H SF=1 | 22/22 PASS |
| SQLLogicTest smoke | 25/25 PASS |
| SOAK | 168h PASS |
| 3 个 GA-claim-caveat | #4846, #4847, #4848 |

### Epic-01~04: GMP Schema 与摄取

| 功能 | 说明 |
|------|------|
| GMP document table | 文档存储 |
| Chunk table | 分块存储 |
| Embedding table | 向量存储 |
| Audit event table | 审计事件 |

### Epic-05: 混合检索

```sql
SELECT doc_id, text, similarity(embedding, ?) as score
FROM documents
WHERE metadata @> '{"type": "SOP"}'
ORDER BY score DESC
LIMIT 10;
```

### Epic-06: 图投影

```sql
SELECT * FROM graph_neighbors('evidence_node', 'CAPA-001', 2);
```

### Epic-07: ALCOA+ 审计

- Hash-chain tamper detection
- Immutable audit trail
- Export audit logs

---

## 五、v4.0.0 核心版本详解（多模型生产级）

### 一等公民多模型能力

#### 1️⃣ Vector DB（核心革命）

```sql
CREATE TABLE products (
    id INT PRIMARY KEY,
    name VARCHAR(255),
    description TEXT,
    embedding VECTOR(768)
);

CREATE INDEX ON products USING HNSW (embedding);
```

#### 2️⃣ Graph DB（图数据库）

```sql
CREATE GRAPH gmp_navigation;

CREATE NODE TABLE user(id INT, name VARCHAR);
CREATE EDGE TABLE knows(FROM user TO user, weight FLOAT);

SELECT * FROM graph_bfs('user', 1, 'id = 100');
```

#### 3️⃣ 统一事务边界

> 关键保证：
> - SQL、Vector、Graph、GMP audit writes 共享同一个 transaction boundary
> - WAL-backed recovery
> - Backup/restore 重建 vector 和 graph indexes

---

## 六、可观测性演进

| 版本 | 新增可观测能力 |
|------|----------------|
| v3.11 | **TPC-H SF=1**（性能基线）, **168h SOAK**（稳定性） |
| v3.12 | **GMP audit hash-chain**（完整性）, **多模型 metrics** |
| v4.0 | **统一多模型可观测性面板** |

---

## 七、成熟度演进

```
L1 (Toy)   →   L2 (Query Engine)   →   L3 (Mini DBMS)   →   L4 (Multi-Model DB)
                   v1.7                    v3.x                v4.0
               MySQL 教学替代          GMP 内审检索        多模型生产级
```

---

## 八、风险与应对

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Vector/Graph 性能不达标 | 高 | 使用成熟 HNSW 库，参考 pgvector |
| 跨模型事务复杂度 | 高 | 先做 SQL+Vector 统一，再扩展 Graph |
| 多模型一致性问题 | 中 | 统一 WAL 和事务边界 |
| 教学文档工作量 | 中 | 参考 v3.12 文档结构 |

---

## 九、版本号与分支策略

| 策略 | 说明 |
|------|------|
| 开发分支 | develop/v3.12.0, develop/v4.0.0 |
| 发布分支 | release/v3.12.0, release/v4.0.0 |
| 主分支 | main 始终指向最新稳定版 |

---

## 十、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-03-12 | 初始版本 |
| 2.0 | 2026-03-13 | v1.x/v2.x/v3.x 重构 |
| 3.0 | 2026-03-13 | 工程优化版 |
| 4.0 | 2026-03-18 | 整合 v1.x 版本 |
| 5.0 | 2026-03-21 | SQL-92 路线图 |
| 6.0 | 2026-03-21 | 教学 DBMS 战略定位 |
| 7.0 | 2026-03-21 | v1.7 合并 v1.8+v1.9，v2.0 独立 |
| 8.0 | 2026-04-09 | v2.4.0 Graph Engine + OpenClaw |
| 9.0 | 2026-05-30 | 集成债务清算 |
| 10.0 | 2026-07-13 | MySQL 5.7 替代基础 |
| 11.0 | 2026-08-09 | 生产稳定版 |
| **12.0** | **2026-09-08** | **GMP 内审检索系统** |
| **13.0** | **规划中** | **多模型能力增强** |

---

**文档状态**: 有效
**制定日期**: 2026-03-21
**最后更新**: 2026-09-12
**制定人**: claude-macmini
**战略定位**: 生产级多模型数据库（SQL + Vector + Graph + GMP）
