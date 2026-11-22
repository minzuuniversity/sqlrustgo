# PR-5785 TEST_DESIGN — clippy 全仓清债（LINT-2）

> Issue: #5362 · PR: #5785 · 分支: `opencode-macmini/clippy-clean`
> 冻结基线: `1cb5d8889e` · 阶段: ALPHA
> 配套: `PR-5785_SPEC.md` / `PR-5785_TEST_PLAN.md` / `PR-5785_TEST_REVIEW.md` / `PR-5785_ACCEPTANCE.md`

---

## 1. 判据承重：先红后绿

计数型/编译型判据最大的风险是「永远绿」。本 PR 的处置是**保留一次真红**：

- run `20261114-clippy2-001`（frozen `e590a50ed1`）
  - 第 10 臂 `cargo test -p sqlrustgo-storage --lib` ⇒ **rc=101**
  - 失败：`vtu_guard::tests::test_vtu_guard_tx_state_via_set_current_tx_id`
    `assertion failed: guarded.in_transaction()`（vtu_guard.rs:536）
  - 根因：`VtuGuard` 未透传事务状态，落 trait 默认 `false`
  - 该 run 原样入仓（manifest + 日志），**不作通过证据**
- run `20261114-clippy2-002`（frozen `1cb5d8889e`）
  - 同一条命令 ⇒ `[ok]`；16 臂全部 rc=0

⇒ 判据能红、能绿；绿不是「从来没红过」的假象。

## 2. 冷编译设计

```text
臂 1: cargo clean                                  # 删除全部 target
臂 2: cargo clippy --locked --all-features \
        --all-targets -- -D warnings               # 全量重编译
```

为什么必须这样：cargo 对新鲜单元会重放上一次的诊断（或干脆不重放）。若不先
`clean`，`-D warnings` 的 rc=0 只证明「这次没编译到有 warning 的单元」。
第 1 臂把这条前提写进证据本身，而不是写在口述里。

## 3. 两处真修的判别力

### 3.1 `decode_lenenc_int` 0xfd

| 输入 | 旧实现 | 正确值 | 断言 |
|---|---|---|---|
| `[0xfd, 0x01, 0x00, 0x00]` | 256 | 1 | `decode_lenenc_int_3byte_via_0xfd` |
| `[0xfd, 0x00, 0x00, 0x01]` | 65536×256 | 65536 | `decode_lenenc_int_3byte_uses_all_three_bytes` |

修复前该测试是 **死函数**（漏 `#[test]`）→ 补属性后成为真测试 → 断言正确值
**必然红** → 改实现后转绿。这不是「改测试迁就实现」，而是「让从未跑过的测试
暴露实现错误」。

### 3.2 `VtuGuard` 事务状态

```rust
assert!(!guarded.in_transaction());
guarded.inner_mut().set_current_tx_id(7);
assert!(guarded.in_transaction());   // 旧实现：false ⇒ 001 真红
assert_eq!(guarded.current_tx_id(), 7);
guarded.inner_mut().set_current_tx_id(0);
assert!(!guarded.in_transaction());
```

修复：`in_transaction` / `current_tx_id` / `set_current_tx_id` /
`set_current_tx_id_shared` / `is_transaction_active` 以及
`begin/commit/rollback`（含 `_for` 与 `*_lockfree(_for)`）一律透传 inner。
`cargo test -p sqlrustgo-storage --lib vtu_guard` ⇒ **21 passed**（含生命周期、
lockfree、嵌套 Guard 三组等价性用例）。

外部评审（chatGPT-web）要求“证明等价而非只跑单测”后补的用例：
- `test_vtu_guard_transaction_lifecycle_matches_inner`
- `test_vtu_guard_lockfree_tx_control_delegates`（用 WalStorage，MemoryStorage
  本身不支持 lockfree）
- `test_nested_vtu_guard_propagates_tx_state`
- `no_namespace_db_refusal_5266::vtu_guard_forwards_foreign_db_to_inner`
  （需 `--features bin_storage_default`，否则 0 测试；run 3b-001 已实证 5 passed）

### 3.3 `decode_lenenc_int` 协议边界

| 输入 | 期望 |
|---|---|
| `[0xfd, 0x01, 0, 0]` | 1 |
| `[0xfd, 0, 0, 1]` | 65536 |
| `[0xfd, 0xff, 0xff, 0xff]` | 16_777_215 |
| `[0xfd, 0x01, 0]` | `None`，只消费 marker |

## 4. 豁免审计

每一处 `#[allow]` 都必须是**可解释**的，不是「让输出变干净」：

| 位置 | 理由 | 为什么不是掩盖 |
|---|---|---|
| `v400_acl_audit.rs` 文件级 `const_is_empty` | 占位契约 `assert!(!contract.is_empty())` 的语义是防手滑清空 | 12 处同族；真实实现落地后断言会换成产品调用；SEC-1 仍是属主待裁项 |
| `local_executor_dml.rs` 测试 mod `allow(deprecated)` | 该 mod 是 deprecated 类型的**过渡期行为守卫** | 删 allow 等于删行为守卫，不是清债 |
| `tpch_*_importer.rs` 的 `allow(dead_code)` | 配置/统计输出面预留（args 已解析、打印面待接） | 删字段会删掉已接的配置面；注释写明预留身份 |
| `not_run_log.rs` 文件头 `duplicate_mod` | `#[path]` 多路径加载的已知形态 | 单看文件无法判断重复；单 target dead_code 另用精确 allow |

## 5. 回归边界

- 本分支触碰的 crate/test target 全部单跑（见 TEST_PLAN 层 3）。
- 未跑：全 workspace 全量测试（`cargo test --workspace`），因此不声明「全仓测试绿」。
- `physical_backup` 的负载 flaky 是本 PR 之前就存在的存量，本 PR 不认领也不掩盖。
