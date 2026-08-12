<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `d77821f6d1`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
> - `src/execution_engine.rs` 1471 行 (AD-001 1500 目标达标, 2630 → 1471)
> - C-ARCH-05 上限锁回 1500 (从 3000/1800 统一)
> - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
> - Open issues (4, 全部硬件阻塞, 本机无法推进):
>   - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
>   - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
>   - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
>   - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)
> - 详见: issue #3667 (closed as state snapshot) + CHANGELOG.md
>
> 本文件原始内容保持不变,仅顶部加 addendum。

---

# v3.9.0 文档一致性检查 (Comprehensive Audit Plan)

> **Date**: 2026-06-13 03:15 CST
> **Author**: Hermes Agent
> **Reference**: `docs/governance/DOC_CHECK_CORRECTION_RULES.md` v1.0.0
> **Scope**: v3.9.0 documentation (all 30+ files in `docs/releases/v3.9.0/` + root README/CHANGELOG/ROADMAP)
> **Actual git state**:
> - Latest tag: `v3.9.0-rc7` at `642ff9cf9` (2026-06-12 16:52:28)
> - Current tip: `8a83e2553` (2026-06-13 03:00)
> - GA Target: 2026-12-15 (per Hermes audit #3252)
> - Latest stable: v3.8.0-GA (2026-06-08)
> - 250 24h soak: 2513+ samples, 0 errors, ~5h elapsed

## 1. Problem List (from Step 1)

| # | File | Problem | Location | Evidence |
|---|------|---------|----------|----------|
| 1 | `README.md` (root) | Current dev branch tip `3c051c458` stale (实际 `8a83e2553`) | line 4 | git log |
| 2 | `README.md` (root) | "Latest beta: v3.8.0-rc1" should be **v3.9.0-rc7** (the current dev RC, not v3.8.0-rc1) | line 6 | tag list shows v3.9.0-rc7 is latest |
| 3 | `README.md` (root) | Badge shows `v3.7.0-GA` (actual latest stable is **v3.8.0**) | line 10 | v3.8.0 GA tag exists |
| 4 | `README.md` (root) | Badge shows `v3.8.0-Strong Beta` (actual: **v3.8.0-GA**) | line 11 | v3.8.0 GA tag exists |
| 5 | `CHANGELOG.md` (root) | "当前状态: RC7 (2026-06-12)" — correct, but no recent commit mention (#3370 ODUK) | line 14 | missing post-#3370 sync |
| 6 | `CHANGELOG.md` (root) | v3.8.0 GA Final "v3.8.0 GA Final Breaking Changes" 不准确 — 实际 "No breaking changes" | line 62-68 | v3.8.0 release notes |
| 7 | `docs/releases/v3.9.0/CHANGELOG.md` | Line 7 still says "GA 目标: 2026-09-23" (should be 2026-12-15) | line 7 | Hermes audit #3252 deferred it |
| 8 | `docs/releases/v3.9.0/CHANGELOG.md` | Line 13 "v3.9.0 (Unreleased - 2026-09-23 目标)" stale | line 13 | GA target moved |
| 9 | `docs/releases/v3.9.0/CHANGELOG.md` | Line 88 "(unreleased, 2026-09-23 目标)" stale | line 88 | same |
| 10 | `docs/releases/v3.9.0/README.md` | Directory tree lists `RC1/RC2/RC3_*` files but NO RC4/RC5/RC6/RC7 docs (only RC3 files exist; later RCs not in rc/ subdir) | lines 42-46 | ls rc/ shows only RC1-3 files |
| 11 | `docs/releases/v3.9.0/README.md` | `ga/` directory referenced at line 47 doesn't exist | line 47 | ls ga/ → No such file |
| 12 | `docs/releases/v3.9.0/GA_GATE_REPORT.md` | Line 5: "Latest tag: v3.9.0-rc7 at `0868910f1`" — actual is `642ff9cf9` (the `0868910f1` was a different commit hash from an earlier session) | line 5 | git show-ref shows `642ff9cf9` is rc7 |
| 13 | `docs/releases/v3.9.0/GA_GATE_REPORT.md` | Line 65-66, 98-100: tag references to v3.9.0-rc4 binary (correct for 250 soak), but doesn't mention rc5/rc6/rc7 binaries | lines 65-66, 98-100 | rc5/rc6/rc7 are documented in CHANGELOG but not in GA_GATE_REPORT |
| 14 | `docs/releases/v3.9.0/GA_GATE_REPORT.md` | Line 71: "Z6G4 5th outage: 2026-06-12 17:21" — actual count was 5+ (now in 6th recovery) | line 71 | session memory shows 6+ outages |
| 15 | `docs/releases/v3.9.0/GA_GATE_STATUS_REPORT.md` | Line 4: "Tag: v3.9.0-rc7 (`0868910f1`) — current tip `541b63c70`" — tip is now `8a83e2553`, tag still wrong | line 4 | current tip moved |
| 16 | `docs/releases/v3.9.0/GA_GATE_STATUS_REPORT.md` | Line 12: Tag `v3.9.0-rc7 (`0868910f1`)` — same as #15 | line 12 | same |
| 17 | `docs/releases/v3.9.0/GA_GATE_STATUS_REPORT.md` | Line 46: C-ARCH-05 line count "1919 > 1800" — accurate but refers to "1863 > 1800" in quote. Old quote from earlier session, current is 1919 | line 46 | wc -l src/execution_engine.rs |
| 18 | `docs/releases/v3.9.0/GA_GATE_STATUS_REPORT.md` | Line 137: "Tag v3.9.0-rc5" — should be **v3.9.0-ga-candidate** (rc5 already exists, this would be rc5) | line 137 | rc5/rc6/rc7 already exist |
| 19 | `docs/releases/v3.9.0/evidence/00-release-summary.md` | "Tag (latest): v3.9.0-rc7 @ `0868910f1`" — wrong SHA | line 9 | git show-ref |
| 20 | `docs/releases/v3.9.0/evidence/00-release-summary.md` | "Tag (current tip): `541b63c70`" — current is `8a83e2553` | line 10 | git log |
| 21 | `docs/releases/v3.9.0/evidence/09-ci-build-log.md` | "git checkout v3.9.0-rc7" — valid, but no mention of post-rc7 commits | line 28 | docs lag |
| 22 | `docs/releases/v3.9.0/evidence/10-approval-record.md` | "Tag v3.9.0-rc5" — already exists, should be **v3.9.0-ga-candidate** | line 35 | rc5 exists |
| 23 | `docs/releases/v3.9.0/EVIDENCE_STATUS.md` | "Generated: 2026-06-07" — outdated, but this is a "status doc" (designed to be regenerated) | line 3 | doc design |
| 24 | `docs/releases/v3.9.0/CHANGELOG.md` | Date "2026-06-05" for rc1-rc2 entries (correct per tag date) but check rc3-rc7 have correct dates | line 89-97 | tags are 2026-06-12 |
| 25 | `docs/releases/v3.9.0/plans/` | No doc correction needed; the development plan is historical | n/a | n/a |

**Total: 25 problems identified across 11 files**

---

## 2. Modification Operations

### 2.1 `README.md` (root)

**Operation 1**: Update tip SHA + latest beta
- oldString: `> **Current dev branch**: \`3c051c458\` @ develop/v3.9.0\n> **Latest stable**: v3.8.0 (GA, 2026-06-08) | v3.9.0-rc7 (in soak, GA target 2026-12-15)\n> **Latest beta**: v3.8.0-rc1 (RC, 2026-06-05, TPC-H 22/22)`
- newString: `> **Current dev branch**: \`8a83e2553\` @ develop/v3.9.0\n> **Latest stable**: v3.8.0 (GA, 2026-06-08) | v3.9.0-rc7 (in soak, GA target 2026-12-15)\n> **Latest RC**: v3.9.0-rc7 (RC, 2026-06-12, G1-G16 PASS, awaiting 24h soak)`

**Operation 2**: Update badge versions
- oldString: `<img src="https://img.shields.io/badge/v3.7.0-GA-green?style=flat-square" alt="GA">\n  <img src="https://img.shields.io/badge/v3.8.0-Strong%20Beta-blue?style=flat-square" alt="Beta">`
- newString: `<img src="https://img.shields.io/badge/v3.8.0-GA-green?style=flat-square" alt="GA">\n  <img src="https://img.shields.io/badge/v3.9.0-rc7-yellow?style=flat-square" alt="RC7">`

### 2.2 `docs/releases/v3.9.0/CHANGELOG.md`

**Operation 3**: Fix GA target date (3 places: lines 7, 13, 88)
- oldString (line 7): `> **GA 目标**: 2026-09-23`
- newString: `> **GA 目标**: 2026-12-15 (per Hermes audit #3252, deferred from 2026-09-23)`

- oldString (line 13): `## v3.9.0 (Unreleased - 2026-09-23 目标)`
- newString: `## v3.9.0 (Unreleased - 2026-12-15 目标)`

- oldString (line 88): `| v3.9.0 | (unreleased, 2026-09-23 目标) | GA baseline placeholder — see HONESTY NOTE below |`
- newString: `| v3.9.0 | (unreleased, 2026-12-15 目标) | GA baseline placeholder — see HONESTY NOTE below |`

### 2.3 `docs/releases/v3.9.0/README.md`

**Operation 4**: Update directory tree
- oldString: `│   ├── RC1_RELEASE_NOTES.md\n│   ├── RC1_GATE_REPORT.md\n│   ├── RC2_RELEASE_NOTES.md\n│   ├── RC2_GATE_REPORT.md\n│   └── RC3_PLAN.md                   # Adjusted rc2 → rc3 → rc4 → ga plan\n├── ga/                                # GA 阶段文档 (Phase 6)`
- newString: `│   ├── RC1_RELEASE_NOTES.md\n│   ├── RC1_GATE_REPORT.md\n│   ├── RC2_RELEASE_NOTES.md\n│   ├── RC2_GATE_REPORT.md\n│   ├── RC3_PLAN.md\n│   ├── RC3_RELEASE_NOTES.md\n│   └── RC3_GATE_REPORT.md\n├── evidence/                          # GA 阶段证据 (Phase 6)\n├── ga/                                # 待创建: GA 发布物目录 (Phase 6)`

### 2.4 `docs/releases/v3.9.0/GA_GATE_REPORT.md`

**Operation 5**: Fix latest tag SHA
- oldString: `> **Latest tag**: \`v3.9.0-rc7\` at \`0868910f1\` (2026-06-12)`
- newString: `> **Latest tag**: \`v3.9.0-rc7\` at \`642ff9cf9\` (2026-06-12 16:52)`

**Operation 6**: Add rc5/rc6/rc7 binary notes
- (After line 66, add a sub-table noting rc5/rc6/rc7 binaries)

**Operation 7**: Update Z6G4 outage count
- oldString: `**Z6G4 5th outage**: 2026-06-12 17:21, ~12min, awaiting user physical restart`
- newString: `**Z6G4 5th+ outage** (6+ total 2026-06-12): network switch intermittent, 252+250 unreachable ~50min; recovered 2026-06-12 17:00; 5th at 17:21; self-healing scripts installed (see REMOTE_LIMITS.md)`

### 2.5 `docs/releases/v3.9.0/GA_GATE_STATUS_REPORT.md`

**Operation 8**: Fix tag + tip SHAs
- oldString: `> **Tag**: v3.9.0-rc7 (\`0868910f1\`) — current tip \`541b63c70\` (post-#3370 ODUK bugfix)`
- newString: `> **Tag**: v3.9.0-rc7 (\`642ff9cf9\`) — current tip \`8a83e2553\` (post-#3378 REMOTE_LIMITS + #3377 .gitattributes)`

**Operation 9**: Fix C-ARCH-05 line count quote
- oldString: `| C-ARCH-05: \`execution_engine.rs\` 1919 > 1800 | 🟡 DRIFT | RELEASE_NOTES.md §8 "C-ARCH-05: execution_engine.rs 1863 > 1800 ... per SSOT this is DRIFT, not blocking for v3.9.0 GA; needs ~63 lines further extraction in v3.9.1" |`
- newString: `| C-ARCH-05: \`execution_engine.rs\` 1919 > 1800 | 🟡 DRIFT | Per SSOT this is DRIFT, not blocking for v3.9.0 GA; needs ~119 lines further extraction to reach 1800 cap (deferred to v3.9.1). RELEASE_NOTES.md §8 confirms governance waiver. |`

**Operation 10**: Fix "Tag v3.9.0-rc5" references
- oldString (line 137): `3. **Tag v3.9.0-rc5** at current tip`
- newString: `3. **Tag v3.9.0-ga-candidate** at current tip (after 24h PASS)`
- oldString (line 141): `7. **Announce v3.9.0-rc5** to stakeholders`
- newString: `7. **Announce v3.9.0-ga-candidate** to stakeholders`
- oldString (line 142): `8. (72h 跑完后) cut **v3.9.0-ga-candidate**, run 168h soak`
- newString: `8. (72h 跑完后) cut **v3.9.0-ga-candidate-2**, run 168h soak`

### 2.6 `docs/releases/v3.9.0/evidence/00-release-summary.md`

**Operation 11**: Fix tag + tip SHAs
- oldString: `| **Tag (latest)** | v3.9.0-rc7 @ \`0868910f1\` |\n| **Tag (current tip)** | \`541b63c70\` (post-#3370 ODUK bugfix) |`
- newString: `| **Tag (latest)** | v3.9.0-rc7 @ \`642ff9cf9\` |\n| **Tag (current tip)** | \`8a83e2553\` (post-#3378 REMOTE_LIMITS + #3377 .gitattributes) |`

### 2.7 `docs/releases/v3.9.0/evidence/09-ci-build-log.md`

**Operation 12**: Add post-rc7 commit notes
- (Add 1 line after line 28: `# Latest commits post-rc7: #3370 ODUK, #3375 4-remote sync, #3377 .gitattributes, #3378 REMOTE_LIMITS`)

### 2.8 `docs/releases/v3.9.0/evidence/10-approval-record.md`

**Operation 13**: Fix "Tag v3.9.0-rc5" reference
- oldString: `3. Tag v3.9.0-rc5 + push to all remotes`
- newString: `3. Tag v3.9.0-ga-candidate + push to all remotes`

---

## 3. Review Checklist

- [ ] All 13 modifications applied successfully
- [ ] No commit log content modified
- [ ] No feature description modified
- [ ] No architecture design modified
- [ ] All referenced .md files exist
- [ ] git diff clean (only factual updates)
- [ ] No new typos introduced
- [ ] 4-remote sync after fixes

## 4. Expected Result

After all 13 modifications:
- All version dates align (GA target 2026-12-15 in 3 places)
- All tag SHAs match `git show-ref` (642ff9cf9 for rc7)
- All tip SHAs match `git log` (8a83e2553)
- All directory structure references match actual `ls`
- All "Tag v3.9.0-rc5" references updated to "ga-candidate" (rc5 already exists)
- 1 evidence file updated to reflect post-rc7 commits
- No new issues introduced
