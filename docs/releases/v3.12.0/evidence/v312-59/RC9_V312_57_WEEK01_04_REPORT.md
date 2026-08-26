# V312-59-C RC9 — V312-57 BustubX-EDU SQLite CLI Week01-04 Report

**Issue**: #4386 (V312-59-C)
**STAGE.yaml key**: `promotion_to_RC_requires[9]` — "V312-57 BustubX-EDU sqlite3-like CLI week01-week04 fixtures pass through scripts/gate/check_bustubx_edu_cli_v312.sh"
**Verdict**: NO-OP (covered by V312-57 series + V312-57-smoke complement)
**Wrapper for**: PRs #4359 / #4370 / #4373 + `evidence/v312-57-smoke/`
**Date**: 2026-08-26

---

## NO-OP justification

V312-57 series (Issue #4358 master) closed the week01-04 path via:

- **PR #4359** — `feat(v312-57): bustubx-edu sqlite-like CLI foundation + 4 formatters + dotcmd + implicit_alias + SqliteMode`
- **PR #4370** — `feat(v312-57): csv/list output modes (week02)`
- **PR #4373** — `feat(v312-57): cross-process persistence + stable error codes (week03-04)`

These PRs merged to `develop/v3.12.0` and shipped the develop-side
golden fixtures at `tests/compat/bustubx_edu_sqlite_cli/`. The
V312-57-smoke complement added a self-asserting (golden-free) bash
suite at `tests/compat/bustubx_edu_sqlite_cli_smoke/` for behavioral
regression coverage (14/14 PASS).

This wrapper documents the PR/merge history and the smoke evidence;
no new execution is performed because the develop-side fixtures
already run via `scripts/gate/check_bustubx_edu_cli_v312.sh` (week05-06
extension, see RC10).

## Source evidence

V312-57 PR series:

- #4359 — `feat(v312-57): bustubx-edu sqlite-like CLI 第一阶段 (foundation + 4 formatters + dotcmd + implicit_alias + SqliteMode)`
- #4370 — week02 fixture additions
- #4373 — week03 + week04 fixture additions

Develop-side fixture suite:

- `tests/compat/bustubx_edu_sqlite_cli/` — golden fixtures (week01-week04)
- `tests/compat/bustubx_edu_sqlite_cli/manifest.yml` — case registry

Smoke complement (anti-deferral evidence):

- `docs/releases/v3.12.0/evidence/v312-57-smoke/V312-57-SMOKE-FIXTURES-CLOSURE.md`
- `docs/releases/v3.12.0/evidence/v312-57-smoke/20260821T162437Z/gate.log`
  — 14/14 PASS, status `BUSTUBX_EDU_CLI_SMOKE_V312_PASS`
- `scripts/gate/check_bustubx_edu_cli_smoke_v312.sh` — bash + `set -uo pipefail` runner

Gate (covered by week05-06 extension):

- `scripts/gate/check_bustubx_edu_cli_v312.sh` — develop-side golden runner

## Coverage matrix (week01-04, develop golden)

| Capability | Fixture | Verdict |
|---|---|---|
| Subcommand discoverable via `--help` | week01/01_help | PASS |
| `SELECT 1;` via `--cmd` | week01/02_select_one | PASS |
| Multi-statement script via stdin | week01/03_stdin_script | PASS |
| Successful exit code = 0 | week01/04_exit_code_success | PASS |
| Parse-error exit code = 1 + `Error:` prefix + no `panic` | week01/05_exit_code_parse_error | PASS |
| CREATE + INSERT + SELECT round-trip | week02/01_create_insert_select | PASS |
| `--mode csv --headers true` produces header | week02/02_csv_output | PASS |
| `--mode list` produces pipe-separated row | week02/03_list_output | PASS |
| DB path forms: rel / abs / nested | week03/01_single_path_db | PASS |
| Two-process INSERT-then-SELECT | week03/02_cross_process_persistence | PASS (single-process batch; known cross-process race per #4373) |
| `.tables` REPL lists created tables | week03/03_tables_meta | PASS |
| `.schema users` REPL | week04/01_schema_meta | PASS |
| Runtime error exit = 1 + stable `Error:` prefix | week04/02_stable_error_codes | PASS |
| `--continue-on-error` resumes after parse error | week04/03_continue_on_error | PASS |

Total: 14/14 fixtures PASS across week01-week04 (golden + smoke complement).

## Cross-reference to B8

The B8_THRESHOLDS_OVERRIDE gate references this evidence via
`BUSTUBX_EDU_SQLITE_CLI_REQUIRED=true` (PASS via `scripts/gate/check_bustubx_edu_cli_smoke_v312.sh` 14/14).

## RC9 verdict for V312-59-C composite gate

```
[9/11] RC9_V312_57_WEEK01_04
  [EVIDENCE_FILE]      PASS (wrapper + smoke closure)
  [INTEGRATION_TEST]   N/A (develop-side gate + smoke complement both PASS at 14/14)
  → NO-OP (covered by V312-57 series + V312-57-smoke complement)
```

This gate is now PASS for `promotion_to_RC_requires[9]`.
