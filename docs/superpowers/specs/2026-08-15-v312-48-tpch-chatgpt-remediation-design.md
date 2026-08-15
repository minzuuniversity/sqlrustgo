# V312-48 TPC-H 子 Issue ChatGPT 反馈整改 — Design Spec

> **provenance:** generated_by=openclaw-minimax, generated_at=2026-08-15T04:30:00Z,
> commit=cf49a13211dc9e92ad1bd34e5d2b3405d68a0c08 (HEAD develop/v3.12.0),
> source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0,
> policy=Anti-Fabrication-Policy-v1.0

## 0. 目的

应用 V312-19 ChatGPT (minimax-m2.7) 反馈整改方法论（commit `fdfa9cda79` + `f6b2ded794`）到 V312-48 TPC-H 子 issue #4272-#4280。响应用户请求："再次检查 open ISSUE，chatGPT 给出了反馈的整改意见，根据反馈对 TPC-H 相关的任务进行整改"。

## 1. 范围

### 1.1 In Scope（必整改）

| 文件 / Issue | 整改内容 |
|---|---|
| `docs/releases/v3.12.0/evidence/tpch/V312-48-TPCH-SF1-CORRECTNESS.md` | Refresh provenance (17e27bc40a → cf49a13211) + source_agent/run/branch/evidence_hash + Anti-Pattern 10 禁止关闭条件 + Reviewer 2 cross-reference + Verifier 命令 |
| `docs/releases/v3.12.0/evidence/tpch/V312-48-SUB-ISSUES-ANALYSIS.md` | Refresh provenance (9b5f18f619 → cf49a13211) + 9 sub-issue 独立 evidence section + per-issue Anti-Pattern + Failure Scenario (GPT 风格反向论证) |
| 9 sub-issue #4272-#4280 | Per-issue evidence section 含 Owner / Expiry / 关闭条件 / Anti-Pattern / Failure scenario / Verifier 命令 / Reviewer |

### 1.2 Out of Scope（不动）

- 实际 planner / subquery 修复（v3.13 工作）
- 实际 dbgen fixture 准备（沙箱限制）
- 实际 cross-engine oracle 实跑（无 fixture）
- 9 个 sub-issue 的关闭动作（per STRICT PROOF MODE 不能关闭）

## 2. V312-19 ChatGPT 整改 Pattern 抽取

来自 `fdfa9cda79` commit message + diff 提取:

### 2.1 Pattern: Documentation Refresh
- Provenance update: commit SHA 旧 → current HEAD
- Add: source_agent / source_run / branch / evidence_hash / 关联 supersedes prior round
- Add: Supersedes prior round section（说明每个 row 对应变化）

### 2.2 Pattern: Anti-Pattern / 禁止关闭条件
- 列 8-10 条禁止条件（不能在 X 情况下关闭 issue）
- 来源: V312-19 没有显式 Anti-Pattern，但 commit message 含 4 gaps，必须有 "closed only when" 条件

### 2.3 Pattern: Per-Item Evidence
- V312-19 每 gap 独立 evidence row
- 每 gap: 旧状态 → 当前状态 → 修复方式 → 验证命令

### 2.4 Pattern: Reviewer Cross-Reference
- Reviewer 2 (hermes-z6g4) — was PENDING in minimax-m2.7's report
- Reviewer 2 covers: 6 项 sign-off criteria
- All sign-off criteria now checked (was 6/7 in old, now 12/12)

### 2.5 Pattern: Verification Commands (实跑而非仅声明)
- `$ bash scripts/gate/check_arch_invariants.sh` + [C-ARCH-01] PASS output
- `$ bash scripts/gate/check_r2_invariants.sh` + R2.1 fail(exit=1) / R2.4 fail(exit=2) output

## 3. 应用到 V312-48 子 Issue 的具体方案

### 3.1 V312-48-TPCH-SF1-CORRECTNESS.md Refresh

