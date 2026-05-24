# v1.3.0 功能追踪矩阵

> **版本**: 1.3.0
> **更新日期**: 2026-05-24
> **维护人**: Hermes Agent
> **分支**: `develop/v1.3.0`

---

## 一、功能追踪总表

| ID | 功能 | Priority | Owner | 状态 | 分支 | 依赖 | 里程碑 |
|----|------|----------|-------|------|------|------|--------|
| P0-1 | 向量索引生成流水线 | P0 | — | 🔄 开发中 | `feature/issue-122-vector-embeddings` | Ollama bge-m3 | v1.3.0 |
| P0-2 | DiskGraphStore WAL 持久化 | P0 | — | 🔄 开发中 | `feature/issue-123-disk-graph-wal` | SQLRustGo #1326 | v1.3.0 |
| P0-3 | Layer 0 术语库 | P0 | — | 🔄 规划中 | 待创建 | 无 | v1.3.0 |
| P0-4 | Layer 0 NER 实体识别 | P0 | — | 🔄 规划中 | 待创建 | P0-3 | v1.3.0 |
| P0-5 | Layer 0 关系抽取 | P0 | — | 🔄 规划中 | 待创建 | P0-4 | v1.3.0 |
| P0-6 | Layer 0 多维分类 | P0 | — | 🔄 规划中 | 待创建 | P0-4 | v1.3.0 |
| P0-7 | Layer 0 相关性预评分 | P0 | — | 🔄 规划中 | 待创建 | P0-5 | v1.3.0 |
| P1-1 | Event Bus Crate | P1 | Hermes | ✅ 已完成 | `develop/v1.3.0` | 无 | v1.3.0 |
| P1-2 | Audit Middleware | P1 | Hermes | ✅ 已完成 | `develop/v1.3.0` | P1-1 | v1.3.0 |
| P1-3 | MQTT Gateway | P1 | Hermes | ✅ 已完成 | `develop/v1.3.0` | P1-1 | v1.3.0 |
| P1-4 | Binary BLOB Embeddings | P1 | Hermes | ✅ 已完成 | `develop/v1.3.0` | 无 | v1.3.0 |
| P1-5 | HNSW Index Auto-build | P1 | Hermes | ✅ 已完成 | `develop/v1.3.0` | P1-4 | v1.3.0 |
| P1-6 | clippy 清理 | P1 | Hermes | ✅ 已完成 | `develop/v1.3.0` | 无 | v1.3.0 |
| P1-7 | cargo audit 清理 | P1 | Hermes | ✅ 已完成 | `develop/v1.3.0` | 无 | v1.3.0 |
| P2-1 | Recall@K / MRR 指标 | P2 | — | 🔄 规划中 | 待创建 | P0-1 | v1.3.0 |
| P2-2 | 图谱可视化增强 | P2 | — | 🔄 规划中 | 待创建 | P0-2 | v1.3.0 |

---

## 二、P0 功能详情

### P0-1 向量索引生成流水线

**当前状态**: 🔄 开发中（feature 分支）
**Binary BLOB 部分**: ✅ 已完成（`3aeccf0`）
**待完成**: 13809 embeddings 全量生成脚本 + Ollama bge-m3 调用

**验收标准**:
- `SELECT COUNT(*) FROM gmp_embeddings` = 13809
- HNSW 索引查询 recall@20 ≥ 70%
- 生成耗时 < 2h

### P0-2 DiskGraphStore WAL 持久化

**当前状态**: 🔄 开发中（feature 分支）
**WAL 基础架构**: ✅ 已完成（`cf65908`）
**待完成**: WAL recovery 完整逻辑 + 崩溃恢复测试

**验收标准**:
- 重启后图数据完整恢复
- WAL 日志持久化到磁盘

### P0-3~7 Layer 0 知识抽取管线

**当前状态**: 🔄 规划中
**规划功能**:
- 术语库: 600+ GMP 术语 + 300+ 同义词组
- NER: 识别设备、物料、规程、人员、地点
- RE: 规则模板 + LLM 辅助关系抽取
- 分类: 文档按 GMP 八大章分类
- 预评分: 图谱节点与 GMP 条款相关性

---

## 三、已完成功能（Beta 合入）

| 功能 | PR/Commit | 合入日期 | 验证命令 |
|------|-----------|----------|----------|
| Event Bus Crate | PR #133 (`0104cb9`) | 2026-05-24 | `cargo test -p gmp-event-bus` |
| Audit Middleware | `7380869` | 2026-05-24 | `cargo clippy -p gmp-event-bus -- -D warnings` |
| MQTT Gateway | PR #136 (`28d1af5`) | 2026-05-24 | `cargo build -p gmp-event-bus` |
| Binary BLOB + HNSW | `3aeccf0` | 2026-05-24 | `cargo build -p gmp-storage` |
| DiskGraphStore WAL | `cf65908` | 2026-05-24 | `cargo build -p gmp-storage` |
| clippy cleanup | PR #135 | 2026-05-24 | `cargo clippy --all-targets -- -D warnings` |
| cargo audit | PR #135 | 2026-05-24 | `cargo audit` |

---

## 四、状态说明

| 状态 | 含义 |
|------|------|
| ✅ 已完成 | 功能已实现并合入 develop/v1.3.0 |
| 🔄 开发中 | 正在 feature 分支开发 |
| 🔄 规划中 | 已规划，尚未开始开发 |
| ❌ 已取消 | 已取消，不在 v1.3.0 范围 |
