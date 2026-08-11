# V312-OPEN-ISSUE-REVIEW — Follow-up ISSUE INDEX

> **provenance:** generated_by=V312-OPEN-ISSUE-REVIEW, generated_at=2026-08-11T00:00:00+08:00, commit=ba03d8f2bcf7b5847451ccd3fde0f4d3eb131e6d, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, gate_policy_eval_id=v312-followup-index-001, source_agent=minimax-m2.7, source_run=2026-08-10-002, evidence_hash=bd4f91fe709ae978de124798fc12f8900eba03fbadb6167550b2ad995b59bb03

**生成时间**: 2026-08-11  
**HEAD Commit**: `ba03d8f2bcf7b5847451ccd3fde0f4d3eb131e6d`  
**Gitea 评论**: http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3887#issuecomment-89394

---

## Follow-up ISSUE 清单

| 编号 | ISSUE# | 标题 | 严重度 | owner | 截止日期 |
|------|--------|------|--------|-------|----------|
| **F-1** | [#4024](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4024) | typed-wrappers 1 test FAIL | HIGH | executor-agent | 2026-08-25 |
| **F-2** | [#4025](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4025) | e2e_wire_protocol 9 tests FAIL | HIGH | mysql-server-agent | 2026-08-25 |
| **F-3** | [#4026](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4026) | v3.11.0-ga tag 远程仓库同步 | MEDIUM | release-agent | 2026-08-20 |
| **F-4** | [#4027](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4027) | execution_engine.rs 拆分 | MEDIUM | executor-agent | 2026-09-15 |
| **F-5** | [#4028](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4028) | cargo fmt 违规整改 | MEDIUM | dev-tooling-agent | 2026-08-30 |
| **F-6** | [#4029](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4029) | v312_13 DEFERRED items | LOW | (保留 #3959) | v3.13+ |

## 时间表

| 截止 | 完成项 |
|------|--------|
| 2026-08-20 | F-3 |
| 2026-08-25 | F-1, F-2 |
| 2026-08-30 | F-5 |
| 2026-09-15 | F-4 |

## 关闭边界

每个 follow-up 关闭前必须：
1. 对应 `check_v312_*.sh` 门禁 status=pass
2. 提交 PR 含 evidence_hash + log_path
3. 在 #3887 INDEX 评论下回复 "✅ F-N closed at <sha>"
4. 更新 #3887 checklist

## #3887 Checklist 进度

- [x] Evidence binding FAIL=0
- [x] P16 gate test integrity PASS
- [x] SQLLogicTest gate PASS
- [x] AFP v4 PASS
- [x] 16 个 open ISSUE 逐项复核
- [x] 16 个 open ISSUE 整改方案制定
- [x] FAIL/PARTIAL/STUB/DEFERRED follow-up 拆分清单 (F-1 ~ F-6)
- [ ] F-1 closed (deadline 2026-08-25)
- [ ] F-2 closed (deadline 2026-08-25)
- [ ] F-3 closed (deadline 2026-08-20)
- [ ] F-4 closed (deadline 2026-09-15)
- [ ] F-5 closed (deadline 2026-08-30)
- [ ] F-6 closed (v3.13+)
- [ ] All F-1~F-6 closed → #3887 ready to close

---

## 关联提交

- PR #4023: V312-OPEN-ISSUE-REVIEW (merged)
- 报告: docs/releases/v3.12.0/evidence/V312-OPEN-ISSUE-REVIEW.md
- evidence_hash: bd4f91fe709ae978de124798fc12f8900eba03fbadb6167550b2ad995b59bb03
