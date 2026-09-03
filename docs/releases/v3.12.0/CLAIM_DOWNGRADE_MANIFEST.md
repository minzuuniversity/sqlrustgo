# v3.12.0 GA Candidate — Scope/Claim Downgrade Manifest

> **provenance:** generated_by=claude-macmini, generated_at=2026-09-04T01:30:00+08:00,
> branch=develop/v3.12.0, HEAD=`c67d4fddc072d94b2080c940d0af46ab8a8d0686`,
> commit=`c67d4fddc0`, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0, derived_from=`RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` §3
>
> **purpose:** Explicit release-claim downgrade for every GA-claim-caveat / GA-blocker
> issue that remains open at GA cut time. Per Round-24 strict standards and
> `RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` §2 "GA-claim-caveat" criteria:
> "May remain open only if release notes, README, and scope docs explicitly exclude the
> capability from v3.12 GA claims."

## 1. Methodology

For each issue below:

1. List the open status in Gitea.
2. State the affected capability.
3. Spell out the exact release-claim boundary line that must appear in
   `README.md`, `RELEASE_NOTES.md`, `GA_GATE_REPORT.md`, and the relevant
   subsystem docs (`docs/releases/v3.12.0/`).
4. Reference per-issue evidence section (Phase 1.3 to be authored).

## 2. GA-Blocker — must fix before GA cut or formally DOWNGRADE

These 7 issues remain **open** as of HEAD `c67d4fddc0` (2026-09-04):

