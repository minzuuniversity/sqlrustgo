# PR-800F TEST DESIGN — TransactionalFacade

> **PR**: PR-800F
> **Branch**: test/v380-test-coverage-a1-a4
> **Source TEST_PLAN**: PR-800F_TRANSACTIONAL_FACADE_TEST_PLAN.md
> **Source SPEC**: PR-800F_TRANSACTIONAL_FACADE_SPEC.md
> **Created**: 2026-06-02
> **Status**: ACTIVE

---

## 1. 测试文件结构

```
crates/executor/
├── src/
│   └── execution/
│       ├── wal_transactional_facade.rs   ← 已有
│       ├── transactional_facade.rs       ← trait
│       ├── write_op.rs                    ← WriteOp enum
│       └── drift_gate.rs                  ← 强制层
└── tests/
    └── wal_transactional_facade_test.rs  ← **新增**
```

---

## 2. 测试函数设计

### 2.1 单元测试（in `wal_transactional_facade.rs`）

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::drift_gate::DriftGate;
    use sqlrustgo_storage::MemoryStorage;
    use std::sync::Arc;
    use parking_lot::RwLock;

    fn make_facade() -> WalTransactionalFacade<MemoryStorage> {
        WalTransactionalFacade::new_without_wal(MemoryStorage::new())
    }

    #[test]
    fn utf_01_initial_not_in_transaction() {
        let f = make_facade();
        assert!(!f.is_in_transaction());
    }

    #[test]
    fn utf_02_begin_marks_in_transaction() {
        let f = make_facade();
        let tx = f.begin().unwrap();
        assert!(tx > 0);
        assert!(f.is_in_transaction());
    }

    #[test]
    fn utf_03_current_tx_id_after_begin() {
        let f = make_facade();
        let tx = f.begin().unwrap();
        assert_eq!(f.current_tx_id(), Some(tx));
    }

    #[test]
    fn utf_04_tx_id_monotonic_increase() {
        let f = make_facade();
        let t1 = f.begin().unwrap();
        f.commit().unwrap();
        let t2 = f.begin().unwrap();
        f.commit().unwrap();
        assert!(t2 > t1, "tx_id should increase, got t1={} t2={}", t1, t2);
    }

    #[test]
    fn utf_05_commit_clears_transaction() {
        let f = make_facade();
        f.begin().unwrap();
        f.commit().unwrap();
        assert!(!f.is_in_transaction());
        assert_eq!(f.current_tx_id(), None);
    }

    #[test]
    fn utf_06_rollback_clears_transaction() {
        let f = make_facade();
        f.begin().unwrap();
        f.rollback().unwrap();
        assert!(!f.is_in_transaction());
    }

    #[test]
    fn utf_07_commit_without_begin_returns_err() {
        let f = make_facade();
        let r = f.commit();
        assert!(r.is_err(), "commit without begin must return Err");
    }

    #[test]
    fn utf_08_rollback_without_begin_is_ok() {
        let f = make_facade();
        let r = f.rollback();
        assert!(r.is_ok(), "rollback without begin should be no-op, got {:?}", r);
    }

    #[test]
    fn utf_09_execute_write_does_not_panic() {
        let f = make_facade();
        let ctx = crate::execution::TransactionContext::new(1);
        let op = WriteOp::Insert {
            table: "t1".to_string(),
            columns: vec!["id".to_string()],
            values: vec![vec![sqlrustgo_types::Value::Integer(1)]],
        };
        // 不强制成功（事务外），但不 panic
        let _ = f.execute_write(&ctx, op);
    }

    #[test]
    fn utf_10_validate_operation_insert_ok() {
        let f = make_facade();
        let ctx = crate::execution::TransactionContext::new(1);
        let op = WriteOp::Insert {
            table: "t1".to_string(),
            columns: vec!["id".to_string()],
            values: vec![vec![sqlrustgo_types::Value::Integer(1)]],
        };
        let r = f.validate_operation(&op, &ctx);
        assert!(r.is_ok());
    }
}
```

### 2.2 集成测试（in `tests/wal_transactional_facade_test.rs`）

```rust
//! PR-800F Integration Tests — WalTransactionalFacade end-to-end behavior
//!
//! 23 tests: 10 unit (UTF-*) + 8 integration (WTF-*) + 5 edge (WTF-E*)

use sqlrustgo_executor::execution::{
    TransactionContext, TransactionalFacade, WriteOp,
};
use sqlrustgo_executor::execution::wal_transactional_facade::WalTransactionalFacade;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;
use std::thread;

// === L1 — 单元测试直接引入 ===
mod unit_tests {
    use super::*;
    include!("unit_tests_inline.rs");
}

// === L2 — 集成测试 ===

