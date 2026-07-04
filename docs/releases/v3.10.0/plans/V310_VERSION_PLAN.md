# SQLRustGo v3.10.0 版本计划 — MySQL 5.7 替代

> **版本**: v3.10.0
> **类型**: **MySQL 5.7 替代** — 功能稳定 + 基本性能优先
> **分支**: `develop/v3.10.0` (待从 `develop/v3.9.0` 创建, 2026-07-01)
> **当前阶段**: **DRAFT** (设计与规划)
> **创建日期**: 2026-07-01
> **目标**: 成为可生产的 MySQL 5.7 替代版本
> **前版本**: v3.9.0 (RC8, 待 GA)

---

## 1. 战略定位

### 1.1 版本演进关系

```
v3.7.0: MySQL-compatible SQL execution engine (session-level transaction)
v3.8.0: SQL Capability GA (单路径 ACID database, TPC-H 22/22, INT-1 Release Blocker 解除)
    ↓
v3.9.0: Production Readiness Release (Single-Node Production Candidate)
    ↓ (本会话 #3664/#3665/#3666 已落地: execution_engine 拆分 + C-ARCH-05 锁回 + SGL-001 fmt)
v3.10.0: MySQL 5.7 替代 — 功能稳定 + 基本性能优先  ← 当前 DRAFT
    ↓ (计划)
v3.11+: Cypher 扩展 / Vector SQL / 高级 MySQL 兼容 (可选, 未规划)
```

### 1.2 核心原则 (per Hermes audit #7 + 2026-06-05 战略建议)

**v3.10.0 的核心问题是**:
> "能不能成为别人愿意用的 MySQL 替代品?"

这包含 3 个子问题:
1. **功能完整性**: 常用 DML/DDL/DQL 完整, 不含破坏性 bug
2. **事务正确性**: ACID 四项完整, MVCC/ROLLBACK 正确
3. **基本性能**: TPC-H SF=0.01 完整正确, QPS 不显著退化

**v3.10.0 不做** (per `V310_DEVELOPMENT_PLAN.md` §0.2):
- 新语法 (Cypher CREATE/MERGE 等图查询扩展)
- SIMD / Vector SQL / 新优化器
- 高级 MySQL 函数 (GIS, FEOLE, 窗口函数扩展)
- 新索引类型 (自适应哈希, 聚簇索引的磁盘集成)

### 1.3 v3.10.0 vs v3.9.0 关系

| 维度 | v3.9.0 | v3.10.0 |
| --- | --- | --- |
| 主题 | Production Readiness (工程化) | MySQL 5.7 替代 (功能稳定 + 性能) |
| Stage | RC8 (待 GA) | DRAFT (待 ALPHA) |
| 核心任务 | 架构债 40% / 可靠性 35% / GMP 审计 15% / 性能 10% | MySQL 兼容性 11 项 (~180h) / ignore 修复 13 项 (~200h) / 性能基线 1 项 (~40h) / 稳定性 1 项 (~80h) |
| 关键成果 | 4 quick gates 全 PASS, RC8 dry-run 完成 | 待 26 项任务 (~500h) |
| GA blocker | 4 hardware-blocked issues (本机无法推进) | 待 DRAFT → ALPHA → BETA → RC → GA 流程 |

---

## 2. v3.10.0 核心目标

### 2.1 MySQL 5.7 替代版本要求

| 维度 | 要求 | 验证 |
| --- | --- | --- |
| **功能完整性** | 常用 DML/DDL/DQL 完整, 不含破坏性 bug | ignore 测试 0 个 (C-1 ~ C-4) |
| **事务正确性** | ACID 四项完整, MVCC/ROLLBACK 正确 | C-3 ROLLBACK 真正撤销 DML PASS |
| **基本性能** | TPC-H SF=0.01 完整正确; QPS 不显著退化 | TPC-H 22/22 + QPS baseline |
| **稳定性** | 24h+ soak 无错误; Crash recovery 正确 | C-5a 真实 kill -9 + C-5b 24h real soak |
| **兼容性** | 常用 SQL 语法、MySQL wire protocol、错误格式兼容 | wire protocol 兼容测试 |

### 2.2 v3.10.0 必做的 5 大类任务 (per `V310_DEVELOPMENT_PLAN.md` §1)

