# V312-13 Wire + LOAD DATA Hardening Report

- source_agent: `minimax`
- source_run: `minimax-issue-3900-verify-2026-10-04`
- timestamp: `2026-10-03T16:36:13Z`
- branch: `HEAD`
- commit: `d59dbffd1d6b384eeb6ff95d6c1f6b3954ad246e`

| step | command | status | evidence_hash | output_location | timestamp | source_agent | source_run |
|------|---------|--------|---------------|-----------------|-----------|--------------|------------|
| 01-build | `cd /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify && cargo build -p sqlrustgo-mysql-server -p sqlrustgo-mysql-client --tests` | pass | 3dfabc5028d60ea4dac7f3bec84e33a74ed1d9cb6e545f74af10f8ad15ba75d8 | /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/01-build.log | 2026-10-03T16:36:13Z | minimax | minimax-issue-3900-verify-2026-10-04 |
| 02-typed-wrappers | `cd /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify && cargo test --test v312_13_typed_wrappers_test -- --test-threads=1` | pass | 2ba7f991e6dabd5ce4a070b81d67593653a6a09da64b5fd2afff92adbcb7ab32 | /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/02-typed-wrappers.log | 2026-10-03T16:36:13Z | minimax | minimax-issue-3900-verify-2026-10-04 |
| 03-wire-regression | `cd /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify && cargo test --test mysql_wire_protocol_test -- --test-threads=1` | pass | 887dd53fb169370f849a9dc785048d5fb27707cf0b21496d6fec39720ac9e6db | /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/03-wire-regression.log | 2026-10-03T16:36:13Z | minimax | minimax-issue-3900-verify-2026-10-04 |
| 04-prepared-statement-params | `cd /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify && cargo test -p sqlrustgo-mysql-server --test prepared_stmt_params_test -- --test-threads=1` | pass | a2c609eb7fb609019c8f8ab183ade3f38166b37e31d1c8421b4d6b3800a2015a | /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/04-prepared-statement-params.log | 2026-10-03T16:36:13Z | minimax | minimax-issue-3900-verify-2026-10-04 |
| 05-e2e-wire-protocol | `cd /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify && cargo test -p sqlrustgo-mysql-server --test wire_smoke_mysql_cli -- --test-threads=1` | pass | 4f0257d1d2a42c27685eb7d299f6ce32ec72b9f796651d7508d0cf9ba9a86175 | /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/05-e2e-wire-protocol.log | 2026-10-03T16:36:13Z | minimax | minimax-issue-3900-verify-2026-10-04 |
| 06.5-load-data-sf00001-smoke | `cd /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify && cargo test --test v312_13_load_data_sf1_test v312_13_sf1_lineitem_smoke_subset -- --nocapture` | fail | 1ac1019e232ea436381a74fc8b02fdd72bc793a296793f52cf5a9ec9e02fae65 | /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/06.5-load-data-sf00001-smoke.log | 2026-10-03T16:36:13Z | minimax | minimax-issue-3900-verify-2026-10-04 |
| 07-load-data-sf1 | `cd /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify && cargo test --test v312_13_load_data_sf1_test -- --nocapture` | fail | a7a86259411adda8f586d61c18bfd0f914dd8858c0d88a96395b0211caa51116 | /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/07-load-data-sf1.log | 2026-10-03T16:36:13Z | minimax | minimax-issue-3900-verify-2026-10-04 |
| 08-load-data-sf10 | `cd /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify && cargo test --test v312_13_load_data_sf10_test -- --nocapture` | fail | b0f9e9fa7918fbdac2a74034cb66316975afc96123ca483b554df025c799e45f | /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/08-load-data-sf10.log | 2026-10-03T16:36:13Z | minimax | minimax-issue-3900-verify-2026-10-04 |
| 09-tls-handshake | `cd /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify && cargo test --test v312_13_typed_wrappers_test v312_13_force_tls_server_implemented -- --exact` | pass | 080a8c9a43f9da4691a4dc5b0fa53c845ff921715af05d1719ef9c050c11ccb5 | /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/09-tls-handshake.log | 2026-10-03T16:36:13Z | minimax | minimax-issue-3900-verify-2026-10-04 |
| 10-compression | `cd /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify && cargo test --test v312_13_typed_wrappers_test v312_13_compress_primitives_working -- --exact` | pass | fbcfdf45a853f28c781facc0ccc91e85fb70c2d8e63d4a2bf8554dc3a7388f35 | /Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/10-compression.log | 2026-10-03T16:36:13Z | minimax | minimax-issue-3900-verify-2026-10-04 |

---

## Footer

- report_sha256: `454ffb20a60d8b08225bdadfea40f8bcc32a2d93585c0673e8f8f69d62e9986b`
- failed_steps: `1`
- artifact_path: `/Volumes/workspace/dev/sqlrustgo/.worktrees/issue-3900-verify/docs/releases/v3.12.0/evidence/wire_load_data/V312-13-REPORT.md`

This report is regenerated by `scripts/gate/check_v312_13_wire_load_data.sh`.
Any `fail` or non-zero `failed_steps` MUST be addressed before promoting
the v3.12.0 RC tag. `deferred` steps are documented gaps; promoting past
`Beta` requires each `deferred` step to either land or be moved to a
follow-up issue with owner + expiry.
