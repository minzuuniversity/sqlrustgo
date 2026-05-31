# RECOVERY Test Migration Plan
**Author**: HermesC  
**Date**: 2026-05-31  
**Branch**: develop/v3.8.0 (commit b579a903)  
**Status**: DRAFT — for review before opencode implementation

---

## 1. 背景

`tests/wal_tx_contract_test.rs` 包含 23 个 P0 测试（TX-001~006、WAL-001~005、RECOVERY-001~008）。

当前运行结果：
- **15 PASS** (TX + WAL，依赖 L1 MemoryStorage 语义)
- **7 FAIL** (RECOVERY-001~008，依赖 crash recovery 但实际运行在 L1 MemoryStorage)

**根本原因**: RECOVERY 测试使用 `create_engine()` 返回 `ExecutionEngine::with_memory()`（L1），而 crash recovery 是 L3 语义。MemoryStorage drop 后数据永久丢失，无法模拟"重启后恢复"。

---

## 2. 测试分层策略

### 2.1 L1 — MemoryStorage（保持不变）

| 测试 | 理由 |
|------|------|
| TX-001~006 (8个) | 验证 Transaction boundary 语义（begin/commit/rollback），不需要持久化 |
| WAL-001~005 (5个) | 验证 WAL 写入逻辑，`is_wal_enabled()` 返回 true 即可，不需要真实 WAL replay |

**这些测试在 L1 下正确且充分**。保持 `create_engine() -> ExecutionEngine<MemoryStorage>` 不变。

### 2.2 L3 — WalStorage（必须迁移）

| 测试 | 为什么必须 L3 |
|------|--------------|
| RECOVERY-001 `test_begin_then_crash_rolls_back` | 验证"begin 后 crash → rollback"。需要 engine drop 后重建能观察到回滚状态 |
| RECOVERY-002 `test_insert_then_crash_rolls_back` | 验证"insert 后 crash → 数据不出现"。需要持久化 WAL replay |
| RECOVERY-003 `test_insert_then_crash_rolls_back` | 同上（duplicate，可能是 002 重命名残留）|
| RECOVERY-004 `test_commit_flush_crash_replays` | 验证"commit+flush 后 crash → WAL replay 恢复数据"。L3 唯一正确场景 |
| RECOVERY-005~008 `test_partial_*_write_recovery` (4个) | 验证"部分写入 crash → WAL replay 恢复或回滚"。需要 WAL replay |

**结论**: RECOVERY-001~008 共 7 个测试必须运行在 L3。

---

## 3. WAL 路径生命周期管理

### 3.1 三层 factory 方法（已实现）

```rust
// L1: MemoryStorage（内存，非持久化）
ExecutionEngine::with_memory() -> ExecutionEngine<MemoryStorage>

// L2: WAL stub（无持久化，有 flush hook 和 replay mock）
ExecutionEngine::with_wal_stub() -> ExecutionEngine<WalStorage<MemoryStorage>>

// L3: WalStorage + WAL 文件（持久化，真实 crash recovery）
ExecutionEngine::with_wal(PathBuf) -> ExecutionEngine<WalStorage<MemoryStorage>>
```

### 3.2 WAL 文件生命周期

```
with_wal(path)
    └── 创建 WalStorage<MemoryStorage>
        ├── 启用 WAL 文件 (path.join("wal"))
        ├── flush() → 写入 WAL
        └── drop 时根据配置决定是否保留 WAL
```

**关键设计决策**:
- L3 的 `tempfile::tempdir()` 确保测试结束后自动清理
- WAL 文件命名: `wal-<timestamp>.log`
- `recover()` 方法在 engine 创建时自动调用，读取 WAL 文件并 replay

### 3.3 `tempfile` 依赖

当前 `wal_tx_contract_test.rs` 使用 `use tempfile;`。如需使用 L3，需要：

```rust
use tempfile::TempDir;

fn create_engine_l3() -> ExecutionEngine<WalStorage<MemoryStorage>> {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    ExecutionEngine::with_wal(temp_dir.path().into())
        .expect("failed to create WAL engine")
}
```

**注意**: `TempDir` 需要满足 `'static` bound，当前 `create_engine()` 返回类型可能需要调整。

---

## 4. Restart 场景构造

### 4.1 当前（错误的）场景构造

```rust
// 当前 RECOVERY 测试的 restart 逻辑
let engine = create_engine();     // 创建 L1 engine
engine.execute("BEGIN");          // 写入数据
engine.execute("INSERT ...");     // ...
drop(engine);                     // L1 drop → 数据永久丢失
let engine2 = create_engine();     // 重建，但数据已空
let result = engine2.query(...);  // SELECT 0 rows → assert! 失败
```

