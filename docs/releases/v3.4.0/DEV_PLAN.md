# v3.4.0 开发测试计划

> **版本**: v1.0
> **创建日期**: 2026-05-18
> **维护人**: hermes-agent
> **战略定位**: GMP Management Suite（GMP 管理套件）
> **前置版本**: v3.3.0 (Industrial Trust Platform)
> **起点**: v3.3.0 GA release point

---

## 一、版本目标与约束

### 1.1 核心目标

**v3.4.0 战略定位**：GMP Management Suite，在 v3.3.0 可信内核基础上构建完整的管理套件。

**战略依赖**：
```
v3.3.0: Industrial Trust Platform（可信内核）
    ↓
v3.4.0: GMP Management Suite（管理套件）
    ↓
v3.5.0: AI Native GMP Platform
```

**核心判断**：v3.3.0 建立了工业级可信闭环，但缺少面向终端用户的 GMP 管理界面。v3.4.0 填补这一空白，让合规人员、QA、审计员能够通过可视化界面操作可信内核。

### 1.2 核心价值

- **可信内核 → 可信产品**：将 v3.3.0 的可信闭环能力产品化
- **合规人员友好**：无需 SQL，通过 Web 界面完成合规操作
- **审计即服务**：一键导出审计报告，降低第三方审计成本

### 1.3 版本约束

| 约束 | 说明 |
|------|------|
| 必须基于 v3.3.0 GA | 依赖 Trust Infrastructure 内核 |
| 所有 Trust Infrastructure 必须 GA 通过 | 内核不稳定，管理套件无法构建 |
| Web 界面与后端 API 分离 | 便于独立升级和扩展 |
| 响应式设计 | 支持桌面和平板设备 |

---

## 二、功能范围

### 2.1 核心模块

| 模块 | 功能 | 优先级 |
|------|------|--------|
| **Web Dashboard** | GMP 合规仪表板 | P0 |
| **EBR (Electronic Batch Record)** | 电子批次记录全生命周期 | P0 |
| **设备集成** | OPC UA / MQTT / Modbus 接入 | P1 |
| **规则引擎 UI** | 可视化合规规则配置 | P1 |
| **移动审批 API** | 移动端审批接口 | P2 |
| **合规模板库** | 预置 GMP 合规模板 | P2 |

### 2.2 Web Dashboard（P0）

#### 功能列表

| 功能 | 说明 |
|------|------|
| 合规评分概览 | Trust score, compliance rate, active deviations |
| 审计链查看器 | 可视化审计链（SHA-256 哈希链） |
| 批次状态仪表板 | 批次进度、关键节点、偏差状态 |
| 实时告警 | 违规告警、签名缺失、流程超时 |
| 用户角色管理 | Admin, QA, Operator, Auditor 角色 |

#### 技术选型

| 组件 | 技术 | 说明 |
|------|------|------|
| 前端框架 | React 18 | 函数式组件 + Hooks |
| UI 库 | Tailwind CSS + shadcn/ui | 现代化 GMP 风格 |
| 图表 | Recharts / D3.js | 合规仪表板可视化 |
| 状态管理 | Zustand | 轻量状态管理 |
| 后端 API | REST + GraphQL | 混合 API |
| 实时通信 | WebSocket | 实时告警推送 |

#### API 设计

```typescript
// Dashboard API
GET /api/v1/dashboard/summary         // 合规评分概览
GET /api/v1/dashboard/audit-chain/:id // 审计链详情
GET /api/v1/dashboard/batches         // 批次列表
GET /api/v1/dashboard/alerts          // 实时告警

// Audit Chain API
GET  /api/v1/audit-chains             // 审计链列表
GET  /api/v1/audit-chains/:id         // 审计链详情
POST /api/v1/audit-chains/:id/sign    // 电子签名
GET  /api/v1/audit-chains/:id/export  // 导出审计包
```

### 2.3 EBR — Electronic Batch Record（P0）

#### 核心功能

| 功能 | 说明 |
|------|------|
| 批次创建 | 创建新批次，自动分配批次号 |
| 工艺步骤执行 | 步骤解锁 → 执行 → 签名 → 完成 |
| 电子签名 | 符合 21 CFR Part 11 的电子签名 |
| 偏差管理 | 偏差记录、审批、关闭 |
| 批次放行 | QA 放行、释放批次 |
| 批次历史 | 完整审计链追溯 |

#### 状态机

```
┌──────────────────────────────────────────────────────────┐
│                    Batch Lifecycle                        │
└──────────────────────────────────────────────────────────┘

Created → InProgress → PendingQA → Released → Archived
               ↓
           Deviation
               ↓
         OnHold / Rejected
```

#### API 设计

```typescript
// Batch API
POST   /api/v1/batches                    // 创建批次
GET    /api/v1/batches/:id                // 批次详情
PUT    /api/v1/batches/:id/steps/:stepId  // 更新步骤
POST   /api/v1/batches/:id/sign           // 批次签名
POST   /api/v1/batches/:id/release        // 批次放行
GET    /api/v1/batches/:id/audit-chain    // 批次审计链

// Deviation API
POST   /api/v1/deviations                 // 创建偏差
GET    /api/v1/deviations/:id             // 偏差详情
PUT    /api/v1/deviations/:id/status      // 更新偏差状态
POST   /api/v1/deviations/:id/capa        // 关联 CAPA
```

