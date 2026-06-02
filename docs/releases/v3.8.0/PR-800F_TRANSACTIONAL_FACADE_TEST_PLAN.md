# PR-800F TEST PLAN — TransactionalFacade

> **PR**: PR-800F
> **Branch**: test/v380-test-coverage-a1-a4
> **Source SPEC**: PR-800F_TRANSACTIONAL_FACADE_SPEC.md
> **Created**: 2026-06-02
> **Status**: ACTIVE

---

## 1. 测试策略

### 1.1 三层验证

| Layer | 验证目标 | 工具 | 门禁 |
|-------|----------|------|------|
| **L1 单元** | Trait 各方法正确性 | Rust unit test (in src) | 10+ tests PASS |
| **L2 集成** | WalTransactionalFacade 端到端 | Rust integration test (in tests/) | 8+ tests PASS |
| **L3 边界** | 异常路径与压测 | Rust integration test | 5+ tests PASS |

### 1.2 不测什么

- 不测 `execute_read` 的 SQL 解析正确性（属于 parser 测试）
- 不测 storage 的物理 IO（属于 storage 测试）
- 不测事务并发隔离级别（属于 transaction crate 测试）

---

## 2. 测试矩阵

| 用例 ID | Layer | 测什么 | 期望 | 门禁 |
|---------|-------|--------|------|------|
| UTF-01 | L1 | 初始不在事务中 | is_in_transaction=false | PASS |
| UTF-02 | L1 | begin 后在事务中 | is_in_transaction=true | PASS |
| UTF-03 | L1 | current_tx_id 返回 | Some(n) | PASS |
| UTF-04 | L1 | tx_id 单调递增 | tx_id_n+1 > tx_id_n | PASS |
| UTF-05 | L1 | commit 后不在事务中 | false | PASS |
| UTF-06 | L1 | rollback 后不在事务中 | false | PASS |
| UTF-07 | L1 | 未 begin commit | Err | PASS |
| UTF-08 | L1 | 未 begin rollback | Ok(()) | PASS |
| UTF-09 | L1 | execute_write 不 panic | not panic | PASS |
| UTF-10 | L1 | validate_operation Ok | Ok(()) | PASS |
| WTF-01 | L2 | INSERT + commit 持久化 | row exists | PASS |
| WTF-02 | L2 | INSERT + rollback 不持久化 | row not exists | PASS |
| WTF-03 | L2 | 多 INSERT 单事务 | rows.len == 2 | PASS |
| WTF-04 | L2 | 多次 begin/commit tx_id 递增 | tx_id 严格递增 | PASS |
| WTF-05 | L2 | UPDATE + commit | value updated | PASS |
| WTF-06 | L2 | DELETE + commit | row removed | PASS |
| WTF-07 | L2 | DriftGate 拒绝 | Err returned | PASS |
| WTF-08 | L2 | 并发 begin/commit (4×10) | all succeed | PASS |
| WTF-E1 | L3 | 空表名 INSERT | Err (no panic) | PASS |
| WTF-E2 | L3 | UPDATE 不存在的表 | Err | PASS |
| WTF-E3 | L3 | DELETE 空表 | Ok(0) | PASS |
| WTF-E4 | L3 | 1000 INSERT 单事务 | rows.len == 1000 | PASS |
| WTF-E5 | L3 | 嵌套 commit | Err | PASS |

**总计**: 23 tests, 全部 MUST PASS

---

## 3. 测试执行命令

```bash
# 单元测试 (in source)
cargo test -p sqlrustgo-executor --lib transactional_facade

# 集成测试 (新增文件)
cargo test -p sqlrustgo-executor --test wal_transactional_facade_test

# 全跑
cargo test -p sqlrustgo-executor --all-features

# 门禁
cargo clippy -p sqlrustgo-executor --all-features -- -D warnings
cargo fmt -p sqlrustgo-executor --check
```

---

## 4. 资源与时间

| 资源 | 限制 |
|------|------|
| 内存 | 8GB (CI 默认) |
| 单测试时长 | ≤ 30s |
| 集成测试总时长 | ≤ 5min |
| 临时文件 | `tempfile` crate 自动清理 |

---

## 5. 风险与缓解

| 风险 | 缓解 |
|------|------|
| DriftGate 内部实现细节耦合 | 使用真实 facade（不 mock） |
| WalStorage 需要临时目录 | 用 `tempfile::tempdir()` 自动清理 |
| TransactionManager 内部状态 | 通过 trait API 观察，不访问私有字段 |

---

**最后更新**: 2026-06-02
