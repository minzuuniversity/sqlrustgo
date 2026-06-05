# TPC-H 22 Thread: 6-PR Completion Correction (Retrospective)

> **Author**: 李哥 hermes session 2026-06-05 (end-of-thread audit)
> **Date**: 2026-06-05 04:30 UTC+8
> **Purpose**: Correct the completion claims made by the 6-PR TPC-H 22
> thread (#3086, #3089, #3093, #3095, #3098, #3114) with what was
> actually delivered vs. what the thread's stated goals implied.
> **Scope**: Documentary; no code change. Cross-references the
> full evaluation in `docs/discovery/2026-06-05-tpch-22-eval-full.md`.

## 1. The thread's stated goal

**Issue #2977** (TPC-H 22-query wire round-trip): "在
`~/dev/yinglichina163/sqlrustgo` 通过 `sqlrustgo-mysql-server` binary
+ MySQL wire protocol 跑通 TPC-H 22 条 query。要求三方 DB 实际结果
**三方一致**才计入'通过'。"

The thread was scoped to the **wire protocol surface** (server +
MySQL client + LOAD DATA + 22 queries). Mid-Phase 0 this was relaxed
to "SQLite-only as the in-process reference" and later expanded to
include in-process engine coverage when wire proved blocked by a
server bug.

## 2. The 6 PRs and their real vs. claimed delivery

### PR #3086 — Phase 0 (SQLite baseline)

| Field | Value |
|---|---|
| Merged | ✓ `e97985454` |
| Head | `feature/tpch-22-wire-v2` |
| Commit | `57b1ab910` |
| Files | 24: 1 Python script + 22 JSON + 1 SPEC + 0 .rs |
| Claims | "22/22 expected row_counts generated" |
| Real | **22/22 row_counts from SQLite against sf001 fixture ✓** |
| Gap | None for Phase 0's scope. The 22 JSONs are accurate SQLite outputs. The "three-way" was always going to be the next phase, and PR #3086 explicitly noted MySQL/PG blockers. |

**Verdict**: **Delivered as advertised.** No correction needed.

### PR #3089 — Phase 1a (regression markers, 4 ignored)

| Field | Value |
|---|---|
| Merged | ✓ `17adfca06` |
| Head | `feature/tpch-22-bugfixes` |
| Commit | `37809c078` |
| Files | 1: `tests/tpch_bug_regression_test.rs` (255 lines, 6 tests) |
| Claims | "6 in-process regression tests added; 2/6 PASS, 4/6 FAIL (4 `#[ignore]`)" |
| Real | 4/6 fail as expected; CI green due to `#[ignore]`. **None of these tests verify the 22-query row_count correctness** — they verify 6 hand-picked bug cases from a 2026-06-04 audit. |
| Gap | PR title is honest ("regression markers — 4/6 fail intentional"). But the *thread context* implies these are "TPC-H 22 readiness tests," which is misleading. |

**Verdict**: **Delivered as advertised, but the test scope is much
narrower than the thread's TPC-H 22 name suggests.** No correction
to the PR itself, but the framing in the closure report could be
clearer.

### PR #3093 — Phase 1b (un-`#[ignore]` bug #4/#5)

| Field | Value |
|---|---|
| Merged | ✓ `cbb2bc007` |
| Head | `feature/tpch-22-bugfixes` |
| Commit | `7b8381f36` |
| Files | 1: `tests/tpch_bug_regression_test.rs` (modified) |
| Claims | "6/6 regression test PASS. **Root cause was fixture loader, not engine. 0 production code change needed.**" |
| Real | 6/6 PASS confirmed. The "5 bugs → 3 fixed + 2 fixture loader" finding is genuine and well-documented in the commit message. **0 production code change** is accurate. |
| Gap | None for what PR #3093 actually does. The misleading thing is the *upstream claim* "5 engine bugs 真阻塞 22/22" that PR #3089 and #3088 inherited — those are wrong, and the thread's title-sequence ("5 engine bugs 锁定 + 6 regression tests") is now known to be over-counted. |

**Verdict**: **Genuine and useful discovery.** The "0 production
code change" is the key signal — when a test fails, audit the
test's data loading path *first*. This lesson should propagate to
the next session as a SKILL.

### PR #3095 — Macmini Phase 3 (scalar subquery + aggregate division + derived tables)

| Field | Value |
|---|---|
| Merged | ✓ `8d1dafcfb` |
| Head | `feature/tpch-phase3-fix` |
| Commit | `fe8853579` (3 files: `parser.rs`, `engine_select.rs`, `parser/lib.rs`) |
| Claims | "18/22 TPC-H in-process + 3 new query types unblocked" (per Macmini's session paste) |
| Real | **The 18/22 number is "in-process doesn't crash", not "in-process row_count matches SQLite."** My re-audit at `acc8d5a90` (after PR #3098 also merged) shows the actual row_count-correct count is **10/22**, and 4 of those 10 are 0-coincidence matches. So even after Macmini's Phase 3 + 4, only **4/22 queries are demonstrably correct** (Q1, Q6, Q17, Q19). |
| Gap | **The 18/22 number is misleading.** The underlying test gate (`tpch_full_22_test`) is a "doesn't crash" gate using 1-character SQL prefixes, not a "row_count matches SQLite" gate. Macmini's session paste acknowledged this implicitly (Q8 parse ✓ but executor EXTRACT 未支持, etc.) but the PR title and the closure report kept saying "18/22" without qualifying it. |

**Verdict**: **Significant regression in completion-honesty
standards.** The 18/22 number was true only for "doesn't crash" and
should never have been reported as "TPC-H 18/22" in a way that
suggests results are correct.

### PR #3098 — Macmini Phase 4 (derived-table predicate isolation)

| Field | Value |
|---|---|
| Merged | ✓ `d48e440b4` |
| Head | `feature/tpch-phase3-fix` |
| Commit | `f1cd2f795` (1 file: `parser.rs` +52 lines) |
| Claims | "Q15 derived-table predicate isolation" |
| Real | **My re-audit at `acc8d5a90` shows Q15 still ERRs with "Unsupported join condition expression"** in `eval_22_vs_sqlite`. So Phase 4 *partially* fixed the predicate isolation (the `__subq_` skip and the JoinClause-no-ON skip are real changes), but Q15's full path still doesn't produce a result. |
| Gap | **Q15 is not actually fixed by Phase 4.** The PR title implies the feature works; the test surface for Q15 (`tpch_full_22_test` "doesn't crash") doesn't catch the executor error because the 1-char prefix test of "q" is too shallow. |

**Verdict**: **Partial fix, over-claimed in PR title.** Q15 is
still in the "engine error" bucket per `eval_22_vs_sqlite`.

### PR #3114 — Phase 2 closure (docs only)

| Field | Value |
|---|---|
| Merged | ✓ `acc8d5a90` |
| Head | `feature/tpch-22-bugfixes` |
| Commit | `c6b23c9f6` |
| Files | 3: 1 new discovery doc, 1 updated SPEC, `.gitignore` |
| Claims | "Phase 2 wire test deferred; server LOAD DATA EAGAIN bug discovered" |
| Real | **Accurate.** The wire test was written, verified to fail on `orders.tbl` (9 cols / 150 rows) and `lineitem.tbl` (16 cols / 614 rows), then the code was deliberately not committed (cleaner than committing then reverting). The discovery doc identifies 3 hypothesised root causes and points to the source line numbers. |
| Gap | None. **This is the most honest PR in the thread.** It says exactly what happened and what was deferred. |

**Verdict**: **Delivered as advertised and a model for honest
deferral.** Should be the template for future "we found a blocker,
stopping here" commits.

## 3. Net thread-level correction

The 6-PR thread's actual deliverable is:

| Claim | Reality |
|---|---|
| "TPC-H 22 queries running end-to-end" | **0/22 over wire, 22/22 "doesn't crash" in-process, 10/22 row_count matches SQLite (4/22 if you discount 0-coincidence matches).** |
| "5 engine bugs 真阻塞" | **0/5 — 3 already fixed before thread start, 2 were test fixture issues.** |
| "18/22 in-process" (Macmini) | **True only for "doesn't crash" gate; real row_count-correct count is 4/22.** |
| "Phase 2 wire test deferred" | **Honest, well-documented.** |
| "6/6 regression tests PASS" | **True. 6 hand-picked bug cases, not 22-query coverage.** |
| "0 production code change in Phase 1b" | **True and a useful discovery.** |

## 4. What the thread did right

- **5/6 PRs delivered code that compiles and tests pass.** No
  half-broken state was pushed.
- **`#3093` correctly identified the "5 bugs" were test fixture
  issues** and made the fix in 0 production code. This is a
  credit to the audit methodology.
- **`#3114` honestly documented the wire test blocker** rather
  than paper over it.
- **6 new regression tests landed in the codebase** that
  continue to catch regressions on the 6 hand-picked bug cases.
- **22 SQLite baseline JSONs are accurate** and reusable for
  future correctness gates.

## 5. What the thread did wrong

- **Completion claims were systematically optimistic.** "5 bugs"
  → "3 already fixed + 2 fixture". "18/22" → "10/22 row_count
  correct, 4/22 non-coincidental". The PR titles and commit
  messages reported the optimistic number without qualifying it.
- **The "22/22 PASS" gate was misleadingly structured.**
  `tpch_full_22_test` uses 1-character SQL prefixes and only
  asserts "doesn't crash + returns any result." A test named
  "22 queries" should be verifying the 22 queries' results.
- **No one wrote the test that would have caught the
  row_count-mismatch problem.** The 6 hand-picked tests in
  `tpch_bug_regression_test.rs` were always a sample, not a
  coverage, and the sample didn't include the queries that
  fail (Q3, Q4, Q5, Q10, Q12, Q13, Q14, Q16).
- **"Three-way reference" was silently downgraded to "SQLite
  only"** mid-Phase 0 with a clear note in the SPEC, but the
  thread's title-sequence ("TPC-H 22 wire round-trip") still
  implies a wire round-trip that never shipped.

## 6. Required corrections to external artifacts

1. **Issue #2977** should be re-titled from "TPC-H 22-query wire
   round-trip" to "TPC-H 22 query correctness: 4/22 verified
   correct, 12 remaining (4 engine errors + 8 row_count
   mismatches), wire protocol blocked by server LOAD DATA bug."
   (李哥: I can post this comment if you say go.)

2. **`docs/audit/status/2026-06-04-tpch-phase2d-status.md`**
   should be re-read with the knowledge that "5 engine bugs 真阻塞"
   was wrong; only Q1/SELECT projection was a real blocker, the
   rest were either already fixed or were test issues. (Not edited
   in this commit — that's a historical record. Future audits
   should not cite it as the source of truth for current bug
   state.)

3. **`docs/plans/2026-06-05-tpch-22-wire-three-way.md`** should
   be updated to add §9 "Phase 5 options" (A-E) for the next
   session. (Done in the follow-up commit to this discovery doc.)

4. **The `eval_22_vs_sqlite` test (this commit's
   `tests/eval_22_vs_sqlite.rs`)** should be promoted from
   diagnostic to a CI-enforced test as part of Option D
   (gate tightening) — at minimum, it should fail the build
   when row_count drops below the current 10/22, so any future
   change that breaks row_count for a previously-passing query
   gets caught immediately.

## 7. The lesson (for the SKILL)

The thread's failure mode is generic and worth memorialising:

**When a regression test exposes wrong results, audit the test
data path (loader / fixture / INSERT syntax / wire encoding)
*before* declaring an engine bug.** The 5 "engine bugs" in
`2026-06-04-tpch-phase2d-status.md` were 3 already-fixed + 2
test-fixture issues. If the audit had started at the test
loader, the thread would have skipped 2 phantom bugs and
refocused on Q1 correctness, which would have surfaced the
row_count vs SQLite baseline problem one PR earlier.

Stated as a heuristic: **a regression test that fails
post-merge is more often a test bug than an engine bug.**
Verify the data loading path produces the values the test
expects (correct type, correct precision, correct NULL/empty
semantics) before chasing engine internals.

This belongs in `local-repo-dirty-scan` or a new
`tpch-22-regression-audit` SKILL. 李哥: I can add it if
you say go.

## 8. Hand-off to next session

The next session that picks this up should:

1. **Read `docs/discovery/2026-06-05-tpch-22-eval-full.md`
   first** for the actual state of TPC-H 22.
2. **Pick one of A / B / C / D from the §9 options** in
   `docs/plans/2026-06-05-tpch-22-wire-three-way.md`
   (added in follow-up commit).
3. **Reuse the `eval_22_vs_sqlite` test** as the regression
   gate for any fix — every PR that claims a TPC-H 22 fix
   should be measured against the SQLite baseline, not
   against "doesn't crash."
