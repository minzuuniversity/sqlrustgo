## 1. Apply formatting

- [ ] 1.1 Run `cargo fmt --all` to apply canonical formatting
- [ ] 1.2 Verify `cargo fmt --all -- --check` exits 0

## 2. Verify

- [ ] 2.1 `cargo build --all-features` exit 0
- [ ] 2.2 `cargo test --lib` exit 0
- [ ] 2.3 `bash scripts/gate/check_integration_gate.sh` SGL-001 PASS

## 3. PR + comment

- [ ] 3.1 Commit format changes with provenance header
- [ ] 3.2 Push feature branch, create PR
- [ ] 3.3 Merge PR
- [ ] 3.4 Post comment to ISSUE #4028