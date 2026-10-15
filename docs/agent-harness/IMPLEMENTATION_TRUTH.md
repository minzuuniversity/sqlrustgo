# Implementation Truth — SQLRustGo 实现真实性

> **基线**: HEAD `583050bc63` (fix/4597)
> **关键**: SQLRustGo 是数据库内核项目, 实现真实性 = 正确性 + 持久性 + 事务一致性

## 实现状态 5 级 (强制使用)

| 级别 | 含义 | 验证手段 |
|---|---|---|
| **Implemented** | 源代码中存在实现 | `rg` / rust-analyzer |
| **Integrated** | 真实生产调用链使用该实现 | grep production path |
| **Tested** | 有实际执行的测试 | `cargo test --test X` 输出 |
| **Verified** | 通过独立行为或差分验证 | oracle parity, fault injection |
| **Release-ready** | 满足对应发布门禁 | Hard Gates exit 0 + STAGE.yaml |

**Agent 不得**声称已达 N 级除非有证据。

## 严禁行为 (AFP Type A/B/C/D)

| Type | 行为 | 例子 |
|---|---|---|
| **A** | 空函数 / 默认返回值 / 硬编码结果冒充实现 | `fn foo() -> bool { true }` |
| **B** | 声称模块已集成, 但生产路径绕过 | FileStorage "用了 BufferPool", 但实际 HashMap path |
| **C** | 为通过测试而删除断言 / 跳过测试 / 修改参考结果 | `#[ignore]` 一个失败的集成测试 |
| **D** | 未执行测试却报告测试成功 | "cargo test pass" 但 cargo test --no-run |

## 数据库内核专属红线

1. **cargo test --no-run 不算测试** — 必须实际执行 + exit 0
2. **SKIP / TIMEOUT / XFAIL 不算 PASS** — ADR-008
3. **单 Mock 不算验证** — 必须有真实 client wire + 持久化恢复
4. **单测绿不等于生产绿** — 必须走 integration + gate + soak
5. **Agent 自我签发验收 = 违规** — 必须多 AI 协调 (ADR-014) 或人复核

## 当前 known 实现边界 (重要)

### BufferPool / Page / BPlusTree

`src/lib.rs:49-63` 显式标注:

```rust
/// ⚠️ **Test-only capability. Not used by production `FileStorage`.**
pub use sqlrustgo_storage::BufferPool;
pub use sqlrustgo_storage::BPlusTree;
pub use sqlrustgo_storage::Page;
```

- 实际生产 `FileStorage` 是 in-mem `HashMap` + JSON `insert_buffer`, **不是** paged
- #5304 PR2 暴露 counter 表面 (PR-A.5 接 EXPLAIN pages_visited) 是过渡步骤
- 完整 BufferPool 化生产路径仍 OPEN
- See `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md` §3.1

**任何** Agent 描述存储特性时, 必须区分 "production" vs "test-only"。

### WalEntry 真实写路径

`#5192` 系列 commit 揭示:

- `WalStorage::log_insert` 是死代码 (从未被调用)
- 真正写 WAL 的是 `FileStorage::wal_row_entry`
- 任何修 WalEntry 字段必须同步更新所有 fixture (历史症状: `tests/integration/migration/wal_integration_test.rs:16` 缺 `database_name`)

### 实际版本

- workspace.package.version: `3.12.0-fix-zombie` (Cargo.toml, 有意保留)
- 当前 dev: v4.1.0 DRAFT (`docs/releases/v4.1.0/STAGE.yaml`)
- v4.0.0 GA 已撤销 (`docs/releases/v4.0.0/GA_RELEASE_TIMELINE.md`)
- VERSION 文件: `v3.11.0` (last GA tag)

## 验证流程 (Agent 主张前的 hard checklist)

```
□ 1. Implemented: 源码存在? (grep / gitnexus_context)
□ 3. Integrated: 生产路径调用? (gitnexus_impact upstream)
□ 4. Tested: 测试存在 + 实际跑过? (cargo test --test X)
□ 5. Verified: 独立验证? (oracle / fault injection)
□ 6. Release-ready: Hard Gates? (scripts/gate/check_*.sh)
□ 7. 含 source_agent + source_run + timestamp + evidence_hash
```

如果任何一项缺 — 必须明确说"未达 N 级", 不能说"完成"。