# v3.11.0 G3 覆盖率整改计划

> **生成日期**: 2026-08-09
> **用途**: 解释 G3 coverage 口径漂移，并为 v3.12.0 收敛提供整改路径。
> **注意**: 本文件是计划和分析，不是 coverage gate PASS 证据。

## 1. 根因

当前 G3 争议的核心不是单个 crate 是否真的有测试，而是覆盖率命令口径不一致。`cargo llvm-cov --lib` 只统计 `src/*.rs` 中 inline `#[cfg(test)]` 覆盖的路径，不统计 `crates/<name>/tests/*.rs` 的 integration tests。因此，一些已经有大量 integration tests 的 crate，在 `--lib` 口径下仍会表现为低覆盖率。

| crate | 主要问题 | 整改方向 |
|---|---|---|
| `sqlrustgo-executor` | 代码体量大，部分 orphan/stub 模块拉低分母 | 删除无调用孤儿模块或补 inline tests，补 `WalTransactionalFacade` 行为测试 |
| `sqlrustgo-admin` | integration tests 多，但 `--lib` 不计入 | 将关键覆盖逻辑迁入 inline test，或统一采用包含 integration 的 gate 口径 |
| `sqlrustgo-mysql-server` | wire dispatch 路径长，inline 覆盖不足 | 补 `testing`、COM_QUERY、resource monitor、WAL sync mode 等 inline/e2e 测试 |
| `sqlrustgo-mysql-client` | inline tests 很少 | 补认证、连接、prepared statement、错误恢复测试 |

## 2. 整改原则

- 先统一测量方法，再谈 PASS/FAIL。
- 删除代码只能删除确认无调用、无生产入口、无文档承诺的 orphan 模块。
- 不能为了提高覆盖率而把真实功能路径改成伪测试路径。
- coverage 报告必须保留命令输出和 evidence hash。

## 3. v3.12 承接要求

v3.12.0 必须把 G3 coverage 作为 P0 硬化项：

1. 定义唯一 canonical coverage command。
2. 记录每个 crate 是否包含 integration/e2e tests。
3. 对 parser、mysql-server、mysql-client、gmp、spill、vector、cli、sql-corpus 建立专项提升计划。
4. 在 gate 报告中明确“发布裁决口径”和“严格 per-crate 口径”的差异。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# v3.11.0 G3 Coverage Remediation Plan

> **Generated**: 2026-08-09 (today = T0, ADR-008 exception expires 2026-09-01 → T+23 days)
> **Branch baseline**: `develop/v3.11.0` @ `d246f1f52c`
> **GA gate source**: `docs/releases/v3.11.0/GA_GATE_REPORT.md` (2026-08-08 measurements) +
> `docs/releases/v3.11.0/COVERAGE_FULL_2026-08-09.md` (broader sweep, drift)
> **Tool**: `cargo llvm-cov --lib --release` (per GA_GATE_REPORT + CURRENT_VERSION.md)

## 1. Root cause (read this first)

`cargo llvm-cov --lib` only instruments code reachable from inline `#[cfg(test)] mod tests`
inside `crates/<name>/src/*.rs`. **It does NOT measure `crates/<name>/tests/*.rs` integration
files.** That is why all 4 failing crates — which already have extensive integration test
files (27 in executor, 8 in admin, 6 in mysql-server, 1 in mysql-client) — still report low
coverage under `--lib`:

| Crate | LoC (lib) | Inline tests | Integration test files (ignored by --lib) |
|-------|-----------|--------------|---------------------------------------------|
| executor | ~13,200 | abundant | 27 in `crates/executor/tests/` |
| admin | ~1,300 | scattered | 8 in `crates/admin/tests/` |
| mysql-server | ~5,300 (lib.rs 5128 + load_data 179) | 103 in lib.rs, but only narrow | 6 in `crates/mysql-server/tests/` |
| mysql-client | ~900 | 4 (only) | 1 in `crates/mysql-client/tests/unit_tests.rs` |

**Methodology conflict**: `docs/releases/v3.11.0/coverage-baseline/README.md` documents ADR-001
G-04 as `cargo llvm-cov --tests` (i.e. include integration tests). The GA gate currently uses
`--lib`. Closing the conflict is itself a remediation lever: flipping to `--tests` per the
SSOT would lift admin, mysql-server, mysql-client substantially. Verify on 250 first.

## 2. Per-crate remediation

### Crate 1 — sqlrustgo-executor