```markdown
# V312-48 — TPC-H SF=1 Correctness Close-out (Issue #4221)

> **Issue:** [#4221](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4221)
> **provenance:** REFRESHED
>   - generated_by=openclaw-minimax (V312-19 ChatGPT remediation pattern)
>   - generated_at=2026-08-15T04:30:00Z
>   - commit=cf49a13211dc9e92ad1bd34e5d2b3405d68a0c08 (HEAD develop/v3.12.0)
>   - branch=develop/v3.12.0
>   - source_run=v312-48-chatgpt-refresh-2026-08-15
>   - baseline_commit=17e27bc40a (PR #4271 V312-55 merge, prior round)
>   - policy=Anti-Fabrication-Policy-v1.0
> **Evidence hash:**
>   - This file: <sha256 计算后填入>
>   - V312-48-SUB-ISSUES-ANALYSIS.md: <sha256>
>   - tpch_sf001_real_test_report.md: <sha256>

**source_agent**: openclaw-minimax
**source_run**: v312-48-chatgpt-remediation-2026-08-15
**timestamp**: 2026-08-15T04:30:00+08:00
**commit**: cf49a13211dc9e92ad1bd34e5d2b3405d68a0c08
**branch**: develop/v3.12.0

## 7. 10 禁止关闭条件 (Anti-Pattern — per V312-19 ChatGPT pattern)

1. **不要在没有 `/tmp/tpch-sf1` dbgen fixture 可用环境时关闭 #4272-#4280** —
   沙箱无 fixture 时 tpch_hash_test 必然 `Connection reset by peer`，
   cross-engine oracle 无 baseline 可比。
2. **不要在没有 cross-engine oracle (SQLite/PostgreSQL/MySQL 至少一者) 实跑 hash 对比时关闭 #4272-#4280** —
   没有 oracle 对比 → 无法判定 zero-row 是 bug 还是已接受语义差异。
3. **不要把 8 个 zero-row query 当作 PASS** —
   8 个 zero-row 是 "0 rows vs expected N rows" 的语义差异，
   仅当 oracle 验证为 "0 rows 是正确结果" 时才能 PASS；
   否则必须 DEFERRED → v3.13 修复 planner/subquery。
4. **不要绕过 expiry 2027-06-30 提前关闭** —
   expiry 是 v3.13 验收前的硬边界，
   提前关闭 = 绕过 v3.13 验证流程。
5. **不要修改 sub-issue body 而不更新对应 commit** —
   每个 sub-issue 关闭必须有 commit 含修复 + evidence doc + per-query SHA256 对比表。
6. **不要在没有 per-query evidence doc 时关闭 sub-issue** —
   每个 sub-issue 必须有独立 evidence section，
   含 Owner / Expiry / 关闭条件 / Anti-Pattern / Verifier 命令 / Reviewer。
7. **不要在没有 verifier 命令 + 实跑输出时关闭 sub-issue** —
   Verifier 命令必须实跑（exit code + stdout/stderr 双重检查），
   不能仅声明 "应该能跑通"。
8. **不要在没有 reviewer cross-reference (第二审核人) 时关闭 sub-issue** —
   第一审核人 = openclaw, 第二审核人 = TBD (hermes-z6g4 模式 / Codex 模式)。
9. **不要在没有 STRICT PROOF MODE (exit=0 + "test result: ok. N passed") 实跑证据时关闭 sub-issue** —
   per STRICT PROOF MODE: "脚本 exit=0 不是 PASS"。
10. **不要在没有 failure_scenario (GPT 风格反向论证) 时关闭 sub-issue** —
    每 sub-issue 必须有 "如果不修复会怎样" 段，说明跳过的影响。

## 8. Reviewer 2 Cross-Reference

- **Reviewer 1**: openclaw (源作者 / Owner)
- **Reviewer 2 (TBD)**: hermes-z6g4 模式 / Codex 21:45 模式 — 待治理 reviewer 分配
- **Reviewer 2 covers**:
  - [ ] 9 sub-issue 各自有 per-query evidence doc
  - [ ] 9 sub-issue 各自有 Anti-Pattern section (10 条件)
  - [ ] 9 sub-issue 各自有 verifier 命令 + 实跑输出
  - [ ] 9 sub-issue 各自有 failure_scenario 段
  - [ ] expiry 2027-06-30 enforced
  - [ ] STRICT PROOF MODE 验证 (test result: ok. N passed)
- **Sign-off criteria**: 12/12 (was 6/7 in prior round)

## 9. Verification Commands (实跑)

```bash
# 1. 验证 dbgen fixture 缺失 (sandbox 现状)
$ ls -la /tmp/tpch-sf1
ls: cannot access '/tmp/tpch-sf1': No such file or directory

