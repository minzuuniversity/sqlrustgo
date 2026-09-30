# v4.1.0 — ISSUES_PLAN

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Inherits**: `docs/releases/v4.0.0/ISSUES_PLAN.md` (canonical issue catalog for v4.x)

## 1. v4.1.0-specific issues

### 1.1 Inherited from v4.0.0 (open at v4.1.0 entry)

| Issue | Title | v4.0.0 status | v4.1.0 action |
|---|---|---|---|
| 3 alpha gate FAILs | arch_invariants / anti_fabrication / anti_ignore_gate | OPEN (v4.0.0 DRAFT) | **MIGRATE to v4.1.0 scope** |
| V400-09 168h SOAK | 168h multi-model SOAK FINAL_REPORT | KICKED OFF, no FINAL | Carry forward; require before GA |
| WP-C..G deferred items | 4 WP groups / 12 issues deferred to v4.0.1 (WP-C, WP-D, WP-F, WP-G) | DEFERRED | **MIGRATE to v4.1.0 scope** per WP-H intent |
| #4639 | v4.1-targeted legacy item | DEFERRED | Own in v4.1.0 |

### 1.2 v4.1.0-new issues

| Issue | Title | Status |
|---|---|---|
| v4.1.0 governance | STAGE.yaml / VERSION_PLAN / DEV_PLAN / etc. | DONE 2026-09-29 |
| v4.1.0 bugfix carry-forward | zombie-fix core + workers.push + DML regressions | DONE 2026-09-21 |
| v4.1.0 review queue | 4 v4.0.0 commits cherry-picked via 3 review-queue branches | CLOSED 2026-09-26 (commit 0bbb044da3) |
| v4.1.0 5-remote sync tooling | scripts/sync/{5remotes_sync,5remotes_drift_check,README} | DONE 2026-09-28 |
| v4.0.0 main/release sync to 5 ends | release/v4.0.0 created on github; **local `main` gap NOT closed** | PARTIAL 2026-09-28 (see §1.3 note) |
| v4.1.0 PHASE_0 docs | 9 DRAFT-stage docs (this file + 8 others) | DONE 2026-09-29 |

## 2. Issue closure timeline

