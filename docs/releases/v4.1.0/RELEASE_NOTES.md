# SQLRustGo v4.1.0 — Release Notes (DRAFT)

> **基线**：`develop/v4.1.0` @ `c1a73a5320a500ce2aa4e7fc16ebd59a802824af`
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`) — **NOT YET ALPHA**
> **Branch**: `develop/v4.1.0`
> **Base**: v4.0.0 GA **已被撤销** (2026-09-30, verdict: REVOKED，见
> `docs/releases/v4.0.0/STAGE.yaml` `ga_promotion_status` + `GA_RELEASE_TIMELINE.md`)
> **Scope change vs v4.0.0**: bugfix carry-forward + 168h SOAK FINAL + WP-A..G migration
> **本文件是 DRAFT 阶段发布说明草稿**，未到 ALPHA 不得对外宣发。完整缺陷目录见
> **`docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md`**（P0×6 + P1×7 + P2×7）。

Per `docs/releases/v4.1.0/STAGE.yaml`, v4.1.0 is in **DRAFT**. v4.0.0 is **not**
replaced; v4.0.0 remains GA on its release branch.
>
> ⚠️ **2026-09-30 更正**：本文件原写 `Status: ALPHA (released 2026-09-29,
> tag v4.1.0-alpha1)`，称 v4.1.0 已推进至 ALPHA。该 ALPHA 声明经实跑复核
> 不成立 —— `check_anti_fabrication.sh` 实跑 exit 1，属
> `[AFP-VIOLATION: Type-B 伪门禁]`，已按 AFP §7.1 回退为 DRAFT。
> 详见 `docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` 与
> `docs/governance/incidents/2026-09-30-V410-ALPHA-UNSUPPORTED-BY-GATE-EVIDENCE.md`。
> 另注：annotated tag `v4.1.0-alpha1` 实际**已存在**且在 develop/v4.1.0 历史中，
> 该 tag 的处置待治理决策（见 STAGE.yaml 顶部说明）。

## 1. What is v4.1.0?

v4.1.0 is the **post-v4.0.0 maintenance continuation** of SQLRustGo. It is
the active development trunk for v4.x.

v4.1.0 carries v4.0.0 forward through:

1. **Real bugfix carry-forward** (already landed on develop/v4.1.0 HEAD):
   - V400-05/06/07 cross-model transaction + AuditChain ALCOA+
   - zombie-fix core (bulk-insert + DLM repair)
   - workers.push wrapper restore in ServerThreadPool::start
   - DML/storage regression test fixes
2. **5-remote sync tooling** (live):
   - scripts/sync/5remotes_sync.sh
   - scripts/sync/5remotes_drift_check.sh
   - scripts/sync/README.md
3. **V400-02 vector WAL coverage** (closed pre-existing test-compile drift):
   - 6 factory functions in crates/storage/src/wal/mod.rs
   - WALOperation extended to 13 variants in crates/wal-verification/src/lib.rs
4. **Refactoring for alpha-gate compliance**:
   - crates/sqlrustgo/src/execution_engine.rs: 2801 → 1017 lines (impl block extracted
     to crates/sqlrustgo/src/execution_engine_methods.rs)

## 2. v4.1.0 vs v4.0.0: stage claim boundary

Per `docs/governance/STAGE_CONFIG.yaml` ALPHA stage:

> Allowed claims (pre-ALPHA): "SQLRustGo v4.1.0 is in active development toward a continuation of v4.0.0."

Allowed claims (pre-BETA): "SQLRustGo v4.1.0-alpha1 has been cut for early testing of v4.0.0 bugfix carry-forward."

Prohibited pre-BETA: "Production multi-model claim (requires GA, and even v4.0.0 GA has 3 caveat items per CLAIM_DOWNGRADE_MANIFEST)"

Prohibited pre-GA: "168h multi-model SOAK result (per V400-09)"

## 3. Known limitations (carried from v4.0.0)

Per `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md`:

- **SQL surface**: WP-A issues covered; WP-B issues (6: #4721 #4674 #4716 #4676 #4675 #4670) partial; full behavior deferred.
- **DDL surface**: WP-C issues (6: #4682 #4652 #4672 #4669 #4709 #4703) deferred to v4.1.0.
- **Transaction semantics**: WP-E issues (#4847 partial, #4626 not started).
- **Schema**: WP-F #4848 (ALTER TABLE RENAME COLUMN) deferred to v4.1.0.
- **Type/Comparison**: WP-G #4846 (CHAR(n) PAD SPACE) deferred to v4.1.0.

### 3.1 Inherited SQLite-dialect scope exclusions (v4.0.0 §4 — restored 2026-09-30)

> **补回说明**（2026-09-30）：本节原先遗漏 v4.0.0 `LEGACY_ISSUES.md` §4 声明的
> 12 条「继续保持 caveat、不修」issue，其中 11 条在 v4.1.0 全部文档中零出现。
> 按 `GATE_CONDITIONS.md` §2「GA-claim-caveat」判定标准，每条必须在 README /
> RELEASE_NOTES / scope 文档中**显式排除**，否则不得留存为 open。
> 依据：`docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` F-05。

以下 **11** 条在 v4.1.0 **继续保持 caveat**，不在 v4.1.0 声明范围内：

| Issue | 声明排除的能力 |
|---|---|
| #4719 | sqlite statistics / ANALYZE |
| #4711 | advanced ON CONFLICT forms |
| #4698 | GREATEST/LEAST, math functions |
| #4694 | SET TIMEZONE / isolation syntax |
| #4693 | TEMP/TEMPORARY table syntax |
| #4685 | multi-table UPDATE / DELETE USING |
| #4677 | TRUNCATE and complex LIKE ESCAPE |
| #4667 | ORDER BY NULLS FIRST/LAST |
| #4650 | GROUP_CONCAT and related |
| #4646 | JSON_EXTRACT / JSON_EACH |
| #4625 | INDEXED BY optimizer hint |

**#4670 归属已裁决（2026-09-30，人工决策：算 v4.1.0 的）**

#4670 原在 v4.0.0 `LEGACY_ISSUES.md` 中自相矛盾：§3.4 把它列为「v4.0.0 必修」，
§4 又列为「保持 caveat 不修」。**该冲突已裁决：#4670 归属 v4.1.0**，
按 `CLAIM_DOWNGRADE_MANIFEST.md §2` / §3.1 的 WP-B **partial** 定位处理 ——
即 v4.0.0 声称部分支持，v4.1.0 负责补全到完整行为。

因此 #4670 **已从上方 caveat 排除表中移除**（该表由 12 条变为 11 条），
并计入 `ISSUES_PLAN.md` §4.5「WP-B completion」范围（WP-B 6 条：
#4721 #4674 #4716 #4676 #4675 **#4670**），属 v4.1.0 backlog 20 条之一。

v4.1.0 对 #4670 的声明边界：**当前仍为 partial**，在 WP-B 6 条全部补全前，
不得声称 CEIL/FLOOR/TRUNCATE/HEX/MD5/SHA 已完整支持。

理由：v4.x GA claim 只覆盖 vector + graph + GMP + 关系子集，上述 SQLite
dialect 不在声明范围内。

## 3.2 Build artifact rules (inherited from v4.0.0 §6.2)

> **补回说明**（2026-09-30）：v4.0.0 `LEGACY_ISSUES.md` §6.2 记录的用户
> 2026-09-08 强制声明，在 v4.1.0 文档中零引用。依据
> `ALIGNMENT_AUDIT_2026-09-30.md` F-11。

**强制规则**：禁止提交 `*.log`、`*.tbl`、`*.json` 文件。

- 依据：v3.12.0 GA commit 中 `server.log` 单文件 251MB，超 GitHub 100MB 限制
- 配套 CI gate：`scripts/gate/check_no_log_tbl_json.sh`（**已存在** ✅）
- 配套 `.gitignore` 条目；大 JSON evidence 应转 `.json.gz` 或外部 artifact storage

## 4. v4.1.0 ALPHA scope work (backlog)

Per `docs/releases/v4.1.0/ISSUES_PLAN.md` §4:

| Category | Count | Items |
|---|---|---|
| WP-C DDL / Integrity | 6 | #4682, #4652, #4672, #4669, #4709, #4703 |
| WP-D Join / Subquery | 4 | #4668, #4656, #4649, #4636 |
| WP-F Schema Migration | 1 | #4848 |
| WP-G Type / Comparison | 1 | #4846 |
| WP-B completion | 6 | #4721, #4674, #4716, #4676, #4675, #4670 |
| WP-E completion | 1 | #4626 |
| WP-H #4639 carry-forward | 1 | #4639 |
| **Total** | **24** | 6-10 weeks estimated (含 2026-09-30 RE-OPENED 的 WP-A 4 条) |

## 5. Stage state

| Stage | Date | Reason |
|------|------|--------|
| DRAFT | 2026-09-23 | Initial entry per STAGE_CONFIG DRAFT_to_ALPHA trigger; PHASE_0 docs scaffolded |
| ~~ALPHA~~ | ~~2026-09-29~~ | **作废** — 声明「3 个 alpha-gate FAIL 全部解决」未经实跑证据支撑（`check_anti_fabrication.sh` 实跑 exit 1）。`[AFP-VIOLATION: Type-B]`。annotated tag `v4.1.0-alpha1` 实际已存在（指向 `c1a73a5320`，由 `liying <openheart@gaoyuanyiyao.com>` 于 2026-09-29 15:11:41 +0800 创建），tag 处置待治理决策。 |
| **DRAFT（回退）** | **2026-09-30** | **按 AFP §7.1 Type-B（P0）回退。3 个继承门禁阻断项现已全部清除并附实跑证据**（`check_anti_ignore_gate` exit 0；`check_arch_invariants` exit 0 PASS 5/5；`check_anti_fabrication` 修复 CHECK 4 白名单后复跑 exit 0）。证据：`evidence/gate-runs-2026-09-30/`。**门禁转绿不等于阶段推进** —— DRAFT → ALPHA 须显式执行 `STAGE_CONFIG` 流程并记录。 |
| BETA | TBD | Blocked by 50% coverage measurement + WP-C..G closure + DRAFT→ALPHA re-entry |
| RC | TBD | Blocked by BETA completion |
| GA | TBD | Blocked by RC completion + 168h SOAK PASS |

## 6. References

- `docs/releases/v4.1.0/STAGE.yaml` — v4.1.0 stage SSOT (current_stage: **DRAFT**)
- `docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` — v4.0.0→v4.1.0 对齐审计报告
- `docs/governance/incidents/2026-09-30-V410-ALPHA-UNSUPPORTED-BY-GATE-EVIDENCE.md` — AFP Type-B 违规记录
- `docs/releases/v4.1.0/evidence/gate-runs-2026-09-30/` — 门禁实跑输出归档
- `docs/releases/v4.1.0/PHASE_1_SCOPE.md` — DRAFT → ALPHA work plan + resolved items
- `docs/releases/v4.1.0/ISSUES_PLAN.md` §4 — WP-C..G detailed backlog
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` §3 — v4.0.0 GA caveat items (inherited)
- `docs/releases/v4.0.0/GA_GATE_REPORT.md` — v4.0.0 GA CONDITIONAL PASS verdict
- `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` — SOAK deferral rationale
- `scripts/sync/README.md` — 5-remote sync tooling (v4.1.0 deliverable)
- `docs/governance/STAGE_CONFIG.yaml` — version-agnostic stage framework

