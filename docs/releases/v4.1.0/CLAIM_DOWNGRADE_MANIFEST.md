# SQLRustGo v4.1.0 — Scope/Claim Downgrade Manifest

> **provenance**: generated_at=2026-10-14, branch=develop/v4.1.0,
> HEAD=`a1cee6732e`, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
>
> **purpose**: Explicit release-claim downgrade for every GA-claim-caveat
> / GA-blocker issue that remains open at v4.1.0 promotion time. Per
> `docs/governance/GATE_CONDITIONS.md` §2 "GA-claim-caveat" criteria:
> "May remain open only if release notes, README, and scope docs
> explicitly exclude the capability from v4.1.0 claims."

## 1. Methodology

For each item below:

1. State the open status in Gitea.
2. State the affected capability.
3. Spell out the exact release-claim boundary line that must appear in
   `README.md`, `RELEASE_NOTES.md`, `GA_GATE_REPORT.md`, and the relevant
   subsystem docs (`docs/releases/v4.1.0/`).
4. Reference the per-issue evidence section.

## 2. Inherited GA-claim-caveat（from v3.12.0 / v4.0.0）

v4.1.0 inherits all 3 v3.12.0 GA-claim-caveat items, as carried forward
unchanged via
[`docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md`](../v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md):

| Issue | Subsystem | Exclusion |
|-------|-----------|-----------|
| #4846 | Executor / type semantics | `CHAR(n)` byte-padding 主键点查排除 |
| #4847 | Transaction semantics | 显式 `BEGIN`/`COMMIT`/`ROLLBACK` 排除 |
| #4848 | Storage / DDL | `ALTER TABLE ... RENAME COLUMN` 排除 |

The full claim boundary lines for these 3 inherited items live in v4.0.0's
manifest §3.5, §3.6-equivalent, §3.4 respectively. v4.1.0 inherits verbatim
and does **not** rewrite them here.

## 3. v4.1.0-specific downgrade boundaries

### 3.1 BufferPool / Page / Disk B+Tree test-only capability（#5304）

**Issue**: #5304 — `[A05 defect] Buffer Pool / page / disk B+Tree not in
production path (advertised capability is dead code)` — finding
A05-V410-002.

**Affected capability**: `sqlrustgo_storage::BufferPool`,
`sqlrustgo_storage::Page`, and the disk B+Tree in
`crates/storage/src/bplus_tree/index.rs`. Re-exported through
`src/lib.rs` but with zero instantiation in the production path
(`src/`, `crates/mysql-server/src/`, `crates/server/src/`).

**Claim boundary line** (must appear verbatim in README.md,
RELEASE_NOTES.md, GA_GATE_REPORT.md):

> "v4.1.0 does **NOT** ship the advertised `BufferPool` / `Page` /
> `disk B+Tree` capability. The `sqlrustgo_storage::BufferPool` type,
> `sqlrustgo_storage::Page` type, and disk B+Tree in
> `crates/storage/src/bplus_tree/index.rs` are `pub use`-exported from
> `src/lib.rs` for **storage-test code only** (`/// ⚠️ Test-only
> capability. Not used by production FileStorage.` doc-comment on the
> re-export). Production `FileStorage` is an in-mem `HashMap` + JSON
> `insert_buffer` (NON-paged). The capability is excluded from v4.1.0
> release claims; full production wiring is a future workstream and
> tracked under #5304."

**Verification**:

```bash
grep -rn "BufferPool" src/ crates/mysql-server/src/ crates/server/src/
# Only matches src/lib.rs (re-export lines). Zero instantiation.

grep -rn "bplus_tree" crates/storage/src/bplus_tree/index.rs src/ \
                          crates/mysql-server/src/ crates/server/src/
# Disk B+Tree reachable only from itself and from columnar memory-arena
# (src/memory/*). Not in production FileStorage.
```

**Downgrade disposition**: re-export documented as test-only (this PR);
production wiring is a separate future workstream and will, when
landed, supersede this §3.1 entry.

**Related exclusions**:

- `docs/releases/v4.1.0/b2-off-production-path-registry.md` —
  orthogonal "binary on/off production-path" exclusion (relates to
  `crates/planner` reachability under #5193, not to BufferPool per
  se). Cross-references this manifest for the storage-side
  exclusion.
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` — full P0×6 + P1×7 +
  P2×7 catalog (P0 issues are blocking; #5304 is the canonical
  storage-side honesty defect).

## 4. Verification checklist (for ALPHA / BETA / GA promotion)

Before any `alphaN/v4.1.0`, `betaN/v4.1.0`, `rcN/v4.1.0`, or `ga/v4.1.0`
tag:

- [ ] §3.1 claim boundary line appears in `README.md`
- [ ] §3.1 claim boundary line appears in `docs/releases/v4.1.0/RELEASE_NOTES.md`
- [ ] §3.1 claim boundary line appears in `docs/releases/v4.1.0/STAGE.yaml`
      (under the appropriate section)
- [ ] `re-exported as test-only` is enforced by doc-comment on
      `src/lib.rs` (verify with `grep -n "Test-only capability" src/lib.rs`)
- [ ] All 3 inherited items (§2) still mirror v4.0.0 verbatim

## 5. References

- v4.0.0 manifest (3 inherited GA-claim-caveat items live there):
  [`docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md`](../v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md)
- v3.12.0 manifest (origin of the 3 inherited caveats):
  [`docs/releases/v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md`](../v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md)
- Issue #5304 — buffer pool / page / disk B+Tree dead-capability (this PR)
- Finding A05-V410-002 — IAIA-410 audit, work package #5227 (A05)
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` — full defect catalog
- `docs/releases/v4.1.0/b2-off-production-path-registry.md` — orthogonal
  binary-level off-production-path exclusion
- `docs/governance/GATE_CONDITIONS.md` §2 — GA-claim-caveat definition
- `docs/governance/ANTI_FABRICATION_POLICY.md` — claim discipline
