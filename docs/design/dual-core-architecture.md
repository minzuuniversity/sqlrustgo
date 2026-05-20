# GMP-OS 产品架构定义

> 版本：v0.4-draft
> 日期：2026-05-20
> 状态：产品架构定义阶段（Product Architecture Definition）

---

## 一、系统定位

### 1.1 不是"数据库项目"，而是"AI-Native GMP Trust Operating System"

系统定位不是 GMP Management System，而是：

**Trust-Centric GMP Operating System**，简称 **GMP-OS**

### 1.2 核心竞争力

不是 SQL，不是 Rust，不是 Agent，而是：

**"可验证的 GMP Evidence System"**

这是整个系统真正的核心。

### 1.3 最佳切入点

**"生产数据 GMP 合规审核"** — 最成熟、最容易落地、ROI 最高的方向。

不需要替换 MES/ERP/LIMS/QMS，而是**旁路增强（Sidecar Enhancement）**，这是最容易进入真实生产环境的方式。

### 1.4 核心设计思想：Agent-Oriented GMP

传统 GMP 系统：

```
人 → 点表单 → 提交流程 → 系统存数据库
```

GMP-OS 新架构：

```
人 → 移动端采集 → HermesAgent 理解 → Workflow 编排 → Trust Engine 验证 → 自动生成 Evidence → 合规持续评估
```

不是"把 AI 加到 GMP"，而是**重新定义 GMP 的操作方式**。

### 1.5 Evidence-first Architecture

传统系统"记录数据"，GMP-OS **"记录证据"**——这是根本不同。

---

## 二、系统目标

| 能力 | 目标 |
|------|------|
| GMP 合规 | FDA/EMA/21 CFR Part 11 |
| AI Native | HermesAgent 驱动 |
| 移动优先 | 手机采集/审批 |
| Trust Native | 签名/审计/溯源 |
| Workflow Native | Agent 编排 |
| Evidence Native | 自动生成审计证据 |
| Hybrid DB | PG/MySQL + SQLRustGo |
| Knowledge Native | SOP/EBR/Deviation 图谱 |

---

## 三、总体架构

### 3.1 顶层架构

```
                ┌──────────────────────┐
                │     Mobile App       │
                │  (iOS/Android/PWA)   │
                └─────────┬────────────┘
                          │
                          ▼
                ┌──────────────────────┐
                │     API Gateway      │
                │  Auth / JWT / RBAC   │
                └─────────┬────────────┘
                          │
        ┌─────────────────┴─────────────────┐
        │                                   │
        ▼                                   ▼

┌──────────────────┐           ┌────────────────────┐
│   HermesAgent    │           │   Workflow Engine  │
│ AI Orchestration │           │ State / SLA / BPM  │
└────────┬─────────┘           └─────────┬──────────┘
         │                               │
         └────────────┬──────────────────┘
                      ▼

         ┌──────────────────────────────┐
         │      GMP Domain Services     │
         │                              │
         │ - Document Mgmt              │
         │ - EBR                        │
         │ - LIMS                       │
         │ - CAPA                       │
         │ - Training                   │
         │ - Deviation                  │
         │ - Calibration                │
         │ - Audit                      │
         │ - Reporting                  │
         └────────────┬─────────────────┘
                      │
      ┌───────────────┴─────────────────┐
      │                                 │
      ▼                                 ▼

┌──────────────────┐         ┌────────────────────┐
│ PostgreSQL/MySQL │         │ SQLRustGo Trust DB │
│ Primary OLTP DB  │         │ Immutable / Audit  │
└──────────────────┘         └────────────────────┘
```

### 3.2 双轨架构（Dual-Core Architecture）

将系统拆分为两个独立项目，各司其职：

| 项目 | 定位 | 核心职责 |
|------|------|----------|
| **SQLRustGo** | Trust Database Kernel | 可信数据库内核 |
| **GMP-Plat** | GMP Compliance Platform | GMP 合规管理平台 |

### 3.3 为什么必须拆？

数据库和 GMP 平台本质不是同一种软件：

