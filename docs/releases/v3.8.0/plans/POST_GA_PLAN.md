# v3.8.0+1 Post-GA Plan — TX+WAL Contract Gaps Repair
<!-- env:blocked:no-ci -->

> **Version**: v3.8.0+1
> **Type**: Post-GA patch release (target: 2026-08-15)
> **Created**: 2026-06-03
> **Owner**: claude-macmini (architect)
> **Refs**: ISSUE-2743, ADR-006, ADR-007, SPEC-013, ADR-011
> **Branch**: `release/v3.8.0+1` (待创建)

---

## 1. 目标

修复 v3.8.0 GA defer 的 12 个 TX+WAL contract gap，达到 31/31 PASS。

### 1.1 入口标准（Entry Criteria）

- [x] v3.8.0 GA 已发布
- [x] ADR-006 (TX+WAL deferral) accepted
- [x] ADR-007 (WAL architecture 4 decisions) accepted
- [x] ADR-007 Decision-4 (read-only mode) implemented
- [x] ISSUE-2740 (Crash Recovery Empirical) closed

### 1.2 出口标准（Exit Criteria）

- [ ] 31/31 contract tests PASS
- [ ] `tests/tx_wal_contract_tests.rs` from untracked → tracked
- [ ] 不退化 19 PASS + 5 exp_g + 8 e2e = 32 个现有 test
- [ ] v3.8.0+1 RC Gate PASS
- [ ] v3.8.0+1 GA 发布

---

## 2. 12 Gap 详细分类

### 2.1 TX-Lifecycle (4 tests) — Category A

| Test | 当前 | 期望 | 修复路径 |
|------|------|------|----------|
| TX-001 DML without active tx | FAIL (Ok) | FAIL (Err) | dual-mode + strict opt-in |
| TX-002 UPDATE without active tx | FAIL (Ok) | FAIL (Err) | 同上 |
| TX-003 DELETE without active tx | FAIL (Ok) | FAIL (Err) | 同上 |
| TX-004 INSERT without active tx | FAIL (Ok) | FAIL (Err) | 同上 |

**根因**：EEK v0 spec 假设 DML without active tx → Err。当前实现 implicit autocommit。

**修复策略（dual-mode）**：
- 默认（autocommit）：保持当前行为（不破坏 19/19 PASS + 8/8 e2e）
- 严格（`SQLRUSTGO_EEK_MODE=strict`）：DML without active tx → Err
- 测试用 strict 模式

### 2.2 WAL Recovery (8 tests) — Category B

| Test | 失败原因 | 修复路径 |
|------|----------|----------|
| `test_recovery_begin_then_crash_rolls_back` | Uncommitted BEGIN-only tx 没回滚 | WalStorage log_undo + RecoveryEngine undo phase |
| `test_recovery_insert_then_crash_rolls_back` | Uncommitted INSERT 没回滚 | 同上 |
| `test_recovery_prepare_then_crash_rolls_back` | Uncommitted PREPARE 没回滚 | 同上 |
| `test_recovery_multiple_tx_crash_order` | Multi-tx crash recovery order 错 | WAL replay order 算法（按 LSN 严格） |
| `test_recovery_partial_insert_write` | Partial INSERT write 错 | Atomic write + WAL 协调 |
| `test_recovery_partial_update_write` | Partial UPDATE write 错 | 同上 |
| `test_recovery_partial_delete_write` | Partial DELETE write 错 | 同上 |
| `test_recovery_crash_during_undo` | Undo 阶段 crash 错 | CRDT-style undo log |

**根因**：`wal_storage.rs` WAL append logic + `recovery_engine.rs` replay 路径不实现 strict ordering + undo 语义。

**风险**：
- WAL 是数据库核心
- 5/5 `exp_g_wal_contracts_verified` 必须保持绿色
- 19/31 contract test 也用相同 WAL 路径

**缓解**：分 3 步 PR，每步独立 gate + 不退化验证。

---

## 3. 实施路线图

### Phase A: TX-Lifecycle (3 weeks)

