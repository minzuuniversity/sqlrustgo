# v3.5.0 开发测试计划

> **版本**: v1.0
> **创建日期**: 2026-05-24
> **维护人**: hermes-agent
> **战略定位**: AI Native GMP Platform（AI 原生 GMP 平台）
> **前置版本**: v3.4.0 (GMP Management Suite)
> **起点**: v3.4.0 GA release point

---

## 一、版本目标与约束

### 1.1 核心目标

**v3.5.0 战略定位**：AI Native GMP Platform，在 v3.4.0 管理套件基础上将 AI 能力深度嵌入 GMP 全流程。

**战略依赖**：
```
v3.3.0: Industrial Trust Platform（可信内核）
    ↓
v3.4.0: GMP Management Suite（管理套件）
    ↓
v3.5.0: AI Native GMP Platform（AI 原生 GMP 平台）
    ↓
v3.6.0: Enterprise GMP Platform
```

**核心判断**：v3.4.0 建立了完整的 GMP 管理界面，但 AI 能力（检索、聊天、偏差分析）处于插件状态。v3.5.0 将 AI 能力系统性内嵌到 GMP 核心流程——偏差调查、根因分析、合规判断、设备预测性维护——实现 "AI-native" 而非 "AI-augmented"。

### 1.2 核心价值

- **偏差调查 AI**：自然语言查询审计链，AI 直接定位根因
- **合规判断 AI**：规则引擎 + LLM 自动判断批次放行
- **预测性维护**：设备数据 + 时序模型提前预警
- **自然语言报表**：说一句 "本周批次通过率" 自动生成报告

### 1.3 版本约束

| 约束 | 说明 |
|------|------|
| 必须基于 v3.4.0 GA | 依赖 GMP Management Suite 全部功能 |
| AI 能力必须内嵌而非外挂 | LLM 调用延迟 < 2s，支持流式输出 |
| 合规判断必须有可解释性 | 不能是黑盒输出，必须有证据链 |
| 支持本地模型部署 | 不依赖外部云 AI 服务（企业合规） |

---

## 二、功能列表

### 2.1 P0 必须完成

| 功能 | Issue | 验收条件 | 状态 |
|------|-------|----------|------|
| AI 偏差调查助手 | #1360 | 自然语言查询审计链，返回引用证据 | TODO |
| LLM 合规判断引擎 | #1361 | 输入批次数据 + 规则，输出放行建议 + 理由 | TODO |
| GMP Retrieval v3 集成 | #1362 | BM25 + Vector + Graph 三路融合，接入 GMP API | TODO |
| 本地 LLM 推理支持 | #1363 | Ollama/bge-m3 本地推理，延迟 < 2s | TODO |
| AI 流式输出 (Server-Sent Events) | #1364 | SSE 实时输出 AI 推理过程 | TODO |

### 2.2 P1 强烈建议

| 功能 | Issue | 验收条件 | 状态 |
|------|-------|----------|------|
| 预测性设备维护 | #1365 | 时序数据 + 模型预测设备故障，提前 24h 预警 | TODO |
| 自然语言报表生成 | #1366 | 一句话生成批次/合规/偏差报表（Markdown/PDF） | TODO |
| 审计链 AI 摘要 | #1367 | 长审计链自动摘要为 200 字摘要 | TODO |
| 规则自动推荐 | #1368 | 基于历史偏差数据推荐新规则 | TODO |

### 2.3 P2 期望完成

| 功能 | Issue | 验收条件 | 状态 |
|------|-------|----------|------|
| 多模态输入（图片 + 文本） | #1409 | 上传现场图片 + 文字描述，AI 综合判断 | TODO |
| 语音批次录入 | #1410 | 语音转文字，录入批次步骤 | TODO |
| 跨语言合规报告 | #1411 | 中文批次记录 → 英文审计报告 | TODO |
| AI 合规培训助手 | #1412 | 新员工 AI 培训，GMP 规则问答 | TODO |

### 2.4 废弃/移除功能

| 功能 | 废弃原因 | 迁移指南 |
|------|----------|----------|
| GMP Retrieval v2 独立服务 | v3 版本已整合入 GMP API | 迁移到 /api/v1/gmp/retrieve |

---

## 三、技术架构