## 7. What changed since the previous ALPHA snapshot

Sections 1–6 above are the release-note text as it stood at the 2026-09-30 DRAFT
rollback; they are reproduced here unchanged. This section records what has been
done **on the v4.1.0 release line since that snapshot** — the 2026-10-22
CI-truth / CI-resilience batch and the gate corrections pulled in with it.

Every entry below names the evidence document it was read from. **This is a
truth-and-durability batch, not a feature batch.** It removed several conditions
under which the CI pipeline reported a green or a red run for reasons that had
nothing to do with the code under test. It adds no user-visible capability, it
does not advance the stage, and it is not a GA claim.

### 7.1 Batch 24 — making CI actually run the tests (#5508 / #5515 / #5516 / #5533 / #5534)

**What was found.** The `test` job depends on `lint-build`. Across **1452**
instances of that job only **30** ever started, and **none** of them succeeded:
29 failed, 1 was still running. All 29 failure logs were downloaded and
classified:

| Failure class | Count | What it means |
|---|---|---|
| Died on the GitHub checkout | 6 | `unexpected EOF` / TLS handshake timeout / `dial tcp … i/o timeout` against `github.com` — not one line of test code ran |
| Died on crates.io | 4 | index download failure / "Operation too slow" — the build step died before any test ran |
| Reached the tests and genuinely failed | 19 | among them `--test alter_table_test` ×4 and `--test diag_q11` ×4 |
| Succeeded | 0 | — |

