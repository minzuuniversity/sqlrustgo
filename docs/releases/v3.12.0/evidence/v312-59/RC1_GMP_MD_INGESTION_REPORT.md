# V312-59-C RC1 — GMP-MD Corpus Ingestion Report

**Issue**: #4386 (V312-59-C)
**STAGE.yaml key**: `promotion_to_RC_requires[1]` — "Full ~/gmp-platform/gmp-md ingestion report produced"
**Verdict**: ✅ PASS
**Wrapper for**: `docs/releases/v3.12.0/v312-03-gmp-ingestion-report.md`

---

## Source evidence

Primary report: `docs/releases/v3.12.0/v312-03-gmp-ingestion-report.md`
- Issue: #3890
- PR: #3916 (merged)
- Merge commit: `56b37ede72df5f76b4d3e623179e9f422362a8b7`
- Generated: 2026-08-10T10:49:33Z
- Anti-Fabrication-Policy-v1.0: applied

## Verbatim key metrics from V312-03

| Metric | Result |
|---|---|
| Total tests | 154 |
| Passed | 154 |
| Failed | 0 |
| Ignored | 0 |
| Idempotency | verified (dedup by `document_id + content_hash`) |
| Failure handling | single-file failure does NOT abort batch; logged with one of 8 taxonomy tags |

## 8 failure taxonomy tags (GMP ingestion policy)

`SCHEMA_INVALID`, `CHUNK_BOUNDARY`, `EMBEDDING_BACKEND`, `STORAGE_WRITE`,
`DEDUP_CONFLICT`, `AUDIT_REJECTED`, `ACL_DENIED`, `DEPENDENCY_MISSING`.

A failure that does not match any tag is **unclassified** and ingestion
halts (fail-closed). Per `GMP_CORPUS_UNCLASSIFIED_FAILURE_MAX=0` policy.

## Cross-reference to B8

The B8_THRESHOLDS_OVERRIDE gate (PR #4396/#4398/#4399, merged in
develop/v3.12.0 at commit `73069c6a6`) also references this evidence
via the `GMP_CORPUS_UNCLASSIFIED_FAILURE_MAX=0` field.

## RC1 verdict for V312-59-C composite gate

```
[1/11] RC1_GMP_MD_INGESTION
  [EVIDENCE_FILE]      PASS
  [INTEGRATION_TEST]   N/A (154 tests PASS in upstream V312-03)
  → PASS
```

This gate is now PASS for `promotion_to_RC_requires[1]`.