### 3.1 整体架构

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         AI Native GMP Platform (v3.5.0)                       │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      AI Agent Layer (New)                              │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │   │
│  │  │ Deviation   │  │ Compliance  │  │  Report     │  │  Device     │   │   │
│  │  │ Investigator│  │  Judge      │  │  Generator  │  │  Predictor  │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │   │
│  │                              │                                          │   │
│  │                    ┌─────────┴─────────┐                                │   │
│  │                    │  LLM Orchestrator │                                 │   │
│  │                    │  (Ollama / OpenAI) │                                │   │
│  │                    └──────────────────┘                                │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                      GMP API Layer (v3.4.0 GA)                          │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │   │
│  │  │ Batch API   │  │ Audit API   │  │ Device API  │  │ Rule Engine  │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                 Trust Infrastructure (v3.3.0 GA)                        │   │
│  │  Evidence Engine │ Provenance Graph │ Compliance Engine │ Workflow V2    │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 项目结构

```
gmp/
├── gmp-api/                    # GMP Management API (Rust Axum)
│   ├── src/
│   │   ├── ai/                # AI Agent Layer (New, P0)
│   │   │   ├── orchestrator/  # LLM Orchestrator
│   │   │   ├── deviation/     # Deviation Investigator Agent
│   │   │   ├── compliance/    # Compliance Judge Agent
│   │   │   ├── report/         # Report Generator Agent
│   │   │   └── device/        # Device Predictor Agent
│   │   ├── batch/
│   │   ├── audit/
│   │   ├── device/
│   │   ├── signature/
│   │   └── export/
│   └── Cargo.toml
│
├── gmp-web/                   # React Web UI (v3.4.0)
│   └── src/
│       ├── components/ai/    # AI UI Components (New)
│       │   ├── ChatPanel/    # AI Chat Panel
│       │   ├── ReportGen/     # Report Generator UI
│       │   └── DeviationAI/  # Deviation Investigation UI
│       ├── pages/
│       └── stores/
│
├── gmp-llm/                   # LLM Integration (New, P0)
│   ├── src/
│   │   ├── local/            # Ollama integration (bge-m3)
│   │   ├── remote/           # OpenAI-compatible API
│   │   ├── embedding/       # Text embedding (bge-m3)
│   │   └── streaming/        # SSE streaming support
│   └── Cargo.toml
│
└── gmp-shared/
    └── types/
```

### 3.3 依赖关系

```
v3.4.0 GA (GMP Management Suite)
    │
    ├─ AI Agent Layer (gmp-api/src/ai/)         [NEW P0]
    │   ├─ LLM Orchestrator (Ollama/OpenAI)
    │   ├─ Deviation Investigator
    │   ├─ Compliance Judge
    │   └─ Report Generator
    │
    ├─ GMP Retrieval v3 (gmp-api/src/retrieve/) [NEW P0]
    │   └─ BM25 + Vector + Graph 三路融合
    │
    ├─ gmp-llm (Local LLM Inference)             [NEW P0]
    │   ├─ Ollama (qwen2.5/llama3)
    │   └─ bge-m3 embedding
    │
    └─ GMP Web (gmp-web/)                         [v3.4.0]
        └─ AI UI Components (New)
```

### 3.4 AI 模型选型

| 用途 | 推荐模型 | 部署方式 | 延迟目标 |
|------|---------|---------|----------|
| Deviation Investigation | qwen2.5:7b | Ollama 本地 | < 2s |
| Compliance Judgment | qwen2.5:14b | Ollama 本地 | < 3s |
| Report Generation | qwen2.5:7b | Ollama 本地 | < 1s |
| Device Prediction | timeseriesfm | Ollama 本地 | < 5s |
| Embedding | bge-m3:latest | Ollama 本地 | < 500ms |
| **Fallback** | gpt-4o-mini | OpenAI API | < 5s |

> **企业合规**：默认使用本地 Ollama 模型。如需使用云端模型，必须在配置中显式开启。

---

## 四、测试策略

### 4.1 测试分层

| 层级 | 测试类型 | 执行环境 | 频率 |
|------|----------|----------|------|
| L0 | 单元测试（AI Agent 逻辑） | CI | 每次 PR |
| L1 | 集成测试（API + LLM mock） | CI | 每次 PR |
| L2 | E2E 测试（Playwright） | CI | 每次 PR |
| L3 | AI 质量评估（LLM-as-Judge） | Z6G4 | Beta 前 |
| L4 | 本地推理性能测试 | Z6G4 + Ollama | RC 前 |

### 4.2 AI 质量评估方法

