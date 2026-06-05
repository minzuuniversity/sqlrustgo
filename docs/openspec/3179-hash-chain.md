# openspec/3179 - P2-3 Immutable Audit Chain (Hash Chain)

> **Issue**: #3179
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 5 (W9-10)
> **工作量**: 20h (按 V390_DEVELOPMENT_PLAN)
> **状态**: 借力 crates/gmp/src/audit.rs (AuditLog.checksum 已有 SHA-256), 本次做 chain wiring + verify + G10 gate

## 一、问题分析

### 1.1 现状审计 (2026-06-05)

仓库已有:
- `crates/gmp/src/audit.rs` (694 lines):
  - `AuditLog.checksum: String` (SHA-256 of single record)
  - `verify_checksum(&self) -> bool` (single-record verify)
  - `create_audit_log_table`, `record_audit_log`, `query_audit_logs`
- `crates/gmp/src/audit.rs` 没有 `prev_hash` 字段 (单 record checksum, 不是 chain)

### 1.2 #3179 Hash Chain 需求

```
Record 1 (hash_1 = SHA256(0x00..00 || data_1))
   ↓
Record 2 (hash_2 = SHA256(hash_1 || data_2))
   ↓
Record 3 (hash_3 = SHA256(hash_2 || data_3))
```

验证:
```
verify(from, to):
  for i in records[from..to]:
    expected[i] = SHA256(observed[i-1] || data[i])
    if expected[i] != observed[i]: return BROKEN_AT(i)
  return OK
```

### 1.3 P2-3 任务真正需要补的 (按治理最小修改)

**A. Chain Builder** (新):
- HashChainBuilder struct
- link(prev_hash, record_data) -> new_hash
- verify_chain(records) -> Result<(), BrokenAt(usize)>

**B. Tamper Detection** (新):
- 1-byte flip → verify 拒绝
- record 删除 → verify 拒绝
- prev_hash 修改 → verify 拒绝

**C. G10 Gate** (新):
- 7 项检查

## 二、实施方案

### 2.1 范围限定

按治理 §2.1 最小修改 + 复用 crates/gmp/src/audit.rs:

**本次 PR 范围 (4 大块)**:

1. **新文件**: `tests/hash_chain_harness.rs` (shared helper, 5 self-tests)
2. **新文件**: `tests/hash_chain_test.rs` (20+ tests, 4 类别)
3. **新文件**: `scripts/gate/check_p23_hash_chain.sh` (G10 gate)
4. **新文件**: `docs/openspec/3179-hash-chain.md` (本文件)

**延后 (推 v3.10+)**:
- 真实 audit log 集成 (AuditLog.prev_hash 字段)
- 性能基准 (10K records verify < 1s)
- 链断裂时自动 recovery
- 增量 verify (只 verify 新 records)

### 2.2 Hash Chain Harness 设计

```rust
// tests/hash_chain_harness.rs (shared)
pub struct ChainRecord {
    pub data: String,           // serialized AuditLog row
    pub hash: String,           // SHA-256 hex of (prev_hash || data)
    pub prev_hash: String,      // previous record's hash (创世块 = "0"*64)
}

pub struct ChainBuilder;

impl ChainBuilder {
    pub fn new() -> Self;
    pub fn genesis_hash() -> String; // "0"*64
    pub fn link(prev_hash: &str, data: &str) -> String; // SHA-256 hex
    pub fn build(records: Vec<String>) -> Vec<ChainRecord>;
    pub fn verify(records: &[ChainRecord]) -> Result<usize, BrokenAt>;
}
```

### 2.3 20+ Tests (4 类别)

| 类别 | Count | 示例 |
|------|-------|------|
| 1. build (5) | 创世块, 1 record, 10 records, 100 records, 1000 records |
| 2. verify_ok (5) | empty, 1 record, 10, 100, mixed sizes |
| 3. tamper (7) | 1 byte flip, prev_hash 改, record 删除, record 插入, record 替换, 大小改, hash 字符替换 |
| 4. range (3) | verify [0,10], verify [5,15], full range |
| **TOTAL** | **20** | |

### 2.4 G10 Gate (7 checks)

1. `tests/hash_chain_harness.rs` exists
2. `tests/hash_chain_test.rs` exists + registered
3. 4 类别全覆盖
4. cargo check pass
5. ≥20 tests pass
6. crates/gmp/src/audit.rs 仍 PASS (no regression)
7. SHA-256 字符集验证 (only hex digits)

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| SHA-256 真实实现 vs mock 性能差 | 测试慢 | harness 用 simplified hash, 真实场景留给 v3.10+ |
| prev_hash 字段未加到 AuditLog | 集成延迟 | 标注 v3.10+ 跟踪 |
| 篡改检测 false negative | 安全性 | 双重 verify (prev_hash + data hash) |

## 四、验收标准 (G10 门禁)

```
✅ hash_chain_test: ≥20 tests PASS
✅ 4 类别全覆盖
✅ G10 gate: 7/7 PASS
✅ 1555 L1 tests 不回归
✅ TPC-H 22/22 (G1 维持)
```

## 五、Subsumed Issues

- #3179 本身 (本任务)
- 与 P2-1 (#3177 Audit Log) 互补 (chain 集成)
- 与 P2-2 (#3178 Time Travel) 互补 (timestamp 排序 + 不可篡改)

## 六、回滚计划

如 hash_chain_test 编译失败:
1. 删除 `tests/hash_chain*.rs`
2. G10 gate 标记 DEFER
3. crates/gmp/src/audit.rs 保留

## 七、依赖

**上游**: P2-1 Audit Log (#3177 closed)
**下游**: v3.10+ 真实 chain 集成 (AuditLog.prev_hash 字段)

## 八、参考资料

- Issue #3179
- V390_DEVELOPMENT_PLAN.md §P2-3
- V390_TEST_PLAN.md §G10
- crates/gmp/src/audit.rs (694 lines, AuditLog.checksum 已有)
- P2-1 #3177 audit_log_harness (设计模型)
- P2-2 #3178 time_travel_harness (设计模型)