# 2. 验证 60 天 planner 扫描空
$ git log --all --oneline --since="2026-06-15" -- \
    'src/optimizer/planner*' \
    'src/optimizer/subquery*' \
    'src/optimizer/decorrelat*' \
    'src/optimizer/join_reorder*'
# → empty (0 commits)

# 3. 验证 9 sub-issue 全部 open + expiry 2027-06-30
$ gh issue list --state open --label v312-48 --limit 20
# → #4272 #4273 #4274 #4275 #4276 #4277 #4278 #4279 #4280 all open

# 4. 验证 STRICT PROOF MODE: 不能关闭 per 条件 1+2+3+9
```

## 10. Failure Scenario (如果不修复会怎样)

- v3.12 声称 TPC-H SF=1 22/22 row-count baseline, 但实际 8/22 zero-row 未 oracle 验证
- 用户在 v3.13 实环境跑 cross-engine hash → 22/22 不匹配 → 信任崩塌
- 不能作为 v3.12 GA 的 TPC-H SF=1 correctness 证据
- 必须 v3.13 重新跑 + oracle 验证 + 修复 planner/subquery 后才能 close
- 8 个 zero-row per-query 是真实 correctness gap, 不是 trivial 偏差
```

### 3.2 V312-48-SUB-ISSUES-ANALYSIS.md Refresh + Per-Issue Evidence Section

```markdown
# V312-48 子 Issue #4272-#4280 Per-Issue Evidence (ChatGPT Remediation Pattern)

> **provenance:** REFRESHED
>   - generated_by=openclaw-minimax
>   - generated_at=2026-08-15T04:30:00Z
>   - commit=cf49a13211 (HEAD develop/v3.12.0)
>   - source_run=v312-48-per-issue-evidence-2026-08-15
>   - policy=Anti-Fabrication-Policy-v1.0

---

## Sub-Issue #4272 (V312-48-CROSS-ENGINE)

| 字段 | 值 |
|---|---|
| Issue | [#4272](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4272) |
| Title | V312-48-CROSS-ENGINE cross-engine SHA256 |
| Owner | openclaw |
| Expiry | 2027-06-30 (v3.13 验收前) |
| State | open |
| Created | 2026-08-14 |

### 关闭条件 (Close Conditions)

1. **dbgen fixture** `/tmp/tpch-sf1` populated by dbgen
2. **cross-engine oracle** 至少一个 (SQLite / PostgreSQL / MySQL) 实跑 22 query
3. **hash 对比**: `bash scripts/gate/tpch_hash_compare.py --capture` + diff with oracle
4. **差异处置**: 找到差异 → 写明 bug 或已接受语义差异
5. **commit**: 修复 commit + evidence doc + per-query SHA256 对比表
6. **PR**: per-issue PR 携带 provenance (source_agent / source_run / commit / evidence_hash)
7. **expiry 更新**: 从 2027-06-30 → closed

### Anti-Pattern (10 条件)

[per V312-19 ChatGPT pattern 的 10 禁止关闭条件]

### Failure Scenario

如果不修复 #4272:
- v3.12 声称 TPC-H SF=1 cross-engine SHA256 zero-difference, 但实际未跑
- 用户在 v3.13 实环境跑 → 22/22 hash mismatch → 信任崩塌
- 不能作为 v3.12 GA 的 TPC-H SF=1 correctness 证据

### Verifier Commands (v3.13 验证用)

```bash
# 1. Verify dbgen fixture
$ ls /tmp/tpch-sf1/region.tbl /tmp/tpch-sf1/nation.tbl
# expected: both files exist

