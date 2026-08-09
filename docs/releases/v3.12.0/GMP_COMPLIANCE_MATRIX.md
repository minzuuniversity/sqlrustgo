# SQLRustGo v3.12.0 GMP Compliance Matrix

> **Version**: v3.12.0
> **Status**: PLANNED
> **Date**: 2026-08-08

This matrix maps GMP internal-audit retrieval controls to implementation and test evidence. Rows must remain PLANNED until tests exist and have execution evidence.

| Control | Requirement | Planned implementation | Planned test | Status |
|---|---|---|---|---|
| Attributable | Every regulated action has an actor | `gmp_audit_log.actor` | audit insert requires actor | PLANNED |
| Legible | Retrieved evidence is readable and source-linked | chunk text + source path + citation | retrieval result includes citation text | PLANNED |
| Contemporaneous | Event timestamps are recorded at action time | audit timestamp from system clock | timestamp exists on import/search/export | PLANNED |
| Original | Source document version and hash are preserved | document source hash + version table | re-import creates version, not overwrite | PLANNED |
| Accurate | Search evidence can be verified | chunk hash + embedding hash | returned chunk hash matches stored text | PLANNED |
| Complete | Corpus ingestion reports all files | ingestion report | no unclassified failures | PLANNED |
| Consistent | Data model is deterministic | stable document/chunk ids | same corpus yields same ids | PLANNED |
| Enduring | Backup/restore preserves records | backup/restore gate | restored hashes match source | PLANNED |
| Available | Authorized users can retrieve evidence | ACL + retrieval API | permitted role can retrieve | PLANNED |
| Access Control | Unauthorized access fails closed | role checks | restricted chunk denied | PLANNED |
| Audit Trail | Tamper-evident audit chain | previous hash + event hash | tamper test fails closed | PLANNED |
| E-Signature | Approval/export can be signed | signature hook | approval requires signature payload | PLANNED |
| Traceability | Findings link to SOP/CAPA/clause evidence | graph projection tables | path query returns evidence bundle | PLANNED |
| Retrieval Quality | Internal-audit questions find relevant docs | hybrid retrieval + RRF | fixture hit-rate report | PLANNED |
