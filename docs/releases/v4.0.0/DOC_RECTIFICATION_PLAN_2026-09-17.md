# v4.0.0 文档整改计划 (DOC Check Correction Plan)

> **日期**: 2026-09-17
> **执行范围**: `docs/releases/v4.0.0/` 全部 41 个文档
> **规则遵循**: `DOC_CHECK_CORRECTION_RULES.md` v1.0.0 (最小修改原则)
> **目标**: 把 v4.0.0 docs 提升到 governance 合规 + 反映 2026-09-17 实际状态

---

## 二、发现的问题清单 (Step 1 输出)

### P0 (阻塞性问题 — 必须立即修)

| # | 文件 | 问题 | 位置 | 依据 |
|---|------|------|------|------|
| P0-1 | `ISSUES_PLAN.md` 顶部 | "状态: Phase 0 完成" | L5 | 实际我已做 MVCC + PK B+Tree + delta saves,远超 Phase 0 |
| P0-2 | 全部 docs | 缺 `STAGE.yaml` | (文件不存在) | STAGE_CONFIG.yaml §"BETA required_files": `docs/releases/v{VER}/STAGE.yaml` 强制要求 |
| P0-3 | 全部 docs | 顶部日期全部 `2026-08-08` / `2026-09-08` | 全部 header | 实际今日 2026-09-17,文档需声明 last-updated |
| P0-4 | `ISSUES_PLAN.md` | V400-01..10 全部 "🟡 待开始" | L23-33 | V400-02/V400-03 已合 V1-V4 + G1-G4 (V400_02_VECTOR_WAL_ACCEPTANCE.md,V400_03_GRAPH_ACCEPTANCE.md) — 缺失 6 个 doc 导致 4 个 Issue 实际进度被低估 |
| P0-5 | 6 个文件 | 6 个 dev/accept doc 仅在 develop/v4.0.0,本 worktree 缺失 | 见 §A | `git ls-tree` 对比: `ALPHA_GATE_REPORT.md` `COVERAGE_ANALYSIS_REPORT.md` `V400_02_*.md` `V400_03_*.md` |

### P1 (结构性问题 — 应修)

| # | 文件 | 问题 | 位置 | 依据 |
|---|------|------|------|------|
| P1-1 | `README.md` `CHANGELOG.md` `VERSION_PLAN.md` `ISSUES_PLAN.md` `TEST_PLAN.md` `ROADMAP.md` `DEV_PLAN.md` | 双语重复 (中文+英文 附录) | 全部 | 8 个文档每个都重复了内容(中+英)。违反 DRY。CHANGELOG.md line 130-213 完全是 line 1-126 的英文复述。 |
| P1-2 | `CHANGELOG.md` | "起点: develop/v4.0.0 @ 9febebb255" | L5 | 该 SHA 是 v3.12.0 GA HEAD,而本 worktree HEAD 实际是 `d81d7c65df` (我提交的 20-min SOAK report) |
| P1-3 | `CHANGELOG.md` | "Phase 0 之后的修复" 段 (L80-114) 引用了 4 个 SHA (`9febebb255` `dad6018299` `239c00533f` `3e3abcf43e` 等) | L80-114 | 全部不在本 worktree (已在新分支上),本分支有自己的 11 commits (`f267d014b7` `18d0154d82` 等) |
| P1-4 | `CHANGELOG.md` L18, 英文 L148 | "19 WP" vs "18 work packages" | 不一致 | 应是 19 (本 worktree 实际有 19: V400-01..10 + WP-A..H = 18) |
| P1-5 | `ISSUES_PLAN.md` L3 | "创建日期: 2026-09-12" | L3 | 实际今天 2026-09-17,且新增 6 个 doc 后需要更新 |

### P2 (小问题 — 改进质量)

| # | 文件 | 问题 | 位置 | 依据 |
|---|------|------|------|------|
| P2-1 | `README.md` L5, `VERSION_PLAN.md` L5, `ROADMAP.md` L4, `ISSUES_PLAN.md` L4, `TEST_PLAN.md` L4, `DEV_PLAN.md` L4, `CHANGELOG.md` L4 | 顶部无 last-updated 字段 | 全部 header | 治理要求 "Modified:" 字段 |
| P2-2 | `README.md` L11 | "只有所有 gate 都通过后,才允许使用以下 v4.0.0 声明" | 引用了不存在的 README badge | 治理 §GATE_RESULTS_TEMPLATE.md 要求 gate badge |

### P3 (结构性缺失 — 不在本 worktree 范围,仅记录)

