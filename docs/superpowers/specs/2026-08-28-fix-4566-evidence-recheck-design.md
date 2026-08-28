# 2026-08-28 — Fix #4566 Evidence Recheck Design

> **status:** draft (post-brainstorming, awaiting user spec review)
> **branch base:** develop/v3.12.0 (remote HEAD `3d209e882b`, 8 commits ahead of my `169099b3fb`)
> **umbrella issues:** #4497 (V312-59-D v2 promotion cycle), #4499 (GA-2 168h mixed SOAK)
> **supersedes:** my local branch `fix/v312-59-d/4560-evidence` (whose premise — "COM_STMT_* not implemented" — was refuted by PR #4563)

## 1. Background & Motivation

### 1.1 What I learned at session resume

At session resume the remote `develop/v3.12.0` had advanced 8 commits past my local HEAD since I last touched anything:

| commit | PR | title |
|---|---|---|
| `3d209e882b` | #4573 | tools: add #![allow(dead_code)] to repro_4564 |
| `0d43284577` | (part of #4573) | tools: add #![allow(dead_code)] to repro_4564 |
| `391069fc2c` | #4566 | fix(soak / #4564): root-cause sysbench 4/8 TLS-handshake stall + repro tool |
| `1c11addc6b` | (part of #4566) | fix(soak / #4564): root-cause sysbench 4/8 TLS-handshake stall + repro tool |
| `3599bd95b3` | #4565 | feat(soak / #4499, docs / #3887): GA-2 168h SOAK Z6G4 runner + FOLLOWUP-INDEX update |
| `0f407cfdff` | (part of #4565) | docs(v312-59 / #3887): FOLLOWUP-INDEX_UPDATE — F-1..F-6 closure status |
| `8455fc30a2` | (part of #4565) | feat(soak / #4499): GA-2 168h SOAK Z6G4 Docker runner + runbook |
| `4d8e8d44e7` | #4563 | fix(v312-59-d / #4560): correct GA-2 row — COM_STMT_* handlers exist |
| `a93ea79681` | (part of #4563) | fix(v312-59-d / #4560): correct GA-2 row — COM_STMT_* handlers exist |

### 1.2 Diagnosis refutation

My earlier `POST_4558_SOAK_REPORT.md` concluded that "server command dispatch doesn't handle COM_STMT_PREPARE / COM_STMT_EXECUTE / COM_STMT_CLOSE / COM_STMT_RESET". **This is demonstrably false:**

| Symbol | File:Line | Status |
|---|---|---|
| `packet_type::COM_STMT_PREPARE => { ... }` | `crates/mysql-server/src/lib.rs:4791` | full handler present |
| `packet_type::COM_STMT_EXECUTE => { ... }` | `crates/mysql-server/src/lib.rs:4980` | binary payload parse + execute |
| `packet_type::COM_STMT_CLOSE => { ... }` | `crates/mysql-server/src/lib.rs:5101` | present |
| `packet_type::COM_RESET_CONNECTION => { ... }` | `crates/mysql-server/src/lib.rs:5110` | present (note: there is no COM_STMT_RESET command; 0x1F is COM_RESET_CONNECTION) |

Per Anti-Fabrication-Policy-v1.0 a GA sign-off document must not embed false technical claims. PR #4563 (`a93ea79681`) corrected the `GA_GATE_REPORT.md` GA-2 row; my evidence report was not corrected.

### 1.3 Actual root cause (per PR #4566)

The 4/8 sysbench TLS-handshake stall is **server-threads pool starvation under TLS burst**, not a missing handler:

- `server-threads=4` (script default): 8 workers → 5 TCP accept → 4 fully authenticated → 4 completed. Matches the "8× Starting command loop, seq=4+" pattern in my server.log.
- `server-threads=16` (binary default): 8 workers → 8/8 succeed.

`ServerThreadPool::start(n)` uses `sync_channel(n*4)`. With n=4 the buffer=16; the combination of `listener sleep(50ms)` on WouldBlock + `rustls ServerConnection::complete_io + auth work (~30ms/connection)` + 8-way concurrent TLS-handshake burst from sysbench barrier results in net throughput degradation: 4 workers get stuck, 4 are rejected or see broken-pipe.

PR #4566 fix: bump `SOAK_SERVER_THR` default from 4 to 16 in `scripts/soak/run_soak_loop.sh`. The binary's default is already 16.

### 1.4 Issue #4560 status

Issue #4560 was filed by me based on the incorrect diagnosis. PR #4563 closed it implicitly by correcting the GA-2 row. As of session resume, Gitea confirms state=closed with title rewritten to:

> "sysbench oltp_read_write --threads=8 run: 4/8 workers fail to complete TCP+TLS+auth within 30s (client-side TLS-handshake concurrency; NOT a missing server handler)"

My local branch `fix/v312-59-d/4560-evidence` therefore has no live downstream consumer. It can be archived.

## 2. Goals & Non-goals

### 2.1 In scope (this design)

1. **Sync 8 new commits** into local checkout of `develop/v3.12.0`. Method: `git rebase origin/develop/v3.12.0` (or fast-forward if clean).
2. **Update `POST_4558_SOAK_REPORT.md`** with an ERRATA section acknowledging the diagnosis error + citing PR #4563 + PR #4566. Replace the "NEW issue #4560 filed" section with "NEW issue #4560 — LATER CORRECTED". Update verdict to reflect that the bug was the SOAK script's `SOAK_SERVER_THR=4` default, not server command dispatch.
3. **Re-run 1h SOAK** on `develop/v3.12.0` HEAD with `SOAK_SERVER_THR=16` to verify PR #4566 actually unblocks sysbench `oltp_read_write run`. Expected: ≥6 metrics.csv samples, sysbench `run` completes without `Worker threads failed to initialize`, server log shows real command dispatch not just 8× handshake.
4. **Write `POST_4566_SOAK_REPORT.md`** summarizing the new run + GA-2 re-evaluation.
5. **Open 1 PR** "fix(v312-59-d / #4564): verify #4566 TLS-handshake fix + re-evaluate GA-2" containing all of the above.

### 2.2 Out of scope (this design)

- The 6 new mysql-compat GA-blocking issues: **#4567 (CREATE VIEW), #4568 (IN/NOT IN subquery), #4569 (UNIQUE), #4570 (FK), #4571 (ALTER ADD COLUMN), #4572 (1+'1'→1)**. These will be triaged separately after GA-2 evidence closes.
- 168h full SOAK on Z6G4 Docker (PR #4565 infrastructure). That is #4499 umbrella work.
- The deeper fix recommended by PR #4566 (CHANNEL_BUFFER_MULTIPLIER 4→8 in `crates/mysql-server/src/testing.rs ServerThreadPool`, or reducing `listener sleep(50ms)` under TLS bursts). Out of scope per PR #4566 itself.
- Any changes to `GA_GATE_REPORT.md` — already corrected by PR #4563.

## 3. Data Flow

```
┌─ git fetch origin ──────────────────────────┐
│                                              │
│   develop/v3.12.0 (remote)                  │
│   3d209e882b  ← 8 new commits                │
│                                              │
│   rebase / fast-forward                      │
│         ↓                                    │
│   develop/v3.12.0 (local @ 3d209e882b)        │
└──────────────────────────────────────────────┘
                  ↓
┌─ Edit POST_4558_SOAK_REPORT.md ─────────────┐
│   + add ERRATA section                       │
│   + rewrite Section 5 (LATER CORRECTED)      │
│   + update verdict                           │
└──────────────────────────────────────────────┘
                  ↓
┌─ Re-run 1h SOAK (SOAK_SERVER_THR=16) ───────┐
│   SOAK_HOURS=1 SOAK_PORT=3404                │
│   SOAK_SERVER_THR=16 SOAK_SB_THR=8           │
│   SOAK_TABLE_SIZE=10000                      │
│   bash scripts/soak/run_soak_loop.sh         │
│         ↓                                    │
│   /home/openclaw/sqlrustgo-soak-results/     │
│     soak_20260828_post4566/<run_id>/         │
│     {server.log, sysbench.log, metrics.csv,  │
│      monitor.log, periodic_reports.log}      │
└──────────────────────────────────────────────┘
                  ↓
┌─ Copy + report ──────────────────────────────┐
│   cp -r .../soak_20260828_post4566/<run_id>/ │
│     docs/releases/v3.12.0/evidence/issue-4560│
│     /run_20260828_post4566/                  │
│                                              │
│   write docs/releases/v3.12.0/evidence/      │
│     issue-4560/POST_4566_SOAK_REPORT.md      │
└──────────────────────────────────────────────┘
                  ↓
┌─ Push + open PR ─────────────────────────────┐
│   branch: fix/v312-59-d/4566-evidence-       │
│           recheck                            │
│   title: fix(v312-59-d / #4564): verify      │
│          #4566 TLS-handshake fix +           │
│          re-evaluate GA-2                    │
│   body cites PR #4563 + PR #4566 + this spec │
└──────────────────────────────────────────────┘
```

## 4. Risk & Mitigation

| # | Risk | Probability | Mitigation |
|---|---|---|---|
| 1 | 1h SOAK still hangs despite `SOAK_SERVER_THR=16` → PR #4566 fix insufficient | low (PR author verified 8/8 on n=16) | stop run at +5min, open new issue (e.g. #4574) with repro_4564 + new server.log, do NOT claim PASS |
| 2 | Rebase conflict in `POST_4558_SOAK_REPORT.md` (unlikely — file is new, only I have touched it) | very low | manual resolve, no auto-merge; rerere enabled |
| 3 | `git rebase` fails because someone updated the same file | very low | fall back to `git merge origin/develop/v3.12.0` (no-ff) |
| 4 | SOAK_SERVER_THR=16 triggers new server-side bug (panic, OOM, leaked FD) | very low | monitor `monitor.log` + RSS samples; abort + open new issue |
| 5 | My evidence report contradicts PR #4563 / #4566 wording → reviewer rejects | low | cross-reference exactly: cite `commit a93ea79681` (PR #4563) and `commit 1c11addc6b` (PR #4566) by SHA |
| 6 | PR auto-blocked by branch-protection because base diverged | low | push fresh branch off `origin/develop/v3.12.0` not off my stale HEAD |

## 5. Acceptance Criteria

| ID | Criterion | Verification |
|---|---|---|
| AC-1 | Local `develop/v3.12.0` HEAD = `3d209e882b` (or newer if more commits land mid-run) | `git rev-parse develop/v3.12.0` |
| AC-2 | `POST_4558_SOAK_REPORT.md` has new `## 10. ERRATA (added 2026-08-28)` section | grep section header |
| AC-3 | `POST_4558_SOAK_REPORT.md` section 5 title contains "LATER CORRECTED" | grep |
| AC-4 | `POST_4558_SOAK_REPORT.md` verdict reflects diagnosis error + PR #4563 + PR #4566 | manual review |
| AC-5 | New 1h SOAK run completes ≥ 55 min before stop | `metrics.csv` ≥ 6 rows |
| AC-6 | `sysbench.log` does NOT contain "Worker threads failed to initialize" | grep |
| AC-7 | `server.log` shows real command dispatch (≥ 1 "Query [...]" line after the 8× handshake) | grep `Query [` count > 8 |
| AC-8 | `POST_4566_SOAK_REPORT.md` written with 5 evidence fields per ADR-014 | manual review |
| AC-9 | PR opened, body cites PR #4563 (commit `a93ea79681`) + PR #4566 (commit `1c11addc6b`) | manual review |
| AC-10 | NO PASS verdict for GA-2 — this PR only documents 1h SOAK; 168h CI remains #4499 umbrella work | grep "PASS" in new POST_4566_SOAK_REPORT.md — must be absent except in scope qualifications |

## 6. Testing & Verification Plan

1. **Unit / regression**: not applicable — no code changes, only doc + SOAK rerun.
2. **SOAK evidence**: AC-5..AC-7 above.
3. **Anti-fabrication check**: PR body must reference upstream source SHAs (PR #4563 `a93ea79681`, PR #4566 `1c11addc6b`); no claims about server command dispatch unhandled commands.
4. **Branch hygiene**: PR must be opened from a fresh branch off `origin/develop/v3.12.0` HEAD, not from my stale `fix/v312-59-d/4560-evidence` HEAD.

## 7. Open Questions

None blocking. Non-blocking:

- Q1: should I also update the F-1..F-6 entries in `evidence/FOLLOWUP-INDEX.md` (per PR #4565's UPDATE-2026-08-28 doc)? **Out of scope** per design §2.2; defer to a separate task.
- Q2: should I file a follow-up issue for the deeper ServerThreadPool fix recommended by PR #4566 (CHANNEL_BUFFER_MULTIPLIER 4→8)? **Yes, recommended** but optional; could be one-line issue like #4573+.

## 8. Provenance (ADR-014)

- **source_agent**: claude-sonnet (Claude Code)
- **source_run**: session resume continuation 2026-08-28; new design session after PR #4563 + #4566 merged
- **timestamp**: 2026-08-28T(approx 17:00Z, post-fetch)
- **evidence_hash**: remote `gitnexus://repo/sqlrustgo/context` index should be checked; if stale run `npx gitnexus analyze` per CLAUDE.md
- **conflict_resolution**: N/A — no prior design competing