# SQLRustGo v3.12.0 Security Audit Rollup

> **provenance:** generated_by=codex, generated_at=2026-09-11T00:00:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014
> **stage:** GA
> **ga_tag_commit:** `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4`

## Summary

The GA security claim is bound to GA-3 in the full aggregate gate report. The current publication wording preserves the recorded caveats from [`evidence/v312-59/GA3_SECURITY_SCAN_REPORT.md`](evidence/v312-59/GA3_SECURITY_SCAN_REPORT.md) instead of converting them into a broader clean-room security claim.

## Evidence Matrix

| Check | Publication status | Evidence boundary |
|---|---|---|
| SC-1 cargo audit | PASS-WITH-ACKNOWLEDGED-BASELINE | The recorded report carries acknowledged advisories; no HIGH/CRITICAL advisory is recorded in that baseline. |
| SC-2 license check | PASS-WITH-DRIFT | cargo-deny fallback / license metadata drift remains documentation debt, not a hidden clean claim. |
| SC-3 secret scan | PASS | Regex scan over `crates/`, `tests/`, and `src/` found no production hardcoded credentials in the recorded run. |
| SC-4 plaintext password scan | PASS-WITH-FIXTURE-REVIEW | Matches were reviewed as test fixtures, not production paths. |

## Known Advisories Carried By The Recorded Report

- RUSTSEC-2021-0145: `atty` 0.2.14
- RUSTSEC-2026-0002: `lru` 0.12.5
- RUSTSEC-2026-0253: `lru` 0.12.5

## Release Claim Boundary

Allowed:

- GA-3 is included in the `72/72 PASS, blockers 0` aggregate at commit `355b5a3837`.
- The security report may be described as PASS with recorded caveats.

Not allowed:

- A fresh clean advisory-DB claim beyond the checked-in GA-3 evidence.
- A statement that all license metadata drift is resolved.
- A statement that the project is security-certified for workloads outside the GMP internal-audit retrieval scope.
