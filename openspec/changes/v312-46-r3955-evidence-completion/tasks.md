## 1. Compute evidence hashes

- [x] 1.1 `sha256sum docs/releases/v3.12.0/evidence/arch_invariants/R2.7.stdout`
- [x] 1.2 `sha256sum docs/releases/v3.12.0/evidence/sql_corpus/ALL_TARGETS_REPORT.md`
- [x] 1.3 `sha256sum docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md`

## 2. Update #3955 issue body (post-close edit)

- [x] 2.1 Add complete 7-field evidence block (source_agent, source_run, timestamp, commit SHA, commands, PASS/FAIL, evidence_hash, log paths)
- [x] 2.2 Cross-reference #3887 condition-by-condition

## 3. Post #3887 comment

- [x] 3.1 Notification: #3955 evidence completion (evidence_hash, log paths, #3887 勾选)
- [x] 3.2 Cross-reference: minimax agent 4 个 V312 issue 全部 closed (#3900, #3906, #3908, #3972, #3955)

## 4. Verification

- [x] 4.1 Re-read #3955 body to confirm all 7 fields present
- [x] 4.2 Verify #3887 comment cross-references #3955
- [x] 4.3 Update #3887 master checklist (V312-19 R2.7 corpus runner line)
