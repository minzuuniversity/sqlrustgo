# SQLRustGo v3.12.0 RC-GA Issue Triage and Gate Plan

> **provenance:** generated_by=codex-cli, generated_at=2026-09-03T13:45:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, head=`d8e4ebc71c8ec3bd7272d5b35e19f33758e98c18`, gitea=`http://192.168.0.252:3000/openclaw/sqlrustgo`, policy=Anti-Fabrication-Policy-v1.0
> **scope:** RC-to-GA total-control issue plan for BustubX-EDU B-track findings.
> **stage:** RC / GA candidate preparation. This document does not promote v3.12.0 to GA.

## 1. Current Facts

Live checks on 2026-09-03 show:

- `origin/develop/v3.12.0` HEAD: `d8e4ebc71c`.
- Gitea milestone `v3.12.0`: `state=open`, `open_issues=0`, `closed_issues=79`.
- Repository open issue page returned 50 open issues outside the milestone.
- Open issue rough split: parser 26, executor 20, parser/executor 2, parser/planner 1.
- `docs/releases/v3.12.0/evidence/v312-59/ga_gate_report.json` says `mode=full`, `verdict=FAIL`, `totals.blockers=7`.
- Existing GA-only scripts still include fast-path/syntax-only checks. They are not sufficient as final GA evidence.

## 2. Triage Policy

### GA-blocker

Must be fixed before GA, or the release must explicitly remove/downgrade the
claim and provide a governance reclassification.

Criteria:

- Produces silently wrong data.
- Accepts SQL/DDL but does not execute the promised side effect.
- Breaks BustubX-EDU B-track core scripts that v3.12 claims to support.
- Violates data integrity or transaction correctness.
- Causes parser/executor to turn a relational operator into a materially
  different result, such as Cartesian product instead of join.

### GA-claim-caveat

May remain open only if release notes, README, and scope docs explicitly exclude
the capability from v3.12 GA claims.

Criteria:

- Advanced SQL dialect compatibility beyond the verified v3.12 core.
- Useful for teaching or migration, but not required for the declared GMP
  internal-audit database subset.
- Has visible error/unsupported behavior rather than silent corruption.

### v3.13/defer

Can move to v3.13 only with issue owner, rationale, expiry, and release-claim
downgrade.

Criteria:

- Requires broader architectural work such as full recursive CTE, complete
  window frame engine, materialized views, generated columns, or stored
  procedure language runtime.
- Not part of the v3.12 GA public claim after scope downgrade.

## 3. Initial Issue Classification

### GA-blocker: fix before GA or formally reclassify

| Issue | Reason |
|---|---|
| #4722 | Regression: PR #4634 CHAR length check breaks teaching seed load. B-track core data cannot load. |
| #4721 | `round(real, int)` still returns integer; type fidelity bug after prior fix batch. |
| #4709 | CHECK multi-condition constraint silently accepts invalid rows; data integrity failure. |
| #4708 | Chinese identifiers/comments and quoted identifiers fail or produce empty output; B-track scripts with Chinese schema/comments are blocked. |
| #4703 | Upsert and trigger-column syntax failures affect declared compatibility if accepted paths no-op or misparse. |
| #4696 | `UPDATE` without WHERE parse failure blocks normal SQL DML; must either support or document restriction. |
| #4682 | `sqlite_master` / `sqlite_sequence` / `sqlite_temp_master` missing; blocks sqlite-style metadata expectations if claimed. |
| #4674 | `CHAR_LENGTH` / `CHARACTER_LENGTH` returns wrong values; string semantics wrong. |
| #4672 | SQLite `AUTOINCREMENT` still not working; B-track insert/id workflows blocked. |
| #4668 | NATURAL JOIN / multi-column USING returns wrong result shape; relational correctness failure. |
| #4656 | `> ALL` / `= ANY` subquery returns empty result; semantic correctness failure. |
| #4652 | CREATE PROCEDURE/FUNCTION accepted but not stored; DDL fake-success pattern. |
| #4649 | LEFT JOIN USING degrades to Cartesian product; severe wrong-result bug. |
| #4636 | Correlated scalar subquery fails; common teaching/reporting query pattern. |
| #4626 | SELECT FOR UPDATE then ROLLBACK abort behavior may break transaction correctness. |