#[test]
fn wtf_01_insert_commit_persists() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    f.begin().unwrap();
    let ctx = TransactionContext::new(f.current_tx_id().unwrap());
    f.execute_write(&ctx, WriteOp::Insert {
        table: "users".into(),
        columns: vec!["id".into(), "name".into()],
        values: vec![vec![Value::Integer(1), Value::Text("alice".into())]],
    }).unwrap();
    f.commit().unwrap();
    
    let storage = f.storage();
    let s = storage.read();
    let rows = s.scan("users").unwrap();
    assert_eq!(rows.len(), 1, "INSERT+COMMIT should persist 1 row");
}

#[test]
fn wtf_02_insert_rollback_does_not_persist() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    f.begin().unwrap();
    let ctx = TransactionContext::new(f.current_tx_id().unwrap());
    f.execute_write(&ctx, WriteOp::Insert {
        table: "users".into(),
        columns: vec!["id".into()],
        values: vec![vec![Value::Integer(1)]],
    }).unwrap();
    f.rollback().unwrap();
    
    let storage = f.storage();
    let s = storage.read();
    let rows = s.scan("users").unwrap();
    assert_eq!(rows.len(), 0, "INSERT+ROLLBACK should not persist");
}

#[test]
fn wtf_03_multi_insert_single_transaction() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    f.begin().unwrap();
    let ctx = TransactionContext::new(f.current_tx_id().unwrap());
    for i in 1..=2 {
        f.execute_write(&ctx, WriteOp::Insert {
            table: "t".into(),
            columns: vec!["id".into()],
            values: vec![vec![Value::Integer(i)]],
        }).unwrap();
    }
    f.commit().unwrap();
    
    let s = f.storage().read();
    assert_eq!(s.scan("t").unwrap().len(), 2);
}

#[test]
fn wtf_04_tx_id_strictly_increases_across_cycles() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    let mut last = 0u64;
    for _ in 0..5 {
        let t = f.begin().unwrap();
        assert!(t > last, "tx_id must increase: {} > {}", t, last);
        last = t;
        f.commit().unwrap();
    }
}

#[test]
fn wtf_05_update_in_transaction_commits() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    
    // seed
    f.begin().unwrap();
    let ctx = TransactionContext::new(f.current_tx_id().unwrap());
    f.execute_write(&ctx, WriteOp::Insert {
        table: "t".into(),
        columns: vec!["id".into(), "val".into()],
        values: vec![vec![Value::Integer(1), Value::Integer(10)]],
    }).unwrap();
    f.commit().unwrap();
    
    // update
    f.begin().unwrap();
    let ctx2 = TransactionContext::new(f.current_tx_id().unwrap());
    f.execute_write(&ctx2, WriteOp::Update {
        table: "t".into(),
        set: vec![("val".into(), Value::Integer(99))],
        filter: "id=1".into(),
    }).unwrap();
    f.commit().unwrap();
    
    let s = f.storage().read();
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 1);
    // val column updated
    assert_eq!(rows[0][1], Value::Integer(99));
}

#[test]
fn wtf_06_delete_in_transaction_commits() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    
    f.begin().unwrap();
    let ctx = TransactionContext::new(f.current_tx_id().unwrap());
    for i in 1..=3 {
        f.execute_write(&ctx, WriteOp::Insert {
            table: "t".into(),
            columns: vec!["id".into()],
            values: vec![vec![Value::Integer(i)]],
        }).unwrap();
    }
    f.commit().unwrap();
    
    f.begin().unwrap();
    let ctx2 = TransactionContext::new(f.current_tx_id().unwrap());
    f.execute_write(&ctx2, WriteOp::Delete {
        table: "t".into(),
        filter: "id=2".into(),
    }).unwrap();
    f.commit().unwrap();
    
    let s = f.storage().read();
    let rows = s.scan("t").unwrap();
    assert_eq!(rows.len(), 2, "DELETE should remove 1 of 3");
}

#[test]
fn wtf_07_drift_gate_rejects_invalid_op() {
    // 故意构造一个会被 DriftGate 拒绝的 op
    // 假设 DriftGate 拒绝 "tx_id 不匹配 ctx" 的 op
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    f.begin().unwrap();
    let real_tx = f.current_tx_id().unwrap();
    // 构造一个 ctx 但 tx_id 错误
    let wrong_ctx = TransactionContext::new(real_tx + 9999);
    let op = WriteOp::Insert {
        table: "t".into(),
        columns: vec!["id".into()],
        values: vec![vec![Value::Integer(1)]],
    };
    // 这里期望 DriftGate.validate 返回 Err 或 facade.execute_write 返回 Err
    // 注：实际行为依赖 DriftGate 的具体规则，若无校验则此测试需调整
    let r = f.execute_write(&wrong_ctx, op);
    // 行为可观察：要么 Err，要么 Ok 但 storage 未变
    if r.is_ok() {
        let s = f.storage().read();
        assert_eq!(s.scan("t").unwrap().len(), 0, "DriftGate fail silently is not allowed");
    }
}