```bash
# LLM-as-Judge 评估偏差调查报告质量
python3 scripts/eval/llm_judge.py \
  --input eval/datasets/deviation_reports.json \
  --model ollama/qwen2.5:7b \
  --criteria "accuracy,relevance,completeness" \
  --output eval/results/deviation_judge.json

# 评估标准
| 维度 | 指标 | 阈值 |
|------|------|------|
| 准确性 | 偏差根因正确识别 | ≥ 90% |
| 相关性 | 引用证据与判断相关 | ≥ 85% |
| 完整性 | 包含全部关键要素 | ≥ 80% |
| 可解释性 | 判断理由清晰 | ≥ 90% |
```

### 4.3 测试命令

```bash
# 后端测试（包含 AI Agent）
cd gmp-api
cargo test                              # 单元测试
cargo test --test ai_integration        # AI 集成测试（mock LLM）
cargo test --test retrieval_v3         # GMP Retrieval v3 测试

# 前端测试
cd gmp-web
npm test                               # Jest 单元测试
npm run test:e2e                       # Playwright E2E

# LLM 本地推理测试
cd gmp-llm
cargo test --test ollama_integration   # Ollama 连接测试
cargo test --test embedding_perf      # bge-m3 性能测试

# E2E AI 测试（需要 Ollama 运行）
bash scripts/test/ai_e2e.sh
```

---

## 五、Issue 规划

### 5.1 P0 Issue（必须完成）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| #1360 | P0 | AI 偏差调查助手 | Deviation Investigator Agent + UI | 4 周 |
| #1361 | P0 | LLM 合规判断引擎 | Compliance Judge Agent | 3 周 |
| #1362 | P0 | GMP Retrieval v3 集成 | 三路融合检索 API | 2 周 |
| #1363 | P0 | 本地 LLM 推理支持 | gmp-llm crate + Ollama 集成 | 3 周 |
| #1364 | P0 | AI 流式输出 SSE | Server-Sent Events 流式推理 | 1 周 |

### 5.2 P1 Issue（Beta 目标）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| #1365 | P1 | 预测性设备维护 | Device Predictor Agent | 3 周 |
| #1366 | P1 | 自然语言报表生成 | Report Generator Agent | 2 周 |
| #1367 | P1 | 审计链 AI 摘要 | Audit Chain Summarization | 1 周 |
| #1368 | P1 | 规则自动推荐 | Rule Recommendation Engine | 2 周 |

### 5.3 P2 Issue（GA 目标）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| #1369 | P2 | 多模态输入 | Vision + Text AI 判断 | 3 周 |
| #1370 | P2 | 语音批次录入 | Whisper 语音转文字 | 1 周 |
| #1371 | P2 | 跨语言合规报告 | LLM 翻译 + 报告生成 | 1 周 |
| #1372 | P2 | AI 合规培训助手 | GPT-Bot 培训问答 | 2 周 |

---

## 六、门禁

### 6.1 Alpha Gate

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| A1 | Build | `cargo build --release` + `npm run build` | 编译通过 | TODO |
| A2 | Test | `cargo test` + `npm test` | 全部通过 | TODO |
| A3 | Clippy | `cargo clippy -- -D warnings` | 零警告 | TODO |
| A4 | Format | `cargo fmt -- --check` + `npm run lint` | 通过 | TODO |
| A5 | Type Check | `npm run typecheck` | 零错误 | TODO |
| A6 | E2E Smoke | `npm run test:e2e -- --grep "smoke"` | 通过 | TODO |
| A7 | Ollama 连接 | `curl http://localhost:11434/api/tags` | Ollama 就绪 | TODO |

### 6.2 Beta Gate

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| B1 | Full E2E | `npm run test:e2e` | 全部通过 | TODO |
| B2 | AI Deviation | AI 偏差调查返回有效结果 | ≥ 90% 准确率 | TODO |
| B3 | AI Compliance | 合规判断输出可解释理由 | 通过人工评审 | TODO |
| B4 | Retrieval v3 | 三路融合检索相关性 | ≥ 80% NDCG@10 | TODO |
| B5 | Performance | LLM 推理延迟（本地） | < 2s (qwen2.5:7b) | TODO |
| B6 | SSE Streaming | AI 输出流式响应 | 首 token < 500ms | TODO |

### 6.3 RC Gate

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| R1 | Full Regression | `npm run test:e2e -- --full` | 全部通过 | TODO |
| R2 | AI Quality (LLM-as-Judge) | `python3 scripts/eval/llm_judge.py` | 综合评分 ≥ 85% | TODO |
| R3 | Local LLM Performance | 端到端偏差调查报告生成 | < 10s | TODO |
| R4 | Load Test | k6 负载测试 100 并发 | 通过 | TODO |
| R5 | TPC-H SF=1 | `./target/release/sqlrustgo-bench-cli tpch-bench` | 22/22 通过 | TODO |
| R6 | Coverage | `cargo llvm-cov test --lib` L1 CRATES | ≥ 85% | TODO |

