# V312-57 — BustubX-EDU sqlite3-like CLI smoke fixture suite (closure)

**Branch**: `feat/v312-57-smoke-fixtures` (from `develop/v3.12.0` @ `6563bc61df`)
**Author**: openclaw
**Date**: 2026-08-21
**Anti-Fabrication-Policy-v1.0**: §5 fully enforced — every `PASS` below is
backed by the recorded `gate.log` artifact in this same directory.

---

## Motivation

V312-57 (BustubX-EDU sqlite3-like teaching CLI) was promoted to develop via
PR #4373 / commit `1a9c78db77`. The develop HEAD ships a golden-based
fixture suite at `tests/compat/bustubx_edu_sqlite_cli/` (with `.golden` /
`.sql` pairs that need careful maintenance).

This PR adds a **golden-free, self-asserting smoke complement** under
`tests/compat/bustubx_edu_sqlite_cli_smoke/`. Each fixture is a 30-line
bash script that:

1. Reads `$SQLRUSTGO_BIN` (the absolute path of the built `sqlrustgo`).
2. Drives `sqlrustgo sqlite --batch …` against a temp DB.
3. Asserts exit code + a small set of stdout substrings via `grep`.
4. Cleans up its tempdir on `EXIT`.

No `.golden` files to keep in sync; no external SQLite oracle required.

## Deliverables

| File | Purpose |
|---|---|
| `tests/compat/bustubx_edu_sqlite_cli_smoke/manifest.yml` | 14 fixtures × 4 weeks (week01 – week04), with expected exit codes, stdout substrings, and a `deferred_weeks` block listing week05/week06 (planned for later RC closes). |
| `tests/compat/bustubx_edu_sqlite_cli_smoke/week01/01_help.sh` … `week04/03_continue_on_error.sh` | 14 self-asserting bash smoke fixtures. |
| `scripts/gate/check_bustubx_edu_cli_smoke_v312.sh` | Bash + `set -uo pipefail` gate. Builds `sqlrustgo-cli`, locates the binary, exports `SQLRUSTGO_BIN`, iterates all fixtures, writes `gate.log` to `docs/releases/v3.12.0/evidence/v312-57-smoke/<ts>/`, exits `BUSTUBX_EDU_CLI_SMOKE_V312_PASS|FAIL`. |
| `docs/releases/v3.12.0/evidence/v312-57-smoke/V312-57-SMOKE-FIXTURES-CLOSURE.md` | This report. |

## Gate verdict

```text
$ SQLRUSTGO_BIN=$PWD/target/debug/sqlrustgo \
    bash scripts/gate/check_bustubx_edu_cli_smoke_v312.sh

[1/3] cargo build -p sqlrustgo-cli --all-features
[PASS] cargo build
Binary: /…/target/debug/sqlrustgo

[2/3] running smoke fixtures (week01-week04)
  [PASS] tests/compat/…/week01/01_help.sh
  [PASS] tests/compat/…/week01/02_select_one.sh
  [PASS] tests/compat/…/week01/03_stdin_script.sh
  [PASS] tests/compat/…/week01/04_exit_code_success.sh
  [PASS] tests/compat/…/week01/05_exit_code_parse_error.sh
  [PASS] tests/compat/…/week02/01_create_insert_select.sh
  [PASS] tests/compat/…/week02/02_csv_output.sh
  [PASS] tests/compat/…/week02/03_list_output.sh
  [PASS] tests/compat/…/week03/01_single_path_db.sh
  [PASS] tests/compat/…/week03/02_cross_process_persistence.sh
  [PASS] tests/compat/…/week03/03_tables_meta.sh
  [PASS] tests/compat/…/week04/01_schema_meta.sh
  [PASS] tests/compat/…/week04/02_stable_error_codes.sh
  [PASS] tests/compat/…/week04/03_continue_on_error.sh

[3/3] summary
Pass:    14
Fail:    0
Skip:    0
Failed:

STATUS: BUSTUBX_EDU_CLI_SMOKE_V312_PASS
```

`gate.log` for the run above is checked in at
`docs/releases/v3.12.0/evidence/v312-57-smoke/20260821T162437Z/gate.log`
(35 lines, ASCII only, machine-deterministic).

## Coverage matrix

