# SQLRustGo v3.7.0 功能矩阵

> **版本**: v3.7.0
> **分支**: develop/v3.7.0
> **日期**: 2026-05-30

---

## 功能概览

| 类别 | 功能 | v3.6.0 | v3.7.0 | 状态 |
|------|------|--------|--------|------|
| **执行引擎** | Execution Telemetry v2 | ❌ | ✅ | GA |
| | Executor 模块重构 | ⚠️ | ✅ | GA |
| **存储** | WALVerifier | ✅ | ✅ | GA |
| | SIMD 向量化 | ✅ | ✅ | GA |
| **解析器** | 窗口函数 | ✅ | ✅ | GA |
| | SQL Parser 覆盖率 | 85% | 85% | GA |
| **知识集成** | qmd-bridge | ✅ | ✅ | GA |
| **治理** | Alpha/Beta/RC Gate | ✅ | ✅ | GA |

---

## 详细功能列表

### 执行引擎

| 功能 | 说明 | 优先级 |
|------|------|--------|
| trace_id 传播 | OpenTelemetry 链路追踪 | P1 |
| span 管理 | Span 生命周期管理 | P1 |
| 指标采集 | query_latency_ms, rows_scanned | P1 |
| OTLP 导出 | gRPC OTLP 导出器 | P2 |

### SQL 方言

| 特性 | MySQL | PostgreSQL | SQLite |
|------|-------|------------|--------|
| SELECT | ✅ | ✅ | ✅ |
| INSERT/UPDATE/DELETE | ✅ | ✅ | ✅ |
| 窗口函数 | ✅ | ✅ | ✅ |
| CTEs (WITH) | ✅ | ✅ | ✅ |
| prepared statements | ✅ | ✅ | ✅ |