- **Current**: 76.41% (lib), 77.83% (GA gate) — gap -3.59pp / -2.17pp
- **LoC**: 13,200 across 50+ modules
- **Path of least resistance**:

  1. **Delete 4 orphan modules** (zero callers, only inflate denominator):
     - `crates/executor/src/arc_storage_adapter.rs` (20 LoC, "Placeholder" comment)
     - `crates/executor/src/test_framework/{mod,harness,test_data}.rs` (343 LoC)
     - `crates/executor/src/pipeline_executor.rs` (144 LoC)
     - `crates/executor/src/sql_normalizer.rs` (144 LoC)
     - Each removal reduces denominator by `LoC / 13200` of a pp. ~641 LoC removed ≈ +5pp numerator, recovering the gap.

  2. **Add inline tests in 5 orphan modules** (cheap wrappers):
     - `src/execution/facade.rs`, `result.rs`, `engine.rs`, `transactional_facade.rs`
     - `src/vector_executor.rs`, `src/session_config.rs`, `src/join/mod.rs`

  3. **Fill `wal_transactional_facade.rs` stub**: replace single `facade_stub` test with real coverage of `WalTransactionalFacade<S>`.

- **Effort**: 4–6h. **Recommended order**: delete orphans first (lowest risk), then add inline tests.
- **Risk**: removing orphans requires confirming no caller exists outside `archive/` and
  `target/`. Cov5 confirmed zero callers in `crates/`/`src/`. Safe.

### Crate 2 — sqlrustgo-admin

- **Current**: 63.01% (lib), 65.08% (GA gate) — gap -16.99pp / -14.92pp
- **LoC**: ~1,311 across 8 modules
- **Path**:

  1. **Move existing integration test bodies into inline `#[cfg(test)] mod tests`** for the 3 orphan modules:
     - `src/mysqladmin.rs` → `MysqlAdmin::execute_status / execute_show_processlist / execute_kill / execute_shutdown / set_variable / execute_query` (291 LoC, ZERO inline tests).
     - `src/restore.rs` → `physical_restore`, `RestoreResult` (187 LoC, ZERO inline tests).
     - `src/verify.rs` → `verify_backup`, `verify_extracted` (129 LoC, ZERO inline tests).

  2. **Source for the moved tests**: copy from `crates/admin/tests/{admin_test.rs, mysqladmin_tests.rs, verify_tests.rs, admin_coverage_tests.rs, admin_pitr_tests.rs, admin_wire_client_tests.rs, wire_client_integration_tests.rs}`. Coverage tests already exist; the only gap is that they live in `tests/*.rs` and `--lib` ignores them.

- **Effort**: 6–8h (mechanical copy + verify each test still compiles standalone; some may need `start_ephemeral` mocks). **Recommended order**: mysqladmin.rs first (largest LoC), then verify.rs, then restore.rs.
- **Risk**: integration tests may import `start_ephemeral` from `sqlrustgo_mysql_server::testing`; that's fine inside `lib.rs` test module as long as the dep is declared in `crates/admin/Cargo.toml`. Verify before move.

### Crate 3 — sqlrustgo-mysql-server

- **Current**: 40.62% (lib), 42.91% (GA gate) — gap -39.38pp / -37.09pp
- **LoC**: 5,128 (lib.rs) + 179 (load_data.rs) + 744 (main.rs, binary)
- **Path**:

  1. **Add inline tests for `pub mod testing`** (`EphemeralConfig`, `ServerThreadPool`, `start_ephemeral`) — line 4548 onward. These are exercised by integration tests but `--lib` skips integration.

  2. **Add inline tests for wire dispatch** (lines 300–3700, currently 2,571 of 4,484 regions uncovered):
     - `run_server_v2`, `run_server_with_listener_and_shutdown`, `run_server`
     - `spawn_resource_monitor`, `read_executor_parallelism`, `parse_wal_sync_mode`
     - `handle_load_local_infile` happy + error paths
     - `send_result_set` deprecate_eof=true vs classic EOF branches
     - `do_command_loop` COM_QUERY / COM_STMT_PREPARE / COM_STMT_EXECUTE / COM_STMT_CLOSE / COM_QUIT / COM_PING / COM_INIT_DB

  3. **Move tests from `crates/mysql-server/tests/{mysql_server_helper_tests.rs, mysql_server_unit_tests.rs, prepared_stmt_params_test.rs}` into `#[cfg(test)] mod tests` blocks in lib.rs** (MySqlError all variants, StmtParam, parse_stmt_execute_params, replace_placeholders, ACTIVE_CONNECTIONS atomics).

