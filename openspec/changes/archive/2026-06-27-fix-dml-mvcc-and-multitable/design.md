## Context

`tests/dml_integration_test.rs` is the canonical DML smoke test for the
in-memory engine path used by unit tests. After commit `0bcdfabcdf`
(INSERT/SELECT, UPDATE/DELETE subquery) it shows 20 passed / 4
ignored. The four remaining `#[ignore]`s cover two distinct gaps:

- **Transaction rollback on MemoryStorage.** `MemoryStorage::rollback_transaction`
  returns `Err("Transactions not supported by this storage engine")`.
  `WalStorage` is fine, but the test fixtures use `MemoryStorage`.
- **Multi-table UPDATE / DELETE.** `UpdateStatement.table` and
  `DeleteStatement.table` are `String`, so `UPDATE a, b SET ...` and
  `DELETE a, b FROM a, b` cannot even be parsed.

Both are unblock-the-ignore-test work; neither touches the wire
protocol, MVCC semantics on `WalStorage`, or external storage.

## Goals / Non-Goals

**Goals:**

- `MemoryStorage` honours `begin/commit/rollback` for in-memory DML
  such that `BEGIN; INSERT …; ROLLBACK;` is a no-op from a peer's view.
- `UPDATE t1, t2 SET t1.v = t2.v WHERE …` parses, plans, and applies.
- `DELETE t1, t2 FROM t1, t2 WHERE …` parses, plans, and applies.
- The four `#[ignore]` tests turn into `#[test]` and pass.

**Non-Goals:**

- No MVCC / SI for concurrent readers on MemoryStorage — the new
  semantics are "writer sees own uncommitted writes, peers see only
  committed state" (matching the behaviour a developer would expect
  from `BEGIN; …; ROLLBACK` in tests).
- No multi-statement UPDATE / DELETE (i.e. `UPDATE t1 SET v=1; UPDATE t2
  SET v=2` in one statement) — that's a parser-level feature
  unrelated to this work.
- No change to `WalStorage` semantics.

## Decisions

- **MemoryStorage transaction log: per-table `Vec<RowSnapshot>` rather
  than copy-on-write of the entire `HashMap<String, Vec<Vec<Value>>`.**
  CoW is simpler but allocates a full copy on every `begin`, which is
  wasteful for tests that only need the rollback contract for a single
  short transaction. Per-table snapshots record only the rows that were
  touched, so a `ROLLBACK` that touches zero rows is a no-op.

- **`UpdateStatement` / `DeleteStatement` carry `tables: Vec<TableRef>`**
  (name + optional alias). The single-table form `UPDATE t SET ...`
  continues to parse as `tables: vec![TableRef { name: "t", alias: None }]`
  so `execute_update` and `execute_delete` only need one code path.

- **Multi-table executor uses the cartesian-product scan pattern from
  `engine_select::build_combined_schema`**, not a real join optimiser.
  The test cases (`UPDATE a, b SET a.v=10, b.v=20`, `DELETE a, b FROM a, b`)
  don't add a join predicate, so a cartesian scan is exactly correct and
  matches what the spec already does for `SELECT * FROM a, b`.

- **`MemoryStorage::in_transaction()` returns true once `begin` has
  been called and until `commit` or `rollback`.** Already correct in
  the existing `WalStorage`; we mirror the contract.

- **WAL replay is not re-evaluated.** `WalStorage` already has its own
  transaction support, and the fix is gated on `MemoryStorage`. A
  shared trait method (`StorageEngine::begin_transaction`) is the right
  abstraction but is out of scope here; the existing trait already
  declares the method.

## Risks / Trade-offs

- [MemoryStorage nested BEGIN] → block nested calls (return error)
  rather than supporting savepoints. The two failing tests don't nest.
- [MemoryStorage commit failure after partial writes] → keep the
  current "all writes are buffered" model so commit is a single
  HashMap insertion; rollback is a single HashMap swap. Either
  operation is atomic by construction.
- [Multi-table UPDATE with no WHERE] → explicit cartesian scan is
  O(N₁·N₂) per write, identical to `SELECT * FROM a, b` for the same
  shapes. Tests are tiny; production use should hit `WalStorage`.
- [AST breaking change for `UpdateStatement.table` / `DeleteStatement.table`]
  → only the `executor` crate reads these fields, plus the parser.
  All callers in-tree can be updated in this change. No external crate
  consumes the AST.

## Migration Plan

No deployment / rollback concerns — both features are net-new on
the in-memory path and ship behind the existing executor entry point.
The two `WalStorage` and `FileStorage` paths are unchanged.

## Open Questions

None blocking. Two follow-up ideas left intentionally out of scope:

- Should `StorageEngine::begin_transaction` move from returning `u64`
  to a richer `TransactionHandle` struct so engines can return
  per-engine state? Out of scope.
- Should `MemoryStorage` snapshot cover schema changes (`CREATE TABLE`
  inside a TX)? Out of scope; we only handle DML row writes.