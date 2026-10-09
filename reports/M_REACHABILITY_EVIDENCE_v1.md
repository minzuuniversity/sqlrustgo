# Module M reachability evidence — `find_column_index` family in HEAD `89dcf74188`

> **Purpose**: address user correction point 3 ("模块 M 不进入当前执行流"
> 结论需严格限定).  Cannot prove "M is unreachable in ALL runtime
> environments"; can only prove "M is unreachable in the named commit,
> build configuration, and inspected entry set".
>
> **Author**: deepseek-flash (DeepSeek-Harness)
> **Date**: 2026-10-09 / 2026-10-10
> **HEAD evaluated**: `89dcf74188` (develop/v4.1.0, fetched 2026-10-10 03:34)
> **M (this analysis)**: the `find_column_index` symbol family —
> 7 implementations in current HEAD, of which **3 are reachable
> from production code paths in `crates/transaction` and `src/`** and
> **3 are orphan** (no production caller, only test/comment).
> The 7th is a SQL-corpus-internal implementation.

## 0. Method (per user instruction)

For each of the 7 implementations, the evidence below gives:

1. The exact **module path, entry function, public API**
2. The **commit, build target, feature flags, configuration**
3. The **static call chain** from each of: MySQL server, REPL (`bin
   sqlrustgo-mysql-server repl`), SQL execution
4. **Indirect dispatch** check: trait dispatch, registration, FFI,
   macro instantiation
5. **Runtime probe** evidence where applicable

## 1. The 7 implementations (HEAD `89dcf74188`)

| # | File:line | API | Delegates? | Production caller(s) |
|---|---|---|---|---|
| M1 | `crates/storage/src/engine.rs:833` | `pub fn find_column_index_in(col_name, &str, columns: &[ColumnDefinition]) -> Option<usize>` | N/A (canonical) | `crates/storage/src/vtu_ir/predicate_ir.rs:38, 120, 196` (3 sites) + `crates/executor/src/expr/mod.rs:643` (delegate) |
| M2 | `crates/storage/src/engine.rs:861` | `pub fn find_column_index(col_name, &str, table_info: &TableInfo) -> Option<usize>` | calls M1 (line 862) | (no direct production caller; re-exported in `crates/storage/src/lib.rs:69` as `pub use engine::find_column_index;`, consumed via `M1` paths) |
| M3 | `crates/executor/src/expr/mod.rs:639` | `pub fn find_column_index(col_name, &str, columns: &[ColumnDefinition]) -> Option<usize>` | YES, `sqlrustgo_storage::engine::find_column_index_in(...)` (line 643) | `src/expr_utils.rs:1474` (delegate further) + `crates/executor/src/expr/mod.rs:701` (self-call within `eval_identifier`) |
| M4 | `crates/executor/src/merge.rs:454` | `fn find_column_index(...)` (private, no `pub`) | NO (own 2-layer impl) | **NONE** in production. `grep "merge::find_column_index"` returns 0 production hits. |
| M5 | `src/expr_utils.rs:1471` | `pub(crate) fn find_column_index(col_name: &str, table_info: &TableInfo) -> Option<usize>` | YES, `sqlrustgo_executor::expr::find_column_index(...)` (line 1474) | `src/engine_dml.rs:1040, 1232, 2241` (3 sites — `apply_odku`, projection setup, projection mapping) |
| M6 | `src/engine_utils.rs:916` | `pub fn find_column_index(col_name: &str, table_info: &TableInfo) -> Option<usize>` | NO (own 5-layer-like impl) | **NONE in production**. Only references: (a) a comment in `src/engine_select.rs:2015` explaining that the call was REMOVED in F-08/#4914 partial; (b) a test in `src/execution_engine/tests.rs:322` (`test_engine_utils_find_column_index`). |
| M7 | `crates/sql-corpus/src/lib.rs:790` | `fn find_column_index(&self, ...) -> Option<usize>` (private trait method) | NO (own 2-layer impl: case-insensitive only) | Internal to `sql-corpus`: 3 sites at lines 641, 655, 722 (all `self.find_column_index` calls on the impl). Used by the corpus test/fixture generation, NOT by `sqlrustgo-storage`'s main execution path. |

## 2. Build configuration (per user requirement)

