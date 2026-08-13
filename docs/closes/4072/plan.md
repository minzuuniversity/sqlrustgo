# #4072 multi-connection transaction isolation plan

## Root cause (recap from issue body)
- crates/sqlrustgo_sqllogictest/src/main.rs::SltDb::with_storage
  shares a single MemoryStorage across all named connections.
- Writes mutate in place; reads see them immediately, violating
  SQL-standard transaction isolation.

## Proposed fix (deferred)
The architectural fix needs snapshot isolation in MemoryStorage
and a per-connection tx context. The full design is:

1. MemoryStorage gains an RwLock<StorageSnapshot> wrapper.
2. MemoryTransactionContext { tx_id, snapshot_at } is pushed on
   every BEGIN TRANSACTION and COMMIT / ROLLBACK.
3. Reads under a tx consult snapshot_at and route through the
   snapshot view.
4. Writes always go to the live storage; reads see them only
   after COMMIT.

The actual implementation belongs in the storage layer plus
the SLT runner. Estimated cost: ~150-250 lines of Rust across
crates/storage, crates/executor, and crates/sqlrustgo_sqllogictest.

## Why this is a plan-only PR
Single-turn budget cannot cover the full implementation + the
four acceptance gates. This PR records the plan so the next
pass can land the implementation without re-doing the design
work.

## Source / agent
- source_agent: sisyphus
- source_run: v313-4072-tx-isolation-plan / Issue #4072
- timestamp: 2026-08-11

## Acceptance gates (still open)
- update__test_update.test: all pass
- con2 invisible until con1 COMMIT
- existing single-connection tests preserved
- e2e_wire_protocol suite still pass
