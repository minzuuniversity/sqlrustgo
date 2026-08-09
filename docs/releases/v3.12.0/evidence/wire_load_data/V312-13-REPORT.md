# V312-13 Wire + LOAD DATA Hardening Report

- source_agent: `minimax`
- source_run: `minimax-v312-13-8ecb0ddf37`
- timestamp: `2026-08-09T08:32:34Z`
- branch: `feature/v312-19-sql-corpus-invariant`
- commit: `8ecb0ddf371301b419a07cd514022db63d01323b`

| step | command | status | evidence_hash | output_location | timestamp | source_agent | source_run |
|------|---------|--------|---------------|-----------------|-----------|--------------|------------|
| 01-build | `cd /home/ai/dev/sqlrustgo && cargo build -p sqlrustgo-mysql-server -p sqlrustgo-mysql-client --tests` | pass | 34dd202c2752661627eb0c270c70728e36123fd093085eba863e59b3dfb95f77 | /home/ai/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/01-build.log | 2026-08-09T08:32:34Z | minimax | minimax-v312-13-8ecb0ddf37 |
| 02-typed-wrappers | `cd /home/ai/dev/sqlrustgo && cargo test --test v312_13_typed_wrappers_test -- --test-threads=1` | pass | 36f4fdb34fa72180ef546f47f4b330a7cf43201339f48d23bf9149d0c8208bf2 | /home/ai/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/02-typed-wrappers.log | 2026-08-09T08:32:34Z | minimax | minimax-v312-13-8ecb0ddf37 |
| 03-wire-regression | `cd /home/ai/dev/sqlrustgo && cargo test --test mysql_wire_protocol_test -- --test-threads=1` | pass | 86fc9c4dbd83629285acd4cbad808922121087cd69dc3e4d997b24c53095fd6e | /home/ai/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/03-wire-regression.log | 2026-08-09T08:32:34Z | minimax | minimax-v312-13-8ecb0ddf37 |
| 04-prepared-statement-params | `cd /home/ai/dev/sqlrustgo && cargo test -p sqlrustgo-mysql-server --test prepared_stmt_params_test -- --test-threads=1` | pass | af16e305b62522725bace50615e8c2ea533b9d23dbb900ca649624315d1c84f2 | /home/ai/dev/sqlrustgo/docs/releases/v3.12.0/evidence/wire_load_data/04-prepared-statement-params.log | 2026-08-09T08:32:34Z | minimax | minimax-v312-13-8ecb0ddf37 |
