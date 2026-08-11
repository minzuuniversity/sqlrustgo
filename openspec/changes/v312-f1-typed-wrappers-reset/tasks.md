## 1. Root cause analysis

- [x] 1.1 Read `tests/integration/wire/v312_13_typed_wrappers_test.rs:44` to identify exact test assertion
- [x] 1.2 Inspect `crates/mysql-server/src/lib.rs` COM_RESET_CONNECTION handler (search `0x1F`)
- [x] 1.3 Verify client reset sequence handling in `crates/mysql-client/src/lib.rs`

## 2. Server fix

- [ ] 2.1 Locate COM_RESET_CONNECTION dispatch in server
- [ ] 2.2 Reset packet sequence (`packet_seq`) to 0
- [ ] 2.3 Reset session state (charset, status, prepared statements)
- [ ] 2.4 Return OK packet with correct seq=1 (server-side seq after reset)
- [ ] 2.5 Add debug log for traceability

## 3. Client fix

- [ ] 3.1 After COM_RESET_CONNECTION response, expect next packet seq=0 (server's seq)
- [ ] 3.2 Re-init connection-level state if needed (charset negotiation, etc.)

## 4. Verification

- [ ] 4.1 Run `cargo test --test v312_13_typed_wrappers_test -- --test-threads=1` — must PASS 22/22
- [ ] 4.2 Run `bash scripts/gate/check_v312_13_wire_load_data.sh` — step 02 status=pass
- [ ] 4.3 Capture new evidence_hash for `02-typed-wrappers.log`

## 5. PR + comment

- [ ] 5.1 Commit fix with provenance header
- [ ] 5.2 Push feature branch and create PR against develop/v3.12.0
- [ ] 5.3 Merge PR with admin override
- [ ] 5.4 Post comment to ISSUE #4024 with PR#, SHA, evidence_hash