### 4.2 正确的 L3 场景构造

```rust
// 正确的 L3 restart 场景
let temp_dir = TempDir::new().expect("failed to create temp dir");
let engine = ExecutionEngine::with_wal(temp_dir.path().into()).unwrap();
engine.execute("BEGIN");
engine.execute("INSERT ...");
drop(engine);                    // L3: 数据已刷入 WAL 文件
let engine2 = ExecutionEngine::with_wal(temp_dir.path().into()).unwrap();
// engine2 初始化时自动 recover()，WAL replay 恢复数据
let result = engine2.query("SELECT ...");  // 能查到数据
```

**关键**: `drop(engine)` 后 WAL 文件保留，`engine2` 重建时 `recover()` 自动读取 WAL 并 replay。

### 4.3 场景分类

| 场景 | 构造方式 | 验证点 |
|------|----------|--------|
| BEGIN then crash rollback | L3 engine → BEGIN → INSERT → drop → L3 重建 → SELECT 0 rows | 数据未持久化 |
| COMMIT+flush then crash recovery | L3 engine → BEGIN → INSERT → COMMIT → flush → drop → L3 重建 → SELECT 有数据 | WAL replay |
| Partial write recovery | L3 engine → BEGIN → INSERT partial → drop → L3 重建 | WAL 恢复完整或不恢复 |

---

## 5. 修改后预期结果

| 当前状态 | 修改后预期 |
|----------|------------|
| 15 PASS / 7 FAIL | **22 PASS / 0 FAIL**（RECOVERY-001~007 迁移到 L3）|
| RECOVERY-003 疑似 002 重名 | 确认后合并或修正 |

**注意**: RECOVERY-003 和 RECOVERY-002 的测试内容完全相同（需确认测试文件实际内容）。

---

## 6. 实施计划

### Phase 1: 确认测试内容（opencode 之前）
- [ ] 读取 RECOVERY-001~008 完整测试体，确认每个测试的断言逻辑
- [ ] 识别 RECOVERY-003 是否为重复测试
- [ ] 确认 `tempfile::TempDir` 的 `'static` bound 是否影响返回类型

### Phase 2: 修改 `create_engine()` 工厂函数（opencode 实施）
- [ ] 为 L1 和 L3 分别提供 factory 方法：`create_engine_l1()` / `create_engine_l3()`
- [ ] 调整 RECOVERY 测试使用 `create_engine_l3()`
- [ ] 确保 L1 测试不受影响（TX-001~006, WAL-001~005 保持 L1）

### Phase 3: 验证
- [ ] `cargo test --test wal_tx_contract_test` → 22/22 PASS
- [ ] `cargo clippy -p sqlrustgo-storage` → 0 errors
- [ ] Beta Gate B2 验证通过

---

## 7. 风险与缓解

| 风险 | 缓解 |
|------|------|
| `TempDir` 的 `'static` bound 导致类型不兼容 | 将 `create_engine_l3()` 改为接受 `Into<PathBuf>`，测试内管理 TempDir 生命周期 |
| L3 WAL replay 有未覆盖边界 case | 添加更多 WAL replay 边界测试（空 WAL 文件、WAL 文件损坏、WAL 无效 magic number） |
| RECOVERY-003 确实是重复 | Phase 1 确认后删除或合并 |

---

## 8. 不需要修改的测试（保持 L1）

| 测试 | 保持 L1 的理由 |
|------|----------------|
| `test_wal_enabled_check` | 只验证 `is_wal_enabled()` 返回 true，不涉及持久化 |
| `test_wal_write_batch` | 验证 WAL 写入格式，不需要 crash recovery |
| `test_wal_flush_on_commit` | 验证 flush hook，不涉及持久化 |
| `test_wal_replay_basic` | 使用 mock，不依赖真实 WAL 文件 |
| TX-001~006 | Transaction 语义测试，不涉及 crash recovery |

---

## 9. 验收标准

修改完成后：
1. `cargo test --test wal_tx_contract_test` 输出 **22 PASS, 0 FAIL**
2. RECOVERY 测试在 L3 模式下运行，使用真实 `TempDir` 和 `with_wal(PathBuf)`
3. L1 测试（TX/WAL）保持不变，不受影响
4. `cargo clippy -p sqlrustgo-storage` → 0 errors