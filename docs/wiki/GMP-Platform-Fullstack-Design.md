# GMP-Platform Fullstack 设计

> **状态**: 已合并 (PR #63)
> **分支**: `develop/v1.0.0`
> **设计文档路径**: `docs/design/gmp-platform-fullstack-design-draft.md`

---

## 一、系统架构

```
┌──────────────────────────────────────────────────────────────────┐
│                        微信小程序 (操作工端)                         │
│  拍照/语音/扫码/电子签名/偏差上报                                    │
└────────────────────────────┬─────────────────────────────────────┘
                             │ HTTPS + JWT
                             ▼
┌──────────────────────────────────────────────────────────────────┐
│                      Rust Web 后端 (GMP-Platform)                   │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐  │
│  │ REST API    │  │ WebSocket    │  │ OpenAPI Docs (Swagger)  │  │
│  │ (axum)      │  │ (SSE 实时)   │  │ /docs                  │  │
│  └─────────────┘  └──────────────┘  └────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    Business Logic Layer                       │  │
│  │  偏差管理 | 称量记录 | CAPA | 审计追踪 | 用户/权限            │  │
│  └──────────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    Retrieval Layer (L2~L4)                    │  │
│  │  四通道检索 | RRF | Reranker | LLM 问答                      │  │
│  └──────────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │               SQLRustGo Storage Layer (L1)                    │  │
│  │  RDBMS | Vector(HNSW) | FTS(BM25) | Graph | GMP Audit Chain │  │
│  └──────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 二、数据模型 (SQLRustGo)

核心表: `users`, `gmp_records` (审计链), `deviations`, `weighing_records`, `capas`, `gmp_documents`

**审计链不可篡改**: 每条 `gmp_records` 包含 SHA-256 哈希链 (`entry_hash` + `prev_hash`)

---

## 三、REST API 端点

```
认证:
POST   /api/v1/auth/login          登录 (JWT)
POST   /api/v1/auth/refresh        刷新 Token

偏差管理:
GET    /api/v1/deviations          偏差列表
POST   /api/v1/deviations          创建偏差
GET    /api/v1/deviations/:id      偏差详情
PATCH  /api/v1/deviations/:id      更新偏差
POST   /api/v1/deviations/:id/close 关闭偏差

称量记录:
POST   /api/v1/weighing            提交称量记录 (含规则判定)
GET    /api/v1/weighing            称量记录列表

CAPA:
GET    /api/v1/capas               CAPA 列表
POST   /api/v1/capas               创建 CAPA

审计追踪:
GET    /api/v1/audit-chain         审计链查询 (分页)

检索 (复用 GMP-Platform Layer2~4):
GET    /api/v1/search               四通道检索
POST   /api/v1/search/rerank       Reranker 重排
POST   /api/v1/search/llm-answer   LLM 问答 (RAG)

实时:
GET    /api/v1/events              SSE 实时事件流
```

---

## 四、前端页面 (React)

```
/login                    登录页
/dashboard                仪表盘 (今日偏差/称量通过率/CAPA待办)
/deviations               偏差管理
/weighing                 称量记录
/capa                     CAPA 管理
/audit                    审计追踪 (只读)
/search                   GMP 知识检索 (RAG 问答)
/admin/users              用户管理
/admin/rules              规则配置
```

**技术栈**: React 18 + TypeScript + shadcn/ui + Tailwind CSS + Zustand + TanStack Table

---

## 五、微信小程序页面

```
pages/index           首页 (今日任务/待办偏差)
pages/login           登录
pages/weighing        称量操作 (扫码+拍照+提交)
pages/deviation       偏差上报 (拍照+描述+类型选择)
pages/capa_view       CAPA 查看 (操作工视角)
pages/search          GMP 知识查询 (RAG 问答)
pages/profile          个人信息 + 电子签名
```

**技术栈**: 原生微信小程序 + Taro (可选跨端)

---

## 六、GMP 合规特性

| 特性 | 实现 |
|------|------|
| 审计链不可篡改 | SQLRustGo WAL + SHA-256 哈希链 |
| 电子签名 | 密码+时间戳+哈希绑定 |
| ALCOA 原则 | 归属/清晰/同步/原始/准确 5 字段全覆盖 |
| 四层隔离 | L0 本地执行 / L2 云编排 / L3 本地验证 |
| 偏差→CAPA | 自动关联，状态机驱动 |
| 知识检索 | 四通道 RRF + Reranker + LLM RAG |

---

## 七、部署架构 (Z440)

```
Z440 (192.168.0.250)
├── GMP-Platform Web Server (axum, port 8080)
│   ├── REST API: :8080/api/v1/*
│   └── SSE: :8080/api/v1/events
├── Ollama (本地 LLM)
│   ├── bge-m3 embeddings
│   ├── bge-reranker-v2-m3
│   └── qwen2.5:14b
└── SQLRustGo (内嵌数据库，无外部依赖)
```

---

## 八、开发阶段

| 阶段 | 周期 | 交付物 |
|------|------|--------|
| Phase 0: 脚手架 | 1 周 | Rust Axum 后端骨架 + JWT 认证 + 数据库迁移 |
| Phase 1: 核心 CRUD | 2 周 | 用户/偏差/CAPA/称量 REST API + SQLRustGo 审计链 |
| Phase 2: 检索集成 | 1 周 | Layer2~4 接入 API，四通道搜索可用 |
| Phase 3: Web 前端 | 3 周 | React 管理后台，所有页面 |
| Phase 4: 微信小程序 | 2 周 | 操作工端称量+偏差上报 |
| Phase 5: 实时+报表 | 1 周 | SSE 通知 + 数据导出 |

---

## 九、竞品参考

| 产品 | 参考点 |
|------|--------|
| MasterControl | 审计链设计、工作流 |
| Veeva Vault QMS | 文档版本管理 |
| Ideagen QMS | 表单设计、状态机 |
| iQMS | 中文 UI、本地化 |

---

*设计文档: `docs/design/gmp-platform-fullstack-design-draft.md`*
*最后更新: 2026-05-22*