```
Week A.1 (2026-07-01 ~ 07-05)
  ├── SPEC: TX-lifecycle dual-mode 设计
  └── Decision: 默认 autocommit / opt-in strict (环境变量)

Week A.2 (2026-07-08 ~ 07-12)
  ├── PR-A1: SQLRUSTGO_EEK_MODE=strict 实现
  ├── PR-A2: 5 处 DML 调用点改造
  └── Gate: 19/19 旧 test 仍 PASS

Week A.3 (2026-07-15 ~ 07-19)
  ├── PR-A3: 4 个 TX-Lifecycle test 转为 PASS
  └── Gate: 4/4 新 PASS + 19/19 旧 PASS = 23/23
```

### Phase B: WAL Recovery (4 weeks)

```
Week B.1 (2026-07-22 ~ 07-26)
  ├── SPEC: WAL replay order 算法
  └── Decision: LSN 严格排序 + undo log 格式

Week B.2 (2026-07-29 ~ 08-02)
  ├── PR-B1: WalStorage::log_* 完整 ordering
  ├── PR-B2: RecoveryEngine 改写（undo phase）
  └── Gate: 5/5 exp_g + 8/8 e2e 仍 PASS

Week B.3 (2026-08-05 ~ 08-09)
  ├── PR-B3: Partial-write 修复
  └── Gate: 23/23 (Phase A baseline) + 4/4 partial-write PASS = 27/27

Week B.4 (2026-08-12 ~ 08-16)
  ├── PR-B4: Multi-tx ordering
  └── Gate: 27/27 + 4/4 multi-tx PASS = 31/31 ✅
```

### Phase C: Integration + GA (2 weeks)

```
Week C.1 (2026-08-19 ~ 08-23)
  ├── PR-C1: tests/tx_wal_contract_tests.rs → tracked
  └── Gate: 31/31 PASS

Week C.2 (2026-08-26 ~ 08-30)
  ├── v3.8.0+1 RC Gate
  ├── v3.8.0+1 GA 发布
  └── Tag + Release Notes
```

**总时间表**: 9 周（2026-07-01 ~ 2026-08-30）

---

## 4. 关键风险与缓解

| Risk | Impact | Mitigation |
|------|--------|------------|
| 19 PASS 因 lifecycle 改变而 FAIL | High | dual-mode + opt-in strict |
| 5/5 exp_g 因 WAL 改变而 FAIL | High | 分 3 步 PR + 不退化 gate |
| Multi-tx ordering 修复引入 bug | Medium | LSN strict + 严格 gate |
| 时间超期 | Medium | Phase B 可独立延期到 v3.8.0+2 |

---

## 5. 依赖项

### 5.1 前置（已就绪）

- ✅ ADR-006 (deferral)
- ✅ ADR-007 (WAL architecture 4 decisions)
- ✅ ADR-007 Decision-4 (read-only mode)
- ✅ ISSUE-2740 closed

### 5.2 后续

- Task #2771 (F-09 PR-840 Complete DML) — 可能影响 Phase A.2 DML 调用点
- Task #2774 (F-07~F-15 Ghost PR Resolution) — 独立工作流
- Cross-Version Debt 11 ACTIVE — 独立工作流

---

## 6. 状态机

每个 gap item 状态：
- `OPEN` — 未开始
- `IN_PROGRESS` — 正在修复
- `PASS` — 验证通过
- `REGRESSED` — 引起其他 test 失败（需修复）
- `DEFERRED` — 推迟到 v3.8.0+2

---

## 7. 关联

- v3.8.0 V380_RECTIFICATION_PLAN_2026-06-03.md §3.3
- ISSUE-2743 (12 gap details)
- ADR-006 (deferral)
- ADR-007 (architecture)
- ADR-011 (本 SPEC 决策)
- SPEC-013 (本计划)

## 8. Changelog

| 版本 | 日期 | 作者 | 说明 |
|------|------|------|------|
| 1.0 | 2026-06-03 | claude-macmini (architect) | 初始版本：v3.8.0+1 修复计划 |