| 类别 | 任务数 | 工作量 | 来源 |
| --- | --- | --- | --- |
| **C-1 DML 完整性** | 5 (C-1a ~ C-1e) | ~80h | F-1 ignore 审计 (7 个 ignore) |
| **C-2 UNION 集合操作** | 3 (C-2a ~ C-2c) | ~45h | F-2 ignore 审计 (3 个 ignore) |
| **C-3 事务 ACID** | 2 (C-3a, C-3b) | ~60h | SEM-1 + F-4b |
| **C-4 ALTER TABLE** | 4 (C-4a ~ C-4d) | ~20h | SEM-3 历史债务 |
| **C-5 崩溃恢复** | 3 (C-5a, C-5b, C-5c) | ~80h | SEM-1 + T-20 + T-19 |
| **合计** | **17 任务** | **~285h** | (占 500h 预算的 57%) |

### 2.3 v3.10.0 可选 / 低优先级任务 (per `V310_DEVELOPMENT_PLAN.md` §2)

| 类别 | 任务数 | 来源 | 是否 v3.10.0 必做 |
| --- | --- | --- | --- |
| **H-1 ~ H-5 高优先级** (ARCH-2, ARCH-3 剩余, GIS, SEQUENCE, 列级权限) | 5 | 跨版本债务 | ✗ (GIS/SEQUENCE/列级权限 → v3.11) |
| **M-1 ~ M-6 中优先级** (覆盖率标准化, CBO, CREATE EVENT, 查询缓存, INT-2, INT-3) | 6 | 跨版本债务 | ⚠ (INT-2/INT-3 优先, 其他 v3.10.0+ 阶段) |
| **L-1 ~ L-3 低优先级** (FULLTEXT, AES-256, INFORMATION_SCHEMA) | 3 | 跨版本债务 | ✗ (v3.10.0 之后) |

---

## 3. v3.10.0 阶段路线图 (per `V310_DEVELOPMENT_PLAN.md` §4)

| 阶段 | 周期 | 工作量 | 目标 | 必做任务 |
| --- | --- | --- | --- | --- |
| **Phase 0** | 2 周 | ~80h | 关闭所有 ACID 正确性 bug | C-3a ROLLBACK, C-3b MemoryStorage tx, C-4 ALTER TABLE (4 项) |
| **Phase 1** | 2 周 | ~80h | 常用 DML 完整, 支持子查询 | C-1a INSERT SELECT, C-1b UPDATE subquery, C-1c Multi-table UPDATE, C-1d DELETE subquery, C-1e Multi-table DELETE |
| **Phase 2** | 2 周 | ~80h | SQL 集合操作 + 真实崩溃恢复 | C-2a INTERSECT, C-2b EXCEPT, C-2c UNION ORDER BY/LIMIT, C-5a 真实 Crash Matrix, C-5b 24h 真实 Soak |
| **Phase 3** | 2 周 | ~80h | 性能不退化 + 文档完整 | H-2 ARCH-3 VTU 剩余 5%, M-2 CBO 完善, H-1 ARCH-2 双路径统一, TPC-H 22/22, 文档收口 |

总周期: **8 周, 320h (核心任务)** + **~180h (中优先级 M-1 ~ M-6 中部分)** = **~500h**

---

## 4. 关键决策

### 4.1 必须继承的 v3.9.0 改进