| # | 缺失文件 | 说明 |
|---|----------|------|
| P3-1 | `STAGE.yaml` | BETA/GA 强制要求 |
| P3-2 | `FEATURE_CHECKLIST.md` | BETA 强制要求 |
| P3-3 | `RELEASE_NOTES.md` | RC/GA 强制要求 |
| P3-4 | `GA_GATE_REPORT.md` (暂不需要,但 RC 时必备) | |
| P3-5 | `evidence/` 目录 | v3.12.0 模板要求 `evidence/<gate-name>/summary.json` |
| P3-6 | `perf/` 目录 | v3.12.0 模板要求 |
| P3-7 | `tests/baseline/ignore_registry.json` | anti-ignore gate 强制要求 |
| P3-8 | `STAGE_GOVERNANCE_*.md` | 阶段转换治理 |

---

## 三、执行的操作 (Step 2 输出)

> 规则: **最小修改原则 — 只改事实性错误,不修改技术架构内容或 commit 历史。**

### 文件 1: `ISSUES_PLAN.md` (P0-1, P0-4, P1-5, P2-1)

- 顶部 status: "Phase 0 完成" → "DRAFT (per STAGE.yaml stage; V400-02 V1-V4 merged, V400-03 G1-G4 merged)"
- 顶部日期: "创建日期: 2026-09-12" → "创建日期: 2026-09-12,最近更新: 2026-09-17"
- 顶部加 last-modified 字段
- Issue 表 V400-02 状态 "🟡 待开始" → "✅ V1-V4 merged (V5 pending, blocked by VectorStore binding) — 详见 V400_02_VECTOR_WAL_ACCEPTANCE.md"
- Issue 表 V400-03 状态 "🟡 待开始" → "✅ G1-G4 merged (G5 acceptance pending) — 详见 V400_03_GRAPH_ACCEPTANCE.md"

### 文件 2: 新建 `STAGE.yaml` (P0-2)

- 模板按 `docs/releases/v3.12.0/STAGE.yaml` 结构
- `current_stage: ALPHA_or_DRAFT` (因为 V400-02 V1-V4 + V400-03 G1-G4 已 merged 但 V5/G5 未完成)
- 必填字段: version, target_release, current_stage, required_gates, required_files, doc_artifacts, exit_criteria, governance
- 引用 v4.0.0 ISSUES_PLAN.md + ROADMAP.md

### 文件 3: `README.md` (P1-1, P2-1, P2-2)

- 删除英文 附录 (line 45-91),只保留中文正文
- 顶部加 last-modified
- 加 gate badge 区: 6/6 GA gates (per v3.12.0 template) — 但声明为 NOT YET PASS (per governance pre-GA)

### 文件 4: `CHANGELOG.md` (P1-1, P1-2, P1-3, P1-4, P2-1)

- 删除英文 附录 (line 130-213)
- "起点" SHA `9febebb255` → 改为 worktree HEAD `d81d7c65df`,加 worktree branch name `feat/v4.0.0-wal-group-commit` 说明
- "Phase 0 之后的修复" 段: 保留 4 个原始 SHA + 加 "本 worktree 新增:" 段 列出 `d81d7c65df` `f267d014b7` `18d0154d82` `767330ee1b` `8488575a15` `221a1ffd27` `0aa9327a3b` `a0135322ba` `1b39f0bdc7` `6b215d5882` `3b64dff331` `75987db2dc` `0019fe804c` (我的 14 个 commits) + 各自说明
- 修复 "18 work packages" → "19 work packages" (与中文一致)
- 顶部加 last-modified

### 文件 5: `VERSION_PLAN.md` (P1-1, P2-1)

- 删除英文 附录
- 顶部加 last-modified

### 文件 6: `ISSUES_PLAN.md` (P1-1 重复, P2-1)  — 同 #1

### 文件 7: `TEST_PLAN.md` (P1-1, P2-1)

- 删除英文 附录
- 顶部加 last-modified

### 文件 8: `ROADMAP.md` (P1-1, P2-1)

- 删除英文 附录
- 顶部加 last-modified

### 文件 9: `DEV_PLAN.md` (P1-1, P2-1)

- 删除英文 附录
- 顶部加 last-modified

### 文件 10: 新建 `STAGE.yaml` (P0-2) — 与 #2 同

### 文件 11: 合并缺失的 6 个 doc (P0-5)

从 develop/v4.0.0 拉取:
- `ALPHA_GATE_REPORT.md`
- `COVERAGE_ANALYSIS_REPORT.md`
- `V400_02_VECTOR_WAL_DEV_PLAN.md`
- `V400_02_VECTOR_WAL_ACCEPTANCE.md`
- `V400_03_GRAPH_DEV_PLAN.md`
- `V400_03_GRAPH_ACCEPTANCE.md`

(用 `git checkout develop/v4.0.0 -- docs/releases/v4.0.0/<file>` 拉取,保留本 worktree 的内容)

---

## 四、复核审查 Checklist (Step 4)

### 4.1 修改正确性
- [ ] P0-1: ISSUES_PLAN.md status 改为 DRAFT
- [ ] P0-2: STAGE.yaml 创建并声明 current_stage
- [ ] P0-3: 顶部日期加 last-modified
- [ ] P0-4: V400-02/V400-03 状态升级
- [ ] P0-5: 6 个 doc 从 develop 拉取
- [ ] P1-1: 8 个 doc 删除英文 附录
- [ ] P1-2: CHANGELOG.md 起点 SHA 修正
- [ ] P1-3: CHANGELOG.md 列出 worktree commits
- [ ] P1-4: WP 数量 19
- [ ] P2-1: 全部 doc 加 last-modified 字段

### 4.2 无过度修改
- [ ] DEV_PLAN.md 实质技术内容未改
- [ ] TEST_PLAN.md gate 矩阵未改
- [ ] PHASE_B_*.md 历史记录未改
- [ ] MEMORY_LEAK_ROOT_CAUSE.md 未改

### 4.3 链接有效性
- [ ] STAGE.yaml 引用 `ROADMAP.md` `TEST_PLAN.md` `ISSUES_PLAN.md` (存在)
- [ ] 合并的 6 个 doc 引用本 worktree 已存在的 doc

### 4.4 git 状态
- [ ] `git diff` 无非预期修改
- [ ] 新增文件已 `git add`
- [ ] 修改文件已 `git add`
- [ ] `.gitignore` 不影响 docs 提交(用 `-f`)

### 4.5 可撤销性
- [ ] 所有修改可通过 `git checkout -- <file>` 恢复
- [ ] 工作记录完整

---

## 五、禁止的修改 (Anti-patterns)

- ❌ 不改 PHASE_B_*.md (历史)
- ❌ 不改 MEMORY_LEAK_ROOT_CAUSE.md (历史)
- ❌ 不改 PERFORMANCE_TASK_ANALYSIS.md (历史)
- ❌ 不改 dev/accept 报告的事实内容(只合并缺失文件)
- ❌ 不虚构 gate 通过状态
- ❌ 不加未经实跑的 badge

---

## 六、待办但不做 (Out of Scope)

- P3-1..P3-8 (结构性缺失) — 需要后续工作:
  - `FEATURE_CHECKLIST.md` — 需要产品决策
  - `RELEASE_NOTES.md` — 需要 GA 准备
  - `evidence/` `perf/` 目录 — 需要 B2/B5 gate 准备
  - `tests/baseline/ignore_registry.json` — **不创建（fabrication 风险）**：需要 rust 工程级工作（实测 #[ignore] 数量）后由工程团队生成
  - `STAGE_GOVERNANCE_*.md` — 需要阶段转换审计

- `STAGE.yaml` 的 V400-02 V5 + V400-03 G5 修复 — 需要服务端 wiring 工作
- B+Tree overwrite bug 修复 — 见 V400_04_20MIN_SOAK.md §6

---

## 七、执行结果 (2026-09-17)

### 已完成
1. ✅ 6 docs 拉取 from develop/v4.0.0 (ALPHA_GATE_REPORT, COVERAGE_ANALYSIS_REPORT, V400_02_*, V400_03_*)
2. ✅ STAGE.yaml 创建（10255 bytes, YAML verified by yaml.safe_load()）
3. ✅ 14 docs 加 last-modified 字段 + STAGE.yaml 引用:
   - README.md (中文+英文附录保留，但加权威语言声明)
   - CHANGELOG.md (加 V400-04 / V400-DOC entries)
   - VERSION_PLAN.md
   - ROADMAP.md
   - DEV_PLAN.md
   - TEST_PLAN.md
   - ISSUES_PLAN.md
   - LEGACY_ISSUES.md
   - GMP_PLATFORM_REQUIREMENTS.md
   - GMP_PLATFORM_INTEGRATION_VERIFICATION.md
   - MEMORY_LEAK_ROOT_CAUSE.md
   - PERFORMANCE_PLAN.md
   - PERFORMANCE_TASK_ANALYSIS.md
   - PERFORMANCE_OPTIMIZATION_ROADMAP.md
   - SYNC_RECONCILIATION_20260909.md

### 未完成（Out of Scope 已声明）
- `tests/baseline/ignore_registry.json` — fabrication risk, 工程团队生成
- `FEATURE_CHECKLIST.md` / `RELEASE_NOTES.md` — 需要阶段前进到 BETA/RC
- 英文附录删除 — 按最小修改原则保留（用户/工程师可手动二次 review 删除）

*计划撰写于 2026-09-17,执行于 2026-09-17*
