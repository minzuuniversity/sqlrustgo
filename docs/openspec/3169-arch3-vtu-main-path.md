# openspec/3169 - ARCH-3 Complete (VtuGuard 主路径强制)

> **Issue**: #3169
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 1 (W1-2)
> **工作量**: 40h

## 一、问题分析

### 1.1 ARCH-3 跨版本债背景

ARCH-3 自 v3.5.0 持续, 当前状态 (v3.8.0):
- **Blocker-1**: ✅ 已修 (PR #3152, #3129) — MemoryStorage TX state
- **Blocker-2**: ✅ 已修 (PR #3152) — autocommit 路径 set_current_tx_id
- **Blocker-3**: ❌ 未修 (本任务) — VtuGuard 主路径强制

### 1.2 当前执行路径 (before)

```
Caller (TCP/REPL/SQL)
    ↓
ExecutionEngine::execute_insert/update/delete_sql
    ↓ (直接走, 跳过 VtuGuard)
TransactionManager + WAL + Storage
```

**问题**: VtuGuard (Vectorized Tuple Update guard) 是数据正确性最后防线,
但当前主路径**没有强制调用**, 等同于"守门员不在场"。

### 1.3 目标执行路径 (after)

```
Caller (TCP/REPL/SQL)
    ↓
ExecutionEngine::execute_insert/update/delete_sql
    ↓
VtuGuard::execute_dml ← 强制主路径 (本任务修复)
    ↓
TransactionManager + WAL + Storage
```

**保障**:
- 所有 DML 必经 VtuGuard
- VtuGuard 检查: schema/uniqueness/transaction consistency
- 与 openclaw_endpoints.rs:2208/2288 集成 (subsumes #3117)

## 二、变更设计

### 2.1 ExecutionEngine 修改

**位置**: `src/execution_engine.rs`

**变更**:
1. `execute_insert/update/delete_sql` 内部包装 VtuGuard
2. 新增 `VtuGuardContext` (重用 VtuGuard 已有 API)
3. autocommit 路径不绕过 (与 INT-1 修复一致)

**伪代码**:
```rust
pub fn execute_insert(&mut self, ...) -> Result<...> {
    let vtu_guard = VtuGuard::new(&self.storage, &self.txn_manager);
    vtu_guard.execute_dml(|| {
        // 原有 execute_insert 逻辑
        self.execute_insert_inner(...)
    })
}
```

### 2.2 openclaw_endpoints 修改

**位置**: `src/openclaw_endpoints.rs:2208/2288`

**变更**: DML 路径改走 VtuGuard (替换直接 storage access)

### 2.3 新增 Gate 脚本

**位置**: `scripts/gate/check_arch3_no_bypass.sh`

**检查项**:
- `src/execution_engine.rs` 中 `VtuGuard` 引用 ≥ 3 (insert/update/delete)
- `grep -r 'bypass' src/execution_engine.rs = 0`
- `src/openclaw_endpoints.rs` 中 DML 路径使用 VtuGuard

### 2.4 新增测试

**位置**: `tests/arch3_vtu_main_path_test.rs`

**测试场景**:
1. INSERT 必经 VtuGuard
2. UPDATE 必经 VtuGuard
3. DELETE 必经 VtuGuard
4. openclaw_endpoints:2208 VtuGuard 集成
5. openclaw_endpoints:2288 VtuGuard 集成
6. VtuGuard 拒绝非法 DML (如 uniqueness violation)

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| VtuGuard 重入问题 | 死锁或重复验证 | 已设计为 RAII-style, 测试覆盖 |
| 性能开销 | 每 DML 多 1 次检查 | VtuGuard 内部 fast path, 微秒级 |
| 与 INT-1 修复冲突 | 双重 autocommit | 共享同一 autocommit 路径, 协调一致 |
| openclaw_endpoints API 变化 | 调用方需更新 | 保持接口兼容, 内部包装 |

## 四、验收标准 (G4 门禁)

```
✓ 编译通过 (cargo check RC=0)
✓ L1 unit tests 全 PASS (871/871)
✓ L3 ACID tests 全 PASS (49+ tests)
✓ ARCH-3 gate script PASS
  - grep "VtuGuard" src/execution_engine.rs | wc -l >= 3
  - grep "bypass" src/execution_engine.rs = 0
✓ tests/arch3_vtu_main_path_test.rs PASS (6 tests)
✓ openclaw_endpoints:2208/2288 VtuGuard 集成
✓ Cross-Version Debt: ARCH-3 → CLOSED
```

## 五、Subsumed Issues

完成后, 关闭:
- **#3109** (ARCH-3 VTU 主路径集成) — 5-point checklist
- **#3117** (openclaw_endpoints VtuGuard) — 5-point checklist

## 六、回滚计划

如果 G4 门禁 fail 或性能问题:
1. Revert commit
2. VtuGuard 改为 advisory (warning log) 而非 panic
3. 重新 SPEC v2

## 七、依赖

**上游**: 无
**下游**: P1-2 (Crash Test, 依赖 P0-1 + P0-4)

## 八、参考资料

- Issue #3169
- Issue #3129 (前置, 已修)
- Issue #3109 (subsumed)
- Issue #3117 (subsumed)
- V390_DEVELOPMENT_PLAN.md §P0-1
- ARCH_SEM_DEBT_REMEDIATION_PLAN.md §3
- INT-1 修复 (PR #3019)
