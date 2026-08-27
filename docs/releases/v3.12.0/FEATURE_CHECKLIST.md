# v3.12.0 Feature Checklist

> **SSOT chain:**
> 1. `docs/governance/STAGE_CONFIG.yaml` (RC stage definition)
> 2. `docs/releases/v3.12.0/STAGE.yaml` (per-version promotion_to_RC_requires)
> 3. `docs/releases/v3.12.0/FEATURE_CHECKLIST.md` (this file)

> **Stage update (2026-08-26):** v3.12.0 is now in **RC** stage (entered 2026-08-26;
> BETA was 2026-08-19, ALPHA 2026-08-12). All feature rows below were originally
> verified at BETA promotion; they remain PASS for RC per `RC_GATE_REPORT.md`
> (12/12 RC items satisfied + B8 13/13 thresholds_override). The original BETA
> promotion snapshot is preserved below unchanged for traceability.

This document tracks feature-by-feature readiness for the v3.12.0 RC
promotion (originally BETA). Each row maps to one `promotion_to_BETA_requires`
bullet from `STAGE.yaml` and references the underlying evidence report.

| # | Feature | Status | Evidence | Gate Check |
|---|---------|:------:|----------|------------|
| 1 | GMP document/chunk/embedding/audit schema | DONE | `v312-02-gmp-schema-report.md` | B6_GMP_SCHEMA |
| 2 | Idempotent GMP markdown ingestion works on representative corpus | DONE | `v312-03-gmp-ingestion-report.md` | B6_GMP_INGESTION |
| 3 | Hybrid retrieval returns source path, version, chunk hash, citation text | DONE | `v312-05-hybrid-retrieval-report.md` | B6_HYBRID_RETRIEVAL |
| 4 | SQL-backed graph projection supports neighbors and depth-limited paths | DONE | `v312-06-graph-projection-report.md` | B6_GRAPH_PROJECTION |
| 5 | Audit hash-chain tamper tests fail closed | DONE | `v312-08-compliance-audit-report.md` | B6_AUDIT_HASH_CHAIN |
| 6 | SQLLogicTest smoke corpus runs through the gate and writes a report | DONE | `evidence/sqllogictest/smoke-report.md`, `evidence/sqllogictest/sqlite-corpus-manifest.json`, `evidence/sqllogictest/exclusions.yml` | B6_SQLLOGICTEST_SMOKE_GATE / B6_SQLLOGICTEST_MANIFEST / B6_SQLLOGICTEST_OPEN_EXCLUSIONS |
| 7 | TPC-H SF=1 correctness close-out plan has per-query artifact format | DONE | `evidence/G4_tpch_sf1.txt` | B6_TPCH_SF1_G4 |
| - | Embedding provider evidence | DONE | `v312-04-embedding-provider-report.md` | B6_EMBEDDING_PROVIDER |

## Cross-cutting Gate Status

| Gate | Result |
|------|:------:|
| Alpha gate composite (entry + quality) | PASS (B7_ALPHA_ENTRY; B7_ALPHA_QUALITY) |
| Beta v3.12.0 gate (this script) | tracked via `check_beta_v3.12.0.sh`; SQLLogicTest must be executed, not checked by stale report existence |

## Open Items at BETA Promotion

- [ ] All v3.12.0 alpha-targeted issues closed (Gitea unreachable at draft time;
      require re-verification post network restore)
- [ ] No P0 bugs open (same caveat)
- [ ] v3.12.0-beta1 tag cut (post alpha-targeted + P0 closure)

## Provenance

- `generated_by`: claude-macmini (Claude Code)
- `generated_at`: 2026-08-13
- `source_repo`: openclaw/sqlrustgo
- `branch`: develop/v3.12.0
- `gate_policy_eval_id`: v312-beta-feat-checklist-001
- `evidence_hash`: pending (recomputed when all `evidence/` files are
  frozen for BETA promotion)
