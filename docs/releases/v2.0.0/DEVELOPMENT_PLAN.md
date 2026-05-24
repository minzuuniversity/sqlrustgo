# GMP-Platform v2.0.0 开发计划 — GMP 综合管理平台

> **版本**: 1.0
> **日期**: 2026-05-24
> **分支**: `develop/v2.0.0`（计划）
> **目标**: GMP 综合管理平台，涵盖供应商、物料、偏差、CAPA、变更、审计等核心质量流程

---

## 一、愿景与目标

### 1.1 愿景

一个基于 AI 检索与知识图谱的 GMP 综合管理平台，覆盖供应商、物料、偏差、CAPA、变更、审计等核心质量流程，提供"记录即合规、查询即答案"的智能体验。

### 1.2 核心目标

| 目标 | 描述 |
|------|------|
| 功能完整 | 8 大核心模块，覆盖 GMP 全生命周期 |
| 智能化 | AI 检索 + LLM 问答 + 知识图谱 |
| 平台化 | 微服务架构，支持多租户扩展 |
| 合规性 | 审计链完整，数据可追溯 |

---

## 二、功能模块全景

```
┌─────────────────────────────────────────────────────────────────────┐
│                        GMP 综合管理平台 v2.0                          │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐      │
│  │   供应商管理    │  │   物料管理      │  │   偏差管理      │      │
│  │  - 供应商档案   │  │  - 物料主数据   │  │  - 偏差报告     │      │
│  │  - 资质管理     │  │  - 批次库存     │  │  - 调查分析     │      │
│  │  - 审计评估     │  │  - BOM 配方     │  │  - 分级审批     │      │
│  │  - 绩效评分     │  │  - 有效期预警   │  │  - 关联 CAPA   │      │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐      │
│  │   CAPA 管理     │  │   变更控制      │  │   审计管理      │      │
│  │  - CAPA 创建    │  │  - 变更申请     │  │  - 审计计划     │      │
│  │  - 措施跟踪     │  │  - 影响评估     │  │  - 检查表       │      │
│  │  - 效果验证     │  │  - 批准执行     │  │  - 发现项跟踪   │      │
│  │  - 关联偏差     │  │  - 关联偏差/CAPA│  │  - 报告生成     │      │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐      │
│  │   培训管理      │  │   设备管理      │  │   文档管理      │      │
│  │  - 培训矩阵     │  │  - 设备台账     │  │  - 文件生命周期 │      │
│  │  - 课程记录     │  │  - 校准维护     │  │  - 版本控制     │      │
│  │  - 资质到期     │  │  - 状态监控     │  │  - 智能检索     │      │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘      │
├─────────────────────────────────────────────────────────────────────┤
│                    统一能力层 (基于 v1.3 基座)                        │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  智能检索 (语义 + 关键词 + 图谱)  │  LLM 问答  │  审计链     │    │
│  └─────────────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  离线知识管线 (NER/RE/分类/预评分)  │  规则引擎             │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 三、模块开发顺序

| Phase | 模块 | 周期 | 产出 |
|-------|------|------|------|
| **Phase 1** | **供应商管理 + 物料管理** | **5 周** | 两模块完整功能 + 与现有检索集成 |
| Phase 2 | 偏差管理 + CAPA 管理 | 5 周 | 偏差-CAPA 闭环 |
| Phase 3 | 变更控制 + 审计管理 | 4 周 | 变更-审计流程 |
| Phase 4 | 培训管理 + 设备管理 + 仪表板 | 4 周 | 综合管理门户 |

**总周期**: 18 周（约 4.5 个月）

---

## 四、供应商管理模块详细设计

### 4.1 功能需求

| ID | 功能 | 描述 | 优先级 |
|----|------|------|--------|
| S-01 | 供应商档案 | 基本信息、资质文件、联系信息 | P0 |
| S-02 | 供应商分类 | 按物料类型、风险等级、区域分类 | P0 |
| S-03 | 资质管理 | 证书（GMP、ISO）有效期跟踪、到期提醒 | P0 |
| S-04 | 审计评估 | 审计计划、检查表、评分、结论 | P0 |
| S-05 | 绩效评分 | 月度/季度质量、交付、服务评分 | P1 |
| S-06 | 物料关联 | 供应商-物料多对多关系 | P0 |
| S-07 | 黑名单/暂停 | 不合格供应商状态控制 | P1 |
| S-08 | 审计追踪 | 所有操作记录至审计链 | P0 |
| S-09 | 智能检索 | 供应商信息可通过自然语言查询 | P1 |

### 4.2 数据模型

```sql
-- 供应商主表
CREATE TABLE suppliers (
    id              INTEGER PRIMARY KEY,
    supplier_code   TEXT UNIQUE NOT NULL,   -- SUP-0001
    name            TEXT NOT NULL,
    name_en         TEXT,
    tax_id          TEXT,                   -- 统一社会信用代码
    address         TEXT,
    contact_person  TEXT,
    contact_phone   TEXT,
    contact_email   TEXT,
    risk_level      TEXT DEFAULT 'medium',  -- high/medium/low
    status          TEXT DEFAULT 'active',   -- active/suspended/blacklisted
    category        TEXT,                   -- 原料/辅料/包材/设备/服务
    remark          TEXT,
    created_at      INTEGER,
    updated_at      INTEGER,
    created_by      TEXT,
    updated_by      TEXT
);

