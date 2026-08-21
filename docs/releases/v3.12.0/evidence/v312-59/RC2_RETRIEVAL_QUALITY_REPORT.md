# V312-59-C RC2 — Retrieval Quality Report

**Issue**: #4386 (V312-59-C)
**STAGE.yaml key**: `promotion_to_RC_requires[2]` — "Retrieval quality report produced from fixed GMP internal-audit question set"
**Verdict**: ✅ PASS
**Wrapper for**: `docs/releases/v3.12.0/v312-05-hybrid-retrieval-report.md`

---

## Source evidence

Primary report: `docs/releases/v3.12.0/v312-05-hybrid-retrieval-report.md`
- Issue: #3892
- PR: #3921 (merged)
- Merge commit: `ea648a40c07e6d0ac707a8a004bf55c84b1c2790`
- Generated: 2026-08-10T10:49:33Z
- Anti-Fabrication-Policy-v1.0: applied

## Verbatim key features from V312-05

- Hybrid retrieval (vector + keyword) returns source path, version, chunk hash, and citation text.
- Hit@1 / Hit@5 / MRR / citation completeness evaluated against the fixed GMP internal-audit question set.

## Cross-reference to B8

The B8_THRESHOLDS_OVERRIDE gate references this evidence via the
`GMP_RETRIEVAL_CITATION_REQUIRED=true` field (PASS).

## RC2 verdict for V312-59-C composite gate

```
[2/11] RC2_RETRIEVAL_QUALITY
  [EVIDENCE_FILE]      PASS
  [INTEGRATION_TEST]   N/A (V312-05 evidence captured upstream)
  → PASS
```

This gate is now PASS for `promotion_to_RC_requires[2]`.