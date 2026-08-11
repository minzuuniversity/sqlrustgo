# v3.12.0 Issue DAG 分析与并行执行计划

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=ac4c82b6f, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **版本**: v3.12.0
> **状态**: PLANNED (v3.11.0 GA 2026-08-09 已冻结)
> **日期**: 2026-08-09
> **数据源**: 252 Gitea issues #3887-#3911 (25 个 v3.12 任务)
> **分析基准**: `docs/releases/v3.12.0/ISSUES_PLAN.md`

---

## 1. 一、冻结 v3.11.0 + 启动 v3.12.0

### 1.1 v3.11.0 冻结 (只读)

**已完成**: 5 remote (250/252/gitcode/gitee/github) 全部 main/develop/v3.11.0/release/v3.11.0 branches + v3.11.0-ga tag 对齐到 commit `36691ed2b`（"V311-14 comprehensive rewrite"）。

**Branch 保护策略**:
- `main`: 强烈保护（priority=15, 2 reviewers required），恢复为只读
- `develop/v3.11.0`: 历史保留，只读
- `release/v3.11.0`: 历史保留，只读
- `v3.11.0-ga` tag: 不可变

**分支保护配置确认**:
```json
{
  "branch_name": "main",
  "priority": 15,
  "enable_push": false,
  "enable_push_whitelist": false,
  "enable_merge_whitelist": true,
  "merge_whitelist_usernames": ["openclaw"],
  "required_approvals": 2,
  "enable_status_check": true
}
```

### 1.2 v3.12.0 启动

- **Branch**: `develop/v3.12.0` (from 252, commit `9e157ed61` — "docs(v3.12): prepare development and test planning")
- **Master Issue**: #3887 [V312-MASTER] (总控, 25 个子 issue 关联)
- **本地**: 已 checkout 到 `develop/v3.12.0`
- **Planned GA**: 2027-01-31 (按 release/main 节奏)

---

## 2. 二、25 个 v3.12 Issue 清单

### 2.1 总览

| # | 标题 | 优先级 | 工作量 | 类别 |
|---|------|--------|--------|------|
| 3887 | [V312-MASTER] v3.12 总控 | P0 | coord | governance |
| 3888 | V312-01 前序版本阻断项处置 | P0 | 8h | blocker |
| 3889 | V312-02 GMP Schema v3.12 | P0 | 24h | gmp |
| 3890 | V312-03 GMP Corpus Ingestion | P0 | 24h | gmp |
| 3891 | V312-04 Embedding Provider | P0 | 32h | gmp+vector |
| 3892 | V312-05 Hybrid Retrieval | P0 | 40h | gmp+sql |
| 3893 | V312-06 SQL-backed Graph | P0 | 32h | gmp+graph |
| 3894 | V312-07 RAG Evidence Bundle | P0 | 24h | gmp+rag |
| 3895 | V312-08 Compliance + Audit + ACL | P0 | 32h | gmp+compliance |
| 3896 | V312-09 Backup/Restore + Upgrade | P0 | 32h | gmp+infra |
| 3897 | V312-10 Mixed Workload SOAK | P0 | 168h | gmp+soak |
| 3898 | V312-11 SQLite SQLLogicTest Gate | P0 | 16h | sql+gate |
| 3899 | V312-12 TPC-H SF=1 Correctness | P0 | 40h | sql+tpch |
| 3900 | V312-13 MySQL Wire + LOAD DATA | P0 | 32h | mysql+wire |
| 3901 | V312-14 Crash + Upgrade/Downgrade | P0 | 24h | infra |
| 3902 | V312-15 CREATE SEQUENCE executor | P0 | 16h | sql |
| 3903 | V312-16 Window/GIS/JSON | P1 | 80h | sql |
| 3904 | V312-17 Coverage + Disabled Tests | P0 | 32h | quality |
| 3905 | V312-18 SF=10 + Sysbench + Observability | P1 | 80h | perf+infra |
| 3906 | V312-19 SQL Corpus + Invariant + Sign-off | P0 | 16h | governance |
| 3907 | V312-20 v3.6-v3.10 Backlog Disposition | P0 | 16h | governance |
| 3908 | V312-21 MySQL Compat Backlog | P1 | 40h | mysql |
| 3909 | V312-22 Execution Architecture Debt | P1 | 40h | sql+execution |
| 3910 | V312-23 Storage + Index + WAL Tools | P1 | 40h | infra |
| 3911 | V312-24 Test Infrastructure Activation | P0 | 16h | quality |

