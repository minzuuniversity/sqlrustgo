# V312-13 Wire + LOAD DATA Hardening Report

- source_agent: `claude-code`
- source_run: `claude-code-v312-13-868088aa70`
- timestamp: `2026-08-14T20:17:24Z`
- branch: `develop/v3.12.0`
- commit: `868088aa70dc8578dd803b491d6fde586860238d`

| step | command | status | evidence_hash | output_location | timestamp | source_agent | source_run |
|------|---------|--------|---------------|-----------------|-----------|--------------|------------|
| 01-build | `cd /home/openclaw/workspace/dev/sqlrustgo && cargo build -p sqlrustgo-mysql-server -p sqlrustgo-mysql-client --tests` | pass | 2381c5dac46d73c0c85551f5c73c06c122c3a3102475d3cf3bc35588216138c9 | /home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/01-build.log | 2026-08-14T20:17:24Z | claude-code | claude-code-v312-13-868088aa70 |
| 02-typed-wrappers | `cd /home/openclaw/workspace/dev/sqlrustgo && cargo test --test v312_13_typed_wrappers_test -- --test-threads=1` | pass | eae4aa029fda078b6b3d319f968e8033379d916905800d5d329c97daf8f74bef | /home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/02-typed-wrappers.log | 2026-08-14T20:17:24Z | claude-code | claude-code-v312-13-868088aa70 |
| 03-wire-regression | `cd /home/openclaw/workspace/dev/sqlrustgo && cargo test --test mysql_wire_protocol_test -- --test-threads=1` | pass | 7868596f4fc0c67f789f66c807777eb95f47145c33a1def6011806021cf3c78b | /home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/03-wire-regression.log | 2026-08-14T20:17:24Z | claude-code | claude-code-v312-13-868088aa70 |
| 04-prepared-statement-params | `cd /home/openclaw/workspace/dev/sqlrustgo && cargo test -p sqlrustgo-mysql-server --test prepared_stmt_params_test -- --test-threads=1` | pass | 5e002005d9ce149bc36b6d0b6c4a42d7dad065b48d0d843d424ec704f28f965c | /home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/04-prepared-statement-params.log | 2026-08-14T20:17:24Z | claude-code | claude-code-v312-13-868088aa70 |
| 05-e2e-wire-protocol | `cd /home/openclaw/workspace/dev/sqlrustgo && cargo test -p sqlrustgo-mysql-server --test wire_smoke_mysql_cli -- --test-threads=1` | pass | 2da5e488ce5f4937122f301747a1a4837a2ad8ccccb3129b4c4a57016a935fe5 | /home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/05-e2e-wire-protocol.log | 2026-08-14T20:17:24Z | claude-code | claude-code-v312-13-868088aa70 |
| 06.5-load-data-sf00001-smoke | `cd /home/openclaw/workspace/dev/sqlrustgo && cargo test --test v312_13_load_data_sf1_test v312_13_sf1_lineitem_smoke_subset -- --nocapture` | pass | ba7180b7ec5f5bf153784a5913ce973694882bdf4548d1e39d55b26fb8a999d0 | /home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/06.5-load-data-sf00001-smoke.log | 2026-08-14T20:17:24Z | claude-code | claude-code-v312-13-868088aa70 |
| 07-load-data-sf1 | `cd /home/openclaw/workspace/dev/sqlrustgo && cargo test --test v312_13_load_data_sf1_test -- --nocapture` | pass | ab4e51cd248f8280821309556207a850ae888aff7c9a217851eabfb11b7dd92a | /home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/07-load-data-sf1.log | 2026-08-14T20:17:24Z | claude-code | claude-code-v312-13-868088aa70 |
| 08-load-data-sf10 | `cd /home/openclaw/workspace/dev/sqlrustgo && cargo test --test v312_13_load_data_sf10_test -- --nocapture` | fail-environmental | 1aa3566f68c0e46ed7c1b3bc59d12a6840aa0366669f4eb5c35fbca89e241cef | /home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/08-load-data-sf10.log | 2026-08-14T20:17:24Z | claude-code | claude-code-v312-13-868088aa70 |
| 09-tls-handshake | `cd /home/openclaw/workspace/dev/sqlrustgo && cargo test --test v312_13_typed_wrappers_test v312_13_force_tls_server_implemented -- --exact` | pass | dddca8b8b77821dbbe09025905fc969b508950b4c9a0feb816afef6753810856 | /home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/09-tls-handshake.log | 2026-08-14T20:17:24Z | claude-code | claude-code-v312-13-868088aa70 |
| 10-compression | `cd /home/openclaw/workspace/dev/sqlrustgo && cargo test --test v312_13_typed_wrappers_test v312_13_compress_primitives_working -- --exact` | pass | 335f286233c6cfafaff5dc7b8c5728cc0e6f02b6bc7934d2d9138fc203548a46 | /home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/10-compression.log | 2026-08-14T20:17:24Z | claude-code | claude-code-v312-13-868088aa70 |

---

## Footer

- report_sha256: `3f5288cdc20c5580e6ca5478bbdbcd9ca547b1be49cbb80e6c35d66085b53cc5`
- failed_steps: `1` (08-load-data-sf10 — environmental, see below)
- code_steps_failed: `0`
- artifact_path: `/home/openclaw/workspace/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/V312-13-REPORT.md`
- provenance_supersedes: prior round (commit `a0e5fc1a32`, 2026-08-14T13:33:50Z) on branch `fix/v312-4019-3943-evidence-refresh`

## Step 08 — Environmental failure (NOT a code regression)

`tests/integration/tpch/v312_13_load_data_sf10_test.rs:91` panicked with:

```
SF=10 fixture missing: /tmp/tpch-sf10/region.tbl. Generate with:
  cd /tmp/tpch-dbgen && ./dbgen -s 10 -f -T r && ./dbgen -s 10 -f -T n && ./dbgen -s 10 -f -T s && mv *.tbl /tmp/tpch-sf10/
```

Root cause: the `/tmp/tpch-sf10/` directory does not exist on this CI/dev host;
dbgen has not been run. This is a **fixture-availability gate**, not a code
gate. The 16 other tests in `v312_13_load_data_sf10_test` (harness / oracle
round-trip / sha256 / port-allocation / 5 wire-smoke matrix cells) all PASS at
HEAD `868088aa70` — see `08-load-data-sf10.log` lines around the panic for the
green tests run before the smoke step.

Action to close:
- Run `cd /tmp/tpch-dbgen && ./dbgen -s 10 -f -T r -T n -T s && mkdir -p /tmp/tpch-sf10 && mv *.tbl /tmp/tpch-sf10/`
- Re-run `scripts/gate/check_v312_13_wire_load_data.sh`. With fixtures present,
  step 08 returns to `pass` (verified at commit `a0e5fc1a32`, prior round).

This report is regenerated by `scripts/gate/check_v312_13_wire_load_data.sh`.
Any `fail` or non-zero `failed_steps` MUST be addressed before promoting
the v3.12.0 RC tag. `deferred` steps are documented gaps; promoting past
`Beta` requires each `deferred` step to either land or be moved to a
follow-up issue with owner + expiry.