-- 供应商资质证书
CREATE TABLE supplier_certificates (
    id              INTEGER PRIMARY KEY,
    supplier_id     INTEGER NOT NULL REFERENCES suppliers(id),
    cert_type       TEXT NOT NULL,          -- GMP/ISO9001/ISO14001...
    cert_number     TEXT,
    issue_date      INTEGER,
    expiry_date     INTEGER,
    attachment_path TEXT,
    status          TEXT DEFAULT 'valid',   -- valid/expired/revoked
    verified_at     INTEGER,
    verified_by     TEXT,
    created_at      INTEGER
);

-- 供应商审计记录
CREATE TABLE supplier_audits (
    id              INTEGER PRIMARY KEY,
    supplier_id     INTEGER NOT NULL REFERENCES suppliers(id),
    audit_date      INTEGER NOT NULL,
    audit_type      TEXT,                   -- initial/follow-up/special
    auditor         TEXT,
    checklist_ref   TEXT,
    score           INTEGER,                -- 百分制
    conclusion      TEXT,                   -- pass/conditional_pass/fail
    findings        TEXT,
    capa_required   BOOLEAN DEFAULT FALSE,
    next_audit_date INTEGER,
    report_path     TEXT,
    created_at      INTEGER,
    created_by      TEXT
);

-- 供应商绩效评分
CREATE TABLE supplier_performance (
    id              INTEGER PRIMARY KEY,
    supplier_id     INTEGER NOT NULL REFERENCES suppliers(id),
    period_year     INTEGER NOT NULL,
    period_month    INTEGER NOT NULL,
    quality_score   REAL,                   -- 0-100
    delivery_score  REAL,
    service_score   REAL,
    overall_score   REAL,
    comments        TEXT,
    created_at      INTEGER,
    UNIQUE(supplier_id, period_year, period_month)
);

-- 供应商-物料关联
CREATE TABLE supplier_materials (
    supplier_id     INTEGER NOT NULL REFERENCES suppliers(id),
    material_id     INTEGER NOT NULL REFERENCES materials(id),
    is_preferred    BOOLEAN DEFAULT FALSE,
    status          TEXT DEFAULT 'approved', -- approved/qualified/trial/blocked
    approved_at     INTEGER,
    approved_by     TEXT,
    PRIMARY KEY (supplier_id, material_id)
);
```

### 4.3 业务流程

```
供应商注册 → 资质审核 → 审计评估 → 批准加入合格名录
     ↓                                      ↓
  绩效评估 ← 物料关联 ← 定期再审计     不合格 → 暂停/移除
     ↓
   智能检索（自然语言查询供应商状态）
```

### 4.4 API 设计

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v2/suppliers` | GET | 列表（支持分页、筛选、搜索） |
| `/api/v2/suppliers` | POST | 创建供应商 |
| `/api/v2/suppliers/{id}` | GET | 详情 |
| `/api/v2/suppliers/{id}` | PUT | 更新 |
| `/api/v2/suppliers/{id}` | DELETE | 删除（逻辑） |
| `/api/v2/suppliers/{id}/certificates` | GET/POST | 证书管理 |
| `/api/v2/suppliers/{id}/audits` | GET/POST | 审计记录 |
| `/api/v2/suppliers/{id}/performance` | GET/POST | 绩效评分 |
| `/api/v2/suppliers/{id}/materials` | GET/PUT | 物料关联 |
| `/api/v2/suppliers/search` | POST | 智能检索（自然语言） |

### 4.5 与现有系统集成

