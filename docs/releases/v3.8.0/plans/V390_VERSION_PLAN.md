# SQLRustGo v3.9.0 版本计划 — Production Readiness Release

<!-- env:blocked:no-ci -->

> **版本**: v3.9.0
> **类型**: **Production Readiness Release** (工程化版本, 不是功能版本)
> **主题**: Single-Node Production Candidate
> **分支**: `develop/v3.9.0` (从 `main@v3.8.0` fork)
> **创建日期**: 2026-06-05
> **状态**: Draft (基于 ChatGPT 架构师 2026-06-05 战略建议)
> **Auditor**: Hermes Agent

---

## 1. 战略定位 (核心变更)

### 1.1 版本演进关系

```
v3.7.0: MySQL-compatible SQL execution engine (session-level transaction)
v3.8.0: SQL Capability GA (单路径 ACID database, TPC-H 22/22, INT-1 Release Blocker 解除)
    ↓ (ChatGPT 评审: SQL 能力已饱和, 转向工程化)
v3.9.0: Production Readiness Release (Single-Node Production Candidate)
    ↓ (计划)
v3.10+: Distributed Production Architecture (多节点/分布式, 长期)
```

### 1.2 核心问题反转 (ChatGPT 评审 #7 关键)

**v3.8.0 之前**:
> SQL 能力够不够?
> 还缺什么 SQL 函数?
> 解析器覆盖到哪了?

**v3.9.0 开始**:
> 数据库死了以后还能不能回来?
> 备份能不能恢复?
> 升级会不会坏数据?
> 7×24h 运行会不会崩?
> 审计能不能追溯?

### 1.3 拒绝的版本定位

❌ **"Feature Release"** (继续堆 SQL 功能, 收益下降)
❌ **"Distributed Database"** (v3.9.0 阶段分布式太早, 单节点可靠性都没验证)

✅ **"Production Readiness Release"** (工程化 + 可靠性)
✅ **"Single-Node Production Candidate"** (单机生产就绪)

---

## 2. GMP-Platform 适配性

### 2.1 真实评估 (ChatGPT 评审)

| 能力 | GMP 需求 | v3.8.0 满足 | v3.9.0 满足 |
|------|----------|-------------|-------------|
| CRUD | ★★★★★ | ✅ | ✅ |
| 事务一致性 | ★★★★★ | ✅ (INT-1) | ✅ |
| 审计追踪 | ★★★★★ | ⚠️ partial | ✅ (新 G10) |
| 历史版本 | ★★★★★ | ⚠️ MVCC 基础 | ✅ (时间旅行查询) |
| 文档关联 | ★★★★★ | ✅ | ✅ |
| 全文检索 | ★★★★ | ⚠️ partial | ⚠️ partial |
| 权限控制 | ★★★★★ | ✅ (F-29) | ✅ |
| 工作流 | ★★★★★ | ✅ | ✅ |
| OLAP 分析 | ★★★ | ✅ (TPC-H 22/22) | ✅ |
| 超高 TPS | ★ | ❌ | ⚠️ (INT-2 部分) |
| 分布式扩展 | ★ | ❌ | ❌ (v3.10+ 计划) |
| 全球多活 | ☆ | ❌ | ❌ (v3.10+ 计划) |

### 2.2 GMP-Platform 适用场景

**v3.8.0 当前** (80-85% 满足):
- ✅ 开发环境, 测试环境
- ✅ 预生产, 中小规模生产 (单节点, 24h 内)
- ⚠️ 生产 (需要 Soak Test + Backup/Restore)

**v3.9.0 完成后** (90-95% 满足):
- ✅ 中小规模生产 (单节点, 7×24h)
- ⚠️ 大规模生产 (需监控/告警/灾备)
- ❌ 分布式生产 (需 v3.10+)

### 2.3 双后端架构建议 (ChatGPT 评审)

未来 GMP-Platform 应当:
```
GMP Platform
     ↓
Repository Layer (trait StorageEngine)
     ↓                ↓
SQLRustGo (默认)   PostgreSQL (大型客户)
```

SQLRustGo 作为默认/中小规模后端, PostgreSQL 作为可选分布式后端.

---

## 3. v3.9.0 资源分配 (ChatGPT 建议)

| 方向 | 占比 | 工作量 (12 周) |
|------|------|----------------|
| 架构债 (INT/ARCH/SEM) | **40%** | 180h |
| 可靠性 (Recovery/Backup/Soak) | **35%** | 158h |
| GMP 审计能力 | **15%** | 68h |
| 性能优化 | **10%** | 45h |
| **新 SQL 功能** | **0%** | 0h |
| **合计** | **100%** | **~451h** |

**关键决策**:
- v3.9.0 不再新增 SQL 功能 (Window Function, CTE, 高级函数全部推到 v3.10+)
- v3.9.0 重点是**证明数据库能稳定运行 + 能恢复 + 不会坏数据**

---

## 4. 阶段计划 (12 周, 6 Phases)

