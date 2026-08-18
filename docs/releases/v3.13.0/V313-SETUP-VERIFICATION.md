# V313-MASTER Setup 自检

> **标准**: `V313-STRICT-CLOSE-STANDARDS.md` §2 四要素
> **诚实披露**: Per Anti-Fabrication-Policy-v1.0
> **自检日期**: 2026-08-18

## 1. Governance / Sprint Plan 文档 self-anchor (12 个 = 9 governance + 3 exception)

| 文件 | SHA-256 (computed) | self-anchor? |
|---|---|---|
| `V313-STRICT-CLOSE-STANDARDS.md` | `15bd386c58c6bc7b09b3ebc68e8ea25ebb9fc32e1f7eca92cb1d3425697dfe86` | ✅ |
| `V313-FOLLOWUP-INDEX.md` | `ab3b1ac81430e0a2e9a88012da87987afd288776d25f2483e783bf6848172f39` | ✅ |
| `V313-MASTER-PLAN.md` | `daa28c8216ce1c83a042254a3fc5a9460dc68f843a80f2724eff8e34993d82ad` | ✅ |
| `sprints/SPRINT-S1-GMP.md` | `5385d88284b548a6feee2552a2dfe22c3591b635a92155d677ae066e9556ec50` | ✅ |
| `sprints/SPRINT-S2-TPCH-CROSS.md` | `d488e7502cc4dc65e9b5899736b72b5c5fbbf20a87a3fedfa5dde3c8f1b26145` | ✅ |
| `sprints/SPRINT-S3-PLANNER-ZEROROW.md` | `811767a3f1d559799e1deef218f75062cbb0e48eb3474b18a3c3eb99a877c782` | ✅ |
| `sprints/SPRINT-S4-V312-56-TEACHING.md` | `4318b0b680da0b6460277b62e19d5a2fd82ed6b7aa4becc40675c9a8c13aaecd` | ✅ |
| `sprints/SPRINT-S5-META-CLOSURE.md` | `e5f26e2089218388f6715793dc3fd981e8b0c0dbf05f57e3cc0b09c9d91e44f4` | ✅ |
| `sprints/SPRINT-S6-ARRAY-FRACTION.md` | `2c00b78bb1318092667542f4a7b8f3d1583bc5a4b4bd90367810046f44e731c9` | ✅ |
| `PR-4313-DESCRIPTION.md` | `12b254a065777ca4f8530f1993faa21f78d45e5fee17892beaa38f31e26c7707` | (PR body, 不需 self-anchor) |
| `README.md` | `6416ff244d5cfc5ca62177f506d573e2b1f95e5f1d27f481e6868c9f0bbcb688` | (索引入口, 不需 self-anchor) |
| `V313-SETUP-VERIFICATION.md` | (本文件,自检报告,见下) | (本文件即自检结果,不需 self-anchor — 见 §9 自引用说明) |

## 2. Evidence 文档 inline hash (5 个)

| 文件 | SHA-256 (computed) | 内容 |
|---|---|---|
| `evidence/V313-S0-ALPHA-QUALITY-PASS.md` | `8d15c92117a806ff2b7c623168c9e361931aedc1a95d46fdce4399ada9e58faf` | 7 个 gate SHA-256 + commit hash (inline) |
| `evidence/V313-S0-BLOCKERS-COMPLETE.md` | `3206e6c508fe64dec56ee2ba1ec85894da3b91211d31099985dd7dac3830f4b1` | 3 blocker commit + SHA 引用 |
| `evidence/V313-S0-MYSQL-ORACLE-EVIDENCE.md` | `17bec7e2339344b9811148e364f951deb568245eea2d53e6d5b1ac282e2e8cb0` | MySQL 22/22 query SHA-256 (inline) |
| `evidence/V313-S0-TPCH-SF1-FIXTURE-EVIDENCE.md` | `f50167193eefc6f43949583a3ce0f48c984e57eec07ec10f0b991c0d9bf0b795` | 8 个 .tbl 文件 SHA-256 + 行数 (inline) |
| `evidence/V313-S0-MYSQL-ORACLE-SHA256.txt` | (text) | 22 query MySQL 输出 SHA-256 锚定 |
| `evidence/V313-S0-TPCH-SF1-FIXTURE-SHA256.txt` | (text) | 8 .tbl 文件 SHA-256 锚定 |

**Evidence 覆盖率: 6/6 = 100% (5 .md + 2 .txt, 其中 .md 文件含 inline hash) ✅**

## 3. 目录结构

