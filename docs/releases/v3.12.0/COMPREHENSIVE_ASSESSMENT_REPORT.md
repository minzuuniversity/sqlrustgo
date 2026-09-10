# SQLRustGo v3.12.0 综合评估报告

> **provenance:** generated_by=codex, generated_at=2026-09-11T00:00:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **版本**: v3.12.0
> **阶段**: GA
> **GA 日期**: 2026-09-08
> **GA tag commit**: `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4`
> **当前文档刷新基线**: `9febebb255f984387ac78c26510d5b46d73f6046`
> **发布定位**: GMP 合规性内审检索系统数据库底座（受控场景）
> **证据索引**: [`GA_PUBLICATION_EVIDENCE_INDEX.md`](GA_PUBLICATION_EVIDENCE_INDEX.md)

## 1. 总体结论

v3.12.0 已达到受控范围 GA：GMP 内审检索数据库底座、SQL-backed 证据关系、混合检索元数据、审计链、恢复与导入导出能力均有 GA gate 证据支撑。正式 GA 结论绑定于 [`evidence/v312-59/ga_gate_report.json`](evidence/v312-59/ga_gate_report.json)，而不是本文档的文字判断。

| 维度 | 当前结论 | 证据 |
|---|---|---|
| Stage SSOT | GA | [`STAGE.yaml`](STAGE.yaml) |
| GA tags | `v3.12.0`, `v3.12.0-ga` | both dereference to `355b5a3837` |
| GA aggregate | 72/72 PASS, blockers 0 | `ga_gate_report.json`, generated_at `2026-09-08T04:17:15Z` |
| Product scope | GMP internal-audit retrieval database | `STAGE.yaml` positioning + this report |
| Known limitations | #4846/#4847/#4848 outside GA claims | 252 Gitea open issues + open PRs #4868/#4869/#4870 |

## 2. 能力完成度

| # | Capability | v3.12.0 GA status | Evidence boundary |
|---|---|---|---|
| 1 | GMP document / chunk / version / audit schema | PASS | RC1 wrapper evidence |
| 2 | GMP hybrid retrieval metadata | PASS | RC2 wrapper evidence |
| 3 | SQL-backed graph projection | PASS | GMP evidence reports |
| 4 | Audit hash-chain tamper fail-closed | PASS | RC1 / RC4 evidence |
| 5 | TPC-H SF=1 SQLRustGo path | PASS | GA5, 22/22 |
| 6 | Q17 SF=1 regression | PASS | 61.6s + cell-diff tolerance |
| 7 | SQLLogicTest smoke / selected targets | PASS-WITH-EXCLUSIONS | GA4 selected report; full SQLite corpus excluded |
| 8 | MySQL-style wire / LOAD DATA | PASS | GA6 report |
| 9 | Crash recovery / backup / upgrade | PASS | GA6 report |
| 10 | Security scan | PASS-WITH-RECORDED-CAVEATS | GA3 report |
| 11 | BustubX-EDU sqlite3-like CLI verified fixtures | PASS-WITH-SCOPE | V312-57 week01-week06 evidence |
| 12 | 168h production SOAK | NOT-CLAIMED | GA-2 demo/scaffold only; v3.13/post-GA hardening |
| 13 | Explicit transaction semantics | NOT-CLAIMED | #4847 open, PR #4870 open |
| 14 | `CHAR(n)` PAD SPACE equality / point lookup | NOT-CLAIMED | #4846 open, PR #4868 open |
| 15 | `ALTER TABLE ... RENAME COLUMN` | NOT-CLAIMED | #4848 open, PR #4869 open |

## 3. 正式发布允许声明

- SQLRustGo v3.12.0 GA is available for the GMP internal-audit retrieval database workload.
- The GA aggregate at tag commit `355b5a3837` records `72/72 PASS, blockers 0`.
- TPC-H SF=1 SQLRustGo path is recorded as 22/22 PASS within the GA evidence set.
- Wire, LOAD DATA, recovery, backup/restore, upgrade/downgrade, GMP matrix, and security gates are included in the GA aggregate.

## 4. 正式发布禁止声明

- Broad MySQL 5.7 replacement.
- Broad SQLite replacement or full official SQLite corpus compatibility.
- Complete explicit transaction semantics.
- General-purpose vector database.
- General-purpose graph database.
- Completed 168h production SOAK.
- #4846/#4847/#4848 have been fixed. Their PRs are open until merged and verified.

## 5. 风险评估

| Risk | Severity | GA handling |
|---|---|---|
| #4847 explicit transaction semantics | High | Excluded from GA claims; must be fixed before broader database GA wording |
| #4846 `CHAR(n)` PAD SPACE behavior | Medium/High | Excluded; affects teaching schemas using fixed-width keys |
| #4848 rename column | Medium | Excluded; catalog-evolution compatibility gap |
| 168h SOAK absent as completed artifact | Medium | Not claimed; demo/scaffold only |
| Gate log diagnostic in beta log | Medium | Recorded as evidence-quality caveat; future script cleanup required |

## 6. GA Publication Readiness

The release is ready for formal publication only under the scoped language above. The publication bundle must include:

- [`STAGE.yaml`](STAGE.yaml)
- [`GA_GATE_REPORT.md`](GA_GATE_REPORT.md)
- [`GA_RELEASE_REPORT.md`](GA_RELEASE_REPORT.md)
- [`RELEASE_NOTES.md`](RELEASE_NOTES.md)
- [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md)
- [`GA_PUBLICATION_EVIDENCE_INDEX.md`](GA_PUBLICATION_EVIDENCE_INDEX.md)

Any future claim expansion requires a fresh gate run, PR merge evidence for relevant issues, and an updated evidence index.
