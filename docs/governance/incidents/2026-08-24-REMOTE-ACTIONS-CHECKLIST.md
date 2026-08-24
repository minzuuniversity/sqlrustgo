# Governance Remediation — Remote Actions Checklist (Gitea)

> **Companion to `2026-08-24-V313-MILESTONE-PREMATURE.md`.**
> Run these Gitea API actions when the server is reachable again.

---

## Status

| Item | Status |
|------|--------|
| Local commit `docs(governance): record ...` | ✅ done (commit `929cc547d` on branch `fix/v312-58-governance-remediation`) |
| Gitea server reachable | ❌ down (192.168.0.252 unreachable, >10min) |
| Remote actions below | ⏳ pending server recovery |

---

## Remote Actions to Execute When Gitea is Reachable

### 1. Close milestone #39 (v3.13)

```bash
curl -X PATCH "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/milestones/39" \
  -H "Authorization: token $GITEA_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"state":"closed","state_reason":"not_completed"}'
```

### 2. Retitle issues #4441-#4444 with `[v3.12.0-RC]` prefix

```bash
for n in 4441 4442 4443 4444; do
  curl -X PATCH "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/$n" \
    -H "Authorization: token $GITEA_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"title\":\"<see mapping below>\",\"milestone\":0}"
done
```

**Title mapping**:

| Issue | New Title |
|-------|-----------|
| #4441 | `[v3.12.0-RC] TPC-H Q17/Q20 subquery materialization — master tracking (migrated from premature v3.13 milestone)` |
| #4442 | `[v3.12.0-RC] (was v3.13 Phase 1) Extend find_correlated_subqueries for scalar-aggregate-in-WHERE` |
| #4443 | `[v3.12.0-RC] (was v3.13 Phase 2) Materialization driver for scalar subqueries` |
| #4444 | `[v3.12.0-RC] (was v3.13 Phase 3) HashSemiJoin instantiation for Q20 nested EXISTS + scalar` |

`milestone: 0` detaches from milestone #39.

### 3. Append governance-remediation comment to each issue

Comment body (apply to all 4 issues):

```text
---

## ⚠️ Governance Remediation (2026-08-24)

**This issue was originally created under the v3.13 milestone (#39) on 2026-08-24,
which violated docs/governance/RELEASE_LIFECYCLE.md (a new version's planning
must wait until the previous version is GA).**

**State at time of migration**:
- v3.11.0: ✅ GA (released 2026-08-09, commit 83c623835)
- v3.12.0: 🚧 develop branch (not yet GA) — VERSION file still `v3.11.0`
- v3.13 milestone: ⛔ created prematurely (closed as part of this remediation)

**Action taken**:
- Milestone #39 (v3.13) closed
- All 4 sub-issues retitled with `[v3.12.0-RC]` prefix
- Engineering work remains valid and continues under v3.12.0 RC scope
- Master tracking (#4441) remains open until Q17/Q20 close under v3.12.0

**Future v3.13 work**: After v3.12.0 reaches GA, the master scope can be
re-promoted to a new v3.13 milestone without re-creating these issues.
```

```bash
for n in 4441 4442 4443 4444; do
  curl -X POST "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/$n/comments" \
    -H "Authorization: token $GITEA_TOKEN" \
    -H "Content-Type: application/json" \
    -d @<(jq -Rs '{body: .}' comment.md)
done
```

### 4. (Optional) Push local commit to origin

```bash
git push -u origin fix/v312-58-governance-remediation
```

Then file a PR titled `[governance] document v3.13 milestone premature-creation incident + update CURRENT_VERSION`.

---

## Python helper (idempotent)

`/tmp/governance_remediation.py` already exists and implements steps 1-3 with retry. Run:

```bash
python3 /tmp/governance_remediation.py
```

## Provenance

- documented_at: 2026-08-24T16:00:00Z (approx, when Gitea became unreachable)
- gitea_status: DOWN at time of writing (192.168.0.252 unreachable, ~10min)
- policy: docs/governance/RELEASE_LIFECYCLE.md §2.4