# v3.12.0 README and Release Documentation Rectification Report

> **provenance:** generated_by=codex-cli, generated_at=2026-09-08T04:33:08+08:00, source_repo=openclaw/sqlrustgo, branch=codex/v312-docs-readme-refresh, head=`54219264158f86ea23a43edd17e32f7d743b74f5`, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + DOC_CHECK_CORRECTION_RULES

## 1. Scope

This report records a documentation-only rectification pass for:

- `README.md`
- `docs/releases/v3.12.0/README.md`
- `docs/releases/v3.12.0/RELEASE_CHECKLIST.md`
- `docs/releases/v3.12.0/STAGE.yaml`

The work refreshes the current v3.12.0 status after pulling the 252 Gitea
remote to `542192641`. It does not claim GA promotion.

## 2. Findings

| # | File | Problem | Evidence |
|---|---|---|---|
| 1 | `README.md` | The root README was too long for a project entry page and mixed release notes, historical audit detail, feature matrix detail, and contribution guidance. | File length before edit: 347 lines. |
| 2 | `README.md` | Current status metadata was stale and referenced older HEADs such as `d64f0038b9`. | Live fetch on 2026-09-08 fast-forwarded `develop/v3.12.0` to `542192641`. |
| 3 | `docs/releases/v3.12.0/README.md` | The release README retained historical appendices and stale 2026-09-02 milestone/open-issue wording. | Live Gitea API showed open issues #4846, #4847, #4848 and 0 open PRs. |
| 4 | `docs/releases/v3.12.0/RELEASE_CHECKLIST.md` | The open-issue checklist still referenced the older #4607-#4613 batch, which is no longer the live open issue set. | Live Gitea API showed only #4846, #4847, #4848 open at this snapshot. |
| 5 | `docs/releases/v3.12.0/STAGE.yaml` | `last_ga_attempt` was useful but did not record the 2026-09-08 post-BustubX refresh snapshot. | `STAGE.yaml` remained RC; checked-in GA aggregate was `verdict=FAIL` at stale commit `65ef5bea`. |

## 3. Corrections

| # | File | Correction |
|---|---|---|
| 1 | `README.md` | Replaced the verbose entry page with a concise project overview, current evidence snapshot, quick start, release discipline, development checks, and key document index. |
| 2 | `docs/releases/v3.12.0/README.md` | Replaced the historical mixed document with a current RC/GA preparation summary, open issue impact table, RC-GA gate requirements, and final GA checklist. |
| 3 | `docs/releases/v3.12.0/RELEASE_CHECKLIST.md` | Refreshed the release-claim checklist to reference #4846/#4847/#4848. |
| 4 | `docs/releases/v3.12.0/STAGE.yaml` | Added a `last_status_refresh` block for the 2026-09-08 remote snapshot without changing `current_stage`. |

## 4. Evidence Boundary

| Evidence | Status |
|---|---|
| 252 Gitea fetch | Executed; local `develop/v3.12.0` fast-forwarded from `edc5eee8d` to `542192641`. |
| Gitea open issues | Executed via API; open issues were #4846, #4847, #4848. |
| Gitea open PRs | Executed via API; no open PRs returned. |
| GA aggregate | Read from checked-in JSON; `mode=full`, `verdict=FAIL`, `totals.blockers=9`, stale commit `65ef5bea`. |
| GA promotion | Not executed and not claimed. |

## 5. Review Checklist

| Check | Result |
|---|---|
| Root README no longer acts as a release report dump | PASS |
| v3.12.0 README identifies current RC status and open blockers | PASS |
| No new GA/PASS claim without execution evidence | PASS |
| Stale issue list replaced with current live Gitea issue set | PASS |
| Stage remains RC | PASS |

## 6. Remaining Work

- Fix or formally scope #4846, #4847, and #4848.
- Replace skeleton RC-B gates with real oracle-backed checks.
- Re-run `bash scripts/gate/check_ga_v3.12.0.sh --full` at the final release
  commit.
- Refresh docs links, consistency, security, and SOAK evidence after code fixes.

## 7. Conclusion

The documentation now presents v3.12.0 as RC / GA preparation rather than GA.
The root README is shortened into a professional project entry point, while
release-stage detail is kept in `docs/releases/v3.12.0/`.