- **SQLRustGo** 本质是"状态机 + 存储系统"，核心问题是 correctness、crash recovery、MVCC、WAL、lock、buffer、optimizer、replication
- **GMP-Plat** 本质是"业务系统 + 合规系统"，核心问题是 workflow、forms、users、integrations、dashboards、reports、AI、orchestration

两者的工程文化完全不同。如果不拆，会导致：

- 数据库团队被业务拖死
- 内核稳定性越来越差
- Release 周期失控
- QA 爆炸
- 架构耦合
- GMP 需求反向污染数据库内核

---

## 四、架构核心原则（ADR）

### ADR-001：PostgreSQL/MySQL 是 Source of Business Truth

业务数据的主存储，承担所有 CRUD、Workflow State、Reporting。

### ADR-002：SQLRustGo 是 Source of Trust Evidence

可信证据的主存储，承担所有审计链、签名、不可变记录。

### ADR-003：所有 Trust 数据永不 UPDATE/DELETE

只允许：

```
INSERT + Correction Event
```

不可变是可信的基础。修正通过追加 Correction Event 实现，而非覆盖。

---

## 五、双核心数据库设计

### 5.1 PostgreSQL/MySQL — 业务系统

| 模块 | 说明 |
|------|------|
| 用户 | CRUD |
| 工单 | CRUD |
| Workflow State | 普通状态 |
| Dashboard | 查询 |
| 报表缓存 | OLTP |
| LIMS 业务 | 普通数据 |

### 5.2 SQLRustGo — Trust Layer

| 模块 | 说明 |
|------|------|
| Audit Chain | 审计链 |
| Signature | 数字签名 |
| Immutable Record | 不可变记录 |
| Provenance | 溯源 |
| Timestamp | RFC3161 |
| Evidence | 合规证据 |
| Workflow Proof | 工作流证明 |
| Compliance Log | 合规日志 |

### 5.3 为什么这是正确路线？

- PostgreSQL/MySQL 保底稳定性，SQLRustGo 提供可信能力
- 不需要"证明 SQLRustGo 能替代 PG"（极难），而是"证明 SQLRustGo 能提供 PG 没有的能力"（容易得多）
- 风险可控，双写可比对，方便验证 correctness
- 不会阻碍 GMP-Plat 商业落地
- SQLRustGo 可以慢慢成熟，不用被生产事故压垮

---

## 六、Evidence Engine（核心模块）

### 6.1 为什么需要 Evidence Engine？

当前 Audit、Signature、Provenance、Timestamp、Workflow、CAPA 还只是功能集合，不是统一证据系统。

Evidence Engine 将所有 Trust 事件统一为 Evidence Model。

### 6.2 统一 Evidence Schema

所有 Trust 事件统一结构：

```json
{
  "evidence_id": "...",
  "event_type": "audit|signature|workflow|deviation",
  "actor": "...",
  "timestamp": "...",
  "business_object": "...",
  "payload_hash": "...",
  "signature": "...",
  "previous_hash": "...",
  "reason": "...",
  "references": []
}
```

### 6.3 统一映射

| 模块 | 统一方式 |
|------|----------|
| 审计 | Evidence Event |
| 签名 | Evidence Signature |
| 时间戳 | Evidence Timestamp |
| Workflow | Evidence Transition |
| Provenance | Evidence Lineage |
| CAPA | Evidence Resolution |
| EBR | Evidence Batch |
| SOP | Evidence Reference |

### 6.4 FDA 审计场景

FDA 来审计时，不是查数据库，而是**重建 Evidence Graph**。

---

## 七、Trust Event Bus（核心架构）

这是整个系统的真正核心。所有关键动作都必须：

```
业务系统 → Trust Event → SQLRustGo
```

### 7.1 批次放行

`Batch Released` 生成：

- 签名
- hash
- workflow proof
- operator proof
- SOP version proof

### 7.2 CAPA 审批

生成：

- 审批链
- 电子签名
- 时间戳
- deviation lineage

---

## 八、HermesAgent — Evidence Orchestrator

### 8.1 定位

不是聊天机器人，不是 RAG Agent，而是 **Evidence Orchestrator**（证据编排器）。

AI 是编排层，不是聊天层。不做"ChatGPT for GMP"，而是"AI-driven Compliance Workflow System"。

### 8.2 Agent 架构

