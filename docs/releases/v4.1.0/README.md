# SQLRustGo v4.1.0

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Base version**: v4.0.0 (post-GA maintenance continuation)

## 1. What is v4.1.0?

v4.1.0 is the **post-v4.0.0 maintenance continuation** of SQLRustGo. It
is the active development trunk for v4.x. v4.0.0 remains GA on its
release branch.

v4.1.0 carries v4.0.0 forward through:

1. **Real bugfix carry-forward** — V400-05/06/07 cross-model transaction +
   AuditChain ALCOA+, zombie-fix core, workers.push wrapper restore,
   DML/storage regression fixes
2. **5-remote sync tooling** — `scripts/sync/5remotes_sync.sh`,
   `scripts/sync/5remotes_drift_check.sh`, `scripts/sync/README.md`
3. **(Pending)** WP-C..G deferred items migration, V400-09 168h SOAK
   FINAL, 3 inherited v4.0.0 alpha gate FAILs resolution

## 2. Inherited scope

v4.1.0 inherits ALL scope from v4.0.0. Read
[`docs/releases/v4.0.0/README.md`](../v4.0.0/README.md) for the full
product contract. The 3 GA-claim-caveat items from v4.0.0's
CLAIM_DOWNGRADE_MANIFEST.md (CHAR padding, transaction semantics
divergence, ALTER TABLE RENAME COLUMN) carry forward unchanged.

## 3. v4.1.0-specific notes

### 3.1 Branch

`develop/v4.1.0` — active dev trunk. Push-protected per
`docs/governance/BRANCH_PROTECTION_v4.0.0.md` template.

### 3.2 5-remote sync

All commits must be synced to 5 remotes:
- gitcode (gitcode.com/BreavHeart/sqlrustgo)
- gitea250 (192.168.0.250:3000/openclaw/sqlrustgo) — protected branch
- gitea252 (192.168.0.252:3000/openclaw/sqlrustgo) — protected branch
- gitee (gitee.com/yinglichina/sqlrustgo)
- github (github.com/yinglichina8848/sqlrustgo-public)

Tooling: `bash scripts/sync/5remotes_sync.sh develop/v4.1.0`

### 3.3 Stage progression

```
DRAFT (current)
   ↓ fix 3 inherited alpha gate FAILs + V400-09 168h SOAK FINAL + WP-C..G migration
ALPHA (v4.1.0-alpha1 tag cut)
   ↓ all B1-B5 + D6/D7 integration
BETA (v4.1.0-beta1 tag cut)
   ↓ all RC{N} release-blocking bugs closed
RC (v4.1.0-rc1 tag cut)
   ↓ 168h multi-model SOAK PASS
GA (v4.1.0-ga tag cut, release/v4.1.0 created)
```

See `docs/releases/v4.1.0/STAGE.yaml` for full exit criteria.

## 4. Quick links

- v4.1.0 stage state: `docs/releases/v4.1.0/STAGE.yaml`
- v4.1.0 deltas: `docs/releases/v4.1.0/VERSION_PLAN.md`
- v4.1.0 dev workflow: `docs/releases/v4.1.0/DEV_PLAN.md`
- v4.1.0 roadmap: `docs/releases/v4.1.0/ROADMAP.md`
- v4.1.0 test plan: `docs/releases/v4.1.0/TEST_PLAN.md`
- v4.1.0 issues: `docs/releases/v4.1.0/ISSUES_PLAN.md`
- v4.1.0 legacy: `docs/releases/v4.1.0/LEGACY_ISSUES.md`
- v4.1.0 changelog: `docs/releases/v4.1.0/CHANGELOG.md`
- 5-remote sync: `scripts/sync/README.md`
- v4.0.0 README (inherited): `docs/releases/v4.0.0/README.md`