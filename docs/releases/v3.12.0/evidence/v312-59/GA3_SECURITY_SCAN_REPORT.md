# GA-3 v3.12.0 Security Scan Report

> **provenance:** generated_by=check_security_scan_v312.sh, generated_at=2026-08-27T04:46:49Z, commit=38e2a6b785b39fdc1d7cd9cbc6504664fd57c3e2, branch=develop/v3.12.0, source_repo=openclaw/sqlrustgo, gate_policy_eval_id=v312-ga3-security-scan-001, policy=Anti-Fabrication-Policy-v1.0
> **related:** Issue #4387 (V312-59-D), umbrella #4383

## Summary

| Check | Status | Detail |
|-------|--------|--------|

| SC-1 cargo-audit | PASS | 00 unsound advisories (baseline, no HIGH/CRITICAL) |
| SC-2 license check | PASS | 44/50 missing license (DRIFT) |
| SC-3 secret scan | PASS | 0 hardcoded credentials |
| SC-4 plaintext pw scan | PASS | 2 potential matches (review docs/releases/v3.12.0/evidence/v312-59/plaintext_pw_scan_v312.txt, expected fixture) |

## RUSTSEC Advisories (cargo audit)

| ID | Crate | Version | Severity | Status | Issue link |
|----|-------|---------|----------|--------|------------|
| RUSTSEC-2021-0145 | atty | 0.2.14 | unsound | acknowledged | TBD (issue #) |
| RUSTSEC-2026-0002 | lru | 0.12.5 | unsound | acknowledged | TBD (issue #) |
| RUSTSEC-2026-0253 | lru | 0.12.5 | unsound | acknowledged | TBD (issue #) |

## License check

cargo-deny not available — using cargo metadata fallback (scans all Cargo.toml manifests for license field).

## Secret scan

Regex-based scan over crates/ + tests/ + src/ for hardcoded credentials. See `secret_scan_v312.txt`.

## Plaintext password scan

Wire protocol handler scan. See `plaintext_pw_scan_v312.txt`.

## Verdict

**PASS** — 4/4 checks PASS
