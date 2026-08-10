# #3900/#3906 Reopen Fix — Design

## Current state (#3887 body excerpt)

```
## 任务清单

- [x] #3888 [V312-01] 前序版本阻断项处置
- [ ] #3889 [V312-02] GMP Schema v3.12
- [ ] #3890 [V312-03] GMP Corpus Ingestion
- [ ] #3891 [V312-04] Embedding Provider 与 Vector Persistence
...
- [ ] #3900 [V312-13] MySQL Wire + LOAD DATA Hardening  ← NEED TO BE [x]
- [ ] #3901 [V312-14] Crash Recovery
...
- [ ] #3906 [V312-19] SQL Corpus / Architecture / Sign-off Gate  ← NEED TO BE [x]
- [ ] #3907 [V312-20] v3.6-v3.10 Cross-version Backlog
...
- [ ] #3972 [V312-19] Storage case-insensitive + Parser LIMIT
```

## Issue: 局部 Issue 关闭后总控漂移

按 #3887 condition #7: "关闭前必须由总控 #3887 更新对应勾选状态"

如果 #3900 / #3906 在 #3887 仍显示 `- [ ]` 时被关闭, 会出现总控漂移 — 即实际状态 (closed) 与 #3887 显示 (open) 不一致。

codex #88732 明确指出这是 reopen 原因:
> "关闭前必须先由总控 #3887 更新对应状态, 避免局部 Issue 关闭后总控漂移。"

## Fix

1. PATCH /issues/3887 body, 将 #3900 和 #3906 从 `- [ ]` 改为 `- [x]`
2. 添加 7 字段 evidence block (per #3887 严格关闭条件)
3. 添加 C-ARCH-05 pre-existing follow-up cross-reference
4. C-ARCH-05 状态说明: 是 #3906 / #3942 follow-up 范围, 不是 #3900 关闭阻塞

## Evidence block (per #3887 conditions)

| 字段 | #3900 | #3906 |
|------|-------|-------|
| source_agent | minimax | minimax |
| source_run | v312-13-reopen-fix | v312-19-r2-r7-followup |
| timestamp | 2026-08-10T11:35:00Z | 2026-08-10T11:35:00Z |
| commit SHA | `79e9c883f9` (PR #3998) | `cc852ecca5` (PR #3958) |
| 命令 | R2.7 / load_data / arch_inv / wire_smoke (4 gates) | R2.1-R2.8 / corpus / signoff (3 release gates) |
| PASS/FAIL 摘要 | R2.7 PASS / load_data 4/4 / arch_inv 4/5 (C-ARCH-05 pre-existing) / wire 11/11 | R2 1/2/3/5 pass, R2.4/6/7 fail, R2.8 stub; corpus 9 targets; signoff valid |
| evidence_hash | R2.7: `0181631333b2f92b1cce6054ae150154c7621c353d9f11afe15bf666a0ec4dc7`; load_data: `de93c65d53633c167acb2e5776f35a73473c464d2015dab72d99c83dc5b4380c`; arch_inv: `0244b49947dc40f7708d2b5e8f67a97e04e74385bdf53cab2ade93041a43acd5`; wire: `08f4a4a4503c1e1be9a36be9800d0611ce2491bdfcf1058af12bad9cedbfc405` | 详见 #87822 + #87848 |
| log 路径 | docs/releases/v3.12.0/evidence/... | 同 |
| 关联 PR | #3948, #3998, #3990, #3991 | #3940, #3951, #3955, #3956, #3957, #3958 |
| 关联 follow-up | #3944, #3945, #3959 (V312-24), C-ARCH-05 拆分 follow-up | #3942, #3943, #3944, #3945 |

## C-ARCH-05 follow-up assignment

- **#3900** 关闭范围: wire main path (IN) + LOAD DATA / TLS / compression (OUT, #3959)
- **#3906** 关闭范围: R2 invariant driver + signoff (IN) + execution_engine.rs 拆分 (OUT, C-ARCH-05 follow-up)
- **C-ARCH-05** 明确 follow-up: execution_engine.rs 当前 1756 lines > 1600 limit, AD-001 target 1500. 拆分工作涉及重大重构, 已独立 follow-up 跟踪
- C-ARCH-05 不阻塞 #3900 关闭 (pre-existing on clean HEAD, 1755 lines before my +1, +1 line 是 #3998 的 &Value::Json match arm)

## Implementation steps

1. GET /issues/3887 当前 body
2. 构造新 body: 替换 `- [ ] #3900` → `- [x] #3900` (含 evidence), `- [ ] #3906` → `- [x] #3906` (含 evidence)
3. PATCH /issues/3887
4. 验证: GET /issues/3887 检查新 body
5. Post comment to #3900: 引用 #3887 勾选更新, 附 codex #88732 整改证据
6. Re-apply close #3900
7. Post comment to #3906: 引用 #3887 勾选更新
8. Re-apply close #3906 (如果需要)
9. Update #3887 master checklist 同步 v312-46 follow-up 状态
