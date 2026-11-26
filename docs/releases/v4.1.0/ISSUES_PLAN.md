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
| #4639 | RIGHT / FULL OUTER JOIN 语义（WP-H） | DEFERRED → **FIXED 2026-11-26** | 已闭环，见 §4.10 |

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

3. **WP-C..G deferred items** — 原 4 个 WP 组 / 12 条 issue（WP-C 6 / WP-D 4 /
   WP-F 1 / WP-G 1）待迁移至 v4.1.0；**2026-11-26 实跑复验后，其中 11 条已修好并
   移出待办**，只余 WP-C 的 **#4669** 1 条（函数索引 / 部分索引两个子项）。
   WP-B 6 条 + WP-E #4626 1 条：#4626 已修好移出；WP-B 余 **#4716 / #4676** 2 条。
   WP-E 的另一条 **#4847** 此前未被 §4.7 计入，2026-11-26 复核它仍是 partial ⇒ 补入。
   合计待办 **4 条**（总计见 §4.7，逐条判定见 §4.11）。

4. **v4.0.0 STAGE.yaml SSOT contradiction**（DRAFT vs GA_GATE_REPORT.md 声明
   GA CONDITIONAL PASS）— governance gap，待人工决策。

5. **本地 `main` 与 `develop/v4.0.0` 已分叉**（非单纯落后）：
   实测 `main = f8a149b474`，`rev-list --count main..develop/v4.0.0 = 16905`，
   且 `main` **不是** `develop/v4.1.0` 的祖先。原 §1.2 "16902-commit gap closed on main"
   的 DONE 结论不成立，已改为 PARTIAL。

## 4. WP-A..G detailed backlog (v4.1.0 scope)

