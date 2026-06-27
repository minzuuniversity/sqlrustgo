# SQLRustGo v3.9.0 Changelog

> **版本**: v3.9.0
> **类型**: **Production Readiness Release** (工程化版本, 非功能版本)
> **分支**: `develop/v3.9.0` (从 `main@v3.8.0` fork)
> **创建日期**: 2026-06-05
> **GA 目标**: 2026-12-15 (per Hermes audit #3252, deferred from 2026-09-23)
> **当前阶段**: **RC7** (2026-06-12, awaiting 24h/72h/168h soak for GA cut, see GA_GATE_REPORT.md)
> **前版本**: v3.8.0

---

## v3.9.0 (Unreleased - 2026-12-15 目标)

### 重大变更 (Breaking Changes)

无新功能添加, 仅工程化改进 (数据库可靠性 + 可恢复性 + 可审计性)

### 可靠性 (Reliability) — Phase 3-4 重点

| 改进 | 说明 | Phase | Issue |
|------|------|-------|-------|
| **Backup/Restore 100+ 场景** | 全量/增量/时间点恢复 | 3 | ✅ RC7 PASS (G6, 51 e2e) |
| **Crash Matrix 100+ 场景** | kill -9 / OOM / disk full | 3 | ✅ RC7 PASS (G8, 129 scenarios) |
| **24h Soak Test** | 1M txns 浸泡 | 4 | ✅ RC7 PASS (simulated, 1,440× compression; real pending Z6G4) |
| **Upgrade Test 50+ 路径** | v3.6/3.7/3.8 → 3.9 | 4 | 待创建 |

### 集成债务 (INT Debt Closure) — Phase 1-2 重点

| 改进 | 说明 | Phase | Issue |
|------|------|-------|-------|
| **INT-2 TransactionManager 主路径** | 跨越 5+ 版本的集成债 | 2 | #3108 (P0) |
| **INT-3 expr 完整合并** | 1 周工作量 | 1 | #3146 follow-up |
| **SEM-1 Savepoint MVCC snapshot restore** | ROLLBACK stub 修复 | 2 | #3146 |

### 架构债 (ARCH/SEM Debt Closure) — Phase 1 重点

| 改进 | 说明 | Phase | Issue |
|------|------|-------|-------|
| **ARCH-3 VTU 主路径集成** | VTU Guard 零调用修复 | 1 | #3109 (P1) |

### GMP 审计 (Audit + Time Travel) — Phase 5 重点

| 改进 | 说明 | Phase | Issue |
|------|------|-------|-------|
| **Audit 40+ 测试** | GMP 审计能力 | 5 | 待创建 |
| **Time Travel** | 历史快照查询 | 5 | 待创建 |

### 性能优化 (Performance) — Phase 6 重点

| 改进 | 说明 | Phase | Issue |
|------|------|-------|-------|
| TPC-H SF=1 性能基线 | latency/throughput 优化 | 6 | 待创建 |

### Gates (G1-G10)

| 门禁 | 状态 | 验证 |
|------|------|------|
| G1 22/22 TPC-H 保持 | TBD | Phase 6 末 |
| G2 INT-2 关闭 | TBD | Phase 2 末 |
| G3 INT-3 关闭 | TBD | Phase 1 末 |
| G4 ARCH-3 关闭 | TBD | Phase 1 末 |
| G5 SEM-1 关闭 | TBD | Phase 2 末 |
| G6 Backup/Restore | TBD | Phase 3 末 |
| G7 24h Soak | ✅ PASS (simulated) / ❌ INCOMPLETE (real) | Phase 4 末 |
| G8 Crash Matrix | TBD | Phase 3 末 |
| G9 Upgrade | TBD | Phase 4 末 |
| G10 Audit + Time Travel | TBD | Phase 5 末 |

---

## 维护信息

| 项目 | 值 |
|------|-----|
| Changelog 版本 | v3.9.0-CHANGELOG-1.0 |
| 创建日期 | 2026-06-05 |
| 维护人 | Hermes Agent |
| 状态 | ACTIVE (Unreleased) |
| 下次审查 | 每个 Phase 末尾 |

---

## 版本状态索引

| 版本 | 发布日期 | 阶段 |
|------|---------|------|
| v3.9.0 | (unreleased, 2026-12-15 目标) | GA baseline placeholder — see HONESTY NOTE below |
| v3.9.0-rc2 | 2026-06-05 | RC2 (form-only validation milestone) |
| v3.9.0-rc1 | 2026-06-05 | RC1 (form-only validation milestone) |
| v3.9.0-beta | 2026-06-05 | Beta (form-only validation milestone) |
| v3.9.0-alpha1 | 2026-06-05 | Alpha (entry baseline) |
| v3.9.0-rc3 | 2026-06-12 | G1-G16 form-only + substance PASS; 5 P0 blockers closed |
| v3.9.0-rc4 | 2026-06-12 | G1/G7/G8/G9/G13 PASS; SHA-256 + QPS baseline (on Z6G4, interrupted) |
| v3.9.0-rc5 | 2026-06-12 | G2 substance + cross-version upgrade chain (PR #3361/#3362) |
| v3.9.0-rc6 | 2026-06-12 | INT-2/INT-3 full substance tests (Issues #3146, #3108, PR #3362) |
| v3.9.0-rc7 | 2026-06-12 | Performance docs + MariaDB comparison; 6 ignore tests un-ignored |
⚠️ **RC cut 说明**: RC3-RC7 是 2026-06-12 在 develop/v3.9.0 上打标签的门禁验证里程碑，
但 24h/72h/168h real soak 在 Z6G4 上均未完成（见 SOAK_MASTER_INDEX.md 2026-06-26 修正）。
72h soak 于 2026-06-19 启动后 4 分钟因 Z6G4 网络不稳定中断。
| v3.9.0-ga | (planned, 2026-12-15) | after 24h/72h/168h real soak + all GA blocker issues closed |
| v3.8.0 | 2026-06-04 | Strong Beta |

🔴 **HONESTY NOTE (2026-06-05)**: rc1, beta, rc2 were cut based on form-only gate validation. See
`docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md` for verified findings.
Real production-equivalent coverage at rc2: ~35%. 13 critical-path items (#3221-#3231) must be
closed before legitimate v3.9.0-ga cut.
