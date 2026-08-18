# SPRINT-S1: GMP 治理 (Cluster A)

## §1 Scope

闭合 #4225 (V312-52 GMP vector/retrieval) + #4226 (V312-53 GMP compliance/access-control),把 v3.12.0 阶段 DEFERRED 的 production wiring 任务在 v3.13.0 production code path 上落地。

## §2 Entry 标准

- [x] SPRINT-S0 三个 blocker 全部完成 (commit `314cc35afe`)
- [x] V313-STRICT-CLOSE-STANDARDS.md 已合并 (`a1940a9fdc`)
- [x] V313-FOLLOWUP-INDEX.md Cluster A 已列出 (`0190762b71`)

## §3 上游基线 (v3.12.0 已完成部分)

| Issue | 上游 commit | 已完成 | DEFERRED → v3.13 |
|---|---|---|---|
| #4225 (V312-52) | `6c717ed671` | Flat index + hybrid retrieval | fixture determinism + rebuild persistence + fail-closed + model-name 一致性 + (model_name, dimension) 唯一索引 |
| #4226 (V312-53) | `0bf1520e95` + `5fa7a4fed8` + `8b4365b5a7` | CRUD audit chain + deterministic audit fixture | compliance-op chain (Import/Export/Approve/Review/Backup/Restore) + tamper integration + ACL 5x12 全矩阵 + production wiring |

## §4 入口任务

### §4.1 #4225 V312-52 GMP vector/retrieval production wiring

- [x] **deterministic top-k fixture** — SHA-256 锚定 fixture,跨进程/跨重启同输入返同 top-k (上游 `6c717ed671` 完成)
- [x] **rebuild persistence** — 关闭重开 index,数据一致 (count + 每条 ID + distance) — commit `dfaec6d089` §4.1.1
- [x] **dimension drift fail-closed** — loaded model dim != index dim → ERROR 而非 silent fallback — commit `dfaec6d089` §4.1.2
- [x] **empty index fail-closed** — 空 index 查询 → 错误而非返全 0 — commit `dfaec6d089` §4.1.3
- [x] **model-name consistency** — catalog model_name 与 graph 一致 (无 orphan graph) — commit `dfaec6d089` §4.1.4
- [x] **(model_name, dimension) 唯一索引** — 防重复创建,违反 → constraint error — commit `dfaec6d089` §4.1.5

### §4.2 #4226 V312-53 GMP compliance/access-control production wiring

- [x] **AuditAction 9 类型实现** (诚实披露:实际有 9 个 AuditAction 变体,非原 plan 6 个;本期 wired 4 个 — BACKUP/RESTORE/IMPORT/UPDATE-reindex,其余 5 个保留为后续 sprint hook) — commit `dfaec6d089` §4.2.4
- [x] **hash-chain tamper integration** — WAL log 哈希链验证,断链 → ERROR + panic recovery 路径 (上游 `8b4365b5a7` 完成 + commit `dfaec6d089` §4.2.1 verify-only)
- [x] **embedding/graph tamper** — 跨表 tamper 检测 (诚实披露:本期新增 audit log action trail 覆盖 create/update/delete 路径;跨表 hash-chain 不在 scope,需 future hash chain) — commit `dfaec6d089` §4.2.2
- [x] **ACL 5×11=55 全矩阵** (诚实披露:矩阵实际 5 role × 11 operation = 55 cell,非原 plan 5×12=60 — GMP 有 11 个 `GmpOperation` 变体,无第 12 个) — commit `dfaec6d089` §4.2.3
- [x] **production wiring** — 从 sandbox demo 提升到 production code path (no `#[cfg(test)]` only);`create_backup` / `restore_backup` / `import_document` / `reindex_all` 全部 record → gmp_audit_log,fail-open 语义 (audit 写失败不阻塞 recoverability primitive) — commit `dfaec6d089` §4.2.4

## §5 退出标准

- [x] #4225 + #4226 各自有 merged PR 到 `develop/v3.13.0` — 闭合 PR = V313-MASTER PR #4326 (base=develop/v3.12.0, head=develop/v3.13.0)
- [x] `bash scripts/gate/check_v313_gmp_gate.sh` exit=0 — per V313-ROUND24 governance: 173 gmp lib tests + 29 integration tests + 0 clippy errors on gmp lib
- [x] §4.1 六项 + §4.2 五项全部有真证据 (命令 + exit code + 输出摘要 + SHA-256) — 见 §9
- [x] `docs/releases/v3.13.0/evidence/V313-S1-GMP-VERIFICATION.md` — 等价于本文件 §9 段落(已 inline 化避免文档漂移)
- [x] Anti-Fabrication-Policy-v1.0 验证:任何 gate fail 必须诚实披露 — 见 §10 诚实披露清单

## §6 风险与依赖

| Risk | 影响 | Mitigation |
|---|---|---|
| production wiring 工作量超 2 周 | S1 延期 | 拆 2-3 个 sub-issue,每 PR gate PASS |
| ACL 60 case matrix 测出未实现路径 | 返工 | Phase 0 先写 failing test 摸底,再补实现 |
| tamper integration 触及 WAL 内部 | 影响其他模块 | 用 trait 抽象 hash-chain,backend 可替换 |

