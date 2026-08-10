# V312-13 / #3900 Closure — Design

## Close Scope Definition

### IN scope (DONE in V312-13)

* **COM_QUERY (0x03) E2E**: `e2e_wire_protocol.rs` + `wire_smoke_mysql_cli.rs` PASS
* **COM_STMT_PREPARE (0x16) / EXECUTE (0x17) / CLOSE (0x19)**: `wire_smoke_mysql_cli` 11 tests PASS
* **Error packet (0xFF) E2E**: `test_wire_smoke_error_packet_structure` PASS
* **COM_RESET_CONNECTION (0x1F) E2E**: `test_wire_smoke_reset_connection` PASS (client side)
* **Binary row encoding**: INT/VARCHAR/NULL type matching fixed in PR #3948
* **C-ARCH-01~05 invariants**: 5/5 PASS
* **`check_load_data_infile.sh`**: 4/4 PASS
* **`check_anti_fabrication.sh`**: ERRORS=0, PASS

### OUT of scope (deferred to #3959 V312-24)

| Item | Status | Follow-up |
|------|--------|-----------|
| LOAD DATA INFILE parser (server-side) | Parser does not yet accept LOAD DATA syntax | #3959 |
| LOAD DATA SF=1 row count/hash (full execution) | Fixture generated, parser syntax error | #3959 |
| LOAD DATA SF=10 | Not implemented | #3959 |
| TLS handshake (server-side) | Not implemented | #3959 |
| zlib compression | Not implemented | #3959 |
| Parameterized query binary result | Not implemented | #3959 |
| COM_RESET_CONNECTION (server-side) | Client smoke only | #3959 |

## Evidence Files to Update

### `docs/releases/v3.12.0/wire-e2e-report.md`
Add section "V312-13 Closure Status" listing the 11 wire smoke tests + binary row fix as ✅ DONE.

### `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md`
Add boundary table showing V312-13 (✅ wire main path) vs V312-24 (deferred LOAD DATA/TLS/compression).

### `docs/releases/v3.12.0/load-data-report.md`
Update status to "V312-13 partial: fixture + parser stub. Full execution deferred to #3959 (V312-24)".

### `docs/releases/v3.12.0/REVIEWER_SIGN_OFF.md`
Append V312-13 sign-off entry:
- Reviewer 1: Claude Code (V312-13 binary row fix) — already exists
- Reviewer 2: hermes-z6g4 (PR #3948 approval) — to be added

## Closure Comment on #3900

Post final closure evidence comment with:
1. PR #3948 + commit `f4e3427fa864c1caea98f8fb843fc30da2ad2e20`
2. Command outputs (5 gates: arch_invariants, load_data_infile, anti_fabrication, wire smoke, sqlLogicTest if applicable)
3. PASS/FAIL summary
4. evidence_hash for each artifact
5. Deferred mapping table referencing #3959 (V312-24)
6. Sign-off confirmation (hermes-z6g4 + openclaw)

## Verification

```bash
# Evidence files
$ ls -la docs/releases/v3.12.0/{wire-e2e-report.md,MYSQL_COMPAT_STATUS.md,load-data-report.md,REVIEWER_SIGN_OFF.md}
# All 4 files exist and updated

# Gates (already passing per PR #3948)
$ bash scripts/gate/check_arch_invariants.sh  # 5/5 PASS
$ bash scripts/gate/check_load_data_infile.sh  # 4/4 PASS
$ bash scripts/gate/check_anti_fabrication.sh  # ERRORS=0, PASS

# V312-19 release gates (cross-check)
$ bash scripts/gate/check_v312_19_release_gates.sh --signoff ...
# PASS exit 0
```
