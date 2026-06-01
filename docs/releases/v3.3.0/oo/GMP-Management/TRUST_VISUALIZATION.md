# OO-GM2: Trust Visualization

> **版本**: v1.0
> **日期**: 2026-05-18
> **基于**: v3.3.0
> **维护人**: hermes-agent
> **Issue**: #1242
> **状态**: 新建设计

---

## 一、概述

### 1.1 目标

提供可视化合规视图，包括审计链可视化、合规评分、风险热图等功能。

### 1.2 核心理念

```
Trust Visualization = Graph Visualization + Real-time Metrics + Risk Assessment + Audit Trail UI
```

### 1.3 主要功能

| 功能 | 说明 |
|------|------|
| 审计链可视化 | 交互式查看签名链、记录链 |
| 合规评分 | 实时计算合规评分 |
| 风险热图 | 按批次/设备/操作员展示风险 |
| 趋势仪表盘 | 合规趋势时间序列 |
| 告警中心 | 实时告警和通知 |

---

## 二、架构设计

### 2.1 组件关系

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Trust Visualization Platform                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌────────────────────────────┐  │
│  │   Data       │───▶│   Metrics   │───▶│   Visualization           │  │
│  │   Aggregator │    │   Engine    │    │   Renderer                │  │
│  └──────────────┘    └──────────────┘    └────────────────────────────┘  │
│          │                                       │                         │
│          ▼                                       ▼                         │
│  ┌──────────────┐    ┌──────────────┐    ┌────────────────────────────┐  │
│  │  Provenance  │    │   Risk       │    │   Dashboard               │  │
│  │  Graph API   │    │   Calculator │    │   Service                 │  │
│  └──────────────┘    └──────────────┘    └────────────────────────────┘  │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    Visualization Components                            │  │
│  │  - AuditChainView    - ComplianceScoreView    - RiskHeatmapView   │  │
│  │  - TrendDashboard    - AlertCenter            - BatchTimelineView  │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 组件说明

| 组件 | 说明 | 位置 |
|------|------|------|
| Data Aggregator | 数据聚合器 | `crates/viz-aggregator/` |
| Metrics Engine | 指标计算 | `crates/viz-metrics/` |
| Risk Calculator | 风险计算 | `crates/viz-risk/` |
| Visualization Renderer | 渲染器 | `crates/viz-renderer/` |
| Dashboard Service | 仪表盘服务 | `crates/viz-dashboard/` |

---

## 三、视图设计

### 3.1 审计链视图

```
┌─────────────────────────────────────────────────────────────────┐
│                  Audit Chain Visualization                       │
└─────────────────────────────────────────────────────────────────┘

Batch: 2026-001 - Insulin Glargine
═══════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│    ┌─────────┐                                                 │
│    │ Operator│  John Smith                                      │
│    │ Role    │  Quality Reviewer                                │
│    │ Cert    │  Valid until 2027-01                             │
│    └────┬────┘                                                 │
│         │                                                       │
│         │ SIGNED (2026-05-01 10:30:00)                          │
│         │ Meaning: "I confirm this record is accurate"          │
│         ▼                                                       │
│    ┌─────────┐                                                 │
│    │ Record  │  Weighing #001                                  │
│    │ Type    │  Ingredient quantity                             │
│    │ Hash    │  sha256:abc123...                               │
│    └────┬────┘                                                 │
│         │                                                       │
│         │ LINKED TO                                             │
│         ▼                                                       │
│    ┌─────────┐                                                 │
│    │ Prev    │  Record #000 (genesis)                          │
│    │ Hash    │  sha256:xxx...                                  │
│    └────┬────┘                                                 │
│         │                                                       │
│         │ CHAIN CONTINUES...                                    │
│         ▼                                                       │
│    ┌─────────┐                                                 │
│    │ Batch   │  2026-001                                       │
│    │ Status  │  RELEASED                                        │
│    └─────────┘                                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

Legend: ● Signed  ● Linked  ● Verified
```

### 3.2 合规评分视图

```
┌─────────────────────────────────────────────────────────────────┐
│               Compliance Score Dashboard                          │
└─────────────────────────────────────────────────────────────────┘

Overall Score: 87/100 ⬆️ (+3 from last month)
═══════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   100 ┤                                                         │
│    90 ┤     ╭──────╮                                            │
│    80 ┤     │  87  │                                            │
│    70 ┤  ───│      │────  Trend: ⬆️ Improving                  │
│    60 ┤     │      │                                            │
│    50 ┤     ╰──────╯                                            │
│       └────────────────────────────────────                     │
│         Jan   Feb   Mar   Apr   May                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

Category Breakdown:
═══════════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────────────┐
│ Category             │ Score │ Status │ Trend │                 │
├──────────────────────┼───────┼────────┼───────┤                 │
│ Electronic Signatures│  95   │  ✅    │  ─    │ ████████████░░ │
│ Audit Trail         │  88   │  ✅    │  ⬆️   │ █████████░░░░ │
│ Data Integrity      │  85   │  ✅    │  ⬆️   │ █████████░░░░ │
│ Access Control      │  82   │  ⚠️    │  ⬇️   │ ████████░░░░░ │
│ Change Management   │  90   │  ✅    │  ─    │ ██████████░░░ │
│ Deviation Mgmt      │  78   │  ⚠️    │  ⬆️   │ ███████░░░░░ │
└─────────────────────┴───────┴────────┴───────┴─────────────────┘
```

