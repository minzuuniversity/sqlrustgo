# V312 Round-24 chatgpt/codex Strict Re-review Remediation Notice

> **provenance:** generated_by=openclaw-minimax, generated_at=2026-08-15T04:00:00Z, branch=develop/v3.12.0, commit=da965099ca4d1a2ee84a03f12187656d465b26f4, policy=Anti-Fabrication-Policy-v1.0
> **agent:** openclaw-minimax (Claude Code) · session continuation
> **trigger:** ChatGPT (codex GPT-5) Round-24 strict re-review 2026-08-15T03:47Z-03:48Z

## 1. Purpose

This notice documents the Round-24 chatgpt/codex strict re-review remediation applied to v3.12.0 evidence docs that previously contained `ACCEPTED-WITH-BINDING-MANIFEST (not DONE)` / `SUBSTANTIALLY_COMPLETE` / `DEFERRED` close markers. These markers violate the governance closing rule that **issue closures must provide real evidence (PR + merge commit + exit code + output summary + hash)** — or **keep the issue open as a tracking vehicle**.

## 2. Round-24 Findings (chatgpt/codex, GPT-5)

The strict re-review (2026-08-15T10:40 CST) re-examined all V312 closures and **reopened 24 issues** because their evidence chains contained weak markers:

| Issue | Round-24 finding |
|-------|------------------|
| #3887 V312-MASTER | Evidence chain mixed PASS / DEFERRED / SUBSTANTIALLY_COMPLETE — cannot be master-closed |
| #4216 V312-46 array-fraction | Closed with 0 comments, no PR, no evidence |
| #4220 V312-47 PARTIAL | Critical sub-items still reopened or Deferred-without-tracking |
| #4221 V312-48 TPC-H SF=1 | `DEFERRED-with-issue` / `not DONE` closure language |
| #4225 V312-52 GMP vector | PARTIAL/DEFERRED items for fixture / rebuild / dimension drift / empty index / model-name / unique index |
| #4226 V312-53 GMP compliance | PARTIAL/DEFERRED for AuditAction (Import/Export/Approve/Review/Backup/Restore) / hash-chain tamper / ACL full matrix / production wiring |
| #4250-#4258 V312-56 series (9 issues) | `SUBSTANTIALLY_COMPLETE` + `Next Steps` contradictions |
| #4272-#4279 V312-48 zero-row (8 issues) | `ACCEPTED-WITH-BINDING-MANIFEST (not DONE)` language |

## 3. Remediation Strategy (Deferral Path)

Per ChatGPT's two-path remediation guidance, all 24 issues adopt the **Deferral Path**:

1. **Keep all 24 issues open** (titles updated with `[v3.13 follow-up]` prefix)
2. **Bind to v3.13-MASTER** ([Issue #4313](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4313)) as the unified tracking vehicle
3. **Add `v3.13-followup` label** to all 24 issues
4. **Add deferral-path comment** documenting closing boundary, owner, expiry
5. **No "not DONE" closures** — all evidence docs remain as-is to preserve historical record

## 4. Strict Close Standards Going Forward (V313 governance)

For any V313 (or later) issue closure:

1. **PR merged** to `develop/v3.13.0` (or merge commit reachable on develop branch)
2. **Real evidence**: command + exit code + output summary + SHA-256 evidence hash
3. **No `ACCEPTED-WITH-BINDING-MANIFEST (not DONE)` / `SUBSTANTIALLY_COMPLETE` / `DEFERRED-without-tracking` as close markers**
4. **If deferral is necessary**: must keep issue open + bind to tracking issue with owner / expiry / closing boundary

## 5. Affected Evidence Docs (historical record retained)

The following evidence docs retain historical `DEFERRED` / `ACCEPTED-WITH-BINDING-MANIFEST` / `SUBSTANTIALLY_COMPLETE` language for **provenance purposes**. This notice supersedes that language for v3.13+ work:

- `v312_master/V312-3887-CLOSURE-VERIFICATION.md` — V312-MASTER closure attempt (Round-24 REJECTED)
- `tpch/V312-48-ZERO-ROW-7X-VERIFICATION.md` + 7 per-query docs — zero-row 7x closure (Round-24 REJECTED)
- `tpch/cross_engine_sf001/V312-46-CROSS-ENGINE-VERIFICATION.md` — V312-46 cross-engine (Round-24 REJECTED)
- `tpch/V312-48-TPCH-SF1-CORRECTNESS.md` + `tpch/V312-48-SUB-ISSUES-ANALYSIS.md` — V312-48 binding manifest
- `vector_retrieval/V312-52-REPORT.md` — V312-52 GMP vector (Round-24 REJECTED)
- `gmp_compliance/V312-53-REPORT.md` — V312-53 GMP compliance (Round-24 REJECTED)
- `v312-56/V312-56-VERIFICATION.md` — V312-56 4.0-前 teaching (Round-24 REJECTED)
- `wire_load_data/V312-50-REPORT.md`, `sqllogictest/V312-51-REPORT.md`, `crash_recovery/V312-14-CRASH-RECOVERY-RECHECK.md` — historical context

## 6. v3.12.0 Outcome Declaration

Per the Round-24 chatgpt/codex re-review:

- **v3.12.0 is released as an internal controlled subset** of full production capability
- **GMP vector/retrieval**: 受控基础子集 — not a replacement for standalone vector database
- **GMP compliance/access-control**: 受控基础子集 — not full audit/ACL coverage
- **TPC-H SF=1**: 受控基础子集 — partial coverage; full SF=1 cross-engine SHA256 deferred to v3.13
- **4.0 前补强 (V312-56 series)**: deferred to v3.13
- **Zero-row 7x (Q5/Q8/Q9/Q10/Q13/Q16/Q18)**: deferred to v3.13 with 2027-06-30 expiry

## 7. Reference

- v3.13-MASTER 总控: [Issue #4313](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4313)
- 252 canonical HEAD: `da965099ca4d1a2ee84a03f12187656d465b26f4`
- ChatGPT Round-24 strict re-review comment id 94228 (master #3887)
- Anti-Fabrication-Policy-v1.0

---

*This notice is a v3.12.0 → v3.13.0 transition record. It supersedes earlier closure language but does not rewrite historical evidence.*