### 2.2 优先级分布

- **P0**: 18 issues (75%) — 阻塞 GA 进展
- **P1**: 6 issues (25%) — 改进项
- **P2**: 0 issues

### 2.3 类别分布

- **GMP / RAG / Graph**: 7 issues (02-07 + 08) — GMP 内审检索核心
- **SQL / Completeness**: 4 issues (11, 12, 15, 16) — SQL 完善
- **MySQL / Wire**: 2 issues (13, 21) — MySQL 兼容
- **Infrastructure / Backup**: 3 issues (09, 14, 23) — 备份/恢复
- **Performance / SOAK**: 2 issues (10, 18) — 性能
- **Quality / Coverage**: 3 issues (17, 19, 24) — 质量
- **Governance / Backlog**: 4 issues (01, 20, 22, MASTER) — 治理

---

## 3. 三、Issue 间 DAG 依赖分析

### 3.1 显式依赖 (基于任务描述)

```
V312-01 (前序阻断) ─────────┐
                              │
V312-02 (GMP Schema) ─────────┼────→ V312-03 (Corpus Ingestion)
                              │              │
                              │              ↓
V312-04 (Embedding) ──────────┼──→ V312-05 (Hybrid Retrieval) ←──── V312-06 (Graph Projection)
                              │              │
                              │              ↓
                              │       V312-07 (RAG Evidence Bundle)
                              │              │
                              │              ↓
V312-08 (Compliance/ACL) ←─────┼──→ V312-09 (Backup/Restore)
                              │              │
                              │              ↓
V312-10 (Mixed Workload SOAK) ──┘              │
                              │              │
V312-11 (SQLLogicTest Gate) ──→ V312-19 (Sign-off Gate)
                              │
V312-12 (TPC-H Correctness) ──→ V312-19 (Sign-off Gate)
                              │
V312-13 (MySQL Wire) ──────────┼──→ V312-19 (Sign-off Gate)
                              │
V312-14 (Crash Recovery) ──────┼──→ V312-19 (Sign-off Gate)
                              │
V312-15 (CREATE SEQUENCE) ────→ V312-19 (Sign-off Gate)
                              │
V312-16 (Window/GIS/JSON) ────→ V312-19 (Sign-off Gate)
                              │
V312-17 (Coverage) ─────────────→ V312-19 (Sign-off Gate)
                              │
V312-18 (SF=10 Benchmark) ────→ V312-19 (Sign-off Gate)
                              │
V312-20 (v3.6-v3.10 Backlog) ──→ V312-19 (Sign-off Gate)
                              │
V312-21 (MySQL Compat Backlog)→ V312-19 (Sign-off Gate)
                              │
V312-22 (Execution Architecture) → V312-19 (Sign-off Gate)
                              │
V312-23 (Storage Backlog) ────→ V312-19 (Sign-off Gate)
                              │
V312-24 (Test Infra) ──────────→ V312-19 (Sign-off Gate)
                              │
                              └──→ V312-MASTER (3887)
```

### 3.2 DAG 拓扑排序

