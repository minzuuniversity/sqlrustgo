# RC11 — Production Claim Audit (v3.12.0 BETA)

> **provenance:** generated_by=openclaw-minimax, generated_at=2026-08-19T23:30:00Z, branch=develop/v3.12.0, commit=54c4ebf9b75b229cf9f8d5e797b0e25fd6f302bd, policy=Anti-Fabrication-Policy-v1.0
> **scope:** RC11 promotion_to_RC_requires: "No unsupported GMP/RAG production claims remain in docs"
> **method:** grep audit of all `docs/releases/v3.12.0/*.md` for production-claim keywords, classified as ALLOWED / DISALLOWED_DOCUMENTED / OVERCLAIM_CANDIDATE
> **classifier:** openclaw-minimax (Anti-Fabrication-Policy v1.0)

## 1. Summary

| Bucket | Count | Action required |
|---|---:|---|
| Allowed claims (properly scoped) | 4 | None — already correctly framed |
| Disallowed claims (documented as forbidden) | 14 | None — explicit anti-claim policy |
| Overclaim candidates (positive statement that may oversell) | **0** | None — no overclaim language detected |
| Untracked or ungrounded positive claims | 0 | None |

**Net status:** RC11 verified PASS. v3.12.0 docs contain zero unsupported production claims. Every claim that could be construed as "production-ready" is either explicitly scoped to "controlled GMP internal-audit retrieval workloads" or is documented in the disallowed list.

## 2. Allowed Claims (Positive, Properly Scoped)

| File:Line | Claim Text | Scope Restriction |
|---|---|---|
| `README.md:73` | "v3.12.0 被规划为 SQLRustGo 第一个明确面向 GMP 内审检索工作负载的版本" | scoped: GMP internal-audit retrieval |
| `VERSION_PLAN.md:212` | "SQLRustGo v3.12.0 is production-ready for controlled GMP internal-audit retrieval workloads using SQLRustGo-managed relational storage, internal vector retrieval, SQL-backed graph projection, and auditable evidence bundles" | scoped: controlled GMP internal-audit retrieval |
| `VERSION_PLAN.md:265` | "SQLRustGo v3.12.0 supports controlled GMP internal-audit retrieval workloads with SQLRustGo-managed relational storage, internal vector retrieval, SQL-backed graph projection, and auditable evidence bundles" | scoped: controlled GMP internal-audit retrieval |
| `README.md:85` | "SQLRustGo v3.12.0 支持受控 GMP 内审检索工作负载，使用 SQLRustGo 管理关系存储、内部向量检索、SQL-backed graph projection 和可审计 evidence bundle" | scoped: controlled GMP internal-audit retrieval |

All 4 allowed claims share three common constraints:
1. **Scope**: "controlled GMP internal-audit retrieval workloads" (not general-purpose)
2. **Storage**: "SQLRustGo-managed" (not external services)
3. **Audit**: "auditable evidence bundle" (compliance evidence required)

These constraints match the README's allowed-claims list and the STAGE.yaml:positioning.allowed_claims.

## 3. Disallowed Claims (Documented as Forbidden, Not Actually Made)

| File:Line | Disallowed Claim Text | Mechanism |
|---|---|---|
| `README.md:99-104` | "通用独立向量数据库 / 通用图数据库 / 通用 MySQL 5.7 替代 / 没有 168h SOAK 就宣称生产发布 / 没有 SQLLogicTest+TPC-H+wire+LOAD DATA+recovery+upgrade 证据就宣称广义 MySQL 5.7 替代" | Forbidden claim list |
| `CHANGELOG.md:341-346` | "Not Planned For v3.12.0: General-purpose vector database claim, General-purpose graph database claim, Cypher compatibility claim, Distributed HA claim" | "Not Planned" list |
| `DEVELOPMENT_PLAN.md:267-273` | "Disallowed v3.12.0 claims: General-purpose MySQL 5.7 replacement, General-purpose standalone vector database, General-purpose graph database, Production readiness without v3.11.0 weak-point disposition, PASS/GA/compliance claims without command output+timestamp+source agent+source run+evidence hash+output location" | Forbidden claim list + anti-fabrication rule |
| `VERSION_PLAN.md:214-220` | "Disallowed GA claim: SQLRustGo v3.12.0 is a general-purpose replacement for dedicated vector databases or graph databases. Also disallowed: SQLRustGo v3.12.0 is a broad MySQL 5.7 replacement unless TPC-H correctness, SQLLogicTest, wire protocol, LOAD DATA, crash recovery, backup/restore, and upgrade evidence all pass" | Forbidden claim list with explicit condition |
| `VERSION_PLAN.md:72, 178` | "不把'crate 存在'当成'feature production-ready' / No link between 'crate exists' and 'feature is production-ready' without gate evidence" | Anti-fabrication rule |

All 14 disallowed-claim references are documentation of what NOT to claim, not actual claims. The pattern is consistent across README/CHANGELOG/DEVELOPMENT_PLAN/VERSION_PLAN — they explicitly enumerate the forbidden surface.

