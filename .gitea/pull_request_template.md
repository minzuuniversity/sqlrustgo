## PR Title
<!-- Use Conventional Commits: type(scope): subject -->
<!-- Examples: feat(executor): add F-23 Clustered Index, fix(gate): P1-1 INT debt -->

## Issue Reference
<!-- Required: link to the Gitea issue this PR closes -->
Closes #<issue-number>

## 5-类文档 (Required: 5/5)

<!-- All PRs MUST include or update these 5 documents. Check which apply. -->
- [ ] **SPEC** (`docs/releases/v3.8.0/<FEATURE>_SPEC.md`)
- [ ] **TEST_PLAN** (`docs/releases/v3.8.0/TEST_PLAN_INTEGRATED.md` or per-feature)
- [ ] **TEST_DESIGN** (describes how tests validate the feature)
- [ ] **REVIEW** (`docs/releases/v3.8.0/TEST_REVIEW_INTEGRATED.md` or per-feature)
- [ ] **ACCEPTANCE** (`docs/releases/v3.8.0/TEST_ACCEPTANCE_INTEGRATED.md` or per-feature)

## 5-原则 Compliance

<!-- All 5 principles must be satisfied. Mark each. -->
- [ ] **P1: 有计划必有实现** — Feature is implemented
- [ ] **P2: 有实现必有测试** — Test file added (path, name)
- [ ] **P3: 测试必审** — Test reviewed (per-test audit)
- [ ] **P4: 必须集成到门禁** — Test integrated to gate (D1-D8)
- [ ] **P5: 未过必记** — Failures documented (or 0 failures)

## Test Information

**New tests added**:
- File 1: `tests/<name>_test.rs` (N tests)
- File 2: `tests/<name>_test.rs` (N tests)
- (or "no new tests" with reason)

**Test results**:
```
cargo test --test <name> → N/N PASS
```

## Gate Integration

<!-- Which 8-dim gate does this PR affect? -->
- [ ] D1-Alpha
- [ ] D2-Beta
- [ ] D3-SGL
- [ ] D4-WAL
- [ ] D5-DeepSeek
- [ ] **D6: Test Inventory** (if new test file)
- [ ] **D7: INT Debt** (if affects INT-1~4)
- [ ] **D8: Arch/Sem Debt** (if affects ARCH/SEM)

**Gate command run**:
```bash
bash scripts/gate/check_<name>.sh → PASS/DRIFT/FAIL
```

## Cross-Version Debt (if applicable)

**Affected debt items** (use INT5 / COMPREHENSIVE_FEATURE_TRACKING / INT_DEBT_REMEDIATION_PLAN):
- F-XX: status (CLOSED / PARTIAL / OPEN)
- I-XX: status
- T-XX: status
- INT-XX: status
- ARCH-XX: status
- SEM-XX: status

## Breaking Changes

<!-- List any breaking API/schema changes. Use [NONE] if none. -->

## Checklist

- [ ] Code compiles: `cargo build --all-features`
- [ ] Tests pass: `cargo test --all-features`
- [ ] Lint clean: `cargo clippy --all-features -- -D warnings`
- [ ] Format: `cargo fmt --all`
- [ ] Docs link: `bash scripts/gate/check_docs_links.sh`
- [ ] Gate: all 8 dimensions verified
- [ ] No PENDING placeholders in docs (Truthfulness)

## Auto-Merge Eligibility

- [ ] PR is auto-mergeable (passes all gates)
- [ ] Or specify why manual merge is needed

---

🤖 Generated with [Hermes Agent](https://hermes-agent.nousresearch.com) + OpenSpec
