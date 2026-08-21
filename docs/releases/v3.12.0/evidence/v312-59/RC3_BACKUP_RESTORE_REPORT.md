# V312-59-C RC3 — Backup/Restore Verification Report

**Issue**: #4386 (V312-59-C)
**STAGE.yaml key**: `promotion_to_RC_requires[3]` — "Backup/restore preserves GMP documents, embeddings, graph relations, and audit chain"
**Verdict**: ✅ PASS
**Wrapper for**: `docs/releases/v3.12.0/v312-09-backup-restore-report.md`

---

## Source evidence

Primary report: `docs/releases/v3.12.0/v312-09-backup-restore-report.md`
- Issue: #3896
- PR: #3926 (merged)
- Merge commit: `d50b28330edb376268eb337ad36edc8e5a0a8a16`
- Generated: 2026-08-10T10:49:33Z
- Anti-Fabrication-Policy-v1.0: applied

## Verbatim key features from V312-09

- Backup binary SHA256 captured
- Restore preserves:
  - GMP documents (row-count parity)
  - Embeddings (vector equality)
  - Graph relations (edges + adjacency)
  - Audit chain (event_hash → previous_hash continuity)

## Roundtrip integrity checks (5 tables + 4 subsets per V312-09)

| Subset | Check |
|---|---|
| Documents | row-count restored == row-count backed up |
| Embeddings | vector equality per chunk_hash |
| Graph relations | edge count + adjacency parity |
| Audit chain | hash continuity from genesis → tip |
| Schema | DDL preserved verbatim |

## RC3 verdict for V312-59-C composite gate

```
[3/11] RC3_BACKUP_RESTORE
  [EVIDENCE_FILE]      PASS
  [INTEGRATION_TEST]   N/A (V312-09 evidence captured upstream)
  → PASS
```

This gate is now PASS for `promotion_to_RC_requires[3]`.