### 3.3 风险热图视图

```
┌─────────────────────────────────────────────────────────────────┐
│                    Risk Heatmap                                   │
└─────────────────────────────────────────────────────────────────┘

Time Period: 2026-04-01 to 2026-05-18
═══════════════════════════════════════════════════════════════════

                    │ Mon │ Tue │ Wed │ Thu │ Fri │ Sat │ Sun │
════════════════════╪═════╪═════╪═════╪═════╪═════╪═════╪═════╡
Facility A          │     │     │     │     │     │     │     │
────────────────────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤
  Reactor 1         │  L  │  L  │  L  │  M  │  L  │  L  │  L  │
  Reactor 2         │  L  │  H  │  H  │  H  │  L  │  L  │  L  │
  Fill Line 1       │  L  │  L  │  L  │  L  │  M  │  L  │  L  │
════════════════════╪═════╪═════╪═════╪═════╪═════╪═════╪═════╡
Facility B          │     │     │     │     │     │     │     │
────────────────────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤
  Mixer 1           │  L  │  L  │  L  │  L  │  L  │  L  │  L  │
  Packaging 1       │  M  │  M  │  H  │  H  │  H  │  L  │  L  │
════════════════════╪═════╪═════╪═════╪═════╪═════╪═════╪═════╡

Legend:
  L = Low Risk (Green)    M = Medium Risk (Yellow)    H = High Risk (Red)
```

### 3.4 批次时间线视图

```
┌─────────────────────────────────────────────────────────────────┐
│                 Batch Timeline View                              │
└─────────────────────────────────────────────────────────────────┘

Batch: 2026-001 - Insulin Glargine
═══════════════════════════════════════════════════════════════════

│─────────────────────────────────────────────────────────────────│
2026-05-01                                                        │
│
● Start Manufacturing (08:00)
│  └─ Operator: John Smith
│  └─ SOP: MFG-001 v2.1
│
├─────────────────────────────────────────────────────────────────│
2026-05-01                                                        │
│
● Weighing Completed (10:30)  ──────  ✓ SIGNED
│  └─ Operator: Jane Doe
│  └─ Signature: VALID
│
├─────────────────────────────────────────────────────────────────│
2026-05-02                                                        │
│
● Quality Review (09:00)  ──────  ✓ SIGNED
│  └─ Reviewer: Mike Johnson
│  └─ Status: APPROVED
│
● Batch Released (14:00)  ──────  ✓ SIGNED
│  └─ Quality Manager: Sarah Chen
│  └─ Release #: REL-2026-001
│
│─────────────────────────────────────────────────────────────────│
                                                             Now
```

---

## 四、API 设计

### 4.1 REST API 端点

```yaml
# Dashboard API

GET /api/v1/dashboard/compliance-score
  Response:
    {
      "overall_score": 87,
      "category_scores": [...],
      "trend": "improving",
      "last_updated": "2026-05-18T12:00:00Z"
    }

GET /api/v1/dashboard/risk-heatmap
  Query: ?facility=F001&start=2026-04-01&end=2026-05-18
  Response:
    {
      "cells": [...],
      "legend": {...}
    }

GET /api/v1/audit-chain/{batch_id}
  Response:
    {
      "nodes": [...],
      "edges": [...],
      "verification_status": "valid"
    }

GET /api/v1/alerts
  Query: ?severity=high&acknowledged=false
  Response:
    {
      "alerts": [...],
      "total": 15
    }
```

### 4.2 WebSocket 事件

```yaml
# Real-time updates

channel: dashboard:updates
events:
  - compliance_score_changed
  - alert_created
  - risk_level_changed
  - new_audit_entry
```

---

## 五、验收标准

### 5.1 功能验收

| 检查项 | 命令 | 标准 |
|--------|------|------|
| 合规评分 | `curl /api/v1/dashboard/compliance-score` | 返回有效分数 |
| 风险热图 | `curl /api/v1/dashboard/risk-heatmap` | 返回热图数据 |
| 审计链 | `curl /api/v1/audit-chain/{batch_id}` | 返回图数据 |
| 实时更新 | WebSocket 连接 | 推送更新 |

### 5.2 性能验收

| 指标 | 标准 |
|------|------|
| 页面加载 | < 2s |
| 数据刷新 | < 1s |
| WebSocket 延迟 | < 100ms |
| 并发用户 | > 100 |

---

## 六、相关文档

- `docs/releases/v3.3.0/TRUST_INFRASTRUCTURE_STRATEGY.md` - 战略定位
- `crates/viz-dashboard/` - 仪表盘服务
- `crates/viz-renderer/` - 渲染器
- `crates/viz-risk/` - 风险计算

---

*本文档由 hermes-agent 生成*
*版本 1.0 - 2026-05-18*