| Phase | 周 | 主题 | 关键 PR | 门禁 |
|-------|----|----|---------|------|
| **Phase 0** | W0 | 分支准备 + spec 细化 | branch + 5 个 SPEC | - |
| **Phase 1** | W1-2 | ARCH-3 + INT-3 闭环 | P0-1, P0-2 | G4, G3 |
| **Phase 2** | W3-4 | INT-2 真集成 + Savepoint | P0-3, P0-4 | G2, G5 |
| **Phase 3** | W5-6 | Backup/Restore + Crash Matrix | P1-1, P1-2 | G6, G8 |
| **Phase 4** | W7-8 | Soak Test + Upgrade Test | P1-3, P1-4 | G7, G9 |
| **Phase 5** | W9-10 | GMP 审计 + 时间旅行 | P2-1, P2-2 | G10 |
| **Phase 6** | W11-12 | 性能优化 + RC/GA 收口 | P3-1~P3-5 | G1~G10 ALL |

**注意**: Phase 1-2 是 40% 资源, Phase 3-4 是 35% 资源, Phase 5 是 15%, Phase 6 是 10%.

---

## 5. 入口标准 (Entry Criteria)

- [x] v3.8.0 GA 已发布 (HEAD `1ae74146a` 含 PR #3160)
- [x] GA 收口文档完成 (`ga/V380_GA_CLOSURE_TECHNICAL_DEBT_AND_ROADMAP.md`)
- [x] INT-1 + INT-4 + ARCH-1 + SEM-2 已 CLOSED
- [x] ARCH-3 Blocker-1+2 已 CLOSED (#3129 PR-3152)
- [ ] 5 个 SPEC 完成: ARCH-3-Complete / INT-2-Integration / INT-3-Merge / SEM-1-Savepoint / Backup-Restore

## 6. 出口标准 (Exit Criteria — GA 门禁 G1-G10 全部 PASS)

- [ ] G1: 22/22 TPC-H 保持 PASS (不退化)
- [ ] G2: INT-2 关闭 (ParallelExecutor 主路径集成 + perf 不退化)
- [ ] G3: INT-3 关闭 (Single Expression Engine)
- [ ] G4: ARCH-3 关闭 (VtuGuard 主路径强制, grep bypass = 0)
- [ ] G5: SEM-1 关闭 (Savepoint ROLLBACK 真实还原)
- [ ] G6: Backup/Restore 完整实现 + 100+ scenarios
- [ ] G7: 24h Soak Test PASS (无内存泄漏/句柄泄漏/锁泄漏/WAL 异常增长)
- [ ] G8: Crash Matrix 100+ scenarios PASS
- [ ] G9: Upgrade Test PASS (v3.8 → v3.9 数据可读)
- [ ] G10: Audit Log + 时间旅行查询实现

---

## 7. v3.9.0 GA 阶段时间线 (草案)

| 日期 | 阶段 | 目标 |
|------|------|------|
| W0 (2026-07-01) | 分支 + SPEC | develop/v3.9.0 创建, 5 SPEC 完成 |
| W2 (2026-07-15) | Phase 1 收口 | ARCH-3 + INT-3 关闭 |
| W4 (2026-07-29) | Phase 2 收口 | INT-2 + Savepoint 关闭 |
| W6 (2026-08-12) | Phase 3 收口 | Backup/Restore + Crash Matrix |
| W8 (2026-08-26) | Phase 4 收口 | Soak + Upgrade |
| W10 (2026-09-09) | Phase 5 收口 | GMP 审计 + 时间旅行 |
| W12 (2026-09-23) | Phase 6 收口 | v3.9.0 GA 发布 |

**注**: 实际日期取决于 v3.8.0 何时正式 GA (用户授权 main 同步) 和团队容量.

---

## 8. 风险评估

### 8.1 高风险项

| 风险 | 等级 | 缓解 |
|------|------|------|
| INT-3 14/15 委托回归 | 🟠 中-高 | 全量 regression test + 二阶段发布 |
| ParallelExecutor 并发正确性 | 🟠 中-高 | SF=0.1 + SF=1 双重验证 + MySQL 对比 |
| Savepoint MVCC 真实还原 | 🟠 中-高 | TLA+ 形式化验证 + 多客户端 e2e |
| 7×24h Soak Test 发现隐藏 bug | 🟡 中 | 24h → 72h → 168h 渐进, 暴露问题有时间缓冲 |
| Backup/Restore 数据一致性 | 🟠 中-高 | PITR 测试 + 大量 scenarios |

### 8.2 不在本版本范围

- ❌ Window Function / CTE (推 v3.10+)
- ❌ 分布式 (推 v3.10+)
- ❌ Replication (推 v3.10+)
- ❌ F-30 SEQUENCE / F-36 列级权限 / F-03 GIS (推 v3.10+)
- ❌ SIMD 集成 (推 v3.10+)

---

## 9. 命名建议 (ChatGPT 评审)

**版本名**: `v3.9.0 Production Readiness Release`

可选 tag 名称:
- `v3.9.0-rc1-Production-Readiness`
- `v3.9.0-single-node-production-candidate`

---

**v3.9.0 决定**: 不是"功能版本", 是"工程化版本". 核心问题是 **"数据库死了以后还能不能回来"**.

详细开发任务见 `plans/V390_DEVELOPMENT_PLAN.md`.
详细测试计划见 `plans/V390_TEST_PLAN.md`.