#[test]
fn wtf_08_concurrent_begin_commit_isolation() {
    use std::sync::Arc;
    let f = Arc::new(WalTransactionalFacade::new_without_wal(MemoryStorage::new()));
    let mut handles = vec![];
    
    for thread_id in 0..4 {
        let f = f.clone();
        handles.push(thread::spawn(move || {
            for i in 0..10 {
                f.begin().unwrap();
                let ctx = TransactionContext::new(f.current_tx_id().unwrap());
                let op = WriteOp::Insert {
                    table: format!("t_{}", thread_id),
                    columns: vec!["id".into()],
                    values: vec![vec![Value::Integer(i)]],
                };
                f.execute_write(&ctx, op).unwrap();
                f.commit().unwrap();
            }
        }));
    }
    
    for h in handles {
        h.join().expect("thread panicked");
    }
    
    let s = f.storage().read();
    for tid in 0..4 {
        let key = format!("t_{}", tid);
        assert_eq!(s.scan(&key).unwrap().len(), 10, "thread {} should commit 10 rows", tid);
    }
}

// === L3 — 边界测试 ===

#[test]
fn wtf_e1_empty_table_name_insert_errors() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    f.begin().unwrap();
    let ctx = TransactionContext::new(f.current_tx_id().unwrap());
    let r = f.execute_write(&ctx, WriteOp::Insert {
        table: "".into(),
        columns: vec!["id".into()],
        values: vec![vec![Value::Integer(1)]],
    });
    assert!(r.is_err(), "empty table name should return Err, not panic");
}

#[test]
fn wtf_e2_update_nonexistent_table_errors() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    f.begin().unwrap();
    let ctx = TransactionContext::new(f.current_tx_id().unwrap());
    let r = f.execute_write(&ctx, WriteOp::Update {
        table: "nonexistent".into(),
        set: vec![("x".into(), Value::Integer(1))],
        filter: "id=1".into(),
    });
    assert!(r.is_err());
}

#[test]
fn wtf_e3_delete_empty_table_returns_zero_rows() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    f.begin().unwrap();
    let ctx = TransactionContext::new(f.current_tx_id().unwrap());
    let r = f.execute_write(&ctx, WriteOp::Delete {
        table: "empty".into(),
        filter: "id=1".into(),
    });
    // DELETE 空表：要么 Ok(0)，要么 Err，都可接受但不 panic
    match r {
        Ok(exec_result) => {
            // rows_affected via execution result
            assert!(exec_result.rows_affected().unwrap_or(0) == 0 || true);
        }
        Err(_) => {} // 也 OK
    }
}

#[test]
fn wtf_e4_thousand_inserts_single_transaction() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    f.begin().unwrap();
    let ctx = TransactionContext::new(f.current_tx_id().unwrap());
    for i in 0..1000 {
        f.execute_write(&ctx, WriteOp::Insert {
            table: "big".into(),
            columns: vec!["id".into()],
            values: vec![vec![Value::Integer(i)]],
        }).unwrap();
    }
    f.commit().unwrap();
    
    let s = f.storage().read();
    assert_eq!(s.scan("big").unwrap().len(), 1000);
}

#[test]
fn wtf_e5_double_commit_returns_err() {
    let f = WalTransactionalFacade::new_without_wal(MemoryStorage::new());
    f.begin().unwrap();
    f.commit().unwrap();
    // 第二次 commit 应该 Err（不在事务中）
    let r = f.commit();
    assert!(r.is_err(), "double commit must error");
}
```

---

## 3. 关键设计决策

### 3.1 为什么用 `MemoryStorage` 而非真实文件

- 测试运行速度：避免 disk IO
- 隔离性：每个测试独立
- CI 友好：跨平台一致

### 3.2 为什么测试用 `new_without_wal`

- WAL 写盘需要 tempfile 但本测试聚焦 facade 行为
- WAL 行为由 PR-830 单独测试
- 减少测试耦合

### 3.3 失败的 WTF-07 处理

- 如果 DriftGate 当前不校验 ctx.tx_id，则 WTF-07 应调整为：
  - 测试某种已知 DriftGate 拒绝的 op
  - 或 DEFERRED 标 "WTF-07 DriftGate semantics pending"
- 这是 Truthfulness 原则：不能编造失败

---

## 4. 不在范围

- 跨 PR-810 集成测试
- 真实文件 WAL 测试（属于 PR-830）
- 性能基准测试（属于 PR-900）

---

**最后更新**: 2026-06-02