```
HermesAgent
├── SOP Agent
├── Audit Agent
├── CAPA Agent
├── Workflow Agent
├── Evidence Agent
├── Risk Agent
└── Report Agent
```

### 8.3 Workflow Orchestration

例如："设备校准过期"，Agent 自动：

- 创建 deviation
- 通知 QA
- 锁定批次
- 生成 CAPA
- 启动审批流

### 8.4 Evidence Assembly（极重要）

例如：`Generate audit package for Batch #B2026-001`，Agent 自动：

- 汇总签名
- 汇总 workflow
- 汇总设备状态
- 汇总 SOP
- 汇总校准
- 汇总培训
- 生成 PDF/JSON

### 8.5 Knowledge Retrieval（RAG）

Agent 自动：

- 查 SOP
- 查历史 deviation
- 查 CAPA
- 查 FDA warning letter

---

## 九、Compliance DSL

### 9.1 从 Rule-as-Code 到 Compliance DSL

不要只用 YAML 配置，建立专用的 Compliance DSL。

### 9.2 示例

```yaml
rule:
  id: GMP-001
  applies_to: batch_record
  when:
    temperature > 8
  require:
    deviation_record exists
  severity: critical
  reference:
    - FDA_21_CFR_PART_211
```

### 9.3 长期方向

HermesAgent 自动把法规转换成 Compliance DSL——这是非常强的方向。

---

## 十、核心实体 Evidence 化

所有核心实体不仅记录业务数据，还必须附带完整证据链。

以 EBR 为例，不仅需要 batch record，还必须包含：

- 签名
- 审批链
- 时间戳
- 设备状态
- 操作员状态
- SOP version
- 培训状态

每个核心实体（EBR、CAPA、Deviation、Batch、Calibration 等）都必须满足 Evidence-first 原则。

---

## 十一、Knowledge Graph

GMP Knowledge Graph 是未来 AI Agent 的基础。

核心关系：

```
Operator → Training → SOP → Batch → Device → Calibration → Workflow → CAPA → Deviation
```

通过知识图谱，HermesAgent 可以实现：

- 跨实体关联查询
- 影响分析（某设备校准过期影响哪些批次）
- 智能推荐（基于历史 deviation 推荐 CAPA）
- 合规推理（自动评估合规风险）

---

## 十二、GMP Domain Layer

### 12.1 核心模块

| 模块 | 优先级 |
|------|--------|
| Document Management | P0 |
| EBR | P0 |
| Workflow | P0 |
| Audit | P0 |
| CAPA | P0 |
| Deviation | P0 |
| Training | P0 |
| Calibration | P0 |
| LIMS | P1 |
| MES Lite | P1 |
| Supplier Quality | P2 |

---

## 十三、移动端 — 现场采集终端

### 13.1 定位

不做"移动版网页"，而是**现场采集终端**。

### 13.2 可信采集能力

| 能力 | 说明 |
|------|------|
| 拍照 | 自动 hash |
| GPS | 可选 |
| NFC | 设备绑定 |
| 蓝牙 | 仪器读取 |
| 签名 | 生物识别 |
| 离线缓存 | 关键 |
| 时间戳 | 本地+服务器 |

---

## 十四、SQLRustGo 定位与路线

### 14.1 定位：Trust-Optimized Database Kernel

SQLRustGo 不是 Primary OLTP DB，不是 MySQL 全替代，而是 **Trust-Optimized Database Kernel**。

### 14.2 应该强化的方向

| 方向 | 建议 |
|------|------|
| 审计链 | 强化 |
| 不可变存储 | 强化 |
| 时间戳 | 强化 |
| 签名 | 强化 |
| Provenance | 强化 |
| 验证工具 | 强化 |
| Evidence Query | 强化 |

### 14.3 不应该追的方向

- OLAP
- 分布式 SQL
- HTAP
- 海量 JOIN
- 大数据分析
- 100% MySQL 兼容
- Oracle 级 SQL
- 全生态替代

### 14.4 渐进式演进路线

