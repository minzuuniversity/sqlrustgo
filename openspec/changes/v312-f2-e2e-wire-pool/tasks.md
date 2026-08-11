## 1. Root cause

- [x] 1.1 Identify SERVER_POOL global state location
- [ ] 1.2 Determine why each test sees prior test's row counts
- [ ] 1.3 Verify that test_e2e_select_simple (passing) shows what isolation works

## 2. Fix

- [ ] 2.1 Option A: per-test `with_isolated_pool(|| ...)` closure
- [ ] 2.2 Option B: test-level cleanup using `DROP TABLE` for each table name
- [ ] 2.3 Pick the minimal-blast-radius approach
- [ ] 2.4 Apply fix to e2e_wire_protocol.rs

## 3. Verification

- [ ] 3.1 Run `cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol -- --test-threads=1` — 46/46 PASS
- [ ] 3.2 Capture new evidence_hash for `05-e2e-wire-protocol.log`

## 4. PR + comment

- [ ] 4.1 Commit fix with provenance header
- [ ] 4.2 Push feature branch and create PR
- [ ] 4.3 Merge PR with admin override
- [ ] 4.4 Post comment to ISSUE #4025 with PR#, SHA, evidence_hash