- ✓ **execution_engine 拆分** (PR #3664, 2630 → 1471 行): 让 DML 实施者有清晰的位置
- ✓ **C-ARCH-05 锁回 1500** (PR #3665): 防止重新膨胀
- ✓ **SGL-001 fmt drift fix** (PR #3666): 治理 0 漂移
- ✓ **Stage Control Framework** (PR #3668): v3.10.0 直接使用, 不需要重写
- ✓ **G1-G16 framework** (PR #3669): per-version 门禁列表不再需要

### 4.2 必须开的新 PR

- v3.10.0 README.md (本文件 + STAGE.yaml + plans/INDEX.md): DRAFT 阶段已经准备好
- `develop/v3.10.0` 分支: 待创建, 从 `develop/v3.9.0` 派生
- V310_VERSION_PLAN.md (本文件)
- V310_DEVELOPMENT_PLAN.md 完善 (任务细节, sprint 排期)

### 4.3 不做的事 (避免 scope creep)

- ✗ 不实现 F-03 GIS / F-30 SEQUENCE / F-36 列级权限 (推到 v3.11+)
- ✗ 不重写 G1-G16 详细测试场景 (Stage Control Framework 已覆盖)
- ✗ 不创建新的 governance 文档 (Stage Control Framework 已经是 SSOT)
- ✗ 不实现 Cypher 扩展 (v3.11+)
- ✗ 不实现 SIMD / Vector SQL (v3.11+)

---

## 5. 依赖与阻塞

### 5.1 来自 v3.9.0 的依赖

v3.10.0 是从 `develop/v3.9.0` 派生的 (待创建分支时操作). 派生时, v3.9.0 的状态:

| 维度 | v3.9.0 当前 | v3.10.0 起点 |
| --- | --- | --- |
| Stage | RC8 | DRAFT (新开始) |
| HEAD | `d162bc4ae385` (252 端 develop/v3.9.0) | 同步 (develop/v3.10.0 = develop/v3.9.0) |
| 4 quick gates | 5/5 PASS, G4 PASS, 4/4 PASS, A7-3 PASS | 全部继承 |
| 3 PR merged (session) | #3664/#3665/#3666 | 全部继承 |

### 5.2 GA-P0 hardware-blocked items (不影响 v3.10.0 启动)

- #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
- #3423 TPC-H SF=1.0 跨引擎 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
- #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时)
- #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时)

这些是 **v3.9.0 GA 的 blocker**, 不是 v3.10.0 启动的 blocker.

### 5.3 v3.10.0 启动所需资源

- 1 个开发工作站 (本机)
- 1 个 git remote (Gitea 252 或 250)
- 1 个 CI runner (Z6G4 用于长时跑, 但 DRAFT 阶段不需要)
- 网络可达 (已确认 Gitea 252 在 2026-07-01 恢复)

---

## 6. 推广标准 (Promotion SOP)

### 6.1 DRAFT → ALPHA

1. 创建 `develop/v3.10.0` 分支
2. 完成 V310_VERSION_PLAN.md ✓
3. 完善 V310_DEVELOPMENT_PLAN.md (任务细节, sprint 排期)
4. 4 phase 计划文档 (Phase 0/1/2/3 详细)
5. 初始 gate: build + clippy + fmt PASS
6. 检查 `STAGE.yaml` 所有 required_files 存在
7. `bash scripts/gate/check_stage.sh --version v3.10.0 --stage ALPHA` PASS
8. 人工 architect 签字
9. 更新 `STAGE.yaml`: current_stage=ALPHA, last_transition={to: ALPHA, ...}
10. commit + push

### 6.2 ALPHA → BETA

- Phase 0 (ACID) 全部任务完成
- ignore 测试数从 44 → ≤10 (functional ignores)
- C-3 ROLLBACK 真正撤销 DML PASS
- C-4 ALTER TABLE 完整 PASS

### 6.3 BETA → RC

- Phase 1 (DML 完整性) 全部任务完成
- C-1a ~ C-1e 全部 PASS
- TPC-H SF=0.01 22/22 PASS

### 6.4 RC → GA

- Phase 2 (UNION + 稳定性) 全部任务完成
- C-2a ~ C-2c 全部 PASS
- 真实 24h soak 0 errors
- 真实 crash recovery PASS

### 6.5 GA cut

- Phase 3 (性能 + 文档) 全部任务完成
- GA gate 8/8 PASS
- GA_GATE_REPORT.md 签字
- Tag v3.10.0 cut
- Merge 到 main + release 分支

---

## 7. 维护信息

| 项目 | 值 |
| --- | --- |
| 文档版本 | v1.0 |
| 创建日期 | 2026-07-01 |
| 维护人 | claude-macmini (initial) |
| 状态 | DRAFT |
| 关联 | V310_DEVELOPMENT_PLAN.md, V310_CLI_BINARY_PLAN.md, STAGE.yaml, CHANGELOG.md |
| 下次审查 | DRAFT → ALPHA promotion 时 |

---

*本文档参考 Hermes audit #7 战略建议 + `V310_DEVELOPMENT_PLAN.md` 任务清单 + Stage Control Framework (PR #3668)*