### 2.4 设备集成（P1）

#### OPC UA 集成

| 功能 | 说明 |
|------|------|
| OPC UA Client | 连接到 OPC UA 服务器 |
| 变量订阅 | 实时采集设备数据 |
| 告警事件 | OPC UA 告警事件捕获 |
| 历史存储 | 设备数据历史存储 |

#### MQTT 集成

| 功能 | 说明 |
|------|------|
| MQTT Broker 连接 | 订阅/发布 MQTT 消息 |
| 设备遥测 | 传感器数据采集 |
| 命令下发 | 设备远程控制 |

#### Modbus 集成

| 功能 | 说明 |
|------|------|
| RTU/TCP 支持 | Modbus RTU 和 Modbus TCP |
| 寄存器读写 | 保持寄存器、输入寄存器 |
| 轮询配置 | 采集频率配置 |

### 2.5 规则引擎 UI（P1）

#### 可视化规则编辑器

| 功能 | 说明 |
|------|------|
| 规则模板 | 预置 GMP 规则模板 |
| 条件构建器 | 无代码条件配置 |
| 测试规则 | 规则测试沙箱 |
| 版本管理 | 规则版本控制和回滚 |

#### 预置规则

```yaml
rules:
  - id: R001
    name: 双签必须
    description: 关键步骤需要双人签名
    condition: "step.critical && step.signatures.length < 2"
    severity: critical

  - id: R002
    name: 批次号格式
    description: 批次号必须符合 YYYYMMDD-XXX 格式
    condition: "batch.id.match(/^\\d{8}-\\d{3}$/)"
    severity: error

  - id: R003
    name: 温度范围
    description: 冷藏温度必须在 2-8°C
    condition: "device.temperature >= 2 && device.temperature <= 8"
    severity: warning
```

### 2.6 移动审批 API（P2）

| API | 说明 |
|-----|------|
| `GET /api/v1/mobile/pending` | 待审批列表 |
| `POST /api/v1/mobile/approve/:id` | 审批通过 |
| `POST /api/v1/mobile/reject/:id` | 审批拒绝 |
| `POST /api/v1/mobile/sign/:id` | 移动签名 |

### 2.7 合规模板库（P2）

| 模板 | 说明 |
|------|------|
| 批次记录模板 | 标准批次记录格式 |
| 偏差报告模板 | 偏差调查和 CAPA 模板 |
| 审计报告模板 | 第三方审计报告模板 |
| 放行证书模板 | CoA (Certificate of Analysis) |

---

## 三、Issue 规划

### 3.1 P0 Issue（必须完成）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| #1256 | P0 | Web Dashboard 核心仪表板 | Dashboard UI + API | 3 周 |
| #1257 | P0 | EBR 核心批次管理 | Batch CRUD + 状态机 | 4 周 |
| #1258 | P0 | 电子签名服务 | 21 CFR Part 11 签名 | 2 周 |

### 3.2 P1 Issue（Beta 目标）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| #1259 | P1 | OPC UA 设备集成 | OPC UA Client | 3 周 |
| #1260 | P1 | 规则引擎可视化编辑器 | Rule Builder UI | 2 周 |
| #1261 | P1 | 审计链导出服务 | JSON/PDF 导出 | 1 周 |

### 3.3 P2 Issue（GA 目标）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| #1262 | P2 | MQTT 设备集成 | MQTT Client | 2 周 |
| #1263 | P2 | 移动审批 API | Mobile API | 1 周 |
| #1264 | P2 | 合规模板库 | 模板管理 | 1 周 |

---

## 四、技术架构

### 4.1 整体架构

```
┌──────────────────────────────────────────────────────────────┐
│                        Frontend (React)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │  Dashboard  │  │     EBR     │  │   Settings  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└──────────────────────────────────────────────────────────────┘
                              │
                              │ REST / GraphQL
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                     GMP Management API                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ Batch API   │  │ Audit API   │  │ Device API  │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└──────────────────────────────────────────────────────────────┘
                              │
                              │ Internal Calls
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                   Trust Infrastructure                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │Evidence Eng │  │ Provenance  │  │Compliance   │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│  ┌─────────────┐  ┌─────────────┐                           │
│  │Workflow V2  │  │Trust Viz   │                            │
│  └─────────────┘  └─────────────┘                           │
└──────────────────────────────────────────────────────────────┘
```

### 4.2 项目结构

