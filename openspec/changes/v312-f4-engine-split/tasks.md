## 1. Plan split

- [ ] 1.1 Inventory top-level functions in execution_engine.rs
- [ ] 1.2 Group by responsibility (select/dml/ddl/tx)
- [ ] 1.3 Identify shared helpers that stay in core
- [ ] 1.4 Map all call sites (within and outside execution_engine.rs)

## 2. Apply split

- [ ] 2.1 Create `src/execution_engine/select.rs`
- [ ] 2.2 Create `src/execution_engine/dml.rs`
- [ ] 2.3 Create `src/execution_engine/ddl.rs`
- [ ] 2.4 Create `src/execution_engine/tx.rs`
- [ ] 2.5 Move functions + preserve `pub use` re-exports in core

## 3. Verify

- [ ] 3.1 `wc -l src/execution_engine.rs` ≤ 1500
- [ ] 3.2 `cargo build --all-features` exit 0
- [ ] 3.3 `cargo test --lib` PASS
- [ ] 3.4 `cargo test --test v312_13_*` PASS
- [ ] 3.5 `bash scripts/gate/check_integration_gate.sh` C-ARCH-05 PASS

## 4. PR + comment

- [ ] 4.1 Commit split with provenance header
- [ ] 4.2 Push feature branch, create PR
- [ ] 4.3 Merge PR
- [ ] 4.4 Post comment to ISSUE #4027