### GA-claim-caveat: exclude from v3.12 GA claims unless fixed

| Issue | Required claim boundary |
|---|---|
| #4720 | MySQL user variables are not part of v3.12 core unless implemented. |
| #4719 | sqlite statistics tables / ANALYZE not claimed unless implemented. |
| #4716 | `DATE_TRUNC` partial implementation must be excluded from PostgreSQL compatibility claims. |
| #4711 | Advanced `ON CONFLICT` forms excluded unless parser/executor support lands. |
| #4710 | `TIMESTAMPDIFF(unit, ...)` excluded from date-function compatibility unless fixed. |
| #4698 | `GREATEST/LEAST`, math functions, stats tables, DEFAULT expressions, ANALYZE excluded unless fixed. |
| #4694 | SET TIMEZONE / isolation syntax excluded unless implemented or rejected explicitly. |
| #4693 | TEMP/TEMPORARY table syntax excluded unless supported. |
| #4685 | Multi-table UPDATE / DELETE USING excluded. |
| #4677 | TRUNCATE and complex LIKE ESCAPE behavior excluded unless fixed. |
| #4676 | MOD/POWER/LOG/EXP/SQRT excluded unless implemented. |
| #4675 | POSITION/LOCATE excluded unless implemented. |
| #4670 | CEIL/FLOOR/TRUNCATE/HEX/MD5/SHA partial function support excluded unless fixed. |
| #4667 | ORDER BY NULLS FIRST/LAST excluded. |
| #4650 | GROUP_CONCAT and related aggregate semantics excluded unless fixed and covered by oracle tests. |
| #4646 | JSON_EXTRACT / JSON_EACH excluded unless v3.12 JSON subset is updated and verified. |
| #4625 | INDEXED BY optimizer hint excluded from performance/optimizer claims unless implemented. |

### v3.13/defer candidates

| Issue | Reason |
|---|---|
| #4717 | INSERT with recursive CTE requires broader CTE/DML integration. |
| #4707 | LEAD/LAG/FIRST_VALUE/LAST_VALUE require complete window-function execution semantics. |
| #4706 | Complex subquery + window + CAST depends on window frame and nested parser work. |
| #4705 | FOR EACH STATEMENT trigger support is larger trigger runtime scope. |
| #4704 | VALUES in recursive CTE, sqlite_sequence, multi-anchor UNION should follow recursive CTE/system table design. |
| #4701 | Expression indexes and complex subquery pagination require planner/index design. |
| #4700 | UPDATE OF trigger syntax belongs with trigger runtime completion. |
| #4699 | Recursive CTE execution is explicitly not implemented. |
| #4697 | Generated columns require catalog/storage/executor expression persistence design. |
| #4695 | INTERVAL syntax and LAG overlap date dialect + window engine work. |
| #4692 | Writable CTE and materialized view support require broader planner/executor design. |
| #4689 | User variables plus ROWS/RANGE frame definitions span MySQL session state and window engine. |
| #4688 | CREATE SEQUENCE needs sequence catalog and transaction semantics. |
| #4679 | ROLLUP/CUBE/GROUPING SETS require grouping-set planner/executor support. |
| #4671 | CREATE FUNCTION multi-statement body / RETURNS TABLE requires procedure language runtime. |
| #4669 | Complex DROP INDEX / function index / partial index should follow index DDL design. |
| #4644 | Recursive CTE parser/executor is larger v3.13 class. |
| #4639 | RIGHT/FULL OUTER JOIN can defer if release claims only INNER/LEFT core joins. |

## 4. GA Before-Cut Remediation Requirements

Before v3.12.0 can move from RC to GA:

1. Every GA-blocker issue above is closed by a merged PR with regression tests,
   or reclassified with explicit release-claim downgrade approved by the user.
2. No accepted SQL statement may silently no-op for v3.12 claimed features.
   Parser should return unsupported errors for out-of-scope SQL instead.
3. BustubX-EDU B-track core seed and exercise corpus must run as a real gate,
   not only as a hand-run report.
4. GA aggregate must be regenerated at final HEAD and must not rely on
   script-existence or `bash -n` syntax checks for semantic claims.
5. GA-2 SOAK must be closed with Linux/Docker evidence or formally reclassified.
6. Security scan and docs consistency must be refreshed at final HEAD.

## 5. New RC Gate Requirements

The RC gate must be re-opened as an RC-GA quality gate because the B-track
findings show that prior RC fixtures were too narrow.

Required new checks:

| Gate | Required command | Acceptance |
|---|---|---|
| RC-B1 B-track oracle corpus | `bash scripts/gate/check_bustubx_b_track_v312.sh` | Core B-track seed + exercises pass against SQLite oracle or issue-linked exclusion. |
| RC-B2 Parser real-script gate | `bash scripts/gate/check_v312_parser_real_scripts.sh` | Multi-line DDL, standalone comments, quoted identifiers, Chinese comments/identifiers, basic DML parse. |
| RC-B3 Semantic no-op guard | `bash scripts/gate/check_v312_no_silent_success.sh` | Accepted DDL/DML has observable postcondition; unsupported SQL returns explicit error. |
| RC-B4 Type/function conformance | `bash scripts/gate/check_v312_type_function_semantics.sh` | Float, round, length/char_length, math/date/string core functions match oracle. |
| RC-B5 Join/subquery correctness | `bash scripts/gate/check_v312_join_subquery_semantics.sh` | JOIN USING/NATURAL JOIN, scalar subquery, ANY/ALL, HAVING multi-condition match oracle. |
| RC-B6 DML/integrity correctness | `bash scripts/gate/check_v312_dml_integrity.sh` | CHECK, AUTO_INCREMENT/AUTOINCREMENT, RETURNING, UPDATE without WHERE are correct or scoped out. |
| RC-B7 GA aggregate strictness | `bash scripts/gate/check_ga_v3.12.0.sh --full` | No fast-path/syntax-only PASS for semantic gates; JSON verdict PASS with zero blockers. |

## 6. Test Design Rules

1. Prefer oracle comparison to golden-only output for B-track SQL.
2. Every bug fix must add a failing regression test first, then a passing test
   after implementation.
3. For parser bugs, include both positive valid SQL and negative unsupported
   SQL assertions.
4. For DDL, test the side effect after creation, not just exit code.
5. For DML and expressions, assert row count, value, type, and exit code.
6. For every exclusion, require issue number, owner, scope downgrade, and close
   boundary.
7. CI/gate reports must include command, exit code, timestamp, commit, and
   evidence hash.

## 7. Proposed Work Packages

| Package | Scope | Target |
|---|---|---|
| WP-A | Parser real-script compatibility | #4708, #4696, #4710, selected quoted/comment cases |
| WP-B | Type/function correctness | #4721, #4674, #4676, #4675, #4716 |
| WP-C | DDL/DML no-op and integrity | #4709, #4652, #4672, #4682 |
| WP-D | Join/subquery correctness | #4649, #4668, #4656, #4636 |
| WP-E | Gate hardening | RC-B1 through RC-B7 scripts and evidence docs |
| WP-F | Claim downgrade/defer | All GA-claim-caveat and v3.13/defer rows |

## 8. Immediate Next Actions

1. Create the RC-GA total-control Gitea issue and link this document.
2. Add labels or comments for `GA-blocker`, `GA-claim-caveat`, and `v3.13/defer`.
3. Implement RC-B1 through RC-B7 gate scripts.
4. Run the B-track oracle corpus and generate the first failing evidence report.
5. Fix GA-blockers in small PR batches, with regression tests and issue closure
   only after merged PR evidence exists.