| 集成点 | 说明 |
|--------|------|
| 图谱实体 | 供应商作为 Supplier 节点，关联 Material、Audit、CAPA 节点 |
| 检索增强 | 供应商档案内容索引至 FTS/向量，支持语义搜索 |
| 审计链 | 所有创建/更新/删除操作写入 audit_chain |
| 规则引擎 | 资质到期提醒、绩效预警规则 |
| LLM 问答 | "XX 供应商的审计结论是什么？" → 从图谱/文档中提取 |

### 4.6 实现计划（2.5 周）

| 阶段 | 任务 | 产出 | 工时 |
|------|------|------|------|
| 1 | 数据模型 + 迁移脚本 | SQL 迁移文件 | 1d |
| 2 | CRUD API 实现 | gmp-supplier crate | 2d |
| 3 | 证书管理 + 到期提醒 | 定时任务 + 通知 | 2d |
| 4 | 审计评估模块 | 审计记录 + 检查表 | 2d |
| 5 | 绩效评分模块 | 评分计算 + 仪表板 | 1.5d |
| 6 | 物料关联 + 图谱集成 | 图节点同步 | 1.5d |
| 7 | 智能检索集成 | 向量化 + FTS 索引 | 1d |
| 8 | 单元测试 + 集成测试 | 覆盖率 ≥ 80% | 1d |
| 9 | 文档 + API 规范 | OpenAPI 更新 | 0.5d |

---

## 五、物料管理模块详细设计

### 5.1 功能需求

| ID | 功能 | 描述 | 优先级 |
|----|------|------|--------|
| M-01 | 物料主数据 | 编码、名称、规格、单位、分类 | P0 |
| M-02 | 物料分类 | 原料/辅料/包材/中间体/成品/试剂 | P0 |
| M-03 | 规格标准 | 质量标准、检验方法、存储条件 | P0 |
| M-04 | 库存管理 | 批次、数量、货位、有效期 | P0 |
| M-05 | 物料状态 | 待检/合格/不合格/冻结/退货 | P0 |
| M-06 | 批号管理 | 唯一批号生成、追溯 | P0 |
| M-07 | 物料清单 (BOM) | 产品配方组成 | P1 |
| M-08 | 物料替换/变更 | 替代物料管理 | P1 |
| M-09 | 有效期管理 | 近效期预警、复验提醒 | P0 |
| M-10 | 检验记录 | 与 QC 模块关联 | P1 |
| M-11 | 智能检索 | "查询所有有效期小于 30 天的原料" | P1 |

### 5.2 数据模型

```sql
-- 物料主表
CREATE TABLE materials (
    id              INTEGER PRIMARY KEY,
    material_code   TEXT UNIQUE NOT NULL,   -- MAT-0001
    name            TEXT NOT NULL,
    name_en         TEXT,
    category        TEXT NOT NULL,          -- raw/auxiliary/packaging/intermediate/finished/reagent
    sub_category    TEXT,
    unit            TEXT NOT NULL,          -- kg/g/L/个/箱
    specification   TEXT,
    storage_condition TEXT,                 -- 常温/冷藏/冷冻/避光
    shelf_life_days INTEGER,
    retest_period_days INTEGER,
    safety_class    TEXT,                   -- 非/一般/高
    gmp_grade       TEXT,                   -- A/B/C/D
    status          TEXT DEFAULT 'active',  -- active/inactive
    min_stock       REAL,
    max_stock       REAL,
    reorder_point   REAL,
    remark          TEXT,
    created_at      INTEGER,
    updated_at      INTEGER,
    created_by      TEXT,
    updated_by      TEXT
);

-- 物料质量标准
CREATE TABLE material_specifications (
    id              INTEGER PRIMARY KEY,
    material_id     INTEGER NOT NULL REFERENCES materials(id),
    test_item       TEXT NOT NULL,
    standard_value  TEXT,
    test_method     TEXT,
    unit            TEXT,
    is_critical     BOOLEAN DEFAULT FALSE,
    created_at      INTEGER
);

-- 物料批次/库存
CREATE TABLE material_batches (
    id              INTEGER PRIMARY KEY,
    material_id     INTEGER NOT NULL REFERENCES materials(id),
    batch_no        TEXT NOT NULL,
    supplier_id     INTEGER REFERENCES suppliers(id),
    quantity        REAL NOT NULL,
    unit            TEXT NOT NULL,
    manufacture_date INTEGER,
    expiry_date     INTEGER,
    received_date   INTEGER,
    storage_location TEXT,
    quality_status  TEXT DEFAULT 'pending', -- pending/qualified/unqualified/frozen
    coa_path        TEXT,
    remark          TEXT,
    created_at      INTEGER,
    UNIQUE(material_id, batch_no)
);

-- 库存交易记录
CREATE TABLE inventory_transactions (
    id              INTEGER PRIMARY KEY,
    batch_id        INTEGER NOT NULL REFERENCES material_batches(id),
    transaction_type TEXT NOT NULL,         -- receipt/issue/return/adjustment/transfer
    quantity        REAL NOT NULL,
    transaction_date INTEGER NOT NULL,
    reference_type  TEXT,
    reference_id    INTEGER,
    created_by      TEXT,
    created_at      INTEGER
);

-- 物料替代/替换关系
CREATE TABLE material_substitutes (
    primary_material_id   INTEGER NOT NULL REFERENCES materials(id),
    substitute_material_id INTEGER NOT NULL REFERENCES materials(id),
    priority              INTEGER DEFAULT 1,
    approved_at           INTEGER,
    approved_by           TEXT,
    PRIMARY KEY (primary_material_id, substitute_material_id)
);

-- 物料清单 (BOM)
CREATE TABLE bill_of_materials (
    id              INTEGER PRIMARY KEY,
    product_id      INTEGER NOT NULL REFERENCES materials(id),
    component_id    INTEGER NOT NULL REFERENCES materials(id),
    quantity_per_unit REAL NOT NULL,
    unit            TEXT NOT NULL,
    scrap_rate      REAL DEFAULT 0,
    effective_date  INTEGER,
    expiry_date     INTEGER,
    created_at      INTEGER
);
```