The remaining instances were `skipped` (779) or `cancelled` (643), so a run
could show green while the test job had never executed at all.

**What changed.** The `test` job no longer waits to be woken by another job: it
carries an empty dependency list plus an always-run condition, exactly the
shape the repository's `mutation` and `postcheck` jobs already used, so the test
suite starts **with** the pipeline instead of 12 minutes to 7 hours after it.
The same change was applied to the member-package job. After this batch the two
gated workflows contain **no non-empty dependency list at all**. The checkout
and crates.io fetches were separately given retries and timeouts, and the two
external network dependencies were reclassified as external dependencies rather
than as code failures.

The evidence is explicit about the limits of this fix: it addresses the start
delay and the 30-of-1452 start rate; it does **not** explain the 19 genuine
test failures, and it does **not** explain the 10 deaths on external networks.

**Evidence.** `docs/releases/v4.1.0/evidence/CI_TRUTH_BATCH24_2026-10-22.md` —
§1.3 (the 1452 / 30 / 0 measurement), §1.4 (the per-log classification table),
§1.5 (what "CI is green" therefore means), §1.6 (the fix and its stated
limits), §2 (the two checkout defects), §2bis (crates.io).

### 7.2 Batch 26 — CI resilience after a host kernel panic (#5550 / #5551)

**What was found.** On **2026-10-22 22:57:32 CST** the CI host kernel-panicked
and the crash dump was captured. Runs **8914–8921** died with it: seven jobs
were failed **in the same second**, every log stops mid-compilation, and **not
one test assertion was produced**. The server-side evidence (`action_run_job` /
`action_task` / `journalctl`) shows a host-side kill, not six independent
failures. The host rebooted **5 times** within that day, and the load average
immediately before the panic was **147.82** on an 80-core machine.

Per the repository's own governance wording, that run is **NOT-RUN
(infrastructure red)** and produces no PASS evidence of any kind.

