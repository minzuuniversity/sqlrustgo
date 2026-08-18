# SPRINT-S2: TPC-H Cross-engine Oracle (Cluster B 总控)

## §1 Scope

闭合 #4221 (V312-48 TPC-H SF=1 总控) + #4272 (V312-48-CROSS-ENGINE SHA256)。基于 SPRINT-S0 已就位的 SF=1 fixture + MySQL oracle,完成 SQLite / PostgreSQL / MySQL / sqlrustgo 四向 SHA-256 cross-engine diff。

## §2 Entry 标准

- [x] `/tmp/tpch-sf1/*.tbl` 8 个表 fixture 就位 (commit `e45f57007e`)
- [x] MySQL oracle SHA-256 22/22 就位 (commit `0c752ebb38`)
- [x] SQLite + PostgreSQL oracle 在 SF=0.001 下 15/22 bit-exact (per memory `v312-48-zero-row-7x-closure.md`)
- [x] `V313-STRICT-CLOSE-STANDARDS.md` 已合并

## §3 入口任务

### §3.1 #4221 V312-48 SF=1 总控

- [ ] **22/22 query 在 sqlrustgo 跑 SF=1** — 必须 non-OOM;允许 7 query (Q5/Q8/Q9/Q10/Q13/Q16/Q18) zero-row per planner DEFERRED
- [ ] **sqlrustgo 22 query SHA-256 锚定** — `V313-S2-SQLRUSTGO-SHA256.txt`
- [ ] **SQLite oracle 升到 SF=1** — `V313-S2-SQLITE-SHA256.txt` (沿用 SF=0.001 baseline fixture 升级)
- [ ] **PostgreSQL oracle 升到 SF=1** — `V313-S2-PG-SHA256.txt`
- [ ] **三向 diff (sqlite/pg/sqlrustgo)** — `V313-S2-CROSS-DIFF.txt`,差异 ≤ 7 float-divergence (继承 baseline)
- [ ] **0 差异的 15 query 标记 bit-exact** — `V313-S2-BIT-EXACT-LIST.md`

### §3.2 #4272 V312-48-CROSS-ENGINE SHA256

- [ ] **MySQL oracle 22 query SHA-256** — 已就位 (commit `0c752ebb38`)
- [ ] **4 引擎对比表** — `V313-CROSS-ENGINE-22x4-SHA256.md` (每 query 4 行 + diff 状态)
- [ ] **zero-row 排除声明** — Q5/Q8/Q10/Q13/Q16 的 bit-exact claim 必须排除,允许 row-count==0 但内容不可比
- [ ] **float-divergence 文档** — 7 个允许浮点差异的 query 列出 divergence point

## §4 退出标准

- [ ] #4221 + #4272 merged PR 到 `develop/v3.13.0`
- [ ] `bash scripts/gate/check_v313_cross_engine_gate.sh` exit=0
- [ ] 4 引擎 SHA-256 锚定文件全部存在
- [ ] `docs/releases/v3.13.0/evidence/V313-S2-CROSS-ENGINE-VERIFICATION.md` 已生成
- [ ] Anti-Fabrication-Policy-v1.0:任何 divergence 诚实披露

## §5 已知 Honest Disclosure (per S0 evidence)

| Item | 来源 | 处理 |
|---|---|---|
| sqlrustgo 22-query 60s timeout (SF=1 smoke test) | `V313-S0-BLOCKERS-COMPLETE.md` §Honest Disclosures 1 | S2 实施时调 timeout 到 600s 或 batched |
| SQLite/PG oracle 仍 SF=0.001 | `V313-S0-BLOCKERS-COMPLETE.md` §Honest Disclosures 2 | S2 入口任务 §3.1 第 3-4 项升级 |
| zero-row 7 query (Q5/Q8/Q9/Q10/Q13/Q16/Q18) | `v312-48-zero-row-7x-closure.md` | S2 允许 row-count==0,Q5/Q8/Q10/Q13/Q16 由 S3 修复;Q9/Q18 期望 S2/S3 协作 |

## §6 风险与依赖

| Risk | 影响 | Mitigation |
|---|---|---|
| SQLite/PG 升 SF=1 内存/CPU 超 sandbox 容量 | S2 延期 | 用 SF=0.1 中间档验证流程 OK 后再 SF=1 |
| sqlrustgo SF=1 跑出与 oracle 大量 diff | 大返工 | 先做 Q1 单 query smoke 验证 sqlrustgo 可跑通 |
| MySQL 与 SQLite/PG 自身存在差异 (例如 timestamp) | false-positive diff | 排除 timestamp 字段,只比内容 |
| 4 引擎 SHA-256 体积管理 (22 × 4 = 88 锚定) | 文档维护负担 | 用 single index file 聚合 (`V313-CROSS-ENGINE-22x4-SHA256.md`) |

## §7 预估工作量

约 1 周 (主要等 SQLite/PG SF=1 oracle rerun + sqlrustgo SF=1 跑通)。

## §8 依赖

- SPRINT-S0 ✅
- SPRINT-S3 (等 S2 zero-row diff baseline 后开始)
- SPRINT-S6 (等 S2 cross-engine 框架就位)

## §9 References

- 上游 DEFERRED 提交: `1da964be77` (V312-47+48 meta closure per memory `v312-47-48-meta-closure.md`)
- 上游 SF=0.001 baseline: `72645d493d` (PR #4310 per memory `v312-48-zero-row-7x-closure.md`)
- S0 evidence: `evidence/V313-S0-BLOCKERS-COMPLETE.md`
- 严格关闭标准: `V313-STRICT-CLOSE-STANDARDS.md`

## Evidence Hash

`sha256=499b4e2705bdb87a1bdfd1319406f4a9817b1953f8a29d5623d3742c74bfcb20` (computed on file content at HEAD)