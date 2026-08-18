# SPRINT-S6: Array-fraction Quantile Cross-engine (Cluster E)

## §1 Scope

闭合 #4216 (quantile array-fraction,原 #4155 子项)。代码已合并 (per `V313-ROUND24-EVIDENCE-MANIFEST.md` §3.1),主要差 SF=1 cross-engine 验证。

## §2 Entry 标准

- [ ] SPRINT-S2 完成 (SQLite/PG oracle SF=1 就位)
- [x] quantile_disc/quantile_cont 数组输出代码已 merge (per memory `round-20-open-issue-scope-splits.md`: PR #4177 — feat(aggregate #4155): quantile_disc / quantile_cont — single-fraction form)
- [x] V313-STRICT-CLOSE-STANDARDS.md 已合并

## §3 入口任务

### §3.1 #4216 Array-fraction cross-engine 验证

- [ ] **在 SF=1 fixture 跑 quantile_disc/quantile_cont 数组输出** — 用 `tests/quantile_array_fraction.rs` 或同等
- [ ] **与 SQLite oracle SHA-256 对比** — `V313-S6-QUANTILE-SQLITE-SHA256.txt`
- [ ] **与 PostgreSQL oracle SHA-256 对比** — `V313-S6-QUANTILE-PG-SHA256.txt`
- [ ] **差异文档化** — 允许 float-divergence,bit-exact 优先;`V313-S6-QUANTILE-DIFF.md`
- [ ] **关闭 PR (merged to `develop/v3.13.0`)** — 用 V313-STRICT-CLOSE-STANDARDS.md §2 四要素

## §4 退出标准

- [ ] #4216 merged PR 到 `develop/v3.13.0`
- [ ] `bash scripts/gate/check_v313_array_fraction_gate.sh` exit=0
- [ ] cross-engine SHA-256 锚定文件存在
- [ ] `evidence/V313-S6-ARRAY-FRACTION-VERIFICATION.md` 已生成

## §5 风险与依赖

| Risk | 影响 | Mitigation |
|---|---|---|
| SQLite/PG array-fraction 函数实现差异 | false-positive diff | 列举 SQL 标准允许的差异 (例 array element ordering) |
| sqlrustgo quantile 数组输出格式与 oracle 不一致 | 全部 diff | 比 hash 前先 normalize (sort by quant key) |
| SPRINT-S2 未完成 | S6 入口 fail | 等待 S2 |

## §6 依赖

- SPRINT-S2 必须先完成 (SF=1 SQLite/PG oracle)
- 后续:无

## §7 预估工作量

约 3-5 天 (代码已有,主要差验证)。

## §8 References

- 上游 single-fraction 合并: PR #4177 (per memory `round-20-open-issue-scope-splits.md`)
- scope split 详情: memory `round-20-open-issue-scope-splits.md` (Round-20 #4216 split from #4155)
- Cluster E 索引: `V313-FOLLOWUP-INDEX.md` Cluster E
- 严格关闭标准: `V313-STRICT-CLOSE-STANDARDS.md`

## Evidence Hash

`sha256=58733cd5d1632e5c54877d828883e207c53a3620d0c96911b54ca1c0af11aa7b` (computed on file content at HEAD)