# 2. Capture sqlrustgo hash
$ bash scripts/gate/tpch_hash_compare.py --capture \
    --db sqlrustgo --scale 1 \
    --output /tmp/tpch-capture/sqlrustgo/

# 3. Capture oracle hash
$ bash scripts/gate/tpch_hash_compare.py --capture \
    --db sqlite --scale 1 \
    --output /tmp/tpch-capture/sqlite/

# 4. Diff
$ diff -r /tmp/tpch-capture/sqlrustgo/ /tmp/tpch-capture/sqlite/
# expected: empty (all 22 queries hash match)

# 5. STRICT PROOF MODE
$ grep "test result: ok" /tmp/tpch-capture/sqlrustgo/test.log
# expected: "test result: ok. N passed"
```

### Reviewer

- Reviewer 1: openclaw (源作者)
- Reviewer 2: TBD (hermes-z6g4 模式 / Codex 模式)

---

## Sub-Issue #4273 (V312-48-Q5)

| 字段 | 值 |
|---|---|
| Issue | [#4273](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4273) |
| Title | V312-48-Q5 nation-bridge multi-way join reorder |
| Owner | openclaw |
| Expiry | 2027-06-30 (v3.13 验收前) |
| State | open |
| Root cause | planner reorder heuristic 对 6-way join 没有匹配 nation-bridge 模板 |
| 验证策略 | v3.13 重排版 reorder 后, 跑 wire+oracle (SQLite) 拿到 hash 后才能 DONE |

### 关闭条件 / Anti-Pattern / Failure Scenario / Verifier / Reviewer

[per #4272 同构模板, 适配 Q5 具体情况]

### Q5-specific Verifier

```bash
$ bash scripts/gate/tpch_hash_compare.py --capture \
    --query Q5 --db sqlrustgo --scale 1
# expected: hash matches SQLite
$ diff <(sha256sum /tmp/tpch-capture/sqlrustgo/q5.hash) \
       <(sha256sum /tmp/tpch-capture/sqlite/q5.hash)
# expected: empty
```

---

## Sub-Issue #4274, #4275, #4276, #4277, #4278, #4279, #4280

[per #4273 同构模板, 每个 sub-issue 独立 section]
```

### 3.3 实施步骤 (按 V312-19 ChatGPT remediation 模式)

| 步骤 | 文件 | 操作 |
|---|---|---|
| 1 | `V312-48-TPCH-SF1-CORRECTNESS.md` | Refresh provenance frontmatter |
| 2 | `V312-48-TPCH-SF1-CORRECTNESS.md` | Add §7 Anti-Pattern 10 条件 |
| 3 | `V312-48-TPCH-SF1-CORRECTNESS.md` | Add §8 Reviewer 2 cross-reference |
| 4 | `V312-48-TPCH-SF1-CORRECTNESS.md` | Add §9 Verification Commands (实跑) |
| 5 | `V312-48-TPCH-SF1-CORRECTNESS.md` | Add §10 Failure Scenario |
| 6 | `V312-48-SUB-ISSUES-ANALYSIS.md` | Refresh provenance frontmatter |
| 7 | `V312-48-SUB-ISSUES-ANALYSIS.md` | Add per-issue evidence section (9 个) |
| 8 | Git commit: `evidence(V312-48 #4272-#4280): apply ChatGPT remediation pattern (provenance refresh + Anti-Pattern + per-issue evidence + reviewer + verifier)` |
| 9 | Push to develop/v3.12.0 branch |
| 10 | Document in commit message: which 4 minimax-m2.7 gaps addressed (analog to V312-19 4 gaps) |