### 6.4 GA Gate

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| G1 | Full Regression | `npm run test:e2e -- --full` | 通过 | TODO |
| G2 | AI Accuracy | 偏差根因识别准确率 | ≥ 90% | TODO |
| G3 | AI Explainability | 合规判断可解释性 | 100%（有证据链） | TODO |
| G4 | Security Audit | 依赖安全 + OWASP Top 10 | 无高危 | TODO |
| G5 | Coverage | L1 CRATES 覆盖率 | ≥ 85% | TODO |
| G6 | Documentation | 用户文档完整 | 通过 | TODO |
| G7 | Migration | v3.4.0 数据迁移 | 通过 | TODO |

---

## 七、版本延续任务

### 7.1 v3.4.0 未完成任务映射

| v3.4.0 遗留项 | v3.5.0 处理方式 | 关联 Issue |
|-------------|----------------|-----------|
| 移动审批 API（P2 未能 GA 前完成） | 延期到 v3.5.0 P2 | #1263 → #1370 |
| 合规模板库（P2 未能 GA 前完成） | 延期到 v3.5.0 P2 | #1264 → #1371 |
| gmp-api dashboard 测试覆盖不足 | 在 v3.5.0 AI 功能中补充 | #1360 关联 |

### 7.2 v3.4.0 GA Gate 发现项

| 问题 | 根因 | v3.5.0 修复计划 |
|------|------|----------------|
| G5 Coverage 脚本使用 `bc` 依赖 | Linux 兼容性问题 | v3.5.0 Gate 脚本使用 `awk` |
| rc-gate CI 无 Rust runner | runner 标签错误 | 已在 v3.4.0 修复（PR #1357） |
| G-API2~7 依赖 log_skip | 测试存在性未验证 | v3.5.0 改用 `check_test` |

---

## 八、风险与应对

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|----------|
| 本地 LLM 推理性能不达标 | 用户体验差 | 高 | 预留 qwen2.5:7b（2s 目标）；若不达标换 GPT-4o-mini fallback |
| Ollama 模型下载失败 | CI 环境不可用 | 中 | 在 CI 中预装 docker image；提供离线包 |
| AI 合规判断准确性不足 | 批次放行风险 | 高 | 人工复审机制；AI 仅提供建议不替代决策 |
| v3.4.0 管理套件不稳定 | AI 层无法构建 | 低 | v3.4.0 GA 已通过，概率极低 |
| 多模态模型体积大 | 部署困难 | 中 | 使用量化模型（q4_0）；支持远程调用 |

---

## 九、里程碑

### 9.1 版本阶段计划

| 阶段 | 目标日期 | 完成标准 |
|------|----------|----------|
| Alpha | 2026-06-15 | P0 AI Agent 核心逻辑完成（Mock LLM） |
| Beta | 2026-07-01 | P0 + P1 完成，E2E 测试通过，本地 LLM 集成 |
| RC | 2026-07-20 | 全部功能完成，AI 质量评估通过，Load Test 通过 |
| GA | 2026-08-05 | 35/35 PASS，文档同步，v3.4.0 迁移验证 |

### 9.2 AI 质量关键里程碑

| 日期 | 目标 | 验证方法 |
|------|------|----------|
| 2026-06-20 | Deviation Investigator Demo | 5 个测试偏差案例，AI 返回结果 |
| 2026-06-30 | LLM 本地推理延迟 < 2s | 单次推理计时 benchmark |
| 2026-07-10 | AI Quality Score ≥ 85% | LLM-as-Judge 评估报告 |

---

## 十、关联文档

| 文档 | 说明 |
|------|------|
| `docs/releases/v3.4.0/DEV_PLAN.md` | v3.4.0 开发计划（管理套件） |
| `docs/releases/v3.3.0/DEV_PLAN.md` | v3.3.0 开发计划（可信内核） |
| `docs/releases/VERSION_ROADMAP.md` | 版本演化总览 |
| `docs/governance/gate_spec_v350.md` | v3.5.0 门禁规范 |
| `oo/GMP-Management/WORKFLOW_V2.md` | Workflow V2 详细设计 |
| `oo/AI-Native/AI_AGENT_ARCHITECTURE.md` | AI Agent 架构设计（计划创建） |

---

## 十一、变更历史

| 版本 | 日期 | 变更说明 |
|------|------|----------|
| v1.0 | 2026-05-24 | 初始创建，基于 v3.5.0 战略定位（AI Native GMP Platform） |