The following **4** issues are in v4.1.0 scope. The original list was 24 items:
12 **deferred** from v4.0.0 to v4.0.1 (now v4.1.0) via
`docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` §Detailed Status (§4.1–§4.4 below);
6 WP-B + 1 WP-E (#4626) are **carry-forward completions** of partial work.
The 4 WP-A items (§4.0) were **re-opened** 2026-09-30 and **re-verified fixed
2026-11-26**; the WP-H item #4639 (§4.10) was re-verified fixed the same day;
and 16 further items (WP-B/C/D/E/F/G) were re-verified on 2026-11-26 (§4.11).
All of them stay in their sections for traceability but are **not counted** in
the backlog any more. What remains is **4 items**: **#4669** (WP-C), **#4716**
and **#4676** (WP-B) — the three whose defects are still reproducible, all
**re-opened on Gitea** with the measured evidence — plus **#4847** (WP-E), which
is still *partial* (the wire-side path was never implemented) and had not been
counted in §4.7 at all.

> **计数口径修正**（2026-09-30）：本节原写 "The following 18 issues"，
> 与 §4.7 汇总的 20 不自洽（12 + 6 + 1 + 1 = 20）。已订正为 20 并标明构成。
> 依据：`ALIGNMENT_AUDIT_2026-09-30.md` F-10。
>
> **2026-09-30 第二次修正**：因 WP-A 4 条回填，本节标题从 "WP-C..G" 改为
> "WP-A..G"，总数 20 → **24**。WP-A 编为 **§4.0** 而非 §4.1，是为避免
> 连带重编 §4.1–§4.7 从而破坏 `ALIGNMENT_AUDIT_2026-09-30.md`
> （§4.1/§4.2/§4.7）、`PHASE_1_SCOPE.md`（§4.7）、`DEV_PLAN.md`（§4.7）
> 的既有跨文档小节号引用。**请勿"顺手"把它改成 §4.1**。
>
> **2026-11-26 第三次修正**：WP-A 4 条经实跑复验全部 FIXED（parser 层 +
> E2E 层双绿），**移出待办**，总数 24 → **20**。§4.0 保留为可追溯记录，
> 小节编号不动。依据：
> `docs/releases/v4.1.0/evidence/wp-a-reverify-2026-11-26/wp_a_reverify.txt`
> (sha256 `101682c020bd01027fd6003dc235d13a3e292288749708f1a12abe7ea33b9ee1`)；
> 同步修订 `STAGE.yaml` `legacy_chain_audit_2026_09_30.false_done_wp_a`、
> `FEATURE_MATRIX.md` P-01/P-03、`EVALUATION_REPORT.md` D6/D8、
> `VERSION_PLAN.md` §1。
>
> **2026-11-26 第四次修正**：WP-H #4639 复验为 FIXED（§4.10），**移出待办**，
> 总数 20 → **19**。同时 `DEFECTS_AND_ISSUES.md` 的 P1-03 拆成 7 个按 WP 分组的
> 子项（P1-03a…P1-03g），逐组跟踪销账。依据：
> `docs/releases/v4.1.0/evidence/wp-h-reverify-2026-11-26/wp_h_reverify.txt`
> (sha256 `6848949dd58c795e6813fbd1875f4ce8883efa4ef5aa1edf0b1b6a343e716692`)。

### 4.0 WP-A — Parser legacy (4 issues, RE-OPENED 2026-09-30 → RE-VERIFIED FIXED 2026-11-26)

> **为什么加回来**（`LEGACY_LEDGER_v3.6_to_v4.1.md` §2.1）：
> 这 4 条在 Gitea 上均为 `closed`（2026-09-03），并被 v4.0.0 文档当作
> 已闭环证据引用。但 2026-09-30 逐条复验发现 **#4708 的核心缺陷从未修复**，
> 且被一条**未登记在 `ignore_registry.json` 的 `#[ignore]`** 掩盖。
> 依据：`docs/releases/v4.1.0/evidence/wp-a-reverify-2026-09-30/wp_a_reverify.txt`
> (sha256 `e5f2820f22adaf24ef20a8e06dd5f86c7c607c5d1498752580c95e68cb1f0d4f`)。
>
> **2026-11-26 复验结论（本文档此前的 ❌/⚠️ 判定已失效）**：
> 2026-09-30 的复验在**当时是对的**，但 **#4708 的块注释缺陷已于 2026-10-08
> 在 `Lexer::skip_whitespace` 修好**（`#[ignore]` 被移除、测试保留为回归见证，
> 见 `tests/baseline/ignore_registry.json` → `resolved_removed_ignores`），
> 而 v4.1.0 的这些记录**没有随之刷新**，于是"未修复 / PARTIAL"被一直带到了
> 2026-11-26。2026-11-26 实跑复验（base commit `dc9c035691`）：
> parser 层 35/35 绿（0 ignored），E2E 层逐条绿，且为 #4708 补上了**永久**
> E2E 见证 `tests/integration/sql/chinese_identifier_e2e_test.rs`（3/3 绿）。
> 依据：
> `docs/releases/v4.1.0/evidence/wp-a-reverify-2026-11-26/wp_a_reverify.txt`
> (sha256 `101682c020bd01027fd6003dc235d13a3e292288749708f1a12abe7ea33b9ee1`)。
> **四条 issue 在 Gitea 上保持 closed（2026-09-03），不做 reopen。**

| Issue | Title | Gitea state | 复验结论 | Status | v4.1.0 plan |
|---|---|---|---|---|---|
| #4708 | 中文表名/列名 + 中文注释 + MySQL 反引号 + 双引号标识符 | closed 2026-09-03T18:46:27Z (PR #4746) | 2026-09-30: ❌ **未修复**（块注释 parser 完全不支持）→ **2026-11-26: ✅ 已修复**（块注释 2026-10-08 已修；parser 35/35 + 新增永久 E2E 见证 3/3） | **FIXED**（2026-11-26 复验） | 无（已闭环） |
| #4696 | UPDATE 无 WHERE 子句报 parse error | closed 2026-09-03T15:22:19Z | 2026-09-30: ⚠️ parser 层通过、E2E 未验证 → **2026-11-26: ✅ parser + E2E 双绿**（`UPDATE t SET a=99` → 3 affected rows） | **FIXED**（2026-11-26 复验） | 无（已闭环） |
| #4710 | TIMESTAMPDIFF unit keyword 被当列名 | closed 2026-09-03T14:27:32Z | 2026-09-30: ⚠️ parser 层 9 项通过、E2E 未验证 → **2026-11-26: ✅ parser 9 项 + E2E delta 用例双绿** | **FIXED**（2026-11-26 复验） | 无（已闭环） |
| #4720 | `SET @var` Parse error: Unexpected token LBracket | closed 2026-09-03T14:27:33Z | 2026-09-30: ⚠️ parser 层 7 项通过、E2E 未验证 → **2026-11-26: ✅ parser + MySQL 协议 E2E（`SET @x = 1`）双绿** | **FIXED**（2026-11-26 复验） | 无（已闭环） |

**#4708 复验详情（2026-09-30 的唯一硬 FAIL —— 已被 2026-10-08 的修复推翻）**

> ⚠️ **以下内容是 2026-09-30 的历史记录，保留作审计留痕，不再代表当前状态。**
> 其中"块注释 parser 完全不支持""27 passed; 1 ignored"等结论在 2026-10-08
> 修复块注释支持后即失效；当前状态见上表与
> `docs/releases/v4.1.0/evidence/wp-a-reverify-2026-11-26/wp_a_reverify.txt`。

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
| #4682 | `sqlite_master` / `sqlite_sequence` / `sqlite_temp_master` 缺失 | **FIXED**（2026-11-26 复验；见 §4.11） | — |
| #4652 | CREATE PROCEDURE/FUNCTION 接受但不存储 (DDL fake-success) | **FIXED**（2026-11-26 复验；CLI 显式报错 + 执行器落库） | — |
| #4672 | SQLite AUTOINCREMENT 仍未生效 | **FIXED**（2026-11-26 复验） | — |
| #4669 | 复杂 DROP INDEX / function index / partial index | ❌ **NOT FIXED** — 4 子项修好 2（DROP INDEX / CREATE FUNCTION 体）；**函数索引与部分索引仍复现原症状**，已重开 | own |
| #4709 | CHECK multi-condition 静默接受非法 row | **FIXED**（2026-11-26 复验） | — |
| #4703 | UPSERT / trigger-column syntax failures | **FIXED**（2026-11-26 复验；CLI 侧 VALUES() upsert 仍显式拒收＝OR-downgrade） | — |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — "DDL changes
require schema migration v4.0.1".

### 4.2 WP-D — Join / Subquery (4 issues)

| Issue | Title (per v4.0.0 LEGACY_ISSUES §3.5 — authoritative) | Status | v4.1.0 plan |
|---|---|---|---|
| #4668 | NATURAL JOIN / multi-column USING 错误 | **FIXED**（2026-11-26 复验；已超越原 OR-downgrade 口径） | — |
| #4656 | `> ALL` / `= ANY` 子query 错误 | **FIXED**（2026-11-26 复验；相关量化子查询仍是保守降级） | — |
| #4649 | LEFT JOIN USING 退化为笛卡尔积 | **FIXED**（2026-11-26 复验） | — |
| #4636 | Correlated scalar subquery 失败 | **FIXED**（2026-11-26 复验） | — |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — "graph projection
extends this".

### 4.3 WP-F — Schema Migration (1 issue)

| Issue | Title | Status | v4.1.0 plan |
|---|---|---|---|
| #4848 | ALTER TABLE RENAME COLUMN | **FIXED**（2026-11-26 复验；默认开启无开关，已触及 v4.0.0 claim-boundary §3.4） | — |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` §Detailed Status — "ALTER
TABLE RENAME COLUMN is part of schema migration framework (WP-F). v4.0.0 GA
ships without ALTER RENAME COLUMN; documented in
CLAIM_DOWNGRADE_MANIFEST."

### 4.4 WP-G — Type / Comparison (1 issue)

| Issue | Title | Status | v4.1.0 plan |
|---|---|---|---|
| #4846 | CHAR(n) PAD SPACE semantics | **FIXED**（2026-11-26 复验；仅 `=`/`IN`/`BETWEEN` 走 PAD SPACE，排序比较仍 BINARY；已触及 claim-boundary §3.5） | — |

**Source**: `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — "CHAR(n) byte-vs-char
fix requires deep executor changes".

### 4.5 WP-B completion (carry from v4.0.1 partial)

Per `CLAIM_DOWNGRADE_MANIFEST.md §3.1`:

> "Functions with WP-B issues (#4721, #4674, #4716, #4676, #4675, #4670) are
> partially supported; full behavior is v4.0.1."

The 6 WP-B issues (#4721, #4674, #4716, #4676, #4675, #4670) have
partial test coverage in v4.0.0 but require executor changes to close.
v4.1.0 should own the executor-side closure.

**2026-11-26 实跑复验（逐条判定见 §4.11）**：

| Issue | 判定 | 依据 |
|---|---|---|
| #4721 | **FIXED** | `round(3.5,0)` / `round(70.0*0.5,2)` → `4.0` / `35.0`（Float），parse_lit 不再截断为 Integer |
| #4674 | **FIXED** | `CHAR_LENGTH('abc')` → 3（codepoint 计数） |
| #4675 | **FIXED** | `POSITION('bc' IN 'abcd')` → 2；`LOCATE('bc','abcd',2)` → 2；16 条 E2E 绿 |
| #4670 | **FIXED** | `MD5/SHA1/SHA2/UNHEX/CEIL/FLOOR/TRUNCATE/HEX` 全部实装且实测正确（**但哈希类零回归覆盖**） |
| #4716 | ❌ **NOT FIXED** | `DATE_TRUNC` 已有真实计算，但缺 millennium/century/decade/microsecond；未知单位静默返回 NULL（spec 要求抛错）；输出格式与 spec 不符 —— **已重开** |
| #4676 | ❌ **NOT FIXED** | 主体 5 个函数已修且 16 条 E2E 绿，但 `POW` / `LN` 别名与 `LOG(base,x)` 双参、混合型 `MOD` 仍为 NULL —— **已重开** |

### 4.6 WP-E completion (carry from v4.0.1 partial)

Per `CLAIM_DOWNGRADE_MANIFEST.md §3.8` (Cross-model transaction V400-05):

> "WP-E issues (#4847, #4626) are partial via V400-05 scaffold."

- **#4847**: V400-05 cross-model transaction scaffold covers partial.
  Full closure needs v4.1.0 completion。**2026-11-26 复核：仍属 partial** ——
  引擎/CLI 两条路径已修，但 wire 端「每语句新建连接 ⇒ 显式事务无法跨语句存活」
  未实现；`crates/transaction/tests/v400_cross_model_tx.rs` 是纯内存自模拟，
  不接触真实引擎/线协议路径，不构成闭合证据。
- **#4626**: **FIXED**（2026-11-26 复验）—— issue 于 2026-09-03 即由 PR #4743
  关闭（CLI 顶层 BEGIN 前预冲刷隐式事务 + 引擎隐式 TX drain + ROLLBACK 状态复位），
  `cargo test --test issue_4626_select_for_update_rollback_test` → 3/3 绿。
  残留：引擎层 `SELECT ... FOR UPDATE` 未真正取行锁；`crates/transaction/tests/wp_e_legacy.rs`
  的 8 条用例是同义反复假测试，不构成证据。

### 4.7 Total v4.1.0 backlog size

| Category | Count |
|---|---|
| WP-C DDL / Integrity（#4669 函数索引 / 部分索引两个子项） | 1 |
| WP-B completion（#4716 DATE_TRUNC、#4676 数学函数别名） | 2 |
| WP-E 其余项（#4847，partial，此前未计入） | 1 |
| **Total** | **4** |

> 计数沿革：18 → 20（2026-09-30 第一次修正）→ 24（2026-09-30 第二次修正，
> 因 WP-A 4 条回填）→ 20（2026-11-26 第三次修正：WP-A 4 条复验为 FIXED，
> 移出待办；§4.0 保留可追溯记录，小节编号不动）→ 19（2026-11-26 第四次修正：
> WP-H #4639 复验为 FIXED，移出待办；§4.10 保留记录，编号不动）→ **3**
> （2026-11-26 第五次修正：WP-B/C/D/E/F/G 逐条实跑复验，16 条 FIXED 移出待办，
> 3 条缺陷仍可复现已重开；逐条判定见 §4.11）。**2026-11-26 第六次修正**：
> WP-E 的 #4847 此前未计入本表（复核后确认仍 partial），补入 ⇒ 总数 3 → **4**。
> 2026-09-30 提到的门禁债 `UNREG-IGNORE-WP-A`（`wp_a_legacy.rs:68` 的
> `#[ignore]` 未登记）**已于 2026-10-08 消解**：该 `#[ignore]` 随块注释支持
> 一并移除，registry 以 `resolved_removed_ignores` 记录，故不再计入本表。

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

## 4.8 并发测量发现的阻塞缺陷（2026-10-04）

补跑 B2.2 的并发 A/B 时发现三个缺陷。前两个**先于 Phase B2 存在**（在 baseline
`be665d6bc1` 上同样复现），且它们是 B2.2 并发验证拿不到数字的直接原因；第三个
在修复 BLK-1 后才暴露出来。证据见
[`PERF_B2_4915_AB_MEASUREMENT.md`](./PERF_B2_4915_AB_MEASUREMENT.md) §6。

| # | 缺陷 | 根因 | 影响 | 状态 |
|---|---|---|---|---|
| **BLK-1** | 无 auto-increment 分配器，并发 INSERT 撞主键 | `engine_dml.rs` 用**写锁之外**的 `pre_scanned_rows` 算 `MAX(id)+1`，两个并发 INSERT 读到同一个 MAX | `Duplicate entry '...' for key 'PRIMARY'`，sysbench `oltp_read_write` / `oltp_write_only` 全部中止 | **FIXED** `3c64dc19ec` |
| **BLK-2** | `*_transaction_lockfree(&self)` 从读守卫洗出 `&mut S`，与别的连接的写守卫别名 | `WalStorage::as_inner_mut()` 从 `&self` 派生 `&mut S`；其文档写的前提"engine 用自己的 mutex 串行化"在服务端不成立（每连接一个 engine，共享同一 storage） | 并发读写把服务端打死；`sample` 2458 样本全在 `rollback_transaction_lockfree` 与 `RawRwLock::lock_{shared,exclusive}_slow` | **FIXED** `8ff90269d3` |
| **BLK-3** | AUTO_INCREMENT 序列在多个 autocommit 事务间重用 id | 修复 BLK-1 后暴露：8 线程 × 60 次 INSERT 全部成功且无重复 id，但 480 行只用了 id 1..99 | id 语义错误（MySQL 要求跨事务单调递增）；同时 `COUNT(*)` 只返回 257，重启后 0 行——**另有持久化缺口** | OPEN |

**BLK-1 修复**：把 id 分配移入 `engine.storage.write()` 临界区，读 `MAX(id)` 与
后续 INSERT 对其它写者成为原子步骤。回归测试
`tests/blk1_auto_increment_concurrency_test.rs`（5 例，先红后绿：修前
4 线程 × 25 产生 10 个 `Duplicate entry` 错误）。服务端验证：8 线程 × 60
共 480 次 INSERT，0 报错、0 重复 id。

**BLK-2 修复**：新增 `&self` trait 方法
`set_current_tx_id_shared` / `discard_all_buffers_shared`——
`MemoryStorage` 用 `AtomicU64`，`FileStorage` 走自身内部 `write_lock`，
`MvccStorage` 与 `BoxStorageEngine` 转发。**转发是关键**：
`BoxStorageEngine` 是类型擦除包装，漏掉 override 会静默回落到 trait 的
no-op 默认而非编译失败，而所有服务端创建的 storage 都被它包着——漏转发会让
整个修复在生产环境失效、而所有具体后端的单测照常通过。`as_inner_mut` 已删除，
不留未用的 `&mut`-from-`&self` 陷阱。两份测试：转发侧
`tests/blk2_shared_tx_path_test.rs`（4 例），行为侧
`crates/storage/tests/blk2_lockfree_tx_concurrency_test.rs`（3 例，
**需加 `--test-threads=1`**，否则 libtest 每线程 2 MiB 栈会溢出）。
修前的红是**挂死**而非断言失败，所以 harness 必须带外部 timeout。

**BLK-3 备注**：修复 BLK-1 时为让 `set_current_tx_id_shared` 真正安全，
把 `MemoryStorage.current_tx_id` 改成了 `AtomicU64`。这本身不解释 id 重用，
但暴露了事务边界上 id 状态被跨连接共享的问题——`current_tx_id` 是单个存储级
字段，而 AUTO_INCREMENT 计数器按 MySQL 语义应当是每表单调递增的会话/表级
状态。二者需要分开建模。持久化缺口（`flush()` 只在启动恢复与 LOAD DATA 末尾
调用，周期性/每事务 flush 不存在）同样独立，应单开 issue。

### 4.9 B2.2 重定向时发现的两个正确性缺陷（2026-10-04）

合并 inherent `FileStorage::flush(&self)` 与 `StorageEngine::flush(&mut self)`
——同一段逻辑的两份独立拷贝——时，在 `flush_parallel` 里发现三个缺陷
（`557fe61a74` 修复）。**前两个是正确性问题，不是性能问题**，且都发生在
B2 动工之前。

| # | 缺陷 | 后果 | 状态 |
|---|---|---|---|
| **BLK-4** | `flush_parallel` 的 `dirty.len() <= 2` 分支先 drain `dirty_tables`，再调 `self.flush()`——后者看到的是空集合 | **持久化的行被静默丢弃** | ✅ `557fe61a74` |
| **BLK-5** | `flush_parallel` 的 3+ 分支在 spawn 的线程里读 `self.tables.get(name)`，而该 HashMap 由 `write_lock` 保护 | **data race**（不只是陈旧读） | ✅ `557fe61a74` |

第三个缺陷（两者都未先 push `insert_buffer` 进 `tables`，导致窗口为空、
缓冲插入从未落盘）随合并一并修复，未单独立项——它正是两份实现漂移的证据：
trait 覆写一直会做这一步，inherent 版本没有。

**为什么单独立项而不在 BLK 表里**：这两条是数据正确性与内存安全缺陷，
量级不同于 BLK-1/2/3 的性能与稳定性问题，修复它们的动因也不是"为了测
B2.2"——整理重复实现时顺带暴露出来的。

回归测试 `crates/storage/tests/b22_flush_reroute_test.rs`（5 例），先红后绿：
修前 5 例全红（`flush_parallel_persists_one_table: left: 0, right: 2`），
修后全绿。其中一例断言"表增长后写 delta 而非重写基础快照"，同时校验内容与
mtime，重写无法蒙混过关。

### 4.10 WP-H — #4639 RIGHT / FULL OUTER JOIN 语义（复验 FIXED 2026-11-26）

> **为什么放在这里**：WP-H 是 v4.1.0 台账里最后一条 legacy backlog（见 §1.2 与
> §4.7）。2026-09-30 的分诊快照把它记成「#4639 to v4.1 / Own in v4.1.0」，但
> **Gitea 上该 issue 早在 2026-09-05 就已 closed**（"closing as already-fixed"，
> 附实现位置与两条回归测试）。2026-11-26 实跑复验确认：过期的是**记录**，不是代码。
>
> 依据：`docs/releases/v4.1.0/evidence/wp-h-reverify-2026-11-26/wp_h_reverify.txt`
> (sha256 `6848949dd58c795e6813fbd1875f4ce8883efa4ef5aa1edf0b1b6a343e716692`)

| Issue | Title | Gitea state | 复验结论 | Status | v4.1.0 plan |
|---|---|---|---|---|---|
| #4639 | RIGHT JOIN / FULL OUTER JOIN 退化为 INNER JOIN | closed 2026-09-05T13:42:25Z（openclaw: "closing as already-fixed"） | ✅ **已修复** — `cargo test --test multi_join_test` 7/7 绿（含 `test_right_join_preserves_right_rows`、`test_full_outer_join_preserves_both_sides`）；issue 原始复现逐条比对：LEFT 3 行 / RIGHT 2 行（无 bob）/ FULL OUTER 3 行，与 issue 期望完全一致；实现见 `src/engine_select.rs`（hash-join 臂与 ON 表达式臂均对 Right/Full 补 NULL），planner 有 `JoinType::Right` / `JoinType::Full`，optimizer 无 Right/Full→Inner 改写规则 | **FIXED**（2026-11-26 复验） | 无（已闭环） |

**处置**：保持 closed，不做任何状态变更；WP-H 从 v4.1.0 待办移出（backlog 20 → 19）。
**台账同步**：`DEFECTS_AND_ISSUES.md` P1-03 拆为 7 个 WP 子项，其中 P1-03g
（WP-H）销账；`LEGACY_ISSUES.md` WP-H 行、`STAGE.yaml` `wp_h_v313_defer` /
`phase_1_alpha_status.v410_09…`、`FEATURE_MATRIX.md` P-03、`EVALUATION_REPORT.md` D8、
`FEATURE_CHECKLIST.md`、`GA_RELEASE_TIMELINE.md`、`VERSION_PLAN.md` §1 同步修订。

### 4.11 WP-B..G 逐条实跑复验（2026-11-26，16 条 FIXED / 3 条重开）

> **方法**：按 WP 组逐条取 Gitea issue（状态 + 关闭说明）→ 定位当前实现 → 跑**真跑**
> 回归测试或最小 SQL 探针 → 判定。2026-09-30 的分诊快照把这些条目记成
> 「NOT STARTED / DEFERRED」，实测证明**其中 16 条早已修好**，过期的是台账不是代码。
> 依据：`docs/releases/v4.1.0/evidence/wp-b-g-reverify-2026-11-26/wp_b_g_reverify.txt`
> (sha256 `e82950d96e45f453a7de6f83cfcafdbd57402a0ac4f089a5f380f46dc64d53e5`)。

| Issue | WP | 判定 | 关键证据 |
|---|---|---|---|
| #4682 | C | ✅ FIXED | `v312_71_sqlite_master_test` 4/4、`v312_76_sqlite_system_tables_test` 10/10 |
| #4652 | C | ✅ FIXED | CLI 显式报错（`sqlite_mode.rs:325-366`）+ 执行器落库；`issue_4513_create_procedure_test` 13/13 |
| #4672 | C | ✅ FIXED | id 分配在写锁内（`src/engine_dml.rs:565-595`）；`v312_76…` 10/10 |
| #4669 | C | ❌ **NOT FIXED（已重开）** | DROP INDEX 6/6 绿；但 `CREATE INDEX i1 ON t(UPPER(a))` → `Column not found`、`CREATE INDEX i2 ON t(a) WHERE a > 200` → **WHERE 被静默丢弃** |
| #4709 | C | ✅ FIXED | `repro_v312_85_check` 4/4（issue 原文 `CHECK (val>0 AND val<100)` 插 200 被拒） |
| #4703 | C | ✅ FIXED | `issue_4703_on_duplicate_test` 3/3、`v312_75_update_of_trigger_test` 5/5 |
| #4668 | D | ✅ FIXED | NATURAL JOIN / 多列 USING 真实实现；`wp_d_legacy` 13/13 |
| #4656 | D | ✅ FIXED | `v312_66_quantified_any_all_test` 7/7 |
| #4649 | D | ✅ FIXED | `v312_73_join_using_clause_test` 5/5（LEFT JOIN USING 不再是笛卡尔） |
| #4636 | D | ✅ FIXED | `v312_75_correlated_scalar_subquery_test` 6/6 |
| #4848 | F | ✅ FIXED | `issue_4848_alter_rename_column_test` 3/3、`wp_f_legacy` 8/8、`ddl_e2e_test` 1/1 |
| #4846 | G | ✅ FIXED | `issue_4846_char_pad_space_test` 3/3、`pk_fast_path_char_pad_space_4846` 5/5、`operators_char_padding` 4/4 |
| #4721 | B | ✅ FIXED | `round(3.5,0), round(70.0*0.5,2)` → `4.0, 35.0`（Float） |
| #4674 | B | ✅ FIXED | `CHAR_LENGTH('abc')` → 3；`issue_4674_char_length_test` 3/3 |
| #4675 | B | ✅ FIXED | `POSITION('bc' IN 'abcd')` → 2；`issue_4675_position_locate_test` 16/16 |
| #4670 | B | ✅ FIXED | `MD5/SHA1/SHA2/UNHEX/CEIL/FLOOR/TRUNCATE/HEX` 实测全部正确 |
| #4626 | E | ✅ FIXED | `issue_4626_select_for_update_rollback_test` 3/3 |
| #4716 | B | ❌ **NOT FIXED（已重开）** | `DATE_TRUNC` 单位集缺 4 个；未知单位静默 NULL（spec 要求抛错）；输出格式与 spec 不符 |
| #4676 | B | ❌ **NOT FIXED（已重开）** | `POW`/`LN` 别名 → NULL；`LOG(100,10)` → 4.605（应 2）；`MOD(10.5,3)` → NULL |

**Gitea 处置**：16 条保持 `closed` 并留复验评论；3 条（#4669 / #4716 / #4676）**已重开**，
评论附实测证据。**backlog 19 → 3**；另有 **#4847**（WP-E）复核后仍是 partial 且此前
未计入计数 ⇒ 补入，**最终 backlog = 4**。

**本轮同时暴露的账外债（需另立条目）**

1. **假绿测试**：`crates/types/tests/wp_b_legacy.rs`、`crates/transaction/tests/wp_e_legacy.rs`
   的用例形如 `assert_eq!(expected, expected)`，**SQL 从未执行** —— 而 v4.0.0 台账
   正是拿这两个文件当「partial test coverage」的依据。`crates/executor/tests/wp_c_legacy.rs`
   与 `crates/storage/tests/wp_f_legacy.rs` 同族但为真跑用例。
2. **过期的 `#[ignore]` GAP 注释**：`crates/executor/tests/wp_c_legacy.rs` 的 8 条
   ACTIVE ignore 里，6 条的 GAP 断言已不成立（实跑 `--ignored` 结果：6 FAILED＝缺陷已修，
   2 PASS＝GAP 仍真）；其中 `sqlite_master` 一条只在 **executor crate 的独立引擎**里仍缺
   （根包 `src/system_tables.rs` 已有）。registry 与注释都需重写。
3. **claim-boundary 被默认开启的改动**：#4848 / #4846 的修复 PR 直接合进 `develop/v4.0.0`
   且**无选项开关**，与 §4.3 / §4.4 关键路径要求的「用新开关实现，不静默改默认语义」冲突；
   声明侧文档（v4.0.0/v4.1.0 `CLAIM_DOWNGRADE_MANIFEST.md`、`MIGRATION_GUIDE.md`、
   `README.md`）仍写「deferred / 不支持」⇒ **需属主裁决**。

## 5. Alpha→Beta 整改计划（2026-10-08）

> 本节是工作分解和验收入口，不代表门禁已经通过。总控 Issue：#5117。

| 优先级 | Issue | 工作包 | Alpha→Beta 验收摘要 |
|---|---|---|---|
| P0 | #5112 | 事务与会话上下文正确性 | rollback `0/300`、8000 事务模型一致、多连接/多库不串扰 |
| P0 | #5113 | CI 与门禁可信度 | workflow 可解析、fail-closed、P11-P16 与 anti-ignore 自测通过 |
| P1 | #5114 | BustubX-EDU 与回归矩阵 | B 轨必测 100%、SQL corpus `>=80%`、结果 oracle 无差异 |
| P1 | #5115 | SOAK、恢复与故障注入 | 1h Alpha SOAK 成功；24h/168h runner 和证据链可执行 |
| P1 | #5116 | 性能基线与回归门禁 | 固定环境 A/B、>=5 次、吞吐和 p99 阈值可自动判定 |

既有依赖：#5099、#5057、#5025、#5102、#5103。这些 Issue 不得由新工作包重复替代；
必须按各自合并 PR 和证据关闭。

总控 #5117 的关闭条件为：#5112..#5116 与既有 P0/P1 阻断项全部关闭，AB-01..AB-10
在单一冻结提交上全部 PASS，required checks 全部成功，且 codeowner 与 governance 独立批准。
详细标准见 `ALPHA_TO_BETA_GATE_PLAN.md`。

### 5.1 已合并代码进展快照（2026-10-08，rebase 至 `d312683244`）

> 本表只登记**已合并到 `develop/v4.1.0`** 且影响 Alpha→Beta 范围的 PR；
> 不影响 issue 的 OPEN/CLOSED 状态 —— 是否关闭以 Gitea 实际状态为准。

| 关联 Issue | 已合并 PR | 内容 | 是否关闭 Issue |
|---|---|---|---|
| #5025 | #5111 | SHOW TABLES FROM 接通目标库 + 修 FileStorage default 分支表泄漏 | 否，仍有未完成子项 |
| #5057 | #5108 / #5109 / #5110 / #5118 / #5122 / #5124 / #5127 | SessionContext 多库隔离；MemoryStorage `*_in_db` 14 个全覆盖；FileStorage 剩余 2 个补齐；engine 读路径补库参数归零；ignore_registry 同步 | 否，issue 本体未关闭 |
| #5099 | b0504ab0e2（docs only） | 并发事务丢行排查证据与修订后设计 | 否，事务上下文重构未动工 |
| #5103 | #5126 | 过程体内无 FROM SELECT 被当 0 行修复 | 是（待复核） |
| #5117 | #5120 | 多表 UPDATE 静默 no-op 报 affected=1 修复 | 否，总控仍 open |
| #4944 | 790b3fd64d | check_coverage_v312.sh fail-open 修复（P1-01） | n/a，子项 |
| #5106 | #5119 | ignore_registry 同步 #5106 的 4 修 2 增 | n/a，配套 |

**注意**：上表只反映"代码已合入 develop"，不等于 AB-01..AB-10 任何一条已 PASS。
Alpha→Beta 的判定仍以 §4.7 全部 backlog 清零 + 冻结 commit 上完整复核为准。

## 6. References


- `docs/releases/v4.0.0/ISSUES_PLAN.md` — full v4.0.0 issue catalog
- `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — WP-A..H triage
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` — GA-claim-caveat items
- `docs/releases/v4.1.0/STAGE.yaml` — v4.1.0 stage state