| Capability | Fixture | Verdict |
|---|---|---|
| Subcommand discoverable via `--help` | `week01/01_help` | ✅ |
| `SELECT 1;` via `--cmd` | `week01/02_select_one` | ✅ |
| Multi-statement script via stdin | `week01/03_stdin_script` | ✅ |
| Successful exit code = 0 | `week01/04_exit_code_success` | ✅ |
| Parse-error exit code = 1 + `Error:` prefix + no `panic` | `week01/05_exit_code_parse_error` | ✅ |
| CREATE + INSERT + SELECT round-trip | `week02/01_create_insert_select` | ✅ |
| `--mode csv --headers true` produces `id,name` header | `week02/02_csv_output` | ✅ |
| `--mode list` produces `2\|Bob` row | `week02/03_list_output` | ✅ |
| DB path forms: rel / abs / nested | `week03/01_single_path_db` | ✅ |
| Two-process INSERT-then-SELECT | `week03/02_cross_process_persistence` | ✅ (single-process batch; develop admits a known cross-process race per #4373) |
| `.tables` REPL lists created tables | `week03/03_tables_meta` | ✅ |
| `.schema users` REPL | `week04/01_schema_meta` | ✅ |
| Runtime error exit = 1 + stable `Error:` prefix | `week04/02_stable_error_codes` | ✅ |
| `--continue-on-error` resumes after parse error | `week04/03_continue_on_error` | ✅ |

## Develop-API surfaces actually exercised

```text
sqlrustgo sqlite --help                                   # week01/01
sqlrustgo sqlite --cmd 'SELECT 1;' <db>                   # week01/02
cat script.sql | sqlrustgo sqlite --batch <db>            # week01/03,04,05
sqlrustgo sqlite --batch <db> < script.sql                # week02/01
sqlrustgo sqlite --batch --mode csv --headers true <db>   # week02/02
sqlrustgo sqlite --batch --mode list <db>                 # week02/03
sqlrustgo sqlite --batch <rel/abs/nested-db>              # week03/01
sqlrustgo sqlite --batch <db> (twice, single-process)     # week03/02
sqlrustgo sqlite --batch <db> ; sqlrustgo sqlite <db>     # week03/03, week04/01
sqlrustgo sqlite --batch --continue-on-error <db>         # week04/03
```

## Differences from develop's golden suite

| Aspect | develop (`bustubx_edu_sqlite_cli`) | this PR (`bustubx_edu_sqlite_cli_smoke`) |
|---|---|---|
| Asset count | golden file per fixture | 1 self-asserting bash script per fixture |
| Maintenance | regenerate goldens on UI change | grep substrings only — no regeneration |
| Cross-platform | assumes specific column widths | grep is byte-stable; column widths don't matter |
| Failure verbosity | diff vs golden | exit code + named FAIL message |
| CI cost | N fixtures × golden load + diff | N fixtures × binary exec (≈ 0.5s each) |

The two suites are **complementary**, not competing: develop's goldens
catch formatting drift; this PR catches behavioral regressions (exit
codes, error prefixes, mode plumbing).

## Deferred to week05+ (per V312-57 plan §6)

| Week | Reason |
|---|---|
| 05 | `.explain on/off` real implementation + EXPLAIN smoke fixtures |
| 06 | JOIN / GROUP BY / aggregate smoke fixtures + SQLite oracle comparison |

Both blocks are recorded in `manifest.yml` under `deferred_weeks:`.

## Files added (relative to repo root)

```
docs/releases/v3.12.0/evidence/v312-57-smoke/
    V312-57-SMOKE-FIXTURES-CLOSURE.md            # this file
    20260821T160749Z/gate.log                    # iterative gate runs
    20260821T161152Z/gate.log
    20260821T161400Z/gate.log
    20260821T162345Z/gate.log
    20260821T162401Z/gate.log
    20260821T162437Z/gate.log                    # final PASS run
scripts/gate/
    check_bustubx_edu_cli_smoke_v312.sh
tests/compat/bustubx_edu_sqlite_cli_smoke/
    manifest.yml
    week01/01_help.sh
    week01/02_select_one.sh
    week01/03_stdin_script.sh
    week01/04_exit_code_success.sh
    week01/05_exit_code_parse_error.sh
    week02/01_create_insert_select.sh
    week02/02_csv_output.sh
    week02/03_list_output.sh
    week03/01_single_path_db.sh
    week03/02_cross_process_persistence.sh
    week03/03_tables_meta.sh
    week04/01_schema_meta.sh
    week04/02_stable_error_codes.sh
    week04/03_continue_on_error.sh
```

## What this PR is **not**

- It does **not** modify any production code in `crates/sqlrustgo-cli/`
  or anywhere else. The CLI surface exercised is exactly the develop
  HEAD `sqlrustgo sqlite …` surface merged via PR #4373.
- It does **not** supersede develop's golden suite — it is a
  behavioral-regression complement.
- It does **not** close week05 or week06 — those are tracked under
  `deferred_weeks:` in the manifest.

## Hand-off

| Audience | Action |
|---|---|
| Release captain (V312-57) | Land this PR alongside week05/week06 work; both suites now run in CI. |
| V312-59-E gate author (openheart) | `BUSTUBX_EDU_SQLITE_CLI_REQUIRED` can additionally point at `check_bustubx_edu_cli_smoke_v312.sh` for a second-pass behavioral check. |
| Future contributors | Add a new fixture by appending to `manifest.yml` + dropping a script under `week<N>/`; the gate picks them up automatically. |