# GMP-Platform v1.2.0 测试计划

> **版本**: v1.2.0
> **日期**: 2026-05-23
> **分支**: develop/v1.2.0
> **目标**: L0-Beta → L0-RC → L0-GA

---

## 一、测试策略

### 1.1 测试金字塔

```
         ┌─────────────┐
         │   E2E 测试  │  ← TS-WEI-001~010, 10 scenarios
         ├─────────────┤
         │  集成测试   │  ← gmp-storage, gmp-retrieval, gmp-eval
         ├─────────────┤
         │   单元测试  │  ← cargo test --lib
         └─────────────┘
```

### 1.2 回归检测

```bash
# 每次 PR 执行
cd /home/openclaw/GMP-Platform
./scripts/gate/check_alpha_v120.sh
```

**警戒线**:
- 场景通过率: < 94% 阻止合并
- R8 QPS: < 5.0 阻止合并

---

## 二、L0-Beta 测试目标

| 指标 | v1.1.0 基线 | v1.2.0 Beta 目标 |
|------|-------------|------------------|
| 场景通过率 | 99.8% | 99%+ |
| L0-Alpha | 13/13 PASS | 持续通过 |
| E2E 测试 | 10/10 | 10/10 |
| Clippy | 0 errors | 0 errors |
| cargo test | 0 failures | 0 failures |

---

## 三、测试用例清单

### 3.1 功能测试

| 用例 ID | 描述 | 工具 | 状态 |
|---------|------|------|------|
| F-001 | FTS 检索 | gmp-retrieve | ✅ |
| F-002 | 向量检索 (HNSW) | gmp-retrieval | ✅ |
| F-003 | 混合检索 (RRF) | gmp-retrieval | ✅ |
| F-004 | Cypher 图查询 | gmp-retrieval | ✅ |
| F-005 | Reranker 重排 | gmp-reranker | ✅ |
| F-006 | Web Dashboard | dashboard | ✅ |
| F-007 | mobile_cli demo | gmp_agent | ✅ |
| F-008 | Audit Chain | closed_loop_demo | ✅ |

### 3.2 E2E 测试 (TS-WEI-001~010)

| 用例 ID | 描述 | 状态 |
|---------|------|------|
| TS-WEI-001 | 称重工作流 E2E | ✅ |
| TS-WEI-002 | 偏差处理 E2E | ✅ |
| TS-WEI-003 | CAPA 流程 E2E | ✅ |
| TS-WEI-004 | 设备校准 E2E | ✅ |
| TS-WEI-005 | 文档审核 E2E | ✅ |
| TS-WEI-006~010 | 扩展场景 | ✅ |

---

## 四、缺陷管理

| 缺陷 ID | 描述 | 严重度 | 状态 |
|---------|------|--------|------|
| D-001 | Web Dashboard 前端 UI 完善 | P1 | Open |
| D-002 | DiskGraphStore 持久化集成 | P0 | Open |
| D-003 | TPC-H SF=1 集成 | P0 | Open |
| D-004 | gmp-agent mobile_cli 完善 | P1 | Open |

---

## 五、门禁检查

| Gate | 检查项 | 状态 |
|------|--------|------|
| L0-Alpha | A1~A6, A-F1~F4, A-S1~S3 | ✅ 13/13 PASS |
| L0-Beta | B1~B6, B-I1~I4, B-S1~S4 | 🔄 进行中 |
| L0-RC | R1~R10 | ⏳ 待执行 |
| L0-GA | G1~G7 | ⏳ 待执行 |

---

**审批状态**: 草稿
**下次审查**: 2026-05-26
