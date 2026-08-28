# V312-FOLLOWUP-INDEX Update — 2026-08-28

> **provenance**: generated_by=claude-macmini, generated_at=2026-08-28, branch=develop/v3.12.0, commit=4d8e8d44e7, source_repo=openclaw/sqlrustgo, policy=Anti-Fabrication-Policy-v1.0
> **related**: original snapshot at `docs/releases/v3.12.0/evidence/FOLLOWUP-INDEX.md` (frozen 2026-08-11, evidence_hash bd4f91fe...)
> **scope**: incremental update; does NOT replace the frozen 2026-08-11 snapshot

## 状态更新(2026-08-28)

The original FOLLOWUP-INDEX.md was frozen at 2026-08-11 with a fixed evidence_hash. As of 2026-08-28 commit 4d8e8d44e7, the actual state of the 6 follow-ups (F-1..F-6) is:

| 编号 | ISSUE# | 标题 | 原始截止 | 当前状态 | 关闭 commit / PR | 实际关闭时间 |
|---|---|---|---|---|---|---|
| F-1 | #4024 | typed-wrappers 1 test FAIL (v312_13_reset_connection_ok) | 2026-08-25 | **✅ closed** | (issue close timestamp 2026-08-10) | 2026-08-10 |
| F-2 | #4025 | e2e_wire_protocol 9 tests FAIL (SERVER_POOL state pollution) | 2026-08-25 | **✅ closed** | (issue close timestamp earlier) | < 2026-08-11 |
| F-3 | #4026 | v3.11.0-ga tag 远程仓库同步 (1/5 → 5/5) | 2026-08-20 | **✅ closed** | (issue close timestamp earlier) | < 2026-08-11 |
| F-4 | #4027 | execution_engine.rs 拆分 (C-ARCH-05, 1762 行 → ≤1500 行) | 2026-09-15 | **✅ closed** | (issue close timestamp earlier) | < 2026-08-11 |
| F-5 | #4028 | cargo fmt 违规整改 | 2026-08-30 | **✅ closed** | PR #4562 / commit `55b9bc9cc` | 2026-08-28 |
| F-6 | #4029 | v312_13 DEFERRED items | v3.13+ | **deferred** (未关闭,按 v3.13+ 推迟) | n/a | n/a |

**6 项中 5 项已关闭,F-6 按计划推迟到 v3.13+。**

## 关闭详情(本会话期间的工作)

### F-5 (#4028) — cargo fmt 违规整改

- **提交**: PR #4562 / commit `55b9bc9cc` (2026-08-28)
- **作者**: claude-macmini
- **范围**: `cargo fmt --all` 在 12 个源码文件(非 evidence 文件)
- **验证**:
  - `cargo fmt --all -- --check` exit 0
  - `cargo build --all-features` ok
  - `cargo test --lib` 114 passed / 1 pre-existing failure (`test_engine_grant_column_requires_catalog` — 与 fmt 无关,属 F-1 范畴;F-1 已关,该测试属于已知遗留,不在 F-5 关闭条件中)
- **PR 评论**: https://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4562

## 与 #3887 Checklist 的关系

原 FOLLOWUP-INDEX.md 的 #3887 Checklist 显示:

- [ ] F-1 closed (deadline 2026-08-25)  → **实际已关 2026-08-10**
- [ ] F-2 closed (deadline 2026-08-25)  → **实际已关 < 2026-08-11**
- [ ] F-3 closed (deadline 2026-08-20)  → **实际已关 < 2026-08-11**
- [ ] F-4 closed (deadline 2026-09-15)  → **实际已关 < 2026-08-11**
- [x] F-5 closed (deadline 2026-08-30)  → **本会话关闭**
- [ ] F-6 closed (v3.13+)              → **按计划推迟,未关**

#3887 的 "All F-1~F-6 closed → #3887 ready to close" 项需要 #4313 决策(是否 F-6 推迟符合预期)。

## 引用

- 原快照: `docs/releases/v3.12.0/evidence/FOLLOWUP-INDEX.md` (frozen 2026-08-11)
- umbrella #3887 (V312-MASTER)
- V312-59-D v2 cycle #4497
- Anti-Fabrication-Policy-v1.0 (新文档必须有 provenance + evidence 引用)

provenance: claude-macmini, 2026-08-28, post-#4563-merge.