A second infrastructure red followed: after the reboot, the first CI run
(**8991**) could not start a single job because the docker address pool was
exhausted by leaked CI networks and volumes (#5551). A server-side cleanup that
reclaims only the unused `GITEA-ACTIONS-TASK-*` networks and volumes was
installed; its first execution took the volumes from **193 → 89** and the disk
from **1.1T → 899G**.

**What changed.** Workflow-level concurrency and per-container compile
parallelism were bounded (§7.8), and the docker cleanup was installed as the
server-side counterpart.

**Evidence.** `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md`
— §1.1–§1.3 (runs 8914–8921 classified), §1.4 (run 8991), §2 (the fixes),
§2.4 (server-side counterpart for #5551).

### 7.3 PR-DAG gate: the base ref was permanently unresolvable (#5542)

`scripts/gate/verify_pr_dag.sh` defaulted to `origin/develop/v4.1.0` — a ref
that resolves neither in a local clone nor in CI. The gate was therefore
permanently `DRIFT`, and five cases in
`tests/integration/gate/gate_criterion_existence_test.rs` were red on every
run. The fix makes the base an explicit argument with one deterministic
resolution chain, compares the resolved SHAs as integers, and gives the gate a
`--selftest` so that it can demonstrate it still has discriminating power.

**Evidence.** `docs/releases/v4.1.0/evidence/PR_DAG_BASE_RESOLUTION_5542_2026-10-22.md`
— §1 (the defect and both "there is no such ref" environments), §2 (the fix),
§3 (the AC1–AC7 comparison), §5 (what was **not** achieved).

### 7.4 Coverage gate: the storage feature set is now derived, not hand-written (#5537)

`scripts/gate/check_coverage_v312.sh` passed an incomplete feature set to the
storage crate, so the storage coverage was measured as 0 and the gate could not
tell "the crate has no tests" apart from "the gate forgot a feature". The
feature requirement is now **derived from the crate sources**
(`derive_crate_features` / `features_for_crate`), and the two gated lines are
`crates/storage/src/lib.rs:28` and `crates/storage/src/lib.rs:92`. A
regression suite was added at `tests/integration/gate/alpha_gate_blocker_test.rs`.

Measured effect on the ALPHA stage gate, same command before and after:

| | Result | Exit |
|---|---|---|
| Before | 14 PASS / 2 FAIL | 1 |
| After | 16 PASS / 0 FAIL | 0 |

**Evidence.** `docs/releases/v4.1.0/PR-5537_SPEC.md` §1.1 and §3.1;
`docs/releases/v4.1.0/PR-5537_ACCEPTANCE.md` §2 (AC-1 … AC-9) and §3 (what was
not achieved); the archived run log
`docs/releases/v4.1.0/logs/gate_alpha_6f26128890_20261022_131209Z.log`, whose
last line reads `Stage ALPHA (v4.1.0): 16 PASS, 0 FAIL`.

### 7.5 Wall-clock measurement tests reclassified (#5547)

Two tests in `crates/storage/src/wal_legacy.rs` — `test_wal_perf_100_update`
and `test_wal_perf_recovery_1mb` — assert wall-clock bounds, so they measure
the machine, not the code. Under coverage instrumentation they run several
times slower and the coverage gate died with
`FAIL: could not measure sqlrustgo-storage (exit=101)`. Both were given an
ignore attribute and registered in `tests/baseline/ignore_registry.json` under
exactly the rule already applied to their sibling `test_wal_perf_1000_insert`
(#5545).

The registry moved from `total_allowed` **117 → 119**, the parked count from
**48 → 50** and the `EXTERNAL_PREREQ` count from **46 → 48**. The anti-ignore
gate recounts the tree with the same rules it applies to the registry, so an
ignored test that was never registered cannot pass. After the fix the coverage
gate measured storage at **80.61 %** and the L1_8 average at **87.80 %** against
an 80.0 % threshold, and the storage lib test binary reported
`913 passed; 0 failed; 2 ignored` (it was `914 passed; 1 failed` before).

**Evidence.** `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md`
§3; `tests/baseline/ignore_registry.json`.

### 7.6 Wrapper delegation contract: the premise could not be constructed (#5548)

`tests/e2e/e2e_canonical_subprocess.rs` asserts that the wrapper must fail
honestly when it cannot delegate. It built the "cannot delegate" premise with
an empty working directory and an empty `PATH`, but the wrapper's candidate
table in `crates/sqlrustgo-cli/src/lib.rs` also contains a candidate that
resolves next to the wrapper's own executable — and in CI the wrapper and the
server binary sit in the same directory. The premise therefore could not be
constructed, and **the contract had never actually been verified in CI**.

The fix is **test-only**: the wrapper is copied into an empty directory before
it is executed, and the target directory, the working directory, `PATH` and the
server-binary environment variable are all neutralised, so all five candidates
miss and the wrapper reaches its "nothing found" branch. **The production
candidate table was not changed.**

**Evidence.** `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md`
§4.

### 7.7 g13 concurrency test: the premise was "how fast is this machine" (#5549)

`tests/integration/sql/g13_oltp1_concurrent_select_test.rs` used a fixed
2-second window and required at least two iterations inside it. On an idle
development machine the heavy query completes in tens of milliseconds and the
window holds dozens of iterations; on a loaded CI container a single iteration
can exceed a second, the window holds one iteration, and the assertion fails —
so the assertion that was supposed to guard "is this measurement meaningful"
became the failure point, and the measurement never happened.

The window is now **derived from a measured iteration** —
`clamp(one_iteration × 3, 2s, 60s)` — and the same window is used on the
uncontended and the contended side, so the ratio stays comparable. The contract
itself is unchanged.

**Evidence.** `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md`
§5.

### 7.8 CI concurrency and parallelism hardening (#5550)

Both gated workflows now carry a workflow-level concurrency block with
**separate, fixed, repository-level group names**:

| Workflow | Group | `cancel-in-progress` | Why |
|---|---|---|---|
| `.gitea/workflows/ci.yml` | `sqlrustgo-ci-pipeline-v2` | `true` | A run stuck on a remote runner occupies its group forever and the API cannot release it (the cancel call returns `409`); the next push cancels it instead. |
| `.gitea/workflows/gate.yml` | `sqlrustgo-gate-v2` | `false` | The gate workflow is one long job — the full ALPHA quality gate including a release and fat-LTO build, measured at more than 30 minutes; killing it on every push wastes the work, and its tasks run on the local runner where a stuck state can still be recovered. |

Two measured Gitea 28.1.0 behaviours are recorded in the workflow comments
themselves: joining a concurrency group **cancels the runs in that group that
have not started yet** (so the two workflows must not share a group — the first
attempt did, and one of the two required checks was cancelled on every push),
and a run stuck in `Blocked` **drags its successor down** (the `-v2` group-name
suffix is an empirical workaround for that, not a design choice).

Per-container compile parallelism is bounded by `CARGO_BUILD_JOBS: "16"` in both
workflows — three concurrent jobs × 16 = 48 rustc processes on 80 cores, instead
of the default one-per-core, which allowed up to 240. The cargo network
settings were hardened at the same time: `CARGO_NET_RETRY: "10"`,
`CARGO_HTTP_TIMEOUT: "120"`, `CARGO_HTTP_LOW_SPEED_LIMIT: "1"`.

**Evidence.** `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md`
§2.1–§2.3; the concurrency and environment blocks in
`.gitea/workflows/ci.yml` and `.gitea/workflows/gate.yml`.

### 7.9 Storage db-scope coverage gate (#5401)

`scripts/gate/check_storage_db_scope_coverage.sh` extracted the `StorageEngine`
trait with an awk range that also swallowed `MemoryStorage`'s inherent `impl`,
so it demanded that the wrappers "override" a method that was never in the trait
— the gate was **permanently red**. The extraction was narrowed, and the two
genuinely missing methods, `get_view` and `list_views`, were added as
forwarding implementations to the wrappers in `crates/storage/src/wal_storage.rs`
and `crates/storage/src/mvcc_storage.rs`. The gate's own message bug — a line
count that was always 1 — was corrected to a word count.

Observed result: `PASS: 2 wrappers override all 16 database-scoped methods`.

**Evidence.** `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md`
§7.

### 7.10 Production-storage correctness gate rewritten (#5400)

`scripts/gate/check_correctness_tests_production_storage.sh` was rewritten as
five criterion functions, R1–R5, each matched on the assembly arm (default /
binary / parallel) and restated in production terms: WAL manager plus
`FileStorage::new_with_external_wal` plus the WAL wrapper layer; `FileStorage`
plus `ParallelWalStorage`. The gate carries a `--selftest` with five degraded
fixtures, so the criterion can be shown to still have discriminating power
rather than having been widened into permanent green.

Observed result: `=== Result: PASS=5 FAIL=0 ===`, and the selftest reports
`passed: 5  failed: 0`.

**Evidence.** `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md`
§6.

### 7.11 Release-line documentation pointer cleanup (#5502)

`scripts/gate/check_docs_pointer_paths.sh` treats inline code as a pointer, so
a document that quotes a path known to be broken becomes a broken pointer
itself. The gate reported 8 new broken pointers in this release line. One was a
genuine break — the ALTER COLUMN DEFAULT evidence document
(`docs/releases/v4.1.0/evidence/ALTER_COLUMN_DEFAULT_BATCH13_2026-10-22.md`)
pointed at a storage source file that no longer exists; it was rewritten to
point at the BINT on-disk format file `crates/storage/src/bin_segment.rs` and
to state plainly that the only `default_value` in that file is an in-memory
column-definition field that is always `None`. Seven were genuine debt and were
registered in `tests/baseline/doc_pointer_paths_releases_v4.1.0.json`; one
stale baseline entry was removed.

**Evidence.** `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md`
§8; `tests/baseline/doc_pointer_paths_releases_v4.1.0.json`.

### 7.12 What this batch does and does not establish

**It establishes** that the CI pipeline can now start and finish its test job;
that the conditions under which the pipeline reported a colour unrelated to the
code have been removed or bounded; and that four gates that were permanently
red or permanently green have been given criteria with demonstrable
discriminating power.

**It does not establish** any of the following, and none of it is claimed
anywhere in this file:

- that the CI pipeline is green — the `test` job has still never completed
  successfully, and this batch changed the pipeline so that the truth becomes
  observable at all, not so that it becomes green;
- that the ALPHA → BETA transition criterion is met — see §8.3;
- that v4.1.0 is released, tagged for GA, or fit for production use;
- any performance, throughput, latency or compatibility number for this line.

## 8. Known limitations — not fixed in this line

This section is sourced from the repository's own issue and audit documents.
Every entry names the document it was read from.

### 8.1 The seven open issues registered in the stage SSOT

`docs/releases/v4.1.0/STAGE.yaml` → `open_issues` carried seven entries.
**None of them was closed by anything in §7** — but **six were closed on
2026-10-23**, outside this section. The closures are recorded in
[`evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md`](evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md)
and [`evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md`](evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md),
and in `STAGE.yaml` → `resolved_issues_2026_10_23`; **one remains open.**

| # | Registered open issue | Status as of 2026-10-23 |
|---|---|---|
| 1 | The current-version file and the v4.0.0 release notes still describe v4.0.0 as "GA (CONDITIONAL PASS)", which contradicts the `REVOKED` verdict in the v4.0.0 stage file | **RESOLVED** — both files now carry the SSOT's wording. The code was published (2026-09-19, tag `v4.0.0-final` @ `54571eeca0`, on all five remotes' `main`), but v4.0.0 never legitimately reached GA and is not a GA release |
| 2 | P0-01 — `scripts/gate/check_anti_ignore_gate.sh` was a hollow gate (it did not scan the source tree), so rule P16 was never actually enforced | **RESOLVED** — the gate recounts the tree with the registry's own rule and fails closed on drift; measured 119 == 119, exit 0 |
| 3 | P0-02 — #4708 is a false DONE: the parser does not support block comments at all, masked by an unregistered ignore attribute in the parser legacy test | **RESOLVED** — the parser has a `/* ... */` branch (`crates/parser/src/lexer.rs:78`) and the `#[ignore]` at `crates/parser/tests/wp_a_legacy.rs:68` is gone |
| 4 | P0-05 — the recorded root cause for the debt-registry SEM-4 entry is still the literal placeholder `XYZ`, and the parser figure of 75.16 % recorded there is not reproducible (60.58 % measured) | **RESOLVED** — `SEM-4` is `CLOSED` (`closed_at: "2026-10-23"`, `closing_pr: [5126, 5537]`) with a measured root cause: the coverage had been taken with parallel test threads, so the reading depended on machine load. Measured: 8/8 crates, L1_8 **87.80 %**, `parser 86.46 %` |
| 5 | P1-01 — parser line coverage measured 60.58 % against the ALPHA threshold of 80 %, a gap of 19.42 pp; the coverage gate fails open when its extraction at `scripts/gate/check_coverage_v312.sh:30` cannot proceed | **RESOLVED** for the fail-open: a crate that cannot be measured now fails the gate (`check_coverage_v312.sh:315-349`), and the measured parser figure is **86.46 %** (L1_8 87.80 %), not 60.58 %. The threshold itself remains the stage framework's call |
| 6 | P1-02 — the #4540 Q20 oracle is not closed: the Q20 file is missing and the cross-engine hash check records it as absent, contradicting the "Q20 PASS" line in the v3.12.0 TPC-H report | **OPEN** — `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` |
| 7 | P0-08 — WAL recovery rewrites the whole table row by row: measured, the first full-table query after a restart took 24 min 22 s (the immediately following identical query took 0.08 s) under 68,288 rows / 58,288 inserts / 8.0 MB of WAL, with the delta-append path never reached; recovery completes before the listener, so the server is hard-unavailable during startup, and no historical SOAK covered it because the harness never restarts mid-run | **RESOLVED** — and the report's **attribution was corrected by measurement**: the write path was already taking the delta-append branch (the base snapshot stays at the 650-byte schema while `t.delta` carries every row) and its per-row cost is **constant** (96.4 / 85.8 / 86.4 µs/row at 500 / 2,000 / 8,000 rows), so `save_table_full` was never called per record. The real quadratic term was the **per-entry dedup scan**, which cloned the whole table to ask "is this row already there?" (63.9 → 247.9 → 896.4 µs/row). Fixed with a per-table row index: one scan per table, then an O(1) lookup per replayed entry, with `Update`/`Delete` invalidating the affected table's index because a WAL entry carries no old row content. Measured after the fix on the real `RecoveryEngine::recover()`: 210.3 → 224.0 → 372.4 µs/row — 16× the rows costs only 1.8× per row; extrapolated to 58,288 rows ≈ 21.7 s vs ≈ 461 s before (**≈21×**). Semantics pinned by 5/5 PASS, including the two cases a stale index would silently lose committed data on. **Not claimed:** the recovery-before-listener ordering is unchanged, and the per-row delta fsync remains a constant cost. See `docs/releases/v4.1.0/evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` |

### 8.2 The 168-hour SOAK (V400-09)

`docs/releases/v4.1.0/STAGE.yaml` → `known_gaps.V400_09_soak` records:

- `status: "NOT MET"`, verified 2026-10-07;
- `decision: "OPEN"` — whether to re-run the 168 h SOAK is an architect
  decision, and the stage file deliberately does not make it.

The finding is that the SOAK process has terminated and produced no output for
the later runs: the service process recorded by #4944 is no longer in the
process table, a full-disk search finds no stability report or metrics file
for those runs, and the v4.0.0 final report that does exist is dated 2026-09-30
and documents the **failure of the first run** — its own body says the 168-hour
process never ran to completion. This gap blocks the ALPHA → BETA transition.

### 8.3 The ALPHA → BETA transition and the GA artefacts

- `docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md` §1 makes AB-01 … AB-10
  all-or-nothing on a single frozen commit, and §6 publishes no PASS. **No AB
  verdict is recorded anywhere in the repository.**
- The GA stage still has missing required artefacts — the GA gate report, the
  GA release timeline, the evidence-status document and the post-GA plan — and
  **the GA gate list has never been executed end-to-end for this version**.
  Recorded in `docs/releases/v4.1.0/FEATURE_MATRIX.md` §3 item 1.
- The 8-hour SOAK's evidential weight is downgraded by the IAIA-410 audit; a
  green SOAK, a green stage gate and a closed issue list are three different
  things, and this file claims none of the three.

### 8.4 The parser coverage figure and the ALPHA threshold are both unsettled

Three parser-coverage measurements exist and the repository does not pick one:

| Figure | Source | Date |
|---|---|---|
| 60.58 % | `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` (P1-01) | 2026-10-01 |
| 86.20 % | `docs/releases/v4.1.0/evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md` (COV-01 recorded as `DONE` in the stage file) | 2026-10-07 |
| 86.46 % (full L1_8 run; L1_8 average 87.80 %) | `docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3 — the first reading taken with `--test-threads=1`, so it does not depend on machine load (two consecutive runs of the same crate agree exactly) | 2026-10-23 |

The threshold is unsettled as well: the stage framework sets
`COVERAGE_MIN_ALPHA: 50` and the stage file's exit criteria say
"Coverage >= 50 %", while the defect catalogue and the architecture document
use **80 %**, which is the Beta figure (`COVERAGE_MIN_BETA: 75`); the ALPHA →
BETA gate plan applies the stricter 80 %. `docs/releases/v4.1.0/FEATURE_MATRIX.md`
§3 items 6 and 7 record all readings and pick none.

### 8.5 The current 5-remote sync state is not recorded

The drift figures in the defect catalogue (P0-04) are dated 2026-10-01 and the
stage file's `last_status_refresh` records a different state on 2026-10-03.
Both are dated records; they are not merged here and **no current sync state is
claimed**. The tooling exists — `scripts/sync/5remotes_sync.sh`,
`scripts/sync/5remotes_drift_check.sh` and `scripts/sync/README.md` — but its
current output is not part of this release line.

### 8.6 Capability claim boundaries that this line still carries

The #5304 downgrade boundary, quoted verbatim from
`docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md` §3.1, is reproduced here as
that manifest's §4 checklist requires of this file:

> "v4.1.0 does **NOT** ship the advertised `BufferPool` / `Page` /
> `disk B+Tree` capability. The `sqlrustgo_storage::BufferPool` type,
> `sqlrustgo_storage::Page` type, and disk B+Tree in
> `crates/storage/src/bplus_tree/index.rs` are `pub use`-exported from
> `src/lib.rs` for **storage-test code only** (`/// ⚠️ Test-only
> capability. Not used by production FileStorage.` doc-comment on the
> re-export). Production `FileStorage` is an in-mem `HashMap` + JSON
> `insert_buffer` (NON-paged). The capability is excluded from v4.1.0
> release claims; full production wiring is a future workstream and
> tracked under #5304."

The same manifest's checklist also requires this line in the README, the stage
file and the GA gate report. **The README does not yet carry it, and this
version has no GA gate report at all** — so those checklist items remain
outstanding and are not claimed as satisfied here. The orthogonal "binary off
production path" exclusion lives in
`docs/releases/v4.1.0/b2-off-production-path-registry.md`.

The 11 inherited SQLite-dialect exclusions and the #4670 adjudication are
recorded in §3 and §3.1 above; they are unchanged and are not repeated here.

### 8.7 Two wording conflicts inside this release line that are recorded, not resolved

| Conflict | The two readings | Where it is recorded |
|---|---|---|
| Stage | `docs/releases/v4.1.0/STAGE.yaml` says `current_stage: "ALPHA"`, re-entered 2026-10-06 per `last_transition`; the header of this file says "DRAFT — NOT YET ALPHA" | `docs/releases/v4.1.0/FEATURE_MATRIX.md` §3 item 8 and `docs/releases/v4.1.0/EVALUATION_REPORT.md` row D1. The stage framework makes the stage file the per-version SSOT, so the SSOT governs and the disagreement is recorded rather than hidden. |
| Defect count | The header of this file quotes "P0×6 + P1×7 + P2×7"; `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` §0 records P0 × 8 / P1 × 7 / P2 × 7 = 22 | `docs/releases/v4.1.0/FEATURE_MATRIX.md` §3 item 9 and `docs/releases/v4.1.0/EVALUATION_REPORT.md` row D5. Both numbers are left exactly as they are; this file does not pick a winner. |

A third, smaller document defect: §1 item 4 above cites the two stale source
paths crates/sqlrustgo/src/execution_engine.rs and
crates/sqlrustgo/src/execution_engine_methods.rs. Those files do not exist in
this tree — the current files are `src/execution_engine.rs` and
`src/execution_engine_methods.rs`, at the repository root. The citation is
preserved verbatim because it is part of the original 2026-09-30 text, and it is
recorded here as a stale path rather than silently rewritten.

### 8.8 Two gate failures observed in this worktree that are **not** fixed by this change

Both are pre-existing and both live in files this change does not touch, so
they are recorded rather than repaired:

| Gate | Observed | Owner of the fix |
|---|---|---|
| `scripts/gate/check_arch_invariants.sh` | `FAIL: C-ARCH-05 violated` — `src/execution_engine.rs` is 1604 lines against a 1600-line limit; overall `PASSED: 4  FAILED: 1`, exit 1 | The execution-engine source, not the release documents |
| `scripts/gate/check_docs_pointer_paths.sh --scope docs/releases/v4.1.0` | One broken pointer is not in the baseline: the batch-26 CI resilience evidence document points at the storage binary-format source file that §7.11 records as already removed | The batch-26 evidence document, not this file |

### 8.9 The items this batch explicitly left open

`docs/releases/v4.1.0/PR-5537_ACCEPTANCE.md` §3 records, in the same
"not achieved / not verified" form used throughout this repository:

- the full regression over the whole workspace — **not run** (it exceeded the
  time budget);
- the performance debt in the single-row delete cost test — **not fixed** (it
  already failed on the base commit; it was excluded so that the debt would not
  destroy the coverage measurement alongside it, not because it passes);
- actual behaviour on the CI runner image — **not verified** (the local
  platform is glibc);
- wiring the three new gate scripts into CI — **not done**.

## 9. Verification

### 9.1 How each claim in §7 was verified

Every row below was produced by running the command in this worktree, or by
reading the named evidence document. The **Result** column is what the run
printed.

| Claim | Verified by | Result |
|---|---|---|
| The CI `test` job never succeeded | Read from `docs/releases/v4.1.0/evidence/CI_TRUTH_BATCH24_2026-10-22.md` §1.3 and §1.4 — a server-side query over the job table plus the classification of all 29 failure logs | 1452 instances / 30 started / 0 succeeded; 6 checkout deaths, 4 crates.io deaths, 19 real failures |
| The 2026-10-22 CI reds were infrastructure reds | Read from `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md` §1.1–§1.3 — server-side job, task and boot-log evidence for runs 8914–8921 | Host kernel panic at 2026-10-22 22:57:32 CST; seven jobs failed in the same second; no test assertion was produced; classified NOT-RUN (infrastructure red) |
| The workflows carry separate concurrency groups and bounded compile parallelism | Read from the concurrency and environment blocks of `.gitea/workflows/ci.yml` and `.gitea/workflows/gate.yml`; cross-checked against the batch-26 evidence §2.1–§2.3 | `sqlrustgo-ci-pipeline-v2` and `sqlrustgo-gate-v2`; `CARGO_BUILD_JOBS: "16"`; `CARGO_NET_RETRY: "10"`, `CARGO_HTTP_TIMEOUT: "120"`, `CARGO_HTTP_LOW_SPEED_LIMIT: "1"` |
| The checkout / crates.io consistency gate passes | `bash scripts/gate/check_ci_checkout_consistency.sh` | C1–C4 all OK, `ALL CHECKS PASS`, exit 0 |
| The anti-ignore gate recounts the tree and passes | `bash scripts/gate/check_anti_ignore_gate.sh` | `total_allowed=119 (max=150), active=69 (max=80)`; tree recount 119 real ignore attributes; exit 0 |
| The production-storage correctness gate has five discriminating criteria | `bash scripts/gate/check_correctness_tests_production_storage.sh`, and the same script with `--selftest` | R1–R5 all PASS, `Result: PASS=5 FAIL=0`; selftest `passed: 5  failed: 0` |
| The storage db-scope coverage gate is green and the two missing forwards exist | `bash scripts/gate/check_storage_db_scope_coverage.sh`; the forwarders read in `crates/storage/src/wal_storage.rs` and `crates/storage/src/mvcc_storage.rs` | `PASS: 2 wrappers override all 16 database-scoped methods`, exit 0 |
| The root package no longer mutates process-global environment variables | `bash scripts/gate/check_no_process_env_mutation.sh` | `PASS: C1 — the root package does not mutate process-global environment variables`, exit 0 |
| The PR-DAG base-ref fix and its self-test exist | Read from `docs/releases/v4.1.0/evidence/PR_DAG_BASE_RESOLUTION_5542_2026-10-22.md` §2 and §3, and from `tests/integration/gate/gate_criterion_existence_test.rs` | Explicit base argument, integer SHA comparison, `--selftest` present |
| The ALPHA stage gate went from 14 PASS / 2 FAIL to 16 PASS / 0 FAIL | Read from `docs/releases/v4.1.0/PR-5537_ACCEPTANCE.md` §2 (AC-1) and from the archived run log `docs/releases/v4.1.0/logs/gate_alpha_6f26128890_20261022_131209Z.log` | The log's last line reads `Stage ALPHA (v4.1.0): 16 PASS, 0 FAIL` |
| The parser legacy test target compiles and passes | `cargo test -p sqlrustgo-parser --test wp_a_legacy` | `35 passed; 0 failed; 0 ignored`, exit 0 |
| The wall-clock tests are registered, not merely ignored | Read from `tests/baseline/ignore_registry.json` and from `crates/storage/src/wal_legacy.rs`; counted by `bash scripts/gate/check_anti_ignore_gate.sh` | Registry `total_allowed` 119, active 69, parked 50, `EXTERNAL_PREREQ` 48; tree recount 119 equals registry 119 |
| The documentation-pointer gate still reports one unregistered broken pointer | `bash scripts/gate/check_docs_pointer_paths.sh --scope docs/releases/v4.1.0` | 33 broken pointers, 33 registered as known debt, 1 broken pointer not in the baseline (from the batch-26 evidence document), exit 1 |
| The architecture-invariant gate still reports one violation | `bash scripts/gate/check_arch_invariants.sh` | `FAIL: C-ARCH-05 violated` (1604 lines against a 1600-line limit), `PASSED: 4  FAILED: 1`, exit 1 |
| This file's size, and the other seven mandatory documents | `bash scripts/gate/check_document_completeness.sh v4.1.0` | Before this update: `PASS=9 WARN=11 FAIL=1`, the single failure being this file's size. After this update: `PASS=10 WARN=11 FAIL=0` |

### 9.2 What is still **NOT RUN** in this release line

Stated plainly, because a green line that hides an unrun check is exactly the
failure mode this repository's policy is written against:

| Verification | Status | Why |
|---|---|---|
| The full workspace regression over all test targets | **NOT RUN** | Exceeded the time budget; recorded as such in `docs/releases/v4.1.0/PR-5537_ACCEPTANCE.md` §3 |
| Behaviour on the CI runner image | **NOT RUN** | The local platform is glibc; no new compile-time environment dependency was introduced, but it was not measured |
| The three new gate scripts wired into CI | **NOT DONE** | `docs/releases/v4.1.0/PR-5537_ACCEPTANCE.md` §3 item 4 |
| The GA gate list, end to end, for this version | **NOT RUN** | The GA artefacts do not exist and the list has never been executed for v4.1.0 |
| The ALPHA → BETA AB-01 … AB-10 evaluation | **NOT RUN** | No verdict exists in the repository; the plan publishes no PASS |
| The 168-hour multi-model SOAK (V400-09) | **NOT MET** — `decision: "OPEN"` | An architect decision, deliberately left open |
| A CI run of the `test` job to completion | **NOT RUN to completion** | It has never completed successfully; §7.1 changed the pipeline so that the outcome becomes observable, it does not assert an outcome |
| A local ALPHA stage-gate re-run in this worktree | **NOT RUN to completion** | That gate writes its run logs into `docs/releases/v4.1.0/logs/`, which is outside this change's file ownership, so the run was stopped and the logs it had produced were removed. The 16 PASS / 0 FAIL figure quoted in §7.4 and §9.1 is **read from the archived log**, not from a fresh run |
| A current 5-remote sync / drift measurement | **NOT RUN** | The recorded figures are dated 2026-10-01 and 2026-10-03 (§8.5) |
| `scripts/gate/check_arch_invariants.sh` | **FAIL** (pre-existing) | C-ARCH-05; outside this change's ownership (§8.8) |
| `scripts/gate/check_docs_pointer_paths.sh` | **FAIL** (pre-existing) | One unregistered broken pointer in the batch-26 evidence document; outside this change's ownership (§8.8) |
| Any GA-, BETA- or RC-level claim for v4.1.0 | **NOT CLAIMED** | §2 and §7.12 |

### 9.3 How to reproduce the checks that were run

```bash
cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-batch24

bash scripts/gate/check_document_completeness.sh v4.1.0 /tmp/dc_check_out
bash scripts/gate/check_anti_ignore_gate.sh
bash scripts/gate/check_ci_checkout_consistency.sh
bash scripts/gate/check_correctness_tests_production_storage.sh
bash scripts/gate/check_storage_db_scope_coverage.sh
bash scripts/gate/check_no_process_env_mutation.sh
bash scripts/gate/check_arch_invariants.sh
bash scripts/gate/check_docs_pointer_paths.sh --scope docs/releases/v4.1.0
cargo test -p sqlrustgo-parser --test wp_a_legacy
```

## 10. Explicitly not recorded in this release line

Each of the following is **not recorded in this release line**. None of them is
claimed, implied or estimated anywhere in this file:

1. **No GA, BETA or RC promotion for v4.1.0.** The stage recorded by the SSOT
   is ALPHA, re-entered 2026-10-06. No GA tag, no release date and no GA
   verdict exist for this version, and the version's GA gate report does not
   exist.
2. **No release date.** None is recorded for v4.1.0 anywhere in this release
   line.
3. **No current 5-remote sync state.** Only the dated figures of 2026-10-01
   and 2026-10-03 exist (§8.5).
4. **No single agreed parser-coverage number.** Two measurements exist, 60.58 %
   and 86.20 %, with different dates and different measurement conditions
   (§8.4).
5. **No single agreed ALPHA coverage threshold.** 50 %, 75 % and 80 % are all
   in use in different documents (§8.4).
6. **No Alpha → Beta gate verdict.** AB-01 … AB-10 have never been evaluated
   as a set and published (§8.3).
7. **No performance, throughput or latency guarantee for this line.** The
   numbers that appear in this file — 80.61 % storage coverage, 87.80 % L1_8
   average, 1604 lines, 24 min 22 s, 1452 / 30 / 0 — are measurements quoted
   from the documents named beside them; none of them is a guarantee, a target
   or a baseline for a future release.
8. **No compatibility guarantee.** No statement about wire-protocol
   compatibility, storage-format compatibility or upgrade compatibility for
   v4.1.0 is recorded in this release line.
9. **No claim that the CI pipeline is green.** The `test` job has never
   completed successfully (§7.1, §9.2).
10. **No GA-grade claim of any kind.** A green soak, a green stage gate and a
    closed issue list are three different things, and only the last two are
    even partially in evidence here.

## 11. References added with this update

- `docs/releases/v4.1.0/evidence/CI_TRUTH_BATCH24_2026-10-22.md` — batch 24, CI truth (#5508 / #5515 / #5516 / #5533 / #5534)
- `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md` — batch 26, CI resilience (#5550 / #5551 / #5547 / #5548 / #5549 / #5400 / #5401 / #5502)
- `docs/releases/v4.1.0/evidence/PR_DAG_BASE_RESOLUTION_5542_2026-10-22.md` — PR-DAG base ref (#5542)
- `docs/releases/v4.1.0/evidence/PARALLEL_ENV_RACE_BATCH17_2026-10-22.md` — batch 17, process-level environment race (#5423)
- `docs/releases/v4.1.0/evidence/GATE_BLIND_SPOTS_BATCH23_2026-10-22.md` — batch 23, gate blind spots (#5515 / #5403)
- `docs/releases/v4.1.0/evidence/GATE_HONESTY_5417_5419_2026-10-21.md` — gate honesty audit (#5417 / #5419)
- `docs/releases/v4.1.0/evidence/ALTER_COLUMN_DEFAULT_BATCH13_2026-10-22.md` — the document whose stale pointer was repaired in §7.11
- `docs/releases/v4.1.0/evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md` — the coverage measurement of 2026-10-07 quoted in §8.4
- `docs/releases/v4.1.0/PR-5537_SPEC.md` and `docs/releases/v4.1.0/PR-5537_ACCEPTANCE.md` — the coverage-gate storage-feature fix (#5537)
- `docs/releases/v4.1.0/logs/gate_alpha_6f26128890_20261022_131209Z.log` — the archived ALPHA stage-gate run log
- `docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md` — the AB-01 … AB-10 plan
- `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md` — the #5304 claim boundary and its §4 checklist
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` — the defect catalogue behind §8.1
- `docs/releases/v4.1.0/FEATURE_MATRIX.md` — §3 items 1–13, the registered non-claims
- `docs/releases/v4.1.0/EVALUATION_REPORT.md` — the D-table, rows D1 and D5
- `docs/releases/v4.1.0/ISSUE-RECOVERY-QUADRATIC-REPLAY.md` — the P0-08 recovery defect
- `docs/releases/v4.1.0/b2-off-production-path-registry.md` — the orthogonal binary off-production-path exclusion
- `docs/releases/v4.1.0/STAGE.yaml` — the stage SSOT: `current_stage`, `last_transition`, `known_gaps`, `open_issues`
- `docs/governance/debt/debt-registry.yaml` — the SEM-4 entry behind P0-05
- `tests/baseline/ignore_registry.json` — the ignore registry
- `tests/baseline/doc_pointer_paths_releases_v4.1.0.json` — the documentation-pointer debt baseline