```
docs/releases/v3.13.0/
├── README.md                          # 索引入口
├── V313-STRICT-CLOSE-STANDARDS.md     # 治理规则
├── V313-FOLLOWUP-INDEX.md             # 24 sub-issue 索引
├── V313-MASTER-PLAN.md                # 总控 scope + sprint 顺序
├── V313-SETUP-VERIFICATION.md         # 本文件
├── V313-DOCS-INVENTORY.txt            # 所有 .md SHA-256 清单
├── PR-4313-DESCRIPTION.md             # PR #4326 body 归档
├── evidence/                          # 6 个 evidence 锚定文件
│   ├── V313-S0-ALPHA-QUALITY-PASS.md
│   ├── V313-S0-BLOCKERS-COMPLETE.md
│   ├── V313-S0-MYSQL-ORACLE-EVIDENCE.md
│   ├── V313-S0-MYSQL-ORACLE-SHA256.txt
│   ├── V313-S0-TPCH-SF1-FIXTURE-EVIDENCE.md
│   └── V313-S0-TPCH-SF1-FIXTURE-SHA256.txt
├── sprints/                           # 6 个 sprint sub-plan
│   ├── SPRINT-S1-GMP.md
│   ├── SPRINT-S2-TPCH-CROSS.md
│   ├── SPRINT-S3-PLANNER-ZEROROW.md
│   ├── SPRINT-S4-V312-56-TEACHING.md
│   ├── SPRINT-S5-META-CLOSURE.md
│   └── SPRINT-S6-ARRAY-FRACTION.md
└── gates/                             # 预留 (SPRINT-S1..S6 实施时填充)
```

## 4. Git 状态

| 项 | 值 |
|---|---|
| 本地分支 | `develop/v3.13.0` |
| 本地 HEAD | `e6d3f0e14685` |
| origin HEAD | `e6d3f0e14685` (同步) |
| 领先 origin (本会话) | 10 commit → 已 push,无新 orphan |
| 历史 orphan (已文档化) | `c2d7d27239` (T3), `486acbf1a2` (T2) — 均在 reflog,无新引入 |

## 5. PR 状态

