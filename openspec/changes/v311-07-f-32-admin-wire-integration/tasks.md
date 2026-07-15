# V311-07 F-32 MySQL Admin Wire Integration Task Checklist

## Phase 1: WireClient skeleton (4h)

- [ ] 1.1 Create `crates/admin/src/wire_client.rs`
- [ ] 1.2 Define `WireError` enum (Connect / Query / Io variants)
- [ ] 1.3 Define `StatusReport` struct
- [ ] 1.4 Implement `WireAdmin::connect()` using `MySqlConnection::connect`
- [ ] 1.5 Implement `WireAdmin::ping()` (delegates to `conn.ping()`)
- [ ] 1.6 Implement `WireAdmin::version()` (SELECT @@version)
- [ ] 1.7 Implement `WireAdmin::status()` (parse `SELECT @@global_status`)

## Phase 2: Logical backup via wire (8h)

- [ ] 2.1 Implement `WireAdmin::logical_backup()`
- [ ] 2.2 Use `SHOW TABLES` to enumerate tables
- [ ] 2.3 Use `SELECT COUNT(*)` for row counts
- [ ] 2.4 Stream `SELECT * FROM <table>` per table
- [ ] 2.5 Serialize to CSV (handle mixed types)
- [ ] 2.6 Write tar+gzip manifest + data + checksums

## Phase 3: CLI integration (2h)

- [ ] 3.1 Add `--host`, `--port`, `--user`, `--password` flags to `crates/admin/src/main.rs`
- [ ] 3.2 When `--host` is set, dispatch to `WireAdmin` instead of in-process admin
- [ ] 3.3 New `status` subcommand using wire (when --host set)
- [ ] 3.4 New `logical-backup` subcommand using wire
- [ ] 3.5 Update `--help` output

## Phase 4: Tests (4h)

- [ ] 4.1 Create `tests/integration/admin/` directory
- [ ] 4.2 Add `mysqladmin_e2e_test.rs` with 5 tests
- [ ] 4.3 test_wire_client_connects_to_server
- [ ] 4.4 test_wire_client_runs_show_status
- [ ] 4.5 test_wire_client_ping_returns_ok  
- [ ] 4.6 test_logical_backup_via_wire_protocol_creates_archive
- [ ] 4.7 test_mixed_local_and_wire_protocol_paths_compatible

## Phase 5: Documentation (2h)

- [ ] 5.1 `docs/releases/v3.11.0/admin/WIRE_PROTOCOL_INTEGRATION.md` (usage)
- [ ] 5.2 Update `docs/governance/debt/debt-registry.yaml`: F-32 → CLOSED
- [ ] 5.3 Update `docs/releases/v3.11.0/FEATURE_CHECKLIST.md`: V311-07 → DONE
- [ ] 5.4 `openspec/changes/v311-07-f-32-admin-wire-integration/` (full spec)

## Phase 6: PR + merge (~30min)

- [ ] 6.1 Branch `fix/v311-07-f-32-admin-wire-integration`
- [ ] 6.2 Push to backup
- [ ] 6.3 Create PR on 250
- [ ] 6.4 Lower approval → 0
- [ ] 6.5 Merge
- [ ] 6.6 Force-push to gitcode + gitee
- [ ] 6.7 Restore approval → 2

## Phase 7: Issue closure (~5min)

- [ ] 7.1 Post completion comment on issue #3480
- [ ] 7.2 Close issue #3480 (F-32 → CLOSED)