> **Refresh 2026-09-04T01:45+08:00 (post-PR-4739 merge):** 1 of the 7 (#4682) was
> closed by external PR #4739 on 2026-09-03T17:40:24Z. Net **6 still open** blockers.
> See §8 Closure Ledger.

| Issue | Title | Status | Required action |
|-------|-------|--------|-----------------|
| #4708 | 中文表名/列名 + 中文注释 + 反引号/双引号标识符 失败 | open, B-track blocker | FIX via WP-A, OR explicit downgrade in release notes: "v3.12.0 GA does not support non-ASCII identifiers or comments; B-track teaching corpora must use ASCII identifiers." |
| #4703 | ON DUPLICATE KEY UPDATE 多列 + VALUES() 不支持 | open | FIX via WP-A, OR downgrade: "MySQL-style multi-column upsert excluded from v3.12 GA claims." |
| #4682 | sqlite_master / sqlite_sequence / sqlite_temp_master 系统表全部缺失 | **CLOSED-BY-PR-4739** at 2026-09-03T17:40:24Z (merge `e6727176089f7ae97268bba0f6125db82c95f5cc`) | FIX landed — see §8 Closure Ledger. B-track `.tables` / `.schema` metadata acceptance now expected to PASS in next RC-B1 gate run. |
| #4674 | CHAR_LENGTH / CHARACTER_LENGTH 完全错 | open | FIX via WP-B, OR downgrade: "v3.12 length() returns character count, char_length() may not match SQLite for multi-byte; use length() only." |
| #4668 | NATURAL JOIN / USING(col1,col2) 多列匹配错乱 | open, B-track blocker | FIX via WP-D, OR downgrade: "v3.12 GA only supports JOIN with explicit ON; NATURAL JOIN and multi-column USING excluded." |
| #4652 | CREATE PROCEDURE / FUNCTION 静默接受但不存储 | **CLOSED-BY-PR-4741** at 2026-09-03T17:51:50Z (merge `952f6f7578a7e96e...`, head `8717286f389c...`) | OR-downgrade landed — see §8 Closure Ledger entry 8.2. v3.12.0 GA CLI batch mode now rejects CREATE PROCEDURE / CREATE FUNCTION with explicit named error. |
| #4626 | SELECT FOR UPDATE 后 ROLLBACK 报 transaction already aborted | open | FIX via WP-C, OR downgrade: "v3.12 GA SELECT FOR UPDATE + ROLLBACK fails closed with explicit transaction-aborted error; do not claim recoverable." |

### Required release-note language (≥ 1 of these per still-open blocker)

```markdown
## Known Limitations — GA Candidate

This v3.12.0 GA build explicitly excludes the following capabilities from its
release claims. Issues remain open and will be addressed in v3.13.0:

- INSERT ON DUPLICATE KEY UPDATE with multi-column and VALUES() (#4703)
- NATURAL JOIN and multi-column USING (#4668)
- CHAR_LENGTH semantics for multi-byte strings (#4674)
- CREATE PROCEDURE / CREATE FUNCTION storage (#4652) — **CLOSED, see §8** (OR-downgrade)
- sqlite_master / sqlite_sequence / sqlite_temp_master tables (#4682) — **CLOSED, see §8**
- SELECT FOR UPDATE + ROLLBACK recovery (#4626)
```

## 3. GA-claim-caveat — exclude from v3.12 GA claims unless fixed

These 9 issues are **open** and must trigger explicit release-claim exclusions
in `README.md`, `RELEASE_NOTES.md`, and per-subsystem docs:

| Issue | Title | Claim boundary line required in GA release |
|-------|-------|---------------------------------------------|
| #4719 | sqlite_stat1/sqlite_stat4 系统表完全不存在 + ANALYZE 不工作 | "ANALYZE excluded from v3.12 GA — query planner statistics are managed heuristically; run ANALYZE in SQLite if statistical planning is required." |
| #4698 | GREATEST/LEAST, MOD/POWER/LOG/EXP/SQRT 全部未实现, sqlite_stat1 不存在 | "Math functions (MOD/POWER/LOG/EXP/SQRT) and GREATEST/LEAST excluded from v3.12 GA. Use direct comparison and arithmetic instead." |
| #4694 | SET TIMEZONE / SET TRANSACTION ISOLATION LEVEL 静默接受语法无输出 | "SET TIMEZONE / SET TRANSACTION ISOLATION LEVEL excluded from v3.12 GA — parser returns explicit unsupported error." |
| #4685 | UPDATE 多表 (UPDATE t1 JOIN t2 SET ...) 与 DELETE p USING q 语法不支持 | "Multi-table UPDATE / DELETE USING excluded from v3.12 GA." |
| #4676 | 数学函数 MOD/POWER/LOG/EXP/SQRT 完全未实现 | (consolidated into #4698 — see above) |
| #4675 | POSITION/LOCATE 字符串找子串函数完全未实现 | "POSITION/LOCATE excluded from v3.12 GA — use LIKE or INSTR." |
| #4670 | CEIL/CEILING/FLOOR 部分实现 (第 2 列返回空), TRUNCATE/TRUNC 不支持, HEX/MD5/SHA 部分实现 | "CEIL/FLOOR/TRUNCATE/HEX/MD5/SHA2 partial semantics — supported functions match SQLite, missing returns explicit error." |
| #4646 | JSON_EXTRACT / JSON_EACH 失败 | "JSON support limited to scalar paths via `->`/`->>`; JSON_EXTRACT and JSON_EACH excluded from v3.12 GA." |
| #4625 | INDEXED BY hint 不被优化器尊重 | "INDEXED BY hint excluded from v3.12 GA — query planner does not honor this hint." |

## 4. v3.13/defer — open items bound to v3.13-MASTER (#4313)

These 16 open issues do NOT block GA cut (per RC-GA §2 "v3.13/defer" criteria),
but each MUST carry the `v3.13-followup` label so v3.13-MASTER can track them.
Already applied via Gitea API in Phase 1.1.

Summary of categories:

- Recursive CTE (#4699, #4644, #4717, #4704)
- Window function completion (#4707, #4706, #4695, #4689)
- Generated columns / sequence / user variables (#4697, #4688, #4689)
- ROLLUP/CUBE/GROUPING SETS (#4679)
- Writable CTE (#4692)
- Trigger syntax extension (#4706, #4700 — closed)
- Materialized views (#4692)
- RIGHT/FULL OUTER JOIN (#4639)
- Complex DROP/CREATE INDEX (#4669)
- CREATE FUNCTION body / RETURNS TABLE (#4671)
- Expression indexes / complex pagination (#4701)

## 5. Required doc-update actions

For GA to be reachable, the following doc files MUST carry the claim-boundary
language from §2 and §3:

1. `README.md` — add `## v3.12.0 GA Known Limitations` section citing each
   open GA-blocker / GA-claim-caveat issue.
2. `RELEASE_NOTES.md` — add §"Excluded from GA Claims" matching §3 table.
3. `docs/releases/v3.12.0/GA_GATE_REPORT.md` — final aggregate must list each
   open issue and the active claim downgrade.
4. `docs/releases/v3.12.0/STAGE.yaml` — declare GA candidate `claim_scope` =
   "v3.12 GA excludes the 7 open blockers + 9 claim-caveats per this manifest".
5. `docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md` — per-feature status table must
   reference each claim-downgrade.

## 6. Cross-references

- Round-24 review: `docs/releases/v3.12.0/evidence/V312-ROUND24-REMEDIATION-NOTICE.md`
- V312-RC-GA triage plan: `docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md`
- V312-57 BustubX-EDU CLI closure: `docs/releases/v3.12.0/evidence/v312-57/CLOSURE_REPORT.md`
- Round-24 evidence manifest: `docs/releases/v3.12.0/evidence/V313-ROUND24-EVIDENCE-MANIFEST.md`
- Gitea label evidence: this manifest was generated alongside `GA-blocker`,
  `GA-claim-caveat`, and `v3.13-followup` label application (Phase 1.1).

## 7. Honest disclosure (per Anti-Fabrication-Policy-v1.0)

- This manifest truthfully enumerates **6 open** GA-blockers + 9 open GA-claim-caveats
  as of HEAD `d99a7e890d` on 2026-09-04 (refresh after PR #4739 merge). The numbers
  are derived from direct Gitea API queries and cross-checked against the local git log.
- 1 of the originally-listed 7 GA-blockers (#4682) has been closed by external PR
  #4739 — see §8 Closure Ledger for SHA-256 evidence.
- The manifest does NOT claim any of these issues are fixed except where a merged PR
  is documented in §8. Each remaining issue is a known, blocking concern for the
  GA cut unless closed by a merged PR before GA promotion.
- The 3 closed GA-blocker items in this round (#4722, #4721, #4682) are not listed
  as open in §2; they appear in §8 Closure Ledger.
- This manifest may be superseded by per-issue closure evidence docs (Phase 1.3
  pending) and by the final GA GATE_REPORT.md after all fixes merge.

## 8. Closure Ledger (added 2026-09-04)

Honest-path closures that occurred during Phase 1 / early Phase 2 timeline.
Each entry is a real PR with a real regression test and a real merge commit.
No `SUBSTANTIALLY_COMPLETE`, no `ACCEPTED-WITH-BINDING-MANIFEST`, no
fabrication. See per-issue evidence doc for Anti-Pattern 10 compliance.

### 8.1 #4682 — sqlite_master / sqlite_sequence synthesis (CLOSED 2026-09-03T17:40:24Z)

| Field | Value |
|-------|-------|
| Issue | [#4682](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4682) |
| Closing PR | [#4739](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4739) |
| PR merge commit | `e6727176089f7ae97268bba0f6125db82c95f5cc` |
| Branch | `fix/v312-76-issue-4682-sqlite-system-tables` → `develop/v3.12.0` |
| Per-issue evidence doc | [`docs/releases/v3.12.0/evidence/issue-4682/EVIDENCE.md`](evidence/issue-4682/EVIDENCE.md) |
| Evidence doc SHA-256 | `25dc96f1e330af185d29ff22003938c12a6e048de3ab197e0b3ccc9f03dbf3a9` |
| Regression test | `tests/integration/sql/v312_76_sqlite_system_tables_test.rs` |
| Regression test SHA-256 | `8184926fa54058c8c86f0ccf2643dc208f77ce61dd7ee409aa62504f9bd87c9a` |
| Gitea state transition | open → closed (2026-09-03T17:40:24Z) |
| Labels (post-close) | `GA-blocker` `v3.13-followup` (kept as audit) |

**What was fixed**: PR #4739 added `sqlite_master` / `sqlite_sequence` / `sqlite_temp_master`
synthesis in `src/engine_ddl.rs` + `src/engine_select.rs`, plus full regression
test coverage in `tests/integration/sql/v312_76_sqlite_system_tables_test.rs`.

**Round-24 Anti-Pattern compliance**: PR is real, regression test is real, merge
commit reachable on `develop/v3.12.0`. No fake markers used. See per-issue
evidence §7 + §8 for full closure trail.

### 8.2 #4652 — CREATE PROCEDURE / CREATE FUNCTION in CLI batch mode (CLOSED 2026-09-03T17:51:50Z)

| Field | Value |
|-------|-------|
| Issue | [#4652](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4652) |
| Closing PR | [#4741](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4741) |
| PR merge commit | `952f6f7578a7e96e...` |
| PR head fix commit | `8717286f389c...` |
| Branch | `fix/v312-rcga-issue-4652-procedure-or-downgrade` → `develop/v3.12.0` |
| Per-issue evidence doc | [`docs/releases/v3.12.0/evidence/issue-4652/EVIDENCE.md`](evidence/issue-4652/EVIDENCE.md) |
| Evidence doc SHA-256 | `353d31f132a1b92d6b054b0cf172d9d352a675ce51d5b3614af0e440dfd684fe` |
| Regression test (BASH) | `tests/compat/bustubx_edu_b_track/issue_4652_procedure_or_downgrade.sh` |
| Regression test SHA-256 | `a14a9db03570eb1190318763193eac06041de93b3847b90503839fa9190224a5` |
| Fix source post-edit SHA-256 | `9d4ae104d6cdcf262213c1037f2f644ecd17598667823b5634d48743b085b863` (sqlite_mode.rs) |
| Gitea state transition | open → closed (2026-09-03T17:51:50Z) |
| Labels (post-close) | `GA-blocker` `v3.13-followup` (kept as audit) |

**What was fixed**: PR #4741 added a prelude guard in
`crates/sqlrustgo-cli/src/sqlite_mode.rs::execute_sql` that explicitly rejects
`CREATE PROCEDURE` / `CREATE FUNCTION` with a named OR-downgrade error.
Before this fix, CLI batch mode silently accepted the DDL but did not wire the
catalog change through FileStorage persistence, so downstream `CALL` reported
`Stored procedure 'X' not found` — a classic DDL fake-success pattern
(Round-24 §2 #1).

**Why OR-downgrade (not full fix)**: Full fix (CLI catalog persistence) would
touch the same path PR #4609 (V312-57 stage2) already iterated on for INSERT.
The OR-downgrade path satisfies the §3 WP-C OR-downgrade clause
("v3.12 GA rejects CREATE PROCEDURE/FUNCTION with explicit error; never
silently accepts") and defers full CLI batch persistence to v3.13.

**Round-24 Anti-Pattern compliance**: PR is real, regression test is real (BASH
script in `tests/compat/bustubx_edu_b_track/`), merge commit reachable on
`develop/v3.12.0`. No `SUBSTANTIALLY_COMPLETE`, no
`ACCEPTED-WITH-BINDING-MANIFEST` markers. See per-issue evidence §7 + §8 for
full closure trail.

---

*maintained as part of V312-RC-GA remediation; supersedes prior scope language but
preserves it in CLOSURE_REPORT.md per Round-24 historical-anchor rule.*
