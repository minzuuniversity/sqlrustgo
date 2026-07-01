# SQLRustGo v3.9.0 Changelog

> **版本**: v3.9.0
> **类型**: **Production Readiness Release** (工程化版本, 非功能版本)
> **分支**: `develop/v3.9.0` (从 `main@v3.8.0` fork)
> **创建日期**: 2026-06-05
> **GA 目标**: 2026-12-15 (per Hermes audit #3252, deferred from 2026-09-23)
> **当前阶段**: **RC8** (2026-06-18) + 本机 L1 闭环 (PR #3664/#3665/#3666, 2026-07-01, HEAD `d77821f6d1`); GA 待 24h/72h/168h real soak
> **前版本**: v3.8.0

---

## 2026-07-01 — Gate Lint Drift 修复

`gate.sh` L1 门禁在 RC7 之后暴露 6 个 clippy 错误与 4 文件 fmt 漂移, 已全部修复:

### Fixed

| 文件 | 变更 |
|------|------|
| `crates/storage/src/binary_storage.rs` | 删除未使用 `use std::sync::Arc;` 与 `Read`, 删除死方法 `ensure_loaded` |
| `crates/storage/src/checkpoint.rs:185` | `sort_by` → `sort_by_key(\|b\| Reverse(b.timestamp))` |
| `crates/storage/src/engine.rs:150` | 折叠嵌套 `if` 进 `match` arm guard |
| `crates/storage/src/wal_legacy.rs:913` | `sort_by` → `sort_by_key(\|a\| a.archive_id)` |
| `crates/optimizer/src/stats.rs:401` | 提取闭包 `update_min/update_max`, 消除嵌套 `if` 触发 `collapsible_match` |
| `crates/executor/src/executor_metrics.rs:66` | `if total == 0` 改 `checked_div(...).unwrap_or(0)` |
| `crates/telemetry/src/lib.rs:153` | 同上 `checked_div` 改写 |
| `crates/vector/src/ivfpq.rs:144` | 移除冗余 `.into_iter()` |
| `crates/mysql-server/src/lib.rs:2115` | 死函数 `is_select_stmt` 加 `#[cfg(test)]` |
| `crates/mysql-server/src/lib.rs:3276` | 8-arg `run_server_*` 加 `#[allow(clippy::too_many_arguments)]` |

### fmt 漂移 (4 文件)

- `crates/cli/src/main.rs:98`
- `crates/storage/src/binary_storage.rs:566, 599, 641`
- `crates/tools/src/bin/tbl2bin.rs:9, 15`
- `tests/mixed_workload_deadlock_regression_test.rs:22`

### 验证

```
bash gate/gate.sh v3.9.0
[L1] cargo build...           [PASS]
[L1] cargo test --lib...      [PASS]
[L1] clippy...                [PASS]
[L1] cargo fmt...             [PASS]
=== Gate Result: PASSED ===
```

---

## 2026-07-01 — execution_engine 拆分 + C-ARCH-05 锁回 + SGL-001 fmt (PR #3664/#3665/#3666)

v3.9.0 本机可推进的 L1 lint + 架构整理项已全部闭环。3 个连续 PR 合并至 `develop/v3.9.0` (HEAD `d77821f6d1`)。

### Changed

| PR | 内容 | 验证 |
| --- | --- | --- |
| #3664 (issue #3661) | `refactor(execution_engine)`: 拆分 `src/execution_engine.rs` 2630 → 1471 行 (AD-001 1500 目标达标)。新文件 `engine_helpers.rs` (227), `engine_dml.rs` (840), `engine_cte.rs` (127)。 | C-ARCH-05 PASS / check_arch3_no_bypass.sh PASS / DML 11/11 + lib 25/25 |
| #3665 | `fix(gate)`: C-ARCH-05 上限从 3000/1800 过渡值锁回 1500 (3 个 gate 脚本统一) | check_arch_invariants 5/5 + check_architecture_freeze A7-3 PASS |
| #3666 | `style`: rustfmt drift on 3 test files (SGL-001 gate fix) | SGL-5/5 PASS + integration gate 4/4 |

### State

- 当前分支 `develop/v3.9.0` @ `d77821f6d1`
- 本地 4 个快速 gate 全 PASS: `check_arch_invariants` (5/5), `check_arch3_no_bypass` (G4), `check_integration_gate` (4/4), `check_architecture_freeze` (A7-3 PASS)
- Issue #3667 已开为状态快照 (P0-arch-debt), 立即关闭
- Open issues (4, 全部硬件阻塞, 本机无法推进):
  - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
  - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
  - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
  - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)

### Verification (本机 develop/v3.9.0 @ `d77821f6d1`)

```
bash scripts/gate/check_arch_invariants.sh:    5/5 PASS
  [C-ARCH-05] execution_engine.rs: 1471 lines, limit 1500, AD-001 target 1500
bash scripts/gate/check_arch3_no_bypass.sh:    PASS
bash scripts/gate/check_integration_gate.sh:   PASS (4/4)
  SGL-001 (B4 Format): PASS
  SGL-002..005: PASS
  WAL lifecycle (INV-1/2/3): PASS
bash scripts/gate/check_architecture_freeze.sh:
  A7-3 ExecutionEngine: PASS (< 1500 lines as per AD-001)
cargo fmt --check:                            clean
```

### Refs

- #3667 — 闭环声明 issue (P0-arch-debt, closed as state snapshot)
- AD-001 — 1500 行架构原始目标

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
| **INT-3 expr 完整合并** | ✅ CLOSED (#3200, #3345) — 14/14 分支委托到 executor::expr, expr_single_engine_test 20/20 PASS | 1 | #3146 follow-up |
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
| G3 INT-3 关闭 | ✅ CLOSED (14/14 branches, PRs #3200/#3345) | Phase 1 末 ✅ |
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