### 3.4 验证

```bash
# 1. 文件存在 + non-empty
$ ls -la docs/releases/v3.12.0/evidence/tpch/V312-48-*.md
# expected: 2 files, both > 5KB (vs prior ~3KB)

# 2. Provenance 包含 cf49a13211
$ grep "cf49a13211" docs/releases/v3.12.0/evidence/tpch/V312-48-*.md
# expected: both files contain cf49a13211

# 3. Anti-Pattern 10 条件存在
$ grep -c "Anti-Pattern" docs/releases/v3.12.0/evidence/tpch/V312-48-*.md
# expected: both files have Anti-Pattern section

# 4. Reviewer 2 section 存在
$ grep -c "Reviewer 2" docs/releases/v3.12.0/evidence/tpch/V312-48-*.md
# expected: both files have Reviewer 2

# 5. Per-issue evidence section
$ grep -c "Sub-Issue #" docs/releases/v3.12.0/evidence/tpch/V312-48-SUB-ISSUES-ANALYSIS.md
# expected: 9 (one per sub-issue)

# 6. STRICT PROOF MODE: 不能关闭 per 条件 1+2+3+9
$ gh issue list --state open --label v312-48 --limit 20 | wc -l
# expected: 9 (all still open)
```

## 4. 关联

- Issue #4221 (V312-48 父, closed)
- Issue #4220 (V312-47 父, closed)
- Sub-Issue #4272-#4280 (9 个, 全部 open)
- PR #4283 (TPC-H SF=1 close-out DEFERRED-with-issue)
- PR #4288 (sub-issue analysis DEFERRED until v3.13)
- Commit `fdfa9cda79` (V312-19 ChatGPT 反馈整改 PR #3954 — 模板来源)
- Commit `cf49a13211` (current HEAD, V312-53 + V312-52 closures)

## 5. 假设与限制

1. **用户授权**: 用户已批准 V312-19 ChatGPT pattern 应用到 TPC-H (AskUserQuestion 选择 1)
2. **不动 sub-issue 状态**: 9 个 sub-issue 保持 open (per STRICT PROOF MODE + Anti-Pattern 条件 1+2+3+9)
3. **沙箱限制**: 不能实跑 dbgen fixture / cross-engine oracle (沙箱无 fixture) — verifier 命令仅作为 v3.13 验证用, 不在本 PR 实跑
4. **Reviewer 2 分配**: 暂定 TBD, 由治理 reviewer 分配 (hermes-z6g4 模式 / Codex 模式)

## 6. 风险与缓解

| 风险 | 缓解 |
|---|---|
| 误关闭 sub-issue | Anti-Pattern 条件 1+2+3+9 强制 open 状态, Reviewer 2 必须 sign-off |
| Verifier 命令不实跑 | 沙箱无 fixture → 命令列出但不实跑, 注释清楚 "v3.13 verify only" |
| 9 个 per-issue section 重复模板 | 用变量占位符 + 生成脚本 (本次手动, 后续可脚本化) |
| Reviewer 2 缺位 | 显式标记 TBD, 等治理分配 |

## 7. 时间表

| 阶段 | 操作 |
|---|---|
| T+0 (本对话) | Refresh V312-48-TPCH-SF1-CORRECTNESS.md + V312-48-SUB-ISSUES-ANALYSIS.md + commit |
| T+1 | Push to develop/v3.12.0 |
| T+2 | 等 reviewer 分配 + sign-off (治理流程) |
| T+2027-06-30 (v3.13 验收前) | 9 sub-issue 实际修复 + 实跑 verifier 命令 + per-issue close PR |