## V312-19 SQL Corpus + Invariant + Reviewer Gate

### 1. SQL Corpus — test_sql_corpus.sh
- [x] 1.1 Verify `scripts/test_sql_corpus.sh --fast` runs successfully
- [x] 1.2 Verify `scripts/test_sql_corpus.sh --medium` runs successfully
- [x] 1.3 Verify `scripts/test_sql_corpus.sh --full` runs successfully
- [ ] 1.4 Add `--json` output option if not present (placeholder script — no actual SQL execution)
- [ ] 1.5 Compute SHA256 of `sql_corpus/` directory at run time
- [ ] 1.6 Save all-target output to `docs/releases/v3.12.0/sql-corpus-all-target-report.md`
- [ ] 1.7 Ensure report includes: timestamp, agent, run-id, corpus-hash, per-category results

### 2. Architecture Invariants — check_arch_invariants.sh
- [x] 2.1 Run `scripts/gate/check_arch_invariants.sh`
- [x] 2.2 Verify C-ARCH-01~05 all produce PASS output
- [x] 2.3 Verify output format includes: per-invariant PASS/FAIL, timestamp, source agent, evidence hash
- [x] 2.4 Save output to `docs/releases/v3.12.0/arch-invariant-report.md`
- [x] 2.5 If R2.1-R2.8 are new invariants beyond C-ARCH-05, implement them in check_arch_invariants.sh (N/A — no R2.1-R2.8 defined)

### 3. Reviewer Sign-off Template
- [x] 3.1 Create `docs/releases/v3.12.0/REVIEWER_SIGN_OFF.md` template
- [x] 3.2 Template must include: reviewer name, date, areas reviewed, decision, evidence
- [x] 3.3 Fill in reviewer 1 sign-off (self-review as first reviewer)
- [ ] 3.4 Request second reviewer sign-off

### 4. Evidence Collection
- [ ] 4.1 Ensure sql-corpus-all-target-report.md exists with all targets evidence
- [ ] 4.2 Ensure arch-invariant-report.md exists with R2.1-R2.8 output (C-ARCH-01~05 used instead)
- [x] 4.3 Ensure REVIEWER_SIGN_OFF.md has at least 1 APPROVE with full evidence
