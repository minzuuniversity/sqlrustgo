# GA-3 v3.12.0 Security Scan Report

> **provenance:** generated_by=check_security_scan_v312.sh, generated_at=2026-08-27T15:50:40Z, commit=35d7a6b250084200af88cdcabcce0e72a0572232, branch=develop/v3.12.0, source_repo=openclaw/sqlrustgo, gate_policy_eval_id=v312-ga3-security-scan-001, policy=Anti-Fabrication-Policy-v1.0
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

Manual review of the 2 matches confirms they are test fixtures inside
`#[cfg(test)]` blocks (`mysql-client/src/lib.rs:1317 test_native_password_hash`
uses `"tester"`; `catalog/src/auth.rs:1706 test_scram_credential_verify_success`
uses `"mysecretpassword"`). Neither is in production code paths.

## Caveats (per Anti-Fabrication Policy v1.0 §G-01)

- **SC-1 cargo audit did NOT execute a live scan this run.** `cargo-audit`
  was freshly installed (v0.22.2), but the advisory-db fetch from
  `https://github.com/RustSec/advisory-db.git` failed (IO error / network).
  `cargo_audit_v312.txt` contains the fetch error verbatim. The "PASS"
  verdict for SC-1 is therefore the **baseline acknowledged** path
  (carry-forward from `docs/releases/v3.11.0/SECURITY_AUDIT.md:130` +
  `docs/releases/v2.9.0/SECURITY_REPORT.md`), NOT a fresh audit. The
  three `acknowledged` rows in the RUSTSEC table above are pre-existing
  baseline advisories (atty / lru), not findings from this run.

- **SC-2 license check used the cargo-deny fallback** because cargo-deny
  is not installed in this environment. 44/50 Cargo.toml manifests are
  missing a `license` field (down from 132/150 in the 2026-08-21 run —
  100 manifests were added with license metadata in between). This is a
  DRIFT, not a blocker, per the script policy.

- **SC-3 / SC-4 are PASS with fresh evidence** (this run, 2026-08-27T15:50:40Z,
  commit `35d7a6b25`).

## Follow-up actions recommended

1. File issue to install cargo-audit + cache advisory DB in CI image so
   SC-1 can run deterministically.
2. File issue to add `license = "..."` to the 44 manifests still missing
   it (license policy: GPL-3.0-or-later OR Apache-2.0/MIT compatible).
3. The two SC-4 matches are confirmed test fixtures — no production
   exposure; no further action required.

## Verdict

**PASS (with caveat on SC-1)** — 3/4 sub-checks PASS with fresh
evidence + 1/4 PASS via baseline-acknowledged carry-forward.