| Date | Action |
|---|---|
| 2026-09-19 | v4.0.0 GA CONDITIONAL PASS declared (per GA_GATE_REPORT.md) |
| 2026-09-19 | V400-09 168h SOAK kickoff |
| 2026-09-21 | v4.0.0 zombie-fix core merged to develop/v4.1.0 (PR #3794) |
| 2026-09-21 | v4.0.0 workers.push merged (PR #3793 + #4906) |
| 2026-09-21 | v4.0.0 dml-storage-regressions merged (PR #3792 + #4905) |
| 2026-09-23 | v4.1.0 review queue doc (commit 2bd69b223f) |
| 2026-09-26 | v4.1.0 review queue closed (commit 0bbb044da3 + 9c6767a512) |
| 2026-09-28 | 5-remote sync tooling live (scripts/sync/*) |
| 2026-09-28 | v4.0.0 main + release/v4.0.0 synced to 5 ends |
| 2026-09-29 | v4.1.0 STAGE.yaml + 8 PHASE_0 docs created |

## 3. Open issues (carried into v4.1.0 ALPHA scope)

> **状态核实修正**（2026-09-30）：本节原列 3 项 alpha gate FAIL 为 open。
> 经实跑复核，**2 项已解决、1 项仍未解决**。依据
> `ALIGNMENT_AUDIT_2026-09-30.md` F-01/F-02/F-04 及
> `evidence/gate-runs-2026-09-30/`。

1. **alpha gate 状态（实跑复核 2026-09-30）**：
   - ✅ `check_anti_ignore_gate.sh` — **PASS**, exit 0
     (`ignore_registry.json` 已存在, `total_allowed=125`)
   - ✅ `check_arch_invariants.sh` — **PASS**, exit 0, 5/5
   - ❌ `check_anti_fabrication.sh` **CHECK 4** — **FAIL**
     HEAD author email 实测为 `claude@macmini.dev`，不在
     `check_anti_fabrication.sh:178` 白名单（8 项 `*@gaoyuanyiyao.com` + `ci@sqlrustgo.dev`）内。
     `log_error` 使 `ERRORS>0` → `main()` 退出码 1 → 门禁 FAIL。
     **修复待人工决策**（加白名单 vs 重写提交身份），见审计报告 §6。
   - 注：`PHASE_1_SCOPE.md` §2.1.2 所述 `crates/executor/src/execution_engine.rs`
     2731 行 **不准确** — 该路径不存在；门禁实际读取仓库根 `src/execution_engine.rs`
     （1034 行）。详见审计报告 F-04。

2. **V400-09 168h SOAK FINAL** — ✅ 已完成
   `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` 已存在。

3. **WP-C..G deferred items** — 4 个 WP 组 / 12 条 issue
   （WP-C 6 / WP-D 4 / WP-F 1 / WP-G 1）待迁移至 v4.1.0。
   另加 WP-B 6 条 + WP-E #4626 1 条 + WP-H #4639 1 条，合计 **20**（见 §4.7）。

4. **v4.0.0 STAGE.yaml SSOT contradiction**（DRAFT vs GA_GATE_REPORT.md 声明
   GA CONDITIONAL PASS）— governance gap，待人工决策。

5. **本地 `main` 与 `develop/v4.0.0` 已分叉**（非单纯落后）：
   实测 `main = f8a149b474`，`rev-list --count main..develop/v4.0.0 = 16905`，
   且 `main` **不是** `develop/v4.1.0` 的祖先。原 §1.2 "16902-commit gap closed on main"
   的 DONE 结论不成立，已改为 PARTIAL。

## 4. WP-A..G detailed backlog (v4.1.0 scope)

The following 24 issues are in v4.1.0 scope. 12 were **deferred** from
v4.0.0 to v4.0.1 (now v4.1.0) via `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md`
§Detailed Status (§4.1–§4.4 below); 6 WP-B + 1 WP-E (#4626) are **carry-forward
completions** of partial work; 1 (#4639) is the WP-H v4.1-targeted item;
4 WP-A items (§4.0) are **re-opened** 2026-09-30 after re-verification
found the v4.0.0 closure claim unsupported.

> **计数口径修正**（2026-09-30）：本节原写 "The following 18 issues"，
> 与 §4.7 汇总的 20 不自洽（12 + 6 + 1 + 1 = 20）。已订正为 20 并标明构成。
> 依据：`ALIGNMENT_AUDIT_2026-09-30.md` F-10。
>
> **2026-09-30 第二次修正**：因 WP-A 4 条回填，本节标题从 "WP-C..G" 改为
> "WP-A..G"，总数 20 → **24**。WP-A 编为 **§4.0** 而非 §4.1，是为避免
> 连带重编 §4.1–§4.7 从而破坏 `ALIGNMENT_AUDIT_2026-09-30.md`
> （§4.1/§4.2/§4.7）、`PHASE_1_SCOPE.md`（§4.7）、`DEV_PLAN.md`（§4.7）
> 的既有跨文档小节号引用。**请勿"顺手"把它改成 §4.1**。

### 4.0 WP-A — Parser legacy (4 issues, RE-OPENED 2026-09-30)

> **为什么加回来**（`LEGACY_LEDGER_v3.6_to_v4.1.md` §2.1）：
> 这 4 条在 Gitea 上均为 `closed`（2026-09-03），并被 v4.0.0 文档当作
> 已闭环证据引用。但 2026-09-30 逐条复验发现 **#4708 的核心缺陷从未修复**，
> 且被一条**未登记在 `ignore_registry.json` 的 `#[ignore]`** 掩盖。
> 依据：`docs/releases/v4.1.0/evidence/wp-a-reverify-2026-09-30/wp_a_reverify.txt`
> (sha256 `e5f2820f22adaf24ef20a8e06dd5f86c7c607c5d1498752580c95e68cb1f0d4f`)。

| Issue | Title | Gitea state | 复验结论 | Status | v4.1.0 plan |
|---|---|---|---|---|---|
| #4708 | 中文表名/列名 + 中文注释 + MySQL 反引号 + 双引号标识符 | closed 2026-09-03T18:46:27Z (PR #4746) | ❌ **未修复** — 块注释 `/* */` parser 完全不支持 | **RE-OPENED** | own |
| #4696 | UPDATE 无 WHERE 子句报 parse error | closed 2026-09-03T15:22:19Z | ⚠️ parser 层回归测试通过，E2E 未验证 | **PARTIAL** | own |
| #4710 | TIMESTAMPDIFF unit keyword 被当列名 | closed 2026-09-03T14:27:32Z | ⚠️ parser 层 9 项回归通过，E2E 未验证 | **PARTIAL** | own |
| #4720 | `SET @var` Parse error: Unexpected token LBracket | closed 2026-09-03T14:27:33Z | ⚠️ parser 层 7 项回归通过，E2E 未验证 | **PARTIAL** | own |

**#4708 复验详情（唯一的硬 FAIL）**

实跑 `cargo test -p sqlrustgo-parser --test wp_a_legacy`：

```
test result: ok. 27 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
```

那 1 个 ignored 是 `crates/parser/tests/wp_a_legacy.rs:68`：

```rust
#[ignore] // TODO: 多行注释解析需要修复
fn test_chinese_comment_multi_line() {
    let sql = "SELECT 1 /* comment */";
```

该 `#[ignore]` **不在** `tests/baseline/ignore_registry.json` 中
（registry v3.12.0-V312-95-v2，125 条，全库仅 2 个文件未登记：
本文件 1 条 + `archive/v3.11/archived-crates/graph_cypher_integration_test.rs` 4 条）。
这违反 `ADR-008-test-claim-transparency` P16 门禁"每个 `#[ignore]` 必须登记"的要求。

被掩盖的真实缺陷（临时探针实测，已删除探针文件）：

| SQL | 结果 |
|---|---|
| `SELECT 1 /* comment */` | ❌ `Expected expression` |
| `SELECT /* c1 */ 1` | ❌ `Expected expression` |
| `/* 头部中文注释 */ SELECT 1` | ❌ `Unexpected token: Slash` |
| `SELECT 1; /* 多行中文注释 */` | ✅ OK（分号后被 statement splitter 剥离，不走 parser 主体） |

即：**块注释 `/* */` 在 parser 中整体不支持**——不是"多行中文注释"问题，
是该语法形式本身缺失。影响面远超 #4708 原声明的"中文注释"。
v4.0.0 的 Gitea 关闭说明本身也只声称 2 个子项 OR-downgrade、2 个
anti-regression lockdown，从未声称块注释已实现。

**#4696 / #4710 / #4720 为何只判 PARTIAL**

`wp_a_legacy.rs` 中全部 27 个测试**只断言 `parse(sql).is_ok()`**，即纯
parser AST 接受性，不含 binder / executor / storage。而 3 条 issue 的原始
报告均来自 `sqlrustgo-cli sqlite --batch` 端到端路径。parser 层 PASS
不能证明 E2E 可用。#4710 的实现确实存在（`parser.rs:5778-5812`、
`token.rs:177`、`lexer.rs:539-542`），#4720 有 7 项回归，但均未经 E2E 复验。

**#4708 E2E 补充**：`#4708` 的 Gitea 关闭说明中，子项 #1（中文表名/列名）
与 #3（MySQL 反引号）在 CLI 层被记为 **OR-downgrade**（显式拒绝），
即**未实现**。而 `test_chinese_table_name` / `test_chinese_identifier_quoted`
在 parser 层 PASS，恰好掩盖了这个 OR-downgrade。

### 4.1 WP-C — DDL / Integrity (6 issues)

> **标题来源修正**（2026-09-30）：本节原先使用 "Title (inferred from triage context)"，
> 即**推测标题**。经比对 `docs/releases/v4.0.0/LEGACY_ISSUES.md` §3.1/§3.2 与
> `CLAIM_DOWNGRADE_MANIFEST.md` §3.2，12 条中 9 条与权威源不符，属
> `[AFP-VIOLATION: Type-C 伪证据]`。下表已按权威源订正。
> 依据：`docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` F-08。

| Issue | Title (per v4.0.0 LEGACY_ISSUES §3 — authoritative) | Status | v4.1.0 plan |
|---|---|---|---|
| #4682 | `sqlite_master` / `sqlite_sequence` / `sqlite_temp_master` 缺失 | NOT STARTED | own |
| #4652 | CREATE PROCEDURE/FUNCTION 接受但不存储 (DDL fake-success) | NOT STARTED | own |
| #4672 | SQLite AUTOINCREMENT 仍未生效 | NOT STARTED | own |
| #4669 | 复杂 DROP INDEX / function index / partial index | NOT STARTED | own |
| #4709 | CHECK multi-condition 静默接受非法 row | NOT STARTED | own |
| #4703 | UPSERT / trigger-column syntax failures | NOT STARTED | own |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — "DDL changes
require schema migration v4.0.1".

### 4.2 WP-D — Join / Subquery (4 issues)

| Issue | Title (per v4.0.0 LEGACY_ISSUES §3.5 — authoritative) | Status | v4.1.0 plan |
|---|---|---|---|
| #4668 | NATURAL JOIN / multi-column USING 错误 | NOT STARTED | own |
| #4656 | `> ALL` / `= ANY` 子query 错误 | NOT STARTED | own |
| #4649 | LEFT JOIN USING 退化为笛卡尔积 | NOT STARTED | own |
| #4636 | Correlated scalar subquery 失败 | NOT STARTED | own |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — "graph projection
extends this".

### 4.3 WP-F — Schema Migration (1 issue)

| Issue | Title | Status | v4.1.0 plan |
|---|---|---|---|
| #4848 | ALTER TABLE RENAME COLUMN | NOT STARTED | own |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` §Detailed Status — "ALTER
TABLE RENAME COLUMN is part of schema migration framework (WP-F). v4.0.0 GA
ships without ALTER RENAME COLUMN; documented in
CLAIM_DOWNGRADE_MANIFEST."

### 4.4 WP-G — Type / Comparison (1 issue)

| Issue | Title | Status | v4.1.0 plan |
|---|---|---|---|
| #4846 | CHAR(n) PAD SPACE semantics | NOT STARTED | own |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — "CHAR(n) byte-vs-char
fix requires deep executor changes".

### 4.5 WP-B completion (carry from v4.0.1 partial)

Per `CLAIM_DOWNGRADE_MANIFEST.md §3.1`:

> "Functions with WP-B issues (#4721, #4674, #4716, #4676, #4675, #4670) are
> partially supported; full behavior is v4.0.1."

The 6 WP-B issues (#4721, #4674, #4716, #4676, #4675, #4670) have
partial test coverage in v4.0.0 but require executor changes to close.
v4.1.0 should own the executor-side closure.

### 4.6 WP-E completion (carry from v4.0.1 partial)

Per `CLAIM_DOWNGRADE_MANIFEST.md §3.8` (Cross-model transaction V400-05):

> "WP-E issues (#4847, #4626) are partial via V400-05 scaffold."

- **#4847**: V400-05 cross-model transaction scaffold covers partial.
  Full closure needs v4.1.0 completion.
- **#4626**: not started. Carry-forward to v4.1.0.

### 4.7 Total v4.1.0 backlog size

| Category | Count |
|---|---|
| WP-A Parser legacy (re-opened 2026-09-30, §4.0) | 4 |
| WP-C DDL / Integrity | 6 |
| WP-D Join / Subquery | 4 |
| WP-F Schema Migration | 1 |
| WP-G Type / Comparison | 1 |
| WP-B completion | 6 |
| WP-E completion (#4626) | 1 |
| WP-H #4639 (carried from v4.0.0) | 1 |
| **Total** | **24** |

> 20 → 24 的变更为 2026-09-30 WP-A 回填所致。**同一日新增的门禁债
> `UNREG-IGNORE-WP-A`（`wp_a_legacy.rs:68` 的 `#[ignore]` 未登记）
> 未计入本表**——它属于 §4 范围外的门禁完整性债，登记在
> `STAGE.yaml` `legacy_chain_audit_2026_09_30.gate_debt`，
> 修复后应作为 #4708 的验收前置条件。

**Estimated work**: 6-10 weeks total for v4.1.0 to close these items,
assuming 1-2 issues per WP category per week with proper code review and
testing. This is consistent with `docs/releases/v4.1.0/PHASE_1_SCOPE.md`
§2.2 estimate of "6-8 weeks total (1-2 weeks per WP category)".

**Critical path**: WP-G (#4846 CHAR PAD SPACE, `CLAIM_DOWNGRADE_MANIFEST.md §3.5`)
and WP-F (#4848 ALTER RENAME COLUMN, `CLAIM_DOWNGRADE_MANIFEST.md §3.4`)
have v4.0.0 GA claim-boundary lines carved out in those sections. Closing them requires
executor changes that **must** preserve the v4.0.0 baseline behavior
with new option flags (or new collation types) rather than silently
changing default semantics.

## 5. References

- `docs/releases/v4.0.0/ISSUES_PLAN.md` — full v4.0.0 issue catalog
- `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — WP-A..H triage
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` — GA-claim-caveat items
- `docs/releases/v4.1.0/STAGE.yaml` — v4.1.0 stage state