| 阶段 | SQLRustGo 角色 | 说明 |
|------|----------------|------|
| 第一阶段 | Secondary Trust Core | 旁路可信层，不承担主生产责任 |
| 第二阶段 | Selective Trust Storage | 承载 audit、signatures、immutable records |
| 第三阶段 | 部分主业务切换 | 经过充分验证后逐步接管 |
| 第四阶段 | Hybrid Core | 双核心混合运行 |

### 14.5 竞争定位转变

从"国产 MySQL 替代"（竞争者众多）→ "Trust-Optimized Database Kernel"（竞争者骤减，完全不同的赛道）。

---

## 十五、GMP-Plat 架构

### 15.1 核心模块

| 模块 | 技术选型 |
|------|----------|
| Dashboard | React |
| AI Agent | HermesAgent 内置 |
| Workflow UI | GMP-Plat 内置 |
| CAPA | GMP-Plat 内置 |
| EBR | GMP-Plat 内置 |
| Reporting | GMP-Plat 内置 |
| MES/LIMS | GMP-Plat 内置 |
| OPC UA / Device | GMP-Plat 内置 |

### 15.2 数据库抽象层

GMP-Plat 应完全数据库抽象化，建立 **Storage Adapter Layer**：

| 后端 | 用途 |
|------|------|
| PostgreSQL | 默认生产 |
| MySQL | 兼容 |
| SQLRustGo | Trust Mode |
| Hybrid | 双写验证 |

### 15.3 PostgreSQL/MySQL 作为 Primary System of Record 的原因

| 原因 | 说明 |
|------|------|
| 工业稳定 | 已验证 |
| ORM 生态 | 极成熟 |
| BI 兼容 | 完整 |
| 运维成熟 | 非常重要 |
| HA 成熟 | Patroni/PXC |
| Backup 成熟 | pgBackRest 等 |
| 审计生态 | 丰富 |

---

## 十六、技术架构

### 16.1 Backend

| 组件 | 推荐 |
|------|------|
| API | Rust Axum |
| Workflow | Temporal / 自研轻量 |
| Agent | HermesAgent |
| Queue | NATS |
| Cache | Redis |
| Primary DB | PostgreSQL |
| Trust DB | SQLRustGo |
| Search | OpenSearch |
| Vector DB | Qdrant |
| Graph | Neo4j（后期） |

### 16.2 Mobile

| 技术 | 推荐 |
|------|------|
| Flutter | 最优 |
| React Native | 次选 |
| PWA | 可辅助 |

### 16.3 Frontend

| 技术 | 推荐 |
|------|------|
| React | 推荐 |
| Next.js | 推荐 |
| Tailwind | 推荐 |

---

## 十七、核心系统流

### 17.1 设备数据采集流程

```
设备 → Mobile App → HermesAgent → Workflow Validation → PostgreSQL → Trust Event → SQLRustGo → Evidence Generated
```

---

## 十八、Trust Sidecar

SQLRustGo 不直接承担全部业务，而是作为 **Trust Sidecar** 运行。

这意味着：

- 业务系统正常读写 PostgreSQL/MySQL
- 关键动作通过 Trust Event Bus 旁路写入 SQLRustGo
- SQLRustGo 专注于不可变存储、审计链、签名验证
- 业务系统无需改造即可获得可信能力

---

## 十九、项目结构

```
openclaw/
├── sqlrustgo/            # Trust DB Kernel
├── gmp-plat/             # GMP Platform
├── hermes-agent/         # AI Orchestration
├── trust-sdk/            # 签名/证据 SDK
├── workflow-engine/      # Workflow
├── mobile-app/           # Flutter
├── evidence-engine/      # 审计证据
├── provenance-graph/     # 图谱
└── compliance-engine/    # Compliance-as-Code
```

---

## 二十、迁移策略：绞杀者模式（Strangler Pattern）

### 20.1 原则

不要急着重写 gmp-system。正确做法：

```
旧系统继续运行 → 新系统逐步接管
```

### 20.2 优先接管顺序

| 优先级 | 模块 |
|--------|------|
| P0 | Audit |
| P0 | Evidence |
| P0 | Report |
| P1 | Workflow |
| P1 | CAPA |
| P2 | EBR |
| P2 | LIMS |
| P3 | MES |

---

## 二十一、v0.1 范围定义

### 21.1 v0.1 只做