**Tier 0 (no dependencies)**:
- V312-01 (#3888) — 前序阻断处置
- V312-20 (#3907) — v3.6-v3.10 backlog 清算

**Tier 1 (基础)**:
- V312-02 (#3889) — GMP Schema (依赖: -)

**Tier 2 (基于 schema)**:
- V312-03 (#3890) — Corpus Ingestion (依赖: 02)
- V312-04 (#3891) — Embedding Provider (依赖: 02)
- V312-06 (#3893) — SQL Graph Projection (依赖: 02)

**Tier 3 (基于 ingestion + embedding)**:
- V312-05 (#3892) — Hybrid Retrieval (依赖: 03, 04)
- V312-08 (#3895) — Compliance + ACL (依赖: 03)

**Tier 4 (基于 retrieval)**:
- V312-07 (#3894) — RAG Evidence Bundle (依赖: 05)
- V312-09 (#3896) — Backup/Restore (依赖: 03, 04, 06)

**Tier 5 (基于端到端稳定)**:
- V312-10 (#3897) — Mixed Workload SOAK (依赖: 07, 08, 09)
- V312-14 (#3901) — Crash Recovery (依赖: 09)

**Tier 6 (独立 gate 类)**:
- V312-11 (#3898) — SQLLogicTest Gate (依赖: 24)
- V312-12 (#3899) — TPC-H Correctness (依赖: -)
- V312-13 (#3900) — MySQL Wire Hardening (依赖: -)
- V312-15 (#3902) — CREATE SEQUENCE (依赖: -)
- V312-16 (#3903) — Window/GIS/JSON (依赖: 13)
- V312-17 (#3904) — Coverage (依赖: -)
- V312-18 (#3905) — SF=10 Benchmark (依赖: 12)
- V312-21 (#3908) — MySQL Compat Backlog (依赖: 13)
- V312-22 (#3909) — Execution Architecture (依赖: -)
- V312-23 (#3910) — Storage Backlog (依赖: -)
- V312-24 (#3911) — Test Infrastructure (依赖: -)

**Tier 7 (gate 收口)**:
- V312-19 (#3906) — SQL Corpus + Sign-off Gate (依赖: 11, 12, 13, 14, 15, 16, 17, 18, 20, 21, 22, 23, 24)

**Tier 8 (最终)**:
- V312-MASTER (#3887) — 总控 (依赖: 全部)

---

## 4. 四、可独立并行执行的任务线 (Independent Tracks)

### 4.1 Track A: GMP 核心 (8 issues, 192h)

```
#3889 (Schema) → #3890 (Ingestion) → #3891 (Embedding) → #3892 (Retrieval) → #3894 (RAG Bundle)
                  ↘ #3893 (Graph) ────────────────────────↑
                                                                   
#3895 (Compliance) → #3896 (Backup) → #3897 (SOAK)
```

**依赖**: 仅内部线性
**并行**: Track B/C/D 独立

### 4.2 Track B: SQL 完善 (4 issues, 96h)

```
#3902 (CREATE SEQUENCE) ────→ #3899 (TPC-H Correctness) ────→ #3905 (SF=10 Benchmark)
                                  ↘ #3905 (SF=10)
                                  
#3898 (SQLLogicTest) ────────→ #3906 (Sign-off Gate)
#3903 (Window/GIS/JSON) ──────→ #3906
```

**依赖**: #3902 → #3899 → #3905 (linear)
**并行**: #3898, #3903 (与 B 内部并行)

### 4.3 Track C: MySQL 兼容 (2 issues, 72h)

```
#3900 (Wire + LOAD DATA) ─────→ #3908 (Compat Backlog) ────────→ #3906 (Sign-off Gate)
```

**依赖**: 线性
**并行**: 与 Track B/D 独立

### 4.4 Track D: 基础设施 (5 issues, 144h)

```
#3891 (Embedding) ───────────→ #3901 (Crash Recovery) ────────→ #3906 (Sign-off Gate)
#3896 (Backup) ─────────────→ #3901 (Crash Recovery)
                                  
#3904 (Coverage) ─────────────→ #3906 (Sign-off Gate)
#3907 (v3.6-v3.10 Backlog) ──→ #3906 (Sign-off Gate)
#3909 (Execution Architecture) → #3906 (Sign-off Gate)
#3910 (Storage Backlog) ─────┐
#3911 (Test Infra) ──────────┼──→ #3906 (Sign-off Gate)
```

**依赖**: #3901 依赖 #3896 备份
**并行**: 其他独立

### 4.5 Track Master: 治理与收口 (3 issues, 56h)

```
#3888 (前序阻断) ────────────────→ #3895 (Compliance)
#3887 (V312-MASTER) ────────────────→ 全部
#3906 (Sign-off Gate) ←────────────── 全部 Track 收口
```

**依赖**: 收口
**并行**: Track A/B/C/D 全部完成后

---

## 5. 五、推荐执行顺序 (按 4 条并行 Track)

### 5.1 并行流水线 (4 个 worker)

```
Week 1-2 (同步启动):
  Worker A: #3889 (Schema) ──────────────→ #3890 (Ingestion)
  Worker B: #3902 (SEQUENCE) ────────────→ #3898 (SQLLogicTest)
  Worker C: #3900 (Wire) ────────────────→ #3908 (Compat Backlog)
  Worker D: #3891 (Embedding) ───────────→ #3909 (Execution Arch)

Week 3-4:
  Worker A: #3891 (Embedding) ────────→ #3892 (Retrieval) ──→ #3894 (RAG Bundle)
  Worker B: #3899 (TPC-H) ─────────────→ #3905 (SF=10)
  Worker C: #3903 (Window/GIS/JSON) ────→ #3911 (Test Infra)
  Worker D: #3907 (v3.6-v3.10) ─────────→ #3904 (Coverage)

Week 5-6:
  Worker A: #3893 (Graph) ───────────────→ #3895 (Compliance) ──→ #3896 (Backup) ──→ #3897 (SOAK)
  Worker B: #3910 (Storage Backlog) ──────→ #3911 (Test Infra)
/  Worker C: #3901 (Crash Recovery) ──→ #3906 (Sign-off Gate)
  Worker D: 

Week 7-8 (收口):
  All Workers: #3906 (Sign-off Gate) ──→ #3895 (Compliance) ──→ #3897 (SOAK)
  Final: #3887 (V312-MASTER) — promote to GA
```

### 5.2 完全可并行的子任务组 (无相互依赖)

**Group 1 (可立即并行启动)**:
- #3888 (前序处置) — 纯盘点
- #3907 (v3.6-v3.10 清算) — 纯盘点
- #3900 (MySQL Wire) — 独立模块
- #3902 (CREATE SEQUENCE) — 独立 executor
- #3904 (Coverage) — 独立测试
- #3909 (Execution Arch) — 独立审计
- #3910 (Storage Backlog) — 独立盘点
- #3911 (Test Infra) — 独立盘点
- #3892 (Hybrid Retrieval) — 独立 query planner
- #3893 (SQL Graph) — 独立 SQL 实现

**Group 2 (Group 1 完成后并行)**:
- #3890 (Ingestion) — 依赖 #3889
- #3891 (Embedding) — 依赖 #3889
- #3899 (TPC-H Correctness) — 依赖 #3902
- #3903 (Window/GIS/JSON) — 依赖 #3900
- #3908 (MySQL Compat Backlog) — 依赖 #3900

**Group 3 (Group 2 完成后并行)**:
- #3895 (Compliance) — 依赖 #3890
- #3896 (Backup) — 依赖 #3890/91
- #3905 (SF=10) — 依赖 #3899
- #3901 (Crash Recovery) — 依赖 #3896

**Group 4 (最终收口)**:
- #3897 (SOAK) — 依赖 #3895/96
- #3906 (Sign-off Gate) — 几乎全部
- #3887 (MASTER) — 全部

---

## 6. 六、推荐启动顺序 (Day 1)

### 立即可启动 (无依赖)

| # | 任务 | 工作量 | 类别 |
|---|------|--------|------|
| 3888 | V312-01 前序阻断处置 | 8h | 治理 |
| 3907 | V312-20 v3.6-v3.10 backlog | 16h | 治理 |
| 3900 | V312-13 MySQL Wire Hardening | 32h | MySQL |
| 3902 | V312-15 CREATE SEQUENCE | 16h | SQL |
| 3904 | V312-17 Coverage | 32h | Quality |
| 3909 | V312-22 Execution Architecture | 40h | SQL |
| 3910 | V312-23 Storage Backlog | 40h | Infra |
| 3911 | V312-24 Test Infra | 16h | Quality |

### Schema 启动后立即可启动

| # | 任务 | 工作量 | 类别 |
|---|------|--------|------|
| 3890 | V312-03 Corpus Ingestion | 24h | GMP |
| 3891 | V312-04 Embedding | 32h | GMP+Vector |

### 4 条并行 Track 推荐

| Track | Issue 序列 | 总工时 | 启动依赖 |
|-------|-----------|--------|----------|
| **A: GMP 核心** | 3889 → 3890 → 3891 → 3892 → 3893 → 3894 → 3895 → 3896 → 3897 | 264h | 0 |
| **B: SQL 完善** | 3902 → 3899 → 3898 → 3905 → 3903 → 3906 | 192h | 0 |
| **C: MySQL 兼容** | 3900 → 3908 → 3906 | 72h | 0 |
| **D: Infra + Quality** | 3904 → 3909 → 3910 → 3911 → 3895 → 3901 → 3906 | 168h | 0 |

**总工时**: 696h (3.5 worker-month)。**4 workers 并行，~5-7 周完成**。

---

## 7. 七、识别关键路径 (Critical Path)

```
#3889 (Schema) → #3890 (Ingestion) → #3891 (Embedding)
→ #3892 (Retrieval) → #3894 (RAG Bundle)
→ #3896 (Backup) → #3897 (SOAK) → #3906 (Sign-off)
→ #3887 (MASTER)
```

**Critical Path 总工时**: 24+24+32+40+24+32+168+16 = **360h**

**Critical Path 影响**: GMP 链是 v3.12.0 GA 的关键路径，必须优先执行。

---

## 8. 八、风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| GMP 链关键路径延期 | 中 | 高 | 4 worker 并行 + 早期 schema 验证 |
| SOAK 168h 失败 | 中 | 高 | #3901 在 SOAK 前完成 |
| 多 track 资源冲突 | 低 | 中 | 独立 workspace + 资源预留 |
| 第三方 deps (Chroma → SQLRustGo) | 中 | 高 | #3891 早期验证 |
| Schema 变更影响已有测试 | 高 | 中 | #3898 #3899 早期 sanity check |

---

## 9. 九、推荐 PR 流程

1. **每条 Track 独立 feature branch**: `feature/v312-XX-track-{name}`
2. **PR 提交**: 合并到 `develop/v3.12.0` (与 v3.11 一致)
3. **Code review**: 2 reviewer required (沿用 v3.11 sign-off gate)
4. **CI**: 自动跑 coverage + clippy + test (沿用 v3.11 setup)
5. **Doc**: 每条 issue 关闭时附 evidence hash

---

## 10. 十、结论

**v3.12.0 开发计划**:
- 25 个 issue (V312-01 ~ V312-24 + MASTER)
- **4 条独立并行 Track** (GMP / SQL / MySQL / Infra)
- **总工时**: 696h (4 workers → 5-7 周)
- **关键路径**: GMP 链 (360h)
- **下一步**: 启动 10 个无依赖 issue (Group 1) + 2 个 Schem 后续 (Group 2)

**Coordination**: V312-MASTER (#3887) 监督所有 track 进度；Sign-off Gate (#3906) 收口后触发 GA promotion。

---

Co-Authored-By: hermes-agent <hermes@nousresearch.com>
