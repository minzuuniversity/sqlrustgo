# 当前版本状态

v3.4.0

## 阶段信息

- **阶段**: RC (Release Candidate)
- **当前里程碑**: v3.4.0 RC
- **开始日期**: 2026-05-23
- **开发分支**: develop/v3.4.0
- **RC 分支**: rc/v3.4.0 (commit `27f851a1`)
- **目标**: GA 门禁通过

## 版本概述

v3.4.0 定位为 **GMP Management Suite**，在 v3.3.0 可信内核基础上构建完整的管理套件：

- **GMP Management API**: EBR、Electronic Signature、Audit、Device、Rule Editor、Dashboard
- **GMP Retrieval v2**: BM25 + RRF + Ollama Reranker + LLM Chat
- **Trust Infrastructure**: perf-baseline、crash-sim、WAL verification、compliance、evidence

**战略演进**:
```
v3.2.0: Trust Convergence（可信收敛）✅ GA
v3.3.0: Industrial Trust Platform（可信内核）✅ GA
v3.4.0: GMP Management Suite（管理套件）← 当前
v3.5.0: AI Native GMP Platform
```

## Alpha/Beta 门禁状态

| Gate | 结果 | 日期 |
|------|------|------|
| Alpha (16/16) | ✅ PASS | 2026-05-22 |
| Beta (14/14) | ✅ PASS | 2026-05-22 |
| RC | ⏳ 进行中 | 2026-05-23 |

详见: [docs/releases/v3.4.0/README.md](docs/releases/v3.4.0/README.md)

## v3.4.0 核心功能

### P0 功能

| 模块 | 功能 | PR |
|------|------|-----|
| GMP API | EBR Batch Manager | #1260 |
| GMP API | Electronic Signature Service | #1261 |
| GMP API | Audit Chain API | #1260 |
| GMP API | Device OPC UA Integration | #1262 |
| GMP API | Rule Engine Visual Editor | #1263 |
| GMP Retrieval | BM25 + RRF Fusion | - |
| GMP Retrieval | Ollama Reranker | - |
| GMP Retrieval | LLM Chat (qwen3:8b) | - |

### Executor 覆盖率提升

| 目标 | 结果 |
|------|------|
| Alpha 目标 | ≥70% (v3.3.0: ~24%) |
| Beta 目标 | ≥80% |
| GA 目标 | ≥85% |

## 下一步工作

| 优先级 | 内容 |
|--------|------|
| P0 | RC Gate (R1-R16) |
| P0 | TPC-H SF=1 验证 (Z6G4) |
| P1 | GA Gate (G1-G35) |
| P1 | Coverage 提升至 ≥85% |