| 模块 | 保留 |
|------|------|
| CSV 导入 | ✅ |
| Rule Engine | ✅ |
| 审核报告 | ✅ |
| SQLRustGo 固化 | ✅ |
| 独立验证 | ✅ |

v0.1 的目标是验证 **"Trust Evidence Flow"**，而非完整 GMP 平台。

### 21.2 v0.1 暂不做

| 功能 | 原因 |
|------|------|
| JDBC | 太复杂 |
| 多源接入 | 先别做 |
| 实时流 | 先别做 |
| 工作流复杂审批 | 后面 |
| 移动端复杂能力 | 后面 |

### 21.3 v0.1 交付物：FDA Inspection Demo System

这是最容易验证价值的方式。

### 21.4 Demo Dataset

| 数据 | 内容 |
|------|------|
| Batch Record | 温度超限 |
| Cleaning | 缺失签名 |
| Training | 培训过期 |
| Calibration | 校准失效 |
| CAPA | 未关闭 |

系统自动：审核 → 发现问题 → 生成报告 → 输出证据包 → 验证签名链。

这是第一个真正能展示的东西，比完整 MES 更有价值。

---

## 二十二、关键决策记录

### ADR-001：PostgreSQL/MySQL 是 Source of Business Truth

业务数据主存储，承担所有 CRUD 和业务状态。

### ADR-002：SQLRustGo 是 Source of Trust Evidence

可信证据主存储，承担所有审计链和不可变记录。

### ADR-003：Trust 数据永不 UPDATE/DELETE

只允许 INSERT + Correction Event，不可变是可信的基础。

### ADR-004：立即拆项目

不是以后，而是现在。建立 GMP-Plat 独立项目，SQLRustGo 回归 Trust Database Kernel。

### ADR-005：SQLRustGo 初期不承担主生产数据库责任

通过旁路模式运行，风险可控，可验证，不阻碍商业落地。

### ADR-006：渐进可信替代

不是完全替换 PostgreSQL/MySQL，而是"渐进可信替代"。

### ADR-007：Agent-Oriented GMP

不是"AI 辅助 GMP"，而是以 Agent 为核心重新定义 GMP 操作方式。AI 是编排层，不是聊天层。

### ADR-008：Evidence-first Architecture

系统核心是"记录证据"而非"记录数据"。

### ADR-009：Trust Sidecar 模式

SQLRustGo 以 Sidecar 方式运行，业务系统无需改造即可获得可信能力。

### ADR-010：绞杀者模式迁移

旧系统继续运行，新系统逐步接管，不急着重写。

### ADR-011：v0.1 聚焦 Trust Evidence Flow

v0.1 不做完整 GMP 平台，只验证 Trust Evidence Flow。

---

## 二十三、战略定位

不是"再做一个 GMP 系统"，不是"国产版 SAP GMP"（死亡路线），而是**重新定义 GMP 的运作模式**。

真正应该建立的不是数据库，不是 MES，而是**可信 GMP Evidence 平台**。

SQLRustGo、HermesAgent、GMP-Plat 都只是这个系统里的不同层。

核心壁垒：

- **AI Native** — HermesAgent 驱动认知与编排
- **Evidence Native** — 每个实体自带完整证据链
- **Trust Native** — SQLRustGo 提供不可变可信层
- **Mobile Native** — 现场采集终端，可信采集

长期形态：**Evidence Operating System**

---

## 二十四、下一步行动

- [ ] 初始化 GMP-Plat 项目结构
- [ ] 设计 Storage Adapter Layer 接口
- [ ] 定义 SQLRustGo Trust API 规范
- [ ] 设计 Trust Event Bus 协议
- [ ] 定义 HermesAgent 核心能力接口
- [ ] 设计统一 Evidence Schema
- [ ] 设计 Compliance DSL 语法
- [ ] 设计 GMP Knowledge Graph 模型
- [ ] 构建 FDA Inspection Demo Dataset
- [ ] 确定第一阶段双写验证方案
- [ ] 搭建 v0.1 基础框架（CSV 导入 + Rule Engine + 审核报告 + SQLRustGo 固化）
- [ ] 设计移动端可信采集协议
- [ ] 设计 Trust Sidecar 部署方案