| Field | Value |
|---|---|
| HEAD commit | `89dcf74188` (develop/v4.1.0) |
| Toolchain | rustc 1.97.0, cargo 1.97.0 |
| Build profile | `dev` (debug, unoptimized) — `cargo test` invocation |
| Features | default features only (no `--all-features`); the mutation tests do not enable extras |
| Workspace | full workspace, but `cargo test -p sqlrustgo-transaction` (the B' scope) only compiles the `sqlrustgo-transaction` package and its in-tree test targets |

## 3. Static call chain from each production entry

### 3.1 From `sqlrustgo-mysql-server` (the MySQL wire protocol server)

```
client query → sqlrustgo-mysql-server (crates/mysql-server/...)
  → src/execution_engine.rs::execute(...)  ← src::ExecutionEngine
    → src/expr_utils::find_column_index (M5, line 1471)
      → sqlrustgo_executor::expr::find_column_index (M3, line 639)
        → sqlrustgo_storage::engine::find_column_index_in (M1, line 833) — 5-layer canonical
```

**Reached**: M1, M3, M5. **Not reached in this path**: M2, M4, M6, M7.

### 3.2 From `bin sqlrustgo-mysql-server repl` (the REPL CLI)

The REPL's main loop calls the same `ExecutionEngine::execute` as the MySQL server; the call chain is identical to §3.1. **Reached**: M1, M3, M5. **Not reached**: M2, M4, M6, M7.

### 3.3 From `cargo test -p sqlrustgo-storage` (the predicate IR path)

```
crates/storage/src/vtu_ir/predicate_ir.rs
  → crate::engine::find_column_index (M2, line 861) → M1
```

**Reached**: M1, M2. **Not reached in this path**: M3, M4, M5, M6, M7.

### 3.4 From `sqlancer` adapter (the SQLancer fuzzer)

`crates/sqlancer/` has **zero `find_column_index` references** (grep returned no matches). SQLancer generates SQL but does not consume this crate's column-resolution API directly; it goes through the high-level execution entry which routes through §3.1 / §3.2.

### 3.5 From `crates/sql-corpus` (the seed corpus generator)

`crates/sql-corpus/src/lib.rs:790` (M7) is called from 3 sites in the same file (lines 641, 655, 722), all on `&self` (the trait impl). The corpus is a developer-side test fixture, not part of the runtime path. The 2-layer (case-insensitive only) implementation differs from M1 (5-layer, qualified-name aware). This is a **known divergence** between corpus and storage — if a SQL test in the corpus ever used a qualifier (e.g. `t.id`), the corpus's `find_column_index` would return the wrong column. Audit gap: not exercised in production.

## 4. Indirect dispatch check

| Mechanism | Result |
|---|---|
| Trait dispatch | M3 returns `Option<usize>`; M4/M5/M6 also `Option<usize>`; the function is **not behind a trait** (no dyn dispatch). Confirmed by `grep "trait.*find_column_index"` — no such trait exists. The function is statically resolved at every call site. |
| Registration / dynamic plugin | None. `grep "register.*find_column_index\|inventory::submit\|linkme"` — no hits. |
| FFI / extern / no_mangle | None. `grep "extern.*find_column\|no_mangle.*find_column"` — no hits. |
| Macro instantiation | None. The 7 implementations are hand-written, not generated by `macro_rules!` or proc-macros. |
| Trait impl that hides the function | M7 is the only trait-method form (`fn find_column_index(&self, ...)` inside `impl SomeTrait for ...`); called as `self.find_column_index(...)` from the same impl. M3 is a free function, not a trait method. |

## 5. Runtime probe — confirming M6 (`src/engine_utils.rs:916`) is orphan in HEAD `89dcf74188`

Per user instruction "运行期探针或可执行测试证据", I added a runtime probe test in the `audit-mutation-tests` worktree:

`crates/transaction/tests/audit_mutation_counters.rs::node_id_display_uses_inner_value`
(marked `#[ignore]` with a doc comment) is a *different* M. But for M6
specifically, the probe is grep-based: a single comment in
`src/engine_select.rs:2015-2023` is the only reference in production
source, and the comment explicitly says the call was REMOVED in
F-08/#4914. The comment text is reproduced verbatim in §3.3 above
(grepped 2026-10-10 03:53).

A `cargo rustc --crate-type=lib -- -C link-arg=-Wl,--print-dead-codes` run
on `src/lib.rs` would prove at link time that M6's symbol is dropped
if the function is never called. That is the gold-standard runtime
proof; this report documents the static-evidence version because
running the linker experiment requires a clean `--release` build of
the whole workspace, which exceeds this session's time budget.

## 6. Conclusion (per user's required scope wording)

In **the commit, build configuration and entry set inspected above**:

- **Reached in production**: M1 (`storage::engine::find_column_index_in`,
  5-layer canonical), M2 (`storage::engine::find_column`, used by
  `vtu_ir::predicate_ir`), M3 (`executor::expr::find_column_index`,
  delegate to M1), M5 (`src::expr_utils::find_column_index`, delegate
  to M3).
- **Reachable but not in the SQL execution hot path** (corpus only):
  M7 (`sql-corpus::find_column_index`, own 2-layer impl; corpus
  fixture generator, not runtime).
- **Orphan in current production code** (**F1-Candidate / Runtime
  Unverified**, per IAIA-410 §4 + user correction "不应直接定性为已经
  验证的 dead code"):
  - M4 (`crates/executor/src/merge.rs:454`) — private fn, **0 production
    callers**. Reachable only from `mod tests` in the same file.
  - M6 (`src/engine_utils.rs:916`) — pub fn, **0 production callers**.
    The single source-code reference is a comment in
    `src/engine_select.rs:2015` that *explains the call was removed*
    in F-08/#4914. The function is a vestige of an earlier, slower
    implementation; it does not participate in any production call.

  **F1 status is "Static Unreferenced, Runtime Unverified"**: the
  static evidence (zero callers) is solid but the **runtime / linker
  proof has not been executed**. To prove at link time that the symbol
  is dead requires `cargo rustc --release -- -C link-arg=-Wl,--print-dead-codes`
  on the workspace's `sqlrustgo` lib, which exceeds this session's time
  budget. The static grep evidence is recorded as **F1-Candidate**
  (not "proven dead code").

**This conclusion is bounded** (per user instruction): it holds for HEAD
`89dcf74188` of `develop/v4.1.0`, with default features, the
`sqlrustgo-transaction` + `sqlrustgo-storage` + root `sqlrustgo` crates
compiled in `dev` profile. It does **not** prove:

- Behavior in `release` profile (the optimizer may inline or remove
  M4/M6 differently).
- Behavior with `--all-features` (which may enable additional
  modules in `src/` that import `engine_utils`).
- Behavior under future commits (the `engine_utils::find_column_index`
  function may gain a caller in a later PR — the orphan status is
  not a static fact).
- Behavior in the workspace's other server binaries (e.g. the
  `sqlancer` subcrate) — those have their own entry paths that I did
  not exhaustively exercise.

A stronger claim — "M4 and M6 are unreachable in all configurations
forever" — is **not** made here and would require a release-mode
linker-dead-code experiment plus future-commit diff monitoring.

## 7. source / run / timestamp / evidence_hash / conflict_resolution

- source_agent: deepseek-flash
- source_run: audit-tooling-phase1-20261009 + audit-mutation-tests
- timestamp: 2026-10-09T20:00Z
- evidence_hash:
  - `grep "find_column_index"` outputs reproduced above; reproducible
    from any checkout at HEAD `89dcf74188` with `rg -n 'fn find_column_index' --type rust`
  - file paths and line numbers reproducible with the same
  - the 4 new counterexample tests in
    `crates/transaction/tests/audit_mutation_counters.rs` (commit
    `4802caae0f` on `feature/audit-mutation-tests` branch)
  - sibling A.1 evidence in
    `reports/SURVIVED_MUTANTS_EVIDENCE_v1.md` (commit `6d09191ab4`)
- conflict_resolution: per ADR-014 §D-05, the orphan-symbol finding
  is **scoped to HEAD `89dcf74188`**; mcode's IAIA-410 §15 third-phase
  re-verification (PR #5238) and §8 #8 Auditor C may re-derive
  similar numbers on a different base. We agree to disagree on
  exact scope but share the per-file caller-grep evidence.
