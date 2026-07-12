## 1. Refactor test MysqlAdmin into shared library

- [x] 1.1 Create `crates/admin/src/mysqladmin.rs` with `MysqlAdmin` struct ✓ (already exists)
- [x] 1.2 Move `Connection`, `SystemVariable`, `MysqlAdmin` structs to `crates/admin/src/mysqladmin.rs` ✓ (already exists)
- [x] 1.3 Export `mysqladmin` module from `crates/admin/src/lib.rs` ✓ (already exported)
- [x] 1.4 Update `tests/mysqladmin_test.rs` to use `use sqlrustgo_admin::MysqlAdmin` ✓ N/A per design: tests remain independent in-memory mocks
- [x] 1.5 Verify all 11 existing mysqladmin tests still pass ✓ (11 passed)

## 2. Add mysqladmin CLI subcommands to crates/admin

- [x] 2.1 Add `Status`, `Reload`, `Refresh`, `FlushTables`, `Processlist`, `Kill` variants to `Commands` enum ✓ (already implemented)
- [x] 2.2 Add `--host`, `--port`, `--user`, `--password` connection flag fields to `Cli` struct ✓ (already implemented)
- [x] 2.3 Implement `dispatch` match arm for `Status` command ✓ (already implemented: SHOW GLOBAL STATUS)
- [x] 2.4 Implement `dispatch` match arm for `Reload` command ✓ (already implemented: prints "Reload complete")
- [x] 2.5 Implement `dispatch` match arm for `Refresh` command ✓ (already implemented: prints "Refresh complete")
- [x] 2.6 Implement `dispatch` match arm for `FlushTables` command ✓ (already implemented: prints "Flushing tables successful")
- [x] 2.7 Implement `dispatch` match arm for `Processlist` command ✓ (already implemented: queries information_schema.processlist)
- [x] 2.8 Implement `dispatch` match arm for `Kill <id>` command ✓ (already implemented: KILL query)
- [x] 2.9 Add `sqlrustgo-cli` crate dependency to `crates/admin/Cargo.toml` ✓ N/A: uses sqlrustgo-mysql-client (already present)

## 3. Build and verify CLI binary

- [x] 3.1 Run `cargo build -p sqlrustgo-admin` — verify binary compiles ✓
- [x] 3.2 Run `sqlrustgo-admin --help` — verify all 6 subcommands appear ✓ (status, reload, refresh, flush-tables, processlist, kill)
- [x] 3.3 Run `sqlrustgo-admin status --help` — verify connection flags appear ✓ (--host, --port, --user, --password)
- [x] 3.4 Run `cargo test -p sqlrustgo-admin` — run any unit tests in admin crate ✓

## 4. End-to-end verification (requires running server)

- [ ] 4.1 Start sqlrustgo-mysql-server in background on port 3306
- [ ] 4.2 Run `sqlrustgo-admin status` against running server — verify Uptime/Threads/Questions output
- [ ] 4.3 Run `sqlrustgo-admin processlist` — verify header row printed
- [ ] 4.4 Run `sqlrustgo-admin flush-tables` — verify success output
- [ ] 4.5 Run `sqlrustgo-admin reload` — verify success output
- [ ] 4.6 Run `sqlrustgo-admin refresh` — verify success output
- [ ] 4.7 Run `sqlrustgo-admin kill <id>` with valid and invalid IDs

## 5. Documentation and issue closure

- [x] 5.1 Update `docs/releases/v3.10.0/ARCHITECTURE.md` — mark F-32 as CLOSED ✓ (added V310-14 mysqladmin CLI binary to architecture table and admin binary description)
- [ ] 5.2 Comment on Gitea issue #3768 with verification evidence
- [ ] 5.3 Close issue #3768 after PR merge

## Status

| 1 (library) | claude | ✅ Done |
| 2 (CLI subcommands) | claude | ✅ Done |
| 3 (build/verify) | claude | ✅ Done |
| 4 (E2E) | claude | ⏸ Pending (requires running server) |
| 5 (docs/close) | claude | 5.1 ✅ Done; 5.2-5.3 ⏸ Pending |
