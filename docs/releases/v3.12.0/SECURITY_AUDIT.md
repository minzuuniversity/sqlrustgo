# SQLRustGo v3.12.0 Security Audit Rollup

> **provenance:** generated_by=codex-cli, generated_at=2026-09-02T21:20:00+08:00, source_repo=openclaw/sqlrustgo, remote_head=`b14ad8df0306891e217af19648b210e81d5d2be0`, policy=Anti-Fabrication-Policy-v1.0
> **stage:** RC / GA candidate preparation

## Summary

The latest available GA-3 security evidence is recorded in
`evidence/v312-59/GA3_SECURITY_SCAN_REPORT.md`. It is acceptable as a prior
GA-candidate input, but it must be refreshed at the final GA cut because one
sub-check used a baseline carry-forward path rather than a fresh advisory DB
scan.

## Evidence Matrix

| Check | Prior status | Evidence boundary |
|---|---|---|
| SC-1 cargo audit | PASS-WITH-CAVEAT | Advisory DB fetch failed in the recorded run; verdict used acknowledged baseline advisories. |
| SC-2 license check | PASS-WITH-DRIFT | cargo-deny unavailable; cargo metadata fallback found 44/50 manifests missing license fields. |
| SC-3 secret scan | PASS | Fresh regex scan over `crates/`, `tests/`, and `src/` found 0 hardcoded credentials. |
| SC-4 plaintext password scan | PASS | 2 matches manually reviewed as test fixtures, not production paths. |

## Known Advisories

The recorded security report carries forward acknowledged unsound advisories:

- RUSTSEC-2021-0145: `atty` 0.2.14
- RUSTSEC-2026-0002: `lru` 0.12.5
- RUSTSEC-2026-0253: `lru` 0.12.5

No HIGH/CRITICAL advisory is recorded in the baseline. This rollup does not
claim a fresh 2026-09-02 cargo-audit result.

## GA Cut Requirements

Before v3.12.0 can be promoted to GA:

1. Re-run `scripts/gate/check_security_scan_v312.sh` at the final GA HEAD.
2. Prefer a live `cargo audit` run with an available advisory DB.
3. If cargo-deny remains unavailable, explicitly record fallback mode and
   license drift.
4. Confirm test-fixture password matches are still non-production paths.
5. Update this file or replace it with final GA security evidence.

## Release Claim Boundary

Allowed now:

- Security gate has prior GA-candidate evidence with caveats.
- Secret and plaintext-password scans had fresh PASS evidence in the prior run.

Not allowed yet:

- Final GA security scan PASS at `b14ad8df03` or later.
- Fresh advisory DB clean result on 2026-09-02.
