## 1. Disposition table generator

- [ ] 1.1 New `scripts/gate/check_v312_21_mysql_compat.sh` orchestrating: walk `tests/compat/mysql_v3_12/*.sql`, run each via `MySqlTestClient`, diff output against `*.out`
- [ ] 1.2 Emit `docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md` with columns: `surface | previous_claim | current_evidence | decision (PASS|unsupported|deferred) | evidence_hash | owner | expiry`
- [ ] 1.3 SHA256 the per-row log and the full report; both go into the gate artifact

## 2. Fixture grammar + runner

- [ ] 2.1 Define grammar: `# name:`, `# expect: PASS|UNSUPPORTED:<reason>|DEFERRED:<issue-link>`, then SQL
- [ ] 2.2 New `tests/compat_runner/src/main.rs` (or in-tree `tests/compat_runner.rs`) that reads the fixture dir, runs each file, captures output, compares against `*.out`
- [ ] 2.3 Add `compat_runner` to the workspace (or as a binary in `crates/sqlrustgo-cli`)

## 3. GMP-critical subset — fixtures

- [ ] 3.1 `tests/compat/mysql_v3_12/show_tables.sql` + `show_tables.out`: CREATE/DROP a sequence of tables, verify `SHOW TABLES` output
- [ ] 3.2 `alter_rename.sql` + `.out`: `ALTER TABLE t RENAME TO t2` and verify subsequent `SHOW TABLES`
- [ ] 3.3 `alter_add_column.sql` + `.out`: `ALTER TABLE t ADD COLUMN x INT DEFAULT 0`, verify `SELECT x` returns 0
- [ ] 3.4 `alter_drop_column.sql` + `.out`: opposite of 3.3
- [ ] 3.5 `alter_modify_column.sql` + `.out`: `ALTER TABLE t MODIFY COLUMN x BIGINT`
- [ ] 3.6 `empty_password_auth.sql` + `.out`: connect with empty password, verify allowed-or-denied
- [ ] 3.7 `prepared_stmt_roundtrip.sql` + `.out`: prepare/execute/close sequence (delegates to `binary-prepared-statement-roundtrip` spec)

## 4. Explicit unsupported — fixtures

- [ ] 4.1 `create_procedure_unsupported.sql` + `.out`: expect `UNSUPPORTED: stored procedure tokens not implemented`
- [ ] 4.2 `column_perm_unsupported.sql` + `.out`: `GRANT SELECT(col) ON t TO user` → `UNSUPPORTED: column-level permissions only via V311-09`
- [ ] 4.3 `with_rollup_unsupported.sql` + `.out`: `SELECT ... WITH ROLLUP` → `UNSUPPORTED: WITH ROLLUP not implemented`
- [ ] 4.4 `with_cube_unsupported.sql` + `.out`
- [ ] 4.5 `replace_into_complex_unsupported.sql` + `.out`
- [ ] 4.6 `window_rank_partition_unsupported.sql` + `.out`
- [ ] 4.7 `stddev_pop_unsupported.sql` + `.out`
- [ ] 4.8 `var_pop_unsupported.sql` + `.out`
- [ ] 4.9 `median_unsupported.sql` + `.out`
- [ ] 4.10 `group_concat_unsupported.sql` + `.out`

## 5. Deferred — fixtures (with owner + expiry)

- [ ] 5.1 `timestamp_timezone_deferred.sql` + `.out` referencing a follow-up issue with owner + expiry ≤ 2027-06-30
- [ ] 5.2 `connection_pool_deferred.sql` + `.out`
- [ ] 5.3 `alter_change_full_syntax_deferred.sql` + `.out`

## 6. Release notes boundary

- [ ] 6.1 Modify `docs/releases/v3.12.0/RELEASE_NOTES.md` "MySQL compatibility" section to link to `SURFACE_DISPOSITION.md`
- [ ] 6.2 Remove any "supports X" claim that lacks a `decision=PASS` row in the disposition

## 7. PR

- [ ] 7.1 Open PR on Gitea 252 against `develop/v3.12.0`
- [ ] 7.2 Get 1 reviewer approval
- [ ] 7.3 Force-merge (admin)
- [ ] 7.4 Sync to gitcode + gitee
- [ ] 7.5 Update ISSUE #3908 with PR link + sample `SURFACE_DISPOSITION.md`
