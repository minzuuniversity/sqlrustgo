# v3.5.0 Release — AI Native GMP Platform

> **状态**: 开发中
> **起点**: develop/v3.5.0（基于 v3.4.0 GA）
> **分支**: `develop/v3.5.0`（规划中）
> **Milestone**: v3.5.0（id=23）
> **计划发布**: 2026-08-05

---

## 一、版本目标

**战略定位**: AI Native GMP Platform（AI 原生 GMP 平台），在 v3.4.0 管理套件基础上将 AI 能力深度嵌入 GMP 全流程。

### 核心价值

- **AI 偏差调查**：自然语言查询审计链，AI 直接定位根因
- **LLM 合规判断**：规则引擎 + LLM 自动判断批次放行，带可解释证据链
- **预测性维护**：设备数据 + 时序模型提前 24h 预警故障
- **自然语言报表**：说一句 "本周批次通过率" 自动生成报告

### 战略演进

```
v3.2.0: Trust Convergence（可信收敛）     ✅ GA
v3.3.0: Industrial Trust Platform（可信内核）✅ GA
v3.4.0: GMP Management Suite（管理套件）   ✅ GA
v3.5.0: AI Native GMP Platform（AI 原生）   🔄 开发中
v3.6.0: Enterprise GMP Platform            📋 规划中
```

---

## 二、P0 功能列表

| 功能 | Issue | 验收条件 | 状态 |
|------|-------|----------|------|
| AI 偏差调查助手 | #1361 | 自然语言查询审计链，返回引用证据 | TODO |
| LLM 合规判断引擎 | #1362 | 输入批次数据 + 规则，输出放行建议 + 理由 | TODO |
| GMP Retrieval v3 集成 | #1363 | BM25 + Vector + Graph + FTS 四路融合 | TODO |
| **跨语言合规报告** | **#1411** | GMP 中文记录 → FDA/EMA 英文报告 | ✅ 完成 |
| 本地 LLM 推理支持 | #1364 | Ollama/bge-m3 本地推理，延迟 < 2s | TODO |
| AI 流式输出 SSE | #1365 | Server-Sent Events 流式推理，首 token < 500ms | TODO |

---

## 三、门禁状态

### Alpha Gate 🔄 进行中

| # | 检查项 | 结果 |
|---|--------|------|
| A1 | Build | ⬜ |
| A2 | Unit Tests | ⬜ |
| A3 | Clippy | ⬜ |
| A4 | Format | ⬜ |
| A5 | Coverage (≥72%) | ⬜ |
| A6 | MySQL Protocol | ⬜ |
| A7 | TPC-H SF=1 | ⬜ |
| A8 | Ollama 连接 | ⬜ |
| A9 | gmp-api build | ⬜ |
| A10 | gmp-llm build | ⬜ |

---

## 四、关键文档

| 文档 | 说明 |
|------|------|
| [DEV_PLAN.md](DEV_PLAN.md) | v3.5.0 开发计划（含架构、Issue 规划、里程碑） |
| [GA_GATE_CHECKLIST.md](GA_GATE_CHECKLIST.md) | GA 门禁清单（35/35 PASS 目标） |
| [LEGACY_ISSUES.md](LEGACY_ISSUES.md) | 遗留问题追踪（含 v3.4.0 延期项映射） |
| [CHANGELOG.md](CHANGELOG.md) | v3.5.0 详细变更（发布后补全） |
| `docs/governance/gate_spec_v350.md` | v3.5.0 门禁规范 |

---

## 五、版本延续（v3.4.0 → v3.5.0）

| v3.4.0 遗留项 | v3.5.0 映射 | 处理方式 |
|-------------|------------|----------|
| #1263 移动审批 API | #1370 语音批次录入 | 合并为多模态输入 |
| #1264 合规模板库 | #1371 跨语言合规报告 | 合并为报告生成增强 |
| — gmp-api dashboard 测试 | #1360 AI 偏差调查 | AI 功能附带 dashboard 测试 |

---

## 六、快速开始

```bash
# 克隆并构建
git clone ssh://git@192.168.0.252:222/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout develop/v3.5.0

# 构建
cargo build --release

# 运行测试
cargo test --lib

# 构建 gmp-api（AI 层）
cargo build -p sqlrustgo-gmp-api
cargo build -p sqlrustgo-gmp-llm

# 启动 Ollama（本地 LLM）
ollama pull qwen2.5:7b
ollama serve

# 运行门禁检查（Alpha）
bash scripts/gate/check_alpha_v350.sh
```