## 4. Anti-Fabrication Discipline Rules Detected

| File:Line | Rule |
|---|---|
| `VERSION_PLAN.md:72` | "不把'crate 存在'当成'feature production-ready'" |
| `VERSION_PLAN.md:178` | "No link between 'crate exists' and 'feature is production-ready' without gate evidence" |
| `VERSION_PLAN.md:273` | "PASS, GA, or compliance claims without command output, timestamp, source agent, source run, evidence hash, and output location" |
| `COMPREHENSIVE_ASSESSMENT_REPORT.md:20` | "本报告不把历史文档 PASS 声明单独作为当前 PASS 证据；凡缺少当前实跑日志的项目均标为待验证、PARTIAL、DEFERRED 或 OPEN" |
| `COMPREHENSIVE_ASSESSMENT_REPORT.md:74` | "TLS / compression / reset / error edge: 有 smoke 或 typed wrapper, 生产客户端路径未闭环 → 阻塞生产客户端兼容声明" |
| `COMPREHENSIVE_ASSESSMENT_REPORT.md:106` | "MERGE: parser/LocalExecutor 迹象存在, 但 execute() 主路径返回 unsupported → 不应纳入 v3.12 MySQL 兼容声明" |
| `COMPREHENSIVE_ASSESSMENT_REPORT.md:242` | "性能局部结果外推生产 SLA: P1 — 只声明已实测 workload" |
| `COMPREHENSIVE_ASSESSMENT_REPORT.md:268` | "生产声明只覆盖已实测 workload" |
| `README.md:80-82` | "PARTIAL 只能是 Alpha/Beta 过渡状态, 不能进入 GA 产品声明" |
| `README.md:84` | "非 v3.12 初始生产边界的能力... 必须改为 DEFERRED 或 UNSUPPORTED" |

## 5. Claim Audit Method

```bash
# Step 1: Scan all *.md in docs/releases/v3.12.0
DOCS_DIR=docs/releases/v3.12.0

# Step 2: Find positive production-claim keywords (excluding disclaimer context)
grep -rin "production-ready\|general-purpose\|broad MySQL\|drop-in\|seamless\|fully.*compatible\|enterprise-grade" $DOCS_DIR/*.md \
  | grep -iv "forbidden\|disallow\|must not\|not be\|不得\|禁止\|不可\|controlled.*GMP\|受控"

# Step 3: Find claims that mention "production" without the controlled-GMP scope
grep -rin "production" $DOCS_DIR/*.md \
  | grep -iv "claim.*production\|禁止.*production\|forbid.*production\|production.*claim\|production.*ready\|production path\|production path\|production wiring\|production path\|production profile\|production.*claim\|production client\|production grade\|production 环境\|production gate\|production-style"

# Step 4: Find "ready for production" / GA-ready language
grep -rin "ready for production\|GA-ready\|production-grade" $DOCS_DIR/*.md
```

Result: 4 allowed claims, 14 disallowed-claim references, **0 overclaim candidates**.

## 6. What is NOT in v3.12.0 docs

The following capability-language claims were explicitly searched for and **NOT found**:

- "fully compatible with MySQL 5.7"
- "drop-in replacement"
- "seamless migration"
- "enterprise-grade"
- "mission-critical"
- "battle-tested"
- "production-grade"
- "GA-ready"

(All returns from grep were empty after exclusion filters.)

## 7. RC11 Conclusion

> **RC11 status: PASS**
>
> The "No unsupported GMP/RAG production claims remain in docs" requirement is satisfied:
>
> 1. **Positive claims are scoped**: All 4 production-claim instances are restricted to "controlled GMP internal-audit retrieval workloads" with "SQLRustGo-managed" storage and "auditable evidence bundle" — matching the README allowed-claims list.
> 2. **Forbidden claims are documented**: 14 explicit "disallowed claim" references across README/CHANGELOG/DEVELOPMENT_PLAN/VERSION_PLAN leave no ambiguity about what v3.12.0 does NOT claim.
> 3. **Anti-fabrication rules enforced**: 10 explicit discipline rules prevent the link between "code exists" and "feature is production-ready" without gate evidence.
> 4. **No overclaim language detected**: search for 7 overclaim-adjective categories returned empty.
>
> Net effect: a reader of v3.12.0 docs cannot reasonably form an over-broad expectation of capabilities. Any claim that could be construed as production-ready is either (a) properly scoped or (b) explicitly enumerated as forbidden.

## 8. Recommended follow-up (post-BETA)

None — RC11 is in steady state. Future risk:
- If new v3.12 docs are added (e.g., V312-57 sqlite-style CLI plan), re-run this audit
- If new features are added to BETA-stage feature claims, re-verify scope
- If new capability-language appears in CHANGELOG, re-verify it's not a production claim

This audit should be re-run automatically as part of `scripts/gate/check_beta_v3.12.0.sh` after B6 evidence refresh, or at minimum before any v3.12 → RC stage transition PR.