```
gmp/
├── gmp-api/              # GMP Management API (Rust Axum)
│   ├── src/
│   │   ├── batch/       # EBR 批次管理
│   │   ├── audit/       # 审计链
│   │   ├── device/      # 设备集成
│   │   ├── signature/   # 电子签名
│   │   └── export/      # 导出服务
│   └── Cargo.toml
│
├── gmp-web/             # React Web UI
│   ├── src/
│   │   ├── components/  # 通用组件
│   │   ├── pages/       # 页面
│   │   ├── hooks/       # 自定义 Hooks
│   │   ├── api/         # API 客户端
│   │   └── stores/      # Zustand stores
│   └── package.json
│
└── gmp-shared/          # 共享类型
    ├── types/           # TypeScript 类型定义
    └── openapi/         # OpenAPI 规范
```

### 4.3 依赖关系

```
v3.3.0 GA
    │
    ├─ Trust Infrastructure (可信内核)
    │   ├─ sqlrustgo-evidence-engine
    │   ├─ sqlrustgo-provenance-graph
    │   ├─ sqlrustgo-compliance-engine
    │   ├─ sqlrustgo-workflow-v2
    │   └─ sqlrustgo-trust-viz
    │
    └─ GMP Management Suite (v3.4.0)
        ├─ gmp-api (依赖可信内核)
        │   ├─ Evidence Engine API
        │   ├─ Provenance Graph API
        │   └─ Workflow V2 API
        │
        └─ gmp-web (依赖 gmp-api)
```

---

## 五、测试计划

### 5.1 测试分层

| 层级 | 测试类型 | 执行环境 | 频率 |
|------|----------|----------|------|
| L0 | 单元测试（前端组件） | CI | 每次 PR |
| L1 | 集成测试（API） | CI | 每次 PR |
| L2 | E2E 测试（Playwright） | CI | 每次 PR |
| L3 | 性能测试 | Z6G4 | Beta 前 |
| L4 | 安全测试 | Z6G4 | RC 前 |

### 5.2 测试命令

```bash
# 前端测试
cd gmp-web
npm test                    # Jest 单元测试
npm run test:e2e           # Playwright E2E

# 后端测试
cd gmp-api
cargo test                  # 单元测试
cargo test --test集成      # 集成测试

# E2E 测试
npm run test:e2e:headless   # CI 模式
npm run test:e2e:ui         # 有界面模式
```

---

## 六、门禁

### 6.1 Alpha Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| A1 | Build | `cargo build --release` + `npm run build` | 编译通过 |
| A2 | Test | `cargo test` + `npm test` | 全部通过 |
| A3 | Clippy | `cargo clippy -- -D warnings` | 零警告 |
| A4 | Format | `cargo fmt -- --check` + `npm run lint` | 通过 |
| A5 | Type Check | `npm run typecheck` | 零错误 |
| A6 | E2E Smoke | `npm run test:e2e -- --grep "smoke"` | 通过 |

### 6.2 Beta Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| B1 | Full E2E | `npm run test:e2e` | 全部通过 |
| B2 | Performance | 页面加载 < 2s | 通过 |
| B3 | Mobile API | `curl` 移动审批 API | 通过 |
| B4 | Security Scan | `npm audit` + 依赖检查 | 无高危 |
| B5 | OPC UA 模拟 | 设备集成测试 | 通过 |

### 6.3 GA Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| G1 | Full Regression | `npm run test:e2e -- --full` | 通过 |
| G2 | Load Test | k6 负载测试 | 100 并发通过 |
| G3 | Security Audit | 第三方安全审计 | 无高危 |
| G4 | Documentation | 用户文档完整 | 通过 |
| G5 | Migration | v3.3.0 数据迁移 | 通过 |

---

## 七、发布计划

### 7.1 版本阶段

| 阶段 | 目标日期 | 说明 |
|------|----------|------|
| Alpha | 2026-06-01 | 核心功能完成 |
| Beta | 2026-06-15 | 集成测试通过 |
| RC | 2026-06-30 | 预发布版本 |
| GA | 2026-07-15 | 正式发布 |

### 7.2 发布检查清单

- [ ] 所有 P0 Issue 完成
- [ ] 所有 Beta Gate 通过
- [ ] 安全审计通过
- [ ] 用户文档完成
- [ ] Release Notes 编写
- [ ] 部署脚本验证
- [ ] 数据迁移脚本验证

---

## 八、风险与应对

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| 可信内核不稳定 | 管理套件无法构建 | 中 | v3.3.0 GA 必须通过 |
| Web UI 开发周期长 | 发布时间延迟 | 高 | 核心功能优先 |
| 设备集成复杂 | 联调周期长 | 中 | 使用模拟器开发 |
| 移动端 API 安全 | 合规风险 | 高 | 遵循 21 CFR Part 11 |

---

## 九、关联文档

| 文档 | 说明 |
|------|------|
| `docs/releases/v3.3.0/DEV_PLAN.md` | v3.3.0 开发计划（可信内核） |
| `docs/releases/VERSION_ROADMAP.md` | 版本演化总览 |
| `oo/GMP-Management/WORKFLOW_V2.md` | Workflow V2 详细设计 |
| `oo/Compliance/EVIDENCE_ENGINE.md` | Evidence Engine 详细设计 |

---

## 十、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-05-18 | 初始版本，基于 Trust Infrastructure v3.3.0 |

---

*文档版本: v1.0*
*创建: hermes-agent*
*日期: 2026-05-18*
