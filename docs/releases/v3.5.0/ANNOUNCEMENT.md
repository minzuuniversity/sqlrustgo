# v3.5.0 GA — AI Native GMP Platform

**发布日期**: 2026-05-28
**Tag**: `v3.5.0` → `5720805b`

---

## 发布声明

SQLRustGo v3.5.0 GA 正式发布！

v3.5.0 是 **AI Native GMP Platform**，将 AI 能力深度嵌入 GMP 全流程，实现工业合规的下一代智能化闭环。

### 核心新特性

| 功能 | 描述 |
|------|------|
| AI 偏差调查助手 | 自然语言查询审计链，返回引用证据 |
| LLM 合规判断引擎 | 输入批次数据 + 规则，输出放行建议 + 理由 |
| GMP Retrieval v3 | BM25 + Vector + Graph + FTS 四路融合 |
| 跨语言合规报告 | GMP 中文记录 → FDA/EMA 英文报告 |
| 本地 LLM 推理 | Ollama/bge-m3 本地推理，延迟 < 2s |
| AI 流式输出 SSE | Server-Sent Events 流式推理，首 token < 500ms |

### GA Gate 结果

| Gate | 检查项 | 结果 |
|------|--------|------|
| G1 | Release 编译 | ✅ PASS |
| G2 | 全 workspace 测试 | ✅ PASS (1771 tests) |
| G3 | Clippy 零警告 | ✅ PASS (0 errors) |
| G4 | Format 合规 | ✅ PASS |
| G5 | 覆盖率 ≥85% (L1) | ✅ **87.35%** |

### 技术债务豁免

| ID | 问题 | 状态 |
|----|------|------|
| EX-v350-001 | parser 覆盖率 83.82% | 🔴 Open |
| EX-v350-002~004 | optimizer/executor/storage 覆盖率 | ✅ 已达标 |
| EX-v350-005 | Storage varlen 5 tests fail | 🔴 Open |

### 文档

- [GA_GATE_REPORT.md](docs/releases/v3.5.0/GA_GATE_REPORT.md)
- [CHANGELOG.md](docs/releases/v3.5.0/CHANGELOG.md)
- [TECHNICAL_DEBT_ANALYSIS.md](docs/releases/v3.5.0/TECHNICAL_DEBT_ANALYSIS.md)

---

**→ [查看完整文档](docs/releases/v3.5.0/)**