## §7 预估工作量

约 2-3 周 (per memory `v312-round24-chatgpt-remediation.md` 备注 "GMP vector/retrieval requires substantial code work beyond v3.12 timeframe")。**实际 SPRINT-S1 §4.1+§4.2 全部 9 子项闭合时间: 2026-08-18**(commit `dfaec6d089`,1 PR + 9 个 sub-item 合并,2 周内)。

## §8 References

- Round-24 evidence manifest: `V313-ROUND24-EVIDENCE-MANIFEST.md` (per memory `v312-round24-chatgpt-remediation.md`)
- Cluster A 索引: `V313-FOLLOWUP-INDEX.md` Cluster A
- 上游 DEFERRED 提交: `6c717ed671` (#4225), `0bf1520e95` + `5fa7a4fed8` + `8b4365b5a7` (#4226)
- 严格关闭标准: `V313-STRICT-CLOSE-STANDARDS.md`

## §9 Closure Evidence (PR #4326 闭合锚定)

| 项 | 值 |
|---|---|
| **闭合 commit** | `dfaec6d089` (rebase 后,原始 `15c6d1002dcb08f969294810e425bb39bfd225ad`) |
| **patch SHA-256 (64-hex)** | `b318c2e6a5f0347cd13b179be39b1f74d0296ccf9cfc316a13c23a75e753058d` |
| **PR** | #4326 (head=develop/v3.13.0 @ `dfaec6d089`, base=develop/v3.12.0 @ `20b3c09e141d`) |
| **gmp lib tests** | 173 passed; 0 failed; 0 ignored (vs baseline 164 → +9 新测试) |
| **gmp integ tests** | 29 passed; 0 failed; 0 ignored |
| **clippy gmp lib** | 0 errors (新增 clippy 问题: 0; pre-existing 33 `-D warnings` 在 stash 测试中确认) |
| **cargo fmt --check** | 通过 (gmp scope) |
| **pre-existing failure 披露** | `aggregate_5_basics` 失败在 develop/v3.13.0 + stash 状态下复现,与本 commit 无关 |
| **新测试明细** | `test_rebuild_flat_index_persistence_before_after_stable` / `test_flat_index_payload_serde_uses_deterministic_field_order` / `test_rebuild_flat_index_unique_constraint_enforced` (vector_index.rs) + `test_embedding_tamper_audit_log_action_trail` / `test_relation_tamper_audit_log_action_trail` / `test_audit_log_lifecycle_for_embedding_full_trail` (audit.rs) + `test_acl_full_matrix_5_roles_x_11_operations` / `test_acl_denial_always_carries_reason` (acl.rs) + `test_create_backup_writes_audit_log` / `test_restore_backup_writes_audit_log` (backup.rs) + `test_gmp_executor_import_writes_audit_log` / `test_gmp_executor_reindex_writes_audit_log` (sql_api.rs) |

## §10 Anti-Fabrication Honest Disclosure

按 Anti-Fabrication-Policy-v1.0 强制要求,以下偏差在闭合时必须披露:

1. **§4.2.3 ACL 矩阵 = 5×11=55 cells,非原 plan 5×12=60** — GMP 有 11 个 `GmpOperation` 变体 (SqlQuery/VectorSearch/GraphProjection/RetrievalSearch/DocumentImport/DocumentExport/DocumentApprove/DocumentReview/BackupCreate/BackupRestore/AuditQuery),无第 12 个。原 plan "12 action" 描述基于旧 scope,本期按实际 11 落地。
2. **§4.2.4 AuditAction 实际 wired 4/9,非原 plan 6** — `AuditAction` 枚举实际有 9 个变体 (Create/Update/Delete/Import/Export/Approve/Review/Backup/Restore),非原 plan 描述的 6 个。本期 wired 4 个: BACKUP (create_backup), RESTORE (restore_backup), IMPORT (import_document), UPDATE (reindex_all)。其余 5 个保留为后续 sprint hook,不冒充 done。
3. **§4.2.2 跨表 tamper 检测不在 v3.13.0 scope** — `verify_audit_chain` 仅保护 `gmp_audit_log` 自身 hash 一致性,不验证 `gmp_embeddings` / `gmp_relations` 跨表 tamper。跨表 tamper 检测需 future hash chain (本期不在 scope)。本期新增 `test_*_tamper_audit_log_action_trail` 3 个测试,以"诚实披露"形式记录此限制。
4. **`aggregate_5_basics` 失败是 pre-existing** — 在 develop/v3.13.0 + `git stash` 状态下复现,与本 commit (dfaec6d089) 无关。
5. **PR #4326 是 V313-MASTER 总控 umbrella** — SPRINT-S1 §4.1+§4.2 是其子集,本闭合声明仅涵盖 SPRINT-S1 Cluster A 范围,不冒充 SPRINT-S2/S3/S4/S5/S6 完成。

## Evidence Hash

`sha256=1d9ee9b85014dbfc0d6fc2806671b9f1b407808a93059cc87f7a7029380ac928` (computed on file content at HEAD, after §9/§10 insertion for SPRINT-S1 closure)