- **Effort**: 12–16h (largest module). **Recommended order**: testing module first (smallest surface), then wire dispatch (largest surface), then move existing tests last.
- **Risk**: inline `mod tests` may pull in `tokio`/`async` deps that are already `dev-dependencies`; verify `crates/mysql-server/Cargo.toml` `[dev-dependencies]` includes everything the integration tests need.

### Crate 4 — sqlrustgo-mysql-client

- **Current**: 31.56% (lib), 31.42% (GA gate) — gap -48.44pp / -48.58pp
- **LoC**: 900 (single lib.rs)
- **Path**:

  1. **Move `crates/mysql-client/tests/unit_tests.rs` (~50 tests) into `#[cfg(test)] mod tests` in `src/lib.rs`** (the existing 4 tests stay, the new tests are appended). This is the single highest-leverage move: most of mysql-client's coverage already exists, it just lives in `tests/`.

  2. **Add inline tests for `MySqlConnection` lifecycle** using `std::net::TcpListener::bind("127.0.0.1:0")` and crafting raw wire-protocol packets. Cover: `connect` happy + error, `query`, `ping`, `close`, `is_connected`. ~10 tests.

  3. **Add inline tests for `parse_result_set`, `parse_stmt_execute_params`, `replace_placeholders`** — exercise the result-set parser with crafted `Cursor<&[u8]>` payloads (NULL=0xfb, length-encoded ints, EOF terminator). ~15 tests.

- **Effort**: 6–10h. **Recommended order**: move existing unit_tests.rs first (lowest risk, biggest win), then add inline lifecycle tests.
- **Risk**: `MySqlConnection` tests need the real TCP stack to bind; verify the test runs in sandbox/CI (no firewall rules). May need `#[cfg(feature = "net-tests")]` guard for offline builds.

## 3. Roll-up table

| Crate | Current % (lib / gate) | Target | Gap | Top action | Effort |
|-------|------------------------|--------|-----|------------|--------|
| executor | 76.41 / 77.83 | 80% | -3.59pp / -2.17pp | Delete 4 orphan modules (~641 LoC) + inline tests for 5 wrappers | 4–6h |
| admin | 63.01 / 65.08 | 80% | -16.99pp / -14.92pp | Move 3 modules' integration tests into inline | 6–8h |
| mysql-server | 40.62 / 42.91 | 80% | -39.38pp / -37.09pp | Inline tests for testing module + wire dispatch + move existing tests | 12–16h |
| mysql-client | 31.56 / 31.42 | 80% | -48.44pp / -48.58pp | Move `unit_tests.rs` into lib.rs + lifecycle tests | 6–10h |

**Total**: 28–40h (≈ 4 engineer-days).

## 4. Schedule (aligned with ADR-008 expiry 2026-09-01)

| Date | Milestone |
|------|-----------|
| 2026-08-12 (T+3) | mysql-client move unit_tests.rs → lib.rs; re-measure; expect ≥80% |
| 2026-08-14 (T+5) | executor: delete 4 orphans + add wrapper tests; re-measure |
| 2026-08-18 (T+9) | admin: move 3 modules' tests inline; re-measure |
| 2026-08-25 (T+16) | mysql-server: testing module + wire dispatch inline tests; re-measure |
| 2026-08-29 (T+20) | Final re-measurement of all 4 crates; GA_GATE_REPORT update |
| 2026-08-31 (T+22) | PR + review + merge; GA gate flip G3 ✅ |
| 2026-09-01 (T+23) | ADR-008 G4 exception still active (separate workstream) |

## 5. Cross-cutting remediation (do FIRST, 30 min)

Before per-crate work, decide and lock the measurement method:

- **Option A** (preferred): flip GA gate to `--tests` per ADR-001 G-04 (SSOT is in
  `coverage-baseline/README.md`). This single change may lift mysql-server / mysql-client
  above 80% by itself (those crates' bulk coverage is in `tests/*.rs`).
- **Option B**: keep `--lib` and apply per-crate inline-test work per the table above.

If Option A is adopted, per-crate work shrinks to: delete executor orphans + add inline
tests only for the modules that remain uncovered after the methodology flip. Re-measure
on 250 before committing to either path.

## 6. Validation

After each per-crate push, re-run on host 250:

```bash
cargo llvm-cov --lib --release -p sqlrustgo-<crate> --all-features --html
# Extract line % from target/llvm-cov/html/index.html
# Update docs/releases/v3.11.0/COVERAGE_FULL_<date>.md
```

Acceptance: each of the 4 crates ≥ 80.0% line under whatever measurement the gate
ultimately uses. Document the chosen measurement in `CURRENT_VERSION.md` and
`docs/governance/debt/debt-registry.yaml` (SEM-4 IN_PROGRESS entry — close it).