### 5.3 业务流程

```
物料主数据创建 → 规格标准定义 → 入库检验
                                         ↓
库存更新 ← 合格入库 ← 质量判定 → 不合格处理
     ↓
生产领料 → 出库扣减 → 库存预警（低库存/近效期）
     ↓
定期复验/报废
```

### 5.4 API 设计

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v2/materials` | GET | 物料列表 |
| `/api/v2/materials` | POST | 创建物料 |
| `/api/v2/materials/{id}` | GET | 详情 |
| `/api/v2/materials/{id}` | PUT | 更新 |
| `/api/v2/materials/{id}/specs` | GET/POST | 质量标准 |
| `/api/v2/materials/batches` | GET | 批次列表 |
| `/api/v2/materials/batches` | POST | 入库（创建批次） |
| `/api/v2/materials/batches/{id}/status` | PUT | 更新质量状态 |
| `/api/v2/materials/batches/{id}/transactions` | GET | 交易记录 |
| `/api/v2/materials/inventory/alert` | GET | 库存预警 |
| `/api/v2/materials/bom` | GET/POST | BOM 管理 |
| `/api/v2/materials/search` | POST | 智能检索 |

### 5.5 与供应商管理集成

```
suppliers ←→ supplier_materials ←→ materials
                                    ↑
                    material_batches ← supplier_id
```
- 供应商绩效中自动计算某供应商供货的物料合格率
- 物料入库时验证供应商是否在合格名录
- 供应商暂停时，关联物料标记为"建议暂停使用"

### 5.6 实现计划（2.5 周）

| 阶段 | 任务 | 产出 | 工时 |
|------|------|------|------|
| 1 | 数据模型 + 迁移脚本 | SQL 迁移 + 索引 | 1d |
| 2 | CRUD API 实现 | gmp-material crate | 2d |
| 3 | 批次管理 + 库存交易 | 批次 CRUD + 交易流水 | 2d |
| 4 | 质量标准管理 | 规格 CRUD + 检验项 | 1.5d |
| 5 | 库存预警（低库存/近效期） | 定时任务 + 通知 | 1.5d |
| 6 | BOM 管理 | 物料清单 CRUD | 1d |
| 7 | 与供应商模块集成 | 关联逻辑 + 联合查询 | 1d |
| 8 | 智能检索集成 | 向量化 + FTS 索引 | 1d |
| 9 | 单元测试 + 集成测试 | 覆盖率 ≥ 80% | 1d |
| 10 | 文档 + API 规范 | OpenAPI 更新 | 0.5d |

---

## 六、开发时间线

```
2026-07-01        2026-08-05        2026-09-09        2026-10-14
      │                 │                 │                 │
      ▼                 ▼                 ▼                 ▼
┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│   Phase 1   │   │   Phase 2   │   │   Phase 3   │   │   Phase 4   │
│ 供应商+物料 │──▶│ 偏差+CAPA   │──▶│ 变更+审计   │──▶│ 培训+设备   │
│   (5周)     │   │   (5周)     │   │   (4周)     │   │   (4周)     │
└─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘
      │                 │                 │                 │
      ▼                 ▼                 ▼                 ▼
  v2.0-alpha        v2.0-beta         v2.0-rc          v2.0 GA
  (供应商物料        (偏差CAPA闭环)     (完整流程)        (综合平台)
   基础版)
```

---

## 七、测试计划

### 7.1 Phase 1 测试计划（供应商 + 物料）

| 测试类型 | 范围 | 方法 | 覆盖率目标 |
|---------|------|------|-----------|
| 单元测试 | CRUD、规则引擎、评分计算 | cargo test | ≥ 80% |
| 集成测试 | 供应商-物料关联、图谱同步 | cargo test --test integration | 100% |
| API 测试 | 所有 v2 端点 | REST API 调用 | ≥ 90% |
| 回归测试 | 408 场景 | eval_hybrid_search.py | ≥ 97% |
| E2E 测试 | 完整业务流程 | gmp-e2e TS-SUP-001~010 | 全部 |

### 7.2 测试用例示例

```bash
# 供应商管理
TS-SUP-001: 创建供应商 → 验证数据库记录
TS-SUP-002: 添加证书 → 验证到期提醒触发
TS-SUP-003: 审计评估 → 验证评分计算
TS-SUP-004: 智能检索 "审计结论为通过的供应商" → 验证结果
TS-SUP-005: 供应商暂停 → 验证关联物料状态变更

# 物料管理
TS-MAT-001: 创建物料 + 规格标准 → 验证数据库
TS-MAT-002: 批次入库 → 验证库存数量
TS-MAT-003: 出库扣减 → 验证交易流水
TS-MAT-004: 低库存预警 → 验证通知触发
TS-MAT-005: 近效期预警 → 验证通知触发
TS-MAT-006: BOM 展开 → 验证子物料计算
TS-MAT-007: 智能检索 "有效期 < 30 天的原料" → 验证结果
```

---

## 八、技术架构

### 8.1 Crate 划分

| Crate | 职责 |
|-------|------|
| gmp-supplier | 供应商管理模块 |
| gmp-material | 物料管理模块 |
| gmp-deviation | 偏差管理模块 |
| gmp-capa | CAPA 管理模块 |
| gmp-change | 变更控制模块 |
| gmp-audit | 审计管理模块 |
| gmp-training | 培训管理模块 |
| gmp-equipment | 设备管理模块 |
| gmp-document | 文档管理模块 |
| gmp-api | 统一 REST API 网关 |

### 8.2 数据流向

```
┌─────────────────────────────────────────────────────────────┐
│                      gmp-api (网关)                         │
│                  统一认证 + 路由 + 审计链                     │
└──────────────────────────┬────────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        ▼                  ▼                  ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│gmp-supplier │    │gmp-material│    │gmp-deviation│
└──────┬──────┘    └──────┬──────┘    └──────┬──────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           ▼
              ┌────────────────────────┐
              │   gmp-storage (统一)    │
              │ SQLRustGo + 向量 + FTS │
              └────────────────────────┘
                           │
                           ▼
              ┌────────────────────────┐
              │    gmp-graph（图谱）   │
              │  Supplier/Material/... │
              └────────────────────────┘
```

---

## 九、风险与应对

| 风险 | 概率 | 影响 | 应对 |
|------|------|------|------|
| 数据模型设计遗漏字段 | 中 | 中 | 预留 JSON 扩展字段 |
| 检索性能随数据量下降 | 低 | 中 | 向量索引分片 + 分页 |
| 与现有系统集成复杂度 | 中 | 中 | 分阶段实施 |
| 用户界面开发资源不足 | 中 | 高 | 优先 CLI+API，Dashboard P2 |

---

## 十、总结

| 版本 | 目标 | 周期 |
|------|------|------|
| v1.3.0 | 能力基座（向量索引 + 知识管线） | ~6 周 |
| v2.0.0 | 综合管理平台（8 大模块） | ~18 周 |

**下一步行动：**
1. 确认 v1.3.0 开发排期
2. 启动供应商 + 物料详细技术设计评审
3. 同步开始 Dashboard 前端设计
