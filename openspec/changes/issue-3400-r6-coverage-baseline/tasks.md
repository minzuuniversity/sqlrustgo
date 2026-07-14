# Tasks — Issue #3400

## 1. Tooling
- [x] 1.1 `cargo install cargo-llvm-cov` — installed 0.8.7.
- [x] 1.2 Confirm tool reachable: `cargo llvm-cov --version` → 0.8.7.

## 2. Script
- [ ] 2.1 Create `scripts/coverage/llvm_cov_baseline.sh` (workspace-wide `cargo llvm-cov --lib --json` per crate).
- [ ] 2.2 Script: emit one `<crate>-lib.json` per workspace member to `docs/releases/v3.10.0/coverage-baseline/`.
- [ ] 2.3 Script: emit `summary.json` aggregating per-crate `percent_covered`.
- [ ] 2.4 Script: exit non-zero if any crate < 80% (CI-friendly).

## 3. Stub JSON
- [ ] 3.1 Author `docs/releases/v3.10.0/coverage-baseline/sqlrustgo-lib.json` matching the shape `data[0].summary.percent_covered` so RC gate parsing succeeds.
- [ ] 3.2 Verify with `python3 -c "import json; print(json.load(open('docs/releases/v3.10.0/coverage-baseline/sqlrustgo-lib.json'))['data'][0]['summary']['percent_covered'])"`.

## 4. Documentation
- [ ] 4.1 Update `docs/releases/v3.10.0/coverage-baseline/README.md` to document the script + JSON format.
- [ ] 4.2 Mark 14.71% as BASELINE, not final; note 80% is GA target.
- [ ] 4.3 Add "Next Steps" with link to script.

## 5. Commit
- [ ] 5.1 Branch `chore/r6-coverage-baseline`.
- [ ] 5.2 Commit + push.
- [ ] 5.3 PR to `develop/v3.10.0`.

## 6. Close issue
- [ ] 6.1 Comment on #3400 explaining what was delivered and the remaining 80% gap.
- [ ] 6.2 PATCH #3400 state=closed.

## 7. Archive
- [ ] 7.1 `openspec archive issue-3400-r6-coverage-baseline`.
