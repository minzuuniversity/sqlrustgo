# v3.4.0 OO Design - GMP API & Retrieval V2

> **版本**: v1.0
> **创建日期**: 2026-05-25
> **维护人**: hermes-agent
> **分支**: `develop/v3.4.0`
> **Truthfulness Score**: 100%

---

## 一、概述

v3.4.0 的 OO 设计聚焦于 GMP API 完整实现和 Retrieval V2 的架构设计。

---

## 二、GMP API Framework

### 2.1 模块结构

```
sqlrustgo-gmp-api/
├── api/
│   ├── mod.rs
│   ├── routes.rs        # REST API 路由
│   ├── handlers.rs      # 请求处理器
│   └── middleware.rs    # 认证/日志中间件
├── models/
│   ├── mod.rs
│   ├── gmp_models.rs   # GMP 数据模型
│   └── response.rs     # 响应结构
└── services/
    ├── mod.rs
    ├── gmp_service.rs  # GMP 业务逻辑
    └── workflow.rs     # 工作流集成
```

### 2.2 核心接口

| Trait | 说明 |
|-------|------|
| `GmpService` | GMP 服务核心接口 |
| `WorkflowIntegration` | 工作流集成接口 |
| `AuditTrail` | 审计链接口 |

---

## 三、Retrieval V2 Architecture

### 3.1 四通道检索架构

```
Query → Normalize →rewrite_query
           ↓
    ┌──────┴──────┐
    ↓             ↓
  Rule        Vector
  (BM25)    (bge-m3)
    ↓             ↓
    └──────┬──────┘
           ↓
        RRF Fusion
           ↓
       Reranker
      (bge-m3)
           ↓
      Final Results
```

### 3.2 关键组件

| 组件 | 说明 |
|------|------|
| RuleRetrieval | BM25 规则检索 |
| VectorRetrieval | bge-m3 向量检索 |
| GraphRetrieval | 知识图谱检索 |
| RRF | Reciprocal Rank Fusion |
| Reranker | 重排模型 |

---

## 四、文档映射

| 文档 | 说明 |
|------|------|
| [RELEASE_NOTES.md](./RELEASE_NOTES.md) | v3.4.0 发布说明 |
| [GA_GATE_REPORT.md](./GA_GATE_REPORT.md) | GA 门禁报告 |
| [oo/OO_ROADMAP.md](./oo/OO_ROADMAP.md) | OO 文档路线图 |

---

*最后更新: {today}*
*Truthfulness Score: 100%*
