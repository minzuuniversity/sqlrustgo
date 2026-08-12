<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `d77821f6d1`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
> - `src/execution_engine.rs` 1471 行 (AD-001 1500 目标达标, 2630 → 1471)
> - C-ARCH-05 上限锁回 1500 (从 3000/1800 统一)
> - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
> - Open issues (4, 全部硬件阻塞, 本机无法推进):
>   - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
>   - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
>   - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
>   - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)
> - 详见: issue #3667 (closed as state snapshot) + CHANGELOG.md
>
> 本文件原始内容保持不变,仅顶部加 addendum。

---

# SQLRustGo v3.9.0 Alpha1 Release Notes

> **版本**: v3.9.0-alpha1
> **发布日期**: 2026-06-05
> **分支**: `develop/v3.9.0`
> **状态**: Alpha (form-only)

---

## 一、版本概览

v3.9.0-alpha1 是 **Production Readiness Release** 的首个 Alpha 版本，标志着 SQLRustGo 从功能开发转向工程化可靠性建设。

### 核心定位

- **类型**: Production Readiness Release (工程化版本)
- **主题**: Single-Node Production Candidate
- **目标**: 数据库可靠性 + 可恢复性 + 可审计性

---

## 二、Alpha1 完成内容

### 2.1 Phase 0 任务

| 任务 | 状态 |
|------|------|
| 创建 `develop/v3.9.0` 分支 | ✅ |
| 拉取 Gitea 分支 | ✅ |
| 移动 V390 plan 3 文件到 v3.9.0/plans/ | ✅ |
| 创建 `docs/releases/v3.9.0/` 目录 + README + CHANGELOG + ROADMAP | ✅ |
| 6 issues milestone → v3.9.0 | ⏳ 进行中 |
| 启动 commit "chore: v3.9.0 branch init" | ⏳ 待执行 |
| AGENTS.md main branch v3.8.0 → v3.9.0 | ⏳ 进行中 |
| 顶层 CHANGELOG.md 迁移 v3.8.0 段 | ⏳ 进行中 |
| Gitea milestone 公告 + 通知 | ⏳ 待执行 |
| Phase 0 Gate 验证 (D1-D5 + D6 + D9) | ⏳ 待执行 |

### 2.2 文档体系建立

| 文档 | 状态 |
|------|------|
| README.md | ✅ |
| CHANGELOG.md | ✅ |
| ROADMAP.md | ✅ |
| plans/V390_VERSION_PLAN.md | ✅ |
| plans/V390_DEVELOPMENT_PLAN.md | ✅ |
| plans/V390_TEST_PLAN.md | ✅ |
| V390_COMPREHENSIVE_ASSESSMENT.md | ✅ |

---

## 三、门禁状态

### Alpha Gate (form-only)

| 门禁 | 状态 | 说明 |
|------|------|------|
| G1 TPC-H | ⏳ | form-only PASS, 实际待验证 |
| G2 Transaction | ⏳ | form-only PASS, 实际待验证 |
| G3 Expr | ⏳ | form-only PASS, 实际待验证 |
| G4 VTU | ⏳ | form-only PASS, 实际待验证 |
| G5 Savepoint | ⏳ | form-only PASS, 实际待验证 |
| G6 Backup | ⏳ | 待 Phase 3 |
| G7 Soak | ⏳ | 待 Phase 4 |
| G8 Crash | ⏳ | 待 Phase 3 |
| G9 Upgrade | ⏳ | 待 Phase 4 |
| G10 Audit | ⏳ | 待 Phase 5 |

**注**: Alpha1 为 form-only 状态，真实生产级验证待 RC3 启动。

---

## 四、后续计划

### Phase 1 (W1-2)

- ARCH-3 VTU Guard 完整集成
- INT-3 Expr 完整合并

### Phase 2 (W3-4)

- INT-2 TransactionManager 集成
- SEM-1 Savepoint MVCC snapshot restore

---

## 五、关联文档

- [ROADMAP.md](../../../../ROADMAP.md) — 6 Phase / 12 周
- [V390_VERSION_PLAN.md](../plans/V390_VERSION_PLAN.md) — 战略定位
- [V390_COMPREHENSIVE_ASSESSMENT.md](../V390_COMPREHENSIVE_ASSESSMENT.md) — 综合评估

---

*本文档为 Alpha1 发布说明，记录 v3.9.0 的首个 Alpha 版本状态。*