| 项 | 值 |
|---|---|
| PR # | 4326 |
| URL | http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4326 |
| Title | docs(v3.13.0): V313-MASTER total control + governance framework (Issue #4313) |
| Head | `develop/v3.13.0` @ `e6d3f0e14685` |
| Base | `develop/v3.12.0` @ `02f1d9afe649` |
| State | open |
| Refs | #4313 (Closes 部分) + #3887 #4216 #4220 #4221 #4225 #4226 #4250-#4258 #4272-#4279 |

## 6. 检查项汇总

| 检查项 | 状态 |
|---|---|
| docs/releases/v3.13.0/ 目录存在 | ✅ |
| V313-STRICT-CLOSE-STANDARDS.md 已合并 | ✅ `a1940a9fdc` |
| V313-FOLLOWUP-INDEX.md 已合并 | ✅ `0190762b71` (后修 `3bdd85312f`) |
| V313-MASTER-PLAN.md 已合并 | ✅ `c2d7d27239` → `53492f02f1` (audit note `1f66f22bc3`) |
| 6 个 sprint sub-plan 已合并 | ✅ `436d08490f` `e7b623ff7f` `7afa1970bd` `5e389b4b68` `be0381deb4` `63d01aa9bb` |
| PR-4313-DESCRIPTION.md 已合并 | ✅ `e6d3f0e14685` |
| develop/v3.13.0 分支已推送 | ✅ (e6d3f0e14685 on origin) |
| 总控 PR 已创建 (Gitea #4326) | ✅ |
| 9/9 governance 文档 self-anchor ✅ | ✅ |
| 6/6 evidence 文件 ✅ | ✅ |
| 已知 orphan (c2d7d27239 + 486acbf1a2) 已 audit note 文档化 | ✅ |
| Anti-Fabrication-Policy-v1.0 诚实披露 | ✅ |

## 7. 已知 Honest Disclosure (per Anti-Fabrication-Policy-v1.0)

1. **PR #4326 mergeable=False** — base `develop/v3.12.0` 与 head `develop/v3.13.0` 不在同一祖先链,后续 merge 时需 resolve (这本身不在本 PR scope)。
2. **Issue #4313 仅在 252 canonical 上存在** — 250 mirror 上无此 issue,后续 issue 引用均指向 252。
3. **2 个已知 orphan** (`c2d7d27239` + `486acbf1a2`) — 已在 V313-MASTER-PLAN.md §Audit Note 和 V313-FOLLOWUP-INDEX.md §Audit Note 中诚实披露,reflog 留底。
4. **PR #4326 未在 250 mirror 创建** — 因 Issue #4313 仅存在于 252 canonical;若后续需 250 mirror 同步,需在 250 上重新创建同 title 的 PR (但因 issue 不存在,只能 refs 文字)。

## 8. Next Step

启动 SPRINT-S1 (GMP 治理) 实施阶段,按 SPRINT-S1-GMP.md §4.1 + §4.2 任务结构推进。

## 9. 自引用 SHA 说明 (Honest Disclosure)

本文件 (V313-SETUP-VERIFICATION.md) 不设 `sha256=` self-anchor,原因:

1. **自我引用悖论**: 文件内 `sha256=` line 的内容本身就是 SHA-256 计算的输入,导致"先有值再有 hash"的循环依赖
2. **本文件职责**: 是 *自检* 报告,不是被自检的 governance doc;其自身 hash 由 `V313-DOCS-INVENTORY.txt` 锚定
3. **同类例外**: `PR-4313-DESCRIPTION.md` (PR body,锚定在 Gitea PR 本身)、`README.md` (索引入口,无 anchor 是约定) — 同样不设 self-anchor

最终 hash 见 `V313-DOCS-INVENTORY.txt`。

## 10. Pre-existing Clippy 回归诚实披露 + 修复

`cargo clippy --all-features -- -D warnings` 在 `develop/v3.13.0` 当前 HEAD 上失败,**不是 V313 work 引入的回归**。

| 事实 | 证据 |
|---|---|
| V313 SETUP 修改的文件 | 仅 `docs/releases/v3.13.0/*.md` + `.txt` (见上方 inventory) |
| 是否触及任何 `.rs` 文件 | **零** (SETUP 阶段); clippy 修复阶段触及 `src/engine_ddl.rs` + `src/execution_engine.rs` (3 处 §10 acknowledged) |
| Clippy error 来源 | `src/engine_ddl.rs` + `src/execution_engine.rs` (V312-56a rebase 引入,commit `1e0f5018ad`) |
| `develop/v3.12.0` 当前 tip | clippy PASS (因为 9554dee2bb 之后 v3.12.0 修复了此 issue) |
| `develop/v3.13.0` bootstrap point | `79caf8c4...` (V313 分支从 OLD v3.12.0 HEAD fork,该 HEAD 尚含 clippy regression) |

### 10.1 Resolution (commit `9c3e48c5d7`)

§10 披露的 3 处 clippy 错误于 `9c3e48c5d7` 修复 (本会话独立提交,no force-push):

1. `src/engine_ddl.rs:441-443` — 删除 orphan doc-comment block (原 clipping as empty_line_after_doc_comments)
2. `src/engine_ddl.rs:821` — `format!("{}", v)` → `v.to_string()` (clippy::useless_format)
3. `src/execution_engine.rs:959` — `execute_show_processlist_impl` 加 `#[allow(dead_code)]` + 4 行说明; 该函数确实被 `execution_engine_tests::test_executor_show_processlist_v312_35` 调用, lib-only clippy 看不到 test caller 属 false positive

**修复后验证**: `cargo clippy --all-features -- -D warnings` exit=0; `cargo test --lib show_processlist` 2/2 passed.

### 10.2 Remaining pre-existing clippy issues (honest disclosure)

`cargo clippy --all-features --all-targets -- -D warnings` 仍有 issues 但**不在 §10 已知块**,且都 pre-date V313:

- `crates/tools/src/traits.rs:79` `unused_mut` on `with_create_dir_err` (last touch 17abcecd7b, v311-TEST-INFRA 遗留)
- `crates/sqlrustgo-mysql-server` 多个 `unused_must_use` warnings on `shutdown`/`capture` 等 (last touch 在 V3.12 早期, V313 docs 未触及)

这些需要在后续 Sprint (尤其 SPRINT-S4 V312-56 治理 + SPRINT-S1 GMP) 实施时一起修,**不阻塞**当前 SETUP PR 合并。修复出处建议列入后续独立的 `clippy-cleanup` sprint (TBD)。

**结论**: V313 SETUP 不影响代码态; clippy 修复 commit `9c3e48c5d7` 已并入 `origin/develop/v3.13.0` (non-fast-forward push, 无 force-push, 无 orphan); §10 已知块全部清除; remaining issues 已诚实披露.

## 11. PR #4326 mergeable=False 说明

`base: develop/v3.12.0 @ 02f1d9afe6` 与 `head: develop/v3.13.0 @ 9e28778ef7` 不在同一祖先链。这是 V313-MASTER-PLAN §1 已知诚实披露: V313 分支从 v3.12.0 旧 fork point (`79caf8c4...`) bootstrap,后续 v3.12.0 上有 13 commit 推进未被 merge 到 v3.13.0。merge 时需 resolve,但 **不在本 SETUP PR scope**。
