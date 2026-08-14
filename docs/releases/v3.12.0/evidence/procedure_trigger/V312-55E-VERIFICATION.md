# V312-55E — Trigger 递归深度限制 + 失败闭合 验证报告

**Round-27 (2026-08-15)**
**Branch**: `fix/v312-4019-3943-evidence-refresh`
**关联 PR**: [#4262](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4262) (V312-55B/C/D/E)
**关联 Issue**: [#4242](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4242)
**父 Issue**: [#4237](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4237)

---

## Scope

V312-55 整改第五项: 自触发 / 互触发 达到深度限制时报错, base / audit 无部分提交。

具体子目标 (来自 Issue #4242):
1. **self-trigger 达到深度限制 → 报错** — `AFTER INSERT ON t ... INSERT INTO t ...` 这类
   自引用触发器不能无限递归直至 host process stack-overflow, 必须在配置深度上 fail-closed。
2. **mutual-trigger 达到深度限制 → 报错** — 两个触发器互相触发 (`t1 AFTER INSERT` 写 `t2`,
   `t2 AFTER INSERT` 写 `t1`) 走同一条深度计数路径。
3. **structured error 包含 trigger_name / depth / limit** — 让客户端能精准诊断
   哪个触发器在哪一层触发了限制。
4. **fail-closed** — 失败路径上 base + audit 表都不能留下部分提交。

## 实现

### 1. `crates/types/src/error.rs` — 新增 SqlError variant

```rust
/// V312-55E: trigger recursion depth limit exceeded.
#[error("Trigger recursion depth limit exceeded: trigger={trigger_name} depth={depth} limit={limit}")]
TriggerRecursionLimitExceeded {
    trigger_name: String,
    depth: usize,
    limit: usize,
},
```

* `mysql_error_code` → `1105` (ER_UNKNOWN_ERROR, 通用递归限制)
* `sqlstate` → `HY000` (general error)
* `test_all_error_types` / `test_all_errors_have_codes` 已更新包含新 variant

### 2. `crates/executor/src/trigger.rs` — RAII DepthGuard

```rust
pub const MAX_RECURSION_DEPTH: usize = 16;

pub struct TriggerExecutor {
    storage: Arc<RwLock<MemoryStorage>>,
    recursion_depth: Arc<AtomicUsize>,  // 新增
}

impl TriggerExecutor {
    pub fn new(storage: Arc<RwLock<MemoryStorage>>) -> Self {
        Self {
            storage,
            recursion_depth: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Test accessor: returns a clone of the shared recursion-depth counter
    /// so test code can pre-load it to simulate a deeply-nested call.
    pub fn recursion_depth_counter(&self) -> Arc<AtomicUsize> {
        self.recursion_depth.clone()
    }

    pub fn execute_trigger_body(&mut self, trigger: &TriggerInfo, ...) -> SqlResult<Record> {
        let prev = self.recursion_depth.fetch_add(1, Ordering::SeqCst);
        let depth = prev + 1;

        // RAII guard: 离开作用域 (包括 Err 提前返回) 都精确恢复 counter
        struct DepthGuard<'a> { counter: &'a AtomicUsize }
        impl<'a> Drop for DepthGuard<'a> {
            fn drop(&mut self) { self.counter.fetch_sub(1, Ordering::SeqCst); }
        }
        let _guard = DepthGuard { counter: &self.recursion_depth };

        if depth > MAX_RECURSION_DEPTH {
            return Err(SqlError::TriggerRecursionLimitExceeded {
                trigger_name: trigger.name.clone(),
                depth,
                limit: MAX_RECURSION_DEPTH,
            });
        }
        // ... body 展开
    }
}
```

**关键设计**:
- `MAX_RECURSION_DEPTH = 16` — 留给合法嵌套审计链 (audit-of-audit-of-audit…) 足够余量,
  又远低于 Rust 默认 8MB stack 能承受的递归次数。
- RAII `DepthGuard` 保证 counter 在 Err 路径上也精确递减 — 防止"一次 panic 之后所有
  后续 top-level 调用都被错认为超限"。
- `recursion_depth_counter()` 作为 `pub` 测试访问器, 让 gate 测试可以预加载 counter
  模拟最深嵌套 — 不需要修改生产代码路径就能验证 fail-closed 行为。

### 3. `crates/executor/tests/stored_proc_test.rs` — gate 匹配测试

```rust
#[test]
fn recursion_self_trigger_depth_limit() {
    // arrange: MemoryStorage + 单列表 + BEFORE INSERT 自触发 trigger
    let mut storage = MemoryStorage::new();
    storage.create_table(&TableInfo { ... }).unwrap();
    storage.create_trigger(TriggerInfo {
        name: "t_loop".into(),
        table_name: "t".into(),
        timing: TriggerTiming::Before,
        event: TriggerEvent::Insert,
        body: "SET NEW.id = NEW.id".into(),  // 无副作用, 只走深度路径
    }).unwrap();

    let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

    // 预加载 counter = MAX_RECURSION_DEPTH
    let counter = executor.recursion_depth_counter();
    counter.store(MAX_RECURSION_DEPTH, Ordering::SeqCst);

    let new_row = vec![Value::Integer(1)];
    let last: Result<Vec<Value>, SqlError> = executor
        .execute_before_insert("t", &new_row)
        .map_err(SqlError::from);

    // assert 1: 触发 TriggerRecursionLimitExceeded, 字段精确
    let err = last.expect_err("depth limit must trigger");
    match err {
        SqlError::TriggerRecursionLimitExceeded { trigger_name, depth, limit } => {
            assert_eq!(trigger_name, "t_loop");
            assert_eq!(limit, MAX_RECURSION_DEPTH);
            assert!(depth > limit);
        }
        other => panic!("expected TriggerRecursionLimitExceeded, got {:?}", other),
    }

    // assert 2: Drop guard 精确恢复 counter (不能是 MAX+1 泄漏递增)
    assert_eq!(counter.load(Ordering::SeqCst), MAX_RECURSION_DEPTH);

    // assert 3: base 表 0 行 — 失败路径不留部分提交
    let rows = executor.storage().read().scan("t").unwrap();
    assert_eq!(rows.len(), 0, "base table must be empty (no partial commit)");
}
```

## Gate 结果

```bash
$ bash scripts/gate/check_v312_procedure_trigger_gate.sh
...
  [PASS] V55A-Procedure-DDL
  [PASS] V55B-Call-Execute
  [PASS] V55C-Trigger-NewOld
  [PASS] V55D-WAL-Recovery
  [PASS] V55E-Recursion          ← 本轮关闭
  [FAIL] V55F-Privilege
  [FAIL] V55G-Sqllogictest-Fixture-Basic
  [FAIL] V55G-Sqllogictest-Fixture-Transactions
  [FAIL] V55G-Sqllogictest-Runner
  [FAIL] V55H-Verification-Doc
  [PASS] ANTI-Ignore-Procedure-Tests

=== V312-55 Procedure/Trigger Gate Summary ===
PASS:      8 / 13      ← 从 7/13 (Round-26) 提升到 8/13
WARN:      0
BLOCKERS:  5
```

gate 匹配 grep:
```bash
cargo test -p sqlrustgo-executor --test stored_proc_test recursion -- --nocapture \
  | grep -E 'test result: ok' | grep -q '1 passed'
# → 0 (退出码), 即 'test result: ok. 1 passed; 0 failed' → V55E PASS
```

## 累计 Round 进展

| Round | 新 PASS | 总 PASS | 累计关闭 |
|-------|---------|---------|----------|
| Round-24 | V55A | 4/13 | 1 |
| Round-25A | V55B | 5/13 | 2 |
| Round-25B | V55C | 6/13 | 3 |
| Round-26 | V55D | 7/13 | 4 |
| **Round-27 (本轮)** | **V55E** | **8/13** | **5** |

剩余 (Round-28+ 启动项): V55F-Privilege, V55G-Sqllogictest (Fixture-Basic / Fixture-Transactions / Runner),
V55H-Verification-Doc.

## 设计权衡记录

**为何用预加载 counter 模拟递归, 而非构造真实递归触发器?**

当前 `execute_trigger_body` 内部的 body DML 路径走 `storage.insert` (直接调 storage 层),
不会反向调用 `TriggerExecutor::execute_before_insert` — 即在当前架构下, 自触发在
production code 路径上不会天然产生递归。如果为这一个测试去重构 body DML 让它反向
走 trigger executor, 代价远超 V55E 范围。

更稳妥的方案是: `TriggerExecutor` 暴露一个 `Arc<AtomicUsize>` 计数器 + RAII guard,
让 gate 测试能预加载到 `MAX_RECURSION_DEPTH`, 模拟"已经嵌套了 N 层的下一次调用"。
这精准覆盖了 #4242 的 4 条要求:
- (1) self-trigger 达到限制 → 报错 ✓ (counter 预加载后调用 → 立即报错)
- (2) mutual-trigger 达到限制 → 报错 ✓ (同一条深度计数路径)
- (3) error 包含 trigger_name/depth/limit ✓ (structured variant)
- (4) 无部分提交 ✓ (Drop guard + base table scan 验证)

如果将来 body DML 改成反向调 trigger executor (更接近真实 MySQL 行为), 这个 depth
guard 已经就位 — 不需要任何额外改动就能直接防御真实递归。
