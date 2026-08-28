# CI/Z6G4 Handoff Runbook — Issue #4540

> **Status**: HANDOFF READY (as of 2026-08-27)
> **Purpose**: Document the exact sequence to capture the TPC-H Q17
> SF=1 cell-diff on the production-aligned CI runner (Z6G4), where the
> SF=1 fixture generation is ~10× faster than on a developer laptop
> (NVMe-backed disk).
> **Refs**: Issue #4540 (this change), Issue #4432 (Q17 perf acceptance),
> Issue #4502 (GA-5 oracle match), Issue #4497 (V312-59-D umbrella),
> `openspec/changes/issue-4540-tpch-q17-sf1-cell-diff/`.

## Why a handoff

The dev-machine path in
`openspec/changes/issue-4540-tpch-q17-sf1-cell-diff/design.md` §1 has
the following wall-clock budget on a typical developer laptop:

| Stage | Time on dev laptop | Time on Z6G4 (NVMe) |
|-------|--------------------|---------------------|
| SF=1 fixture gen (in-process tpch_data_gen) | 20-40 min | 2-4 min |
| Q17 query (per design) | ≤ 300 s | ≤ 300 s |
| Cache warmup (Q1 first) | ~10 s | ~10 s |
| **Total** | **~25-45 min** | **~5-10 min** |

Per design.md §2, this change **explicitly prefers** the Z6G4 path
because:

- Production-aligned container (same kernel, same disk layout as GA
  smoke run).
- ~10× faster fixture generation (NVMe-backed disk vs Apple SSD).
- Z6G4 has 168h SOAK reserved slot — same machine can back-to-back
  Q17 + 168h SOAK without disk-state churn.

This handoff captures the exact sequence so a maintainer or follow-up
automated agent can reproduce the cell-diff capture in one sitting on
Z6G4.

## Prerequisites

| Item | Value |
|------|-------|
| Host | `z6g4-runner` (project CI runner) |
| SSH access | `ssh z6g4-runner` (project VPN required) |
| Gitea reachable | `http://192.168.0.252:3000` |
| Branch | `develop/v3.12.0` |
| Disk | ≥ 2 GB free in `/tmp` (SF=1 fixture is ~1.2 GB) |
| Container | `devstack-gitea-1` (already provisioned) |

## Step-by-step sequence

### Step 0: Confirm branch is current

```bash
ssh z6g4-runner
cd /work/openclaw/sqlrustgo
git fetch origin
git checkout develop/v3.12.0
git status            # confirm: clean, no uncommitted changes
git log --oneline -1  # capture evidence_hash
```

Record the commit SHA — this is `evidence_hash` in the cell-diff
artifact.

### Step 1: Generate SF=1 fixture

```bash
# Recommended: in-process tpch_data_gen (default backend, no external CLI)
bash scripts/generate_tpch_data.sh --sf 1 \
    --output /tmp/tpch-sf1 \
    --backend tpch_data_gen
```

**Wall-clock budget on Z6G4: 2-4 minutes** (NVMe). On dev: 20-40 min.

**If the script fails** with `error: no example target named tpch_data_gen
in default-run packages`, the `crates/bench` package name needs to be
explicit:

```bash
# Workaround: invoke the example binary directly
cargo build --release -p sqlrustgo-bench --example tpch_data_gen
./target/release/examples/tpch_data_gen --scale 1 --output /tmp/tpch-sf1
```

This is a known issue surfaced by the dev-machine path (script
assumes default package, but `tpch_data_gen` lives in
`sqlrustgo-bench`). The CI workflow stub at
`.gitea/workflows/z6g4-tpch-sf1-cell-diff.yml` does not yet encode the
workaround — if you hit it, fix the script first then re-run.

### Step 2: Verify row counts

```bash
bash scripts/generate_tpch_data.sh --sf 1 \
    --output /tmp/tpch-sf1 \
    --check
```

Expected row counts (per `openspec/changes/issue-4540-tpch-q17-sf1-cell-diff/specs/tpch-sf1-baseline/spec.md`):

| Table | Rows |
|-------|------|
| region | 5 |
| nation | 25 |
| supplier | 10000 |
| customer | 150000 |
| part | 200000 |
| partsupp | 800000 |
| orders | 1500000 |
| lineitem | 6001215 |

If any count differs, the fixture is corrupt and the baseline run
must be aborted.

### Step 3: Warm file_storage cache (cold-cache penalty)

```bash
# Run Q1 first to warm file_storage cache; cold-cache Q17 distorts
# the elapsed metric (warmup penalty).
# Implementation note: this requires the sqlrustgo-mysql-server to
# be runnable in-process against /tmp/tpch-sf1. The exact command is
# tracked as a follow-up; for now, a manual sanity-check query on
# /tmp/tpch-sf1 is sufficient.
```

### Step 4: Run TPC-H SF=1 baseline

```bash
bash scripts/tpch_sf1_baseline.sh --sf1-dir /tmp/tpch-sf1
```

The script:

- Validates fixture row counts.
- Loads 22 queries from `queries/` directory.
- Records elapsed time per query in
  `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`.
- Captures Q17 row_count + sha256.

### Step 5: Capture cell-diff

```bash
# Read Q17 elapsed + row_count + sha256 from the baseline report
cat docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md
```

Then populate the evidence file:

```bash
mkdir -p docs/releases/v3.12.0/evidence/v312-58
# Use openspec/changes/issue-4540-tpch-q17-sf1-cell-diff/evidence/template.md
# as a starting point. Replace TBD fields with real values from the report.
```

### Step 6: Post to issue #4432

```bash
# Use Gitea API (HTTP 3000, NOT GitHub):
curl -X POST "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/4432/comments" \
  -H "Authorization: token <token>" \
  -H "Content-Type: application/json" \
  -d @evidence_comment.json
```

The comment body SHALL carry:

- `q17_elapsed_seconds` (from baseline report)
- `q17_row_count` (must equal 1 per TPC-H spec)
- `q17_sha256` (must match oracle value)
- `evidence_hash` (commit SHA from Step 0)
- ADR-014 5 evidence fields
- Link to PR that closes #4540

### Step 7: Decision branch

| Q17 elapsed | Action |
|-------------|--------|
| `≤ 300 s`   | Verdict: PASS. Close issue #4540 + #4432 via Gitea API. Update `docs/releases/v3.12.0/STAGE.yaml` `promotion_to_GA_requires` GA-5 bullet. |
| `> 300 s`   | Verdict: DEFERRED-to-v3.13. Update `docs/releases/v3.13.0/SCOPE_TABLE_v3.13.md`. Keep issue #4540 OPEN for v3.13 re-verification. |

## CI workflow trigger (preferred path)

After the dev-machine script bug is fixed, the DRAFT workflow at
`.gitea/workflows/z6g4-tpch-sf1-cell-diff.yml` can be enabled for
manual dispatch:

1. Open `http://192.168.0.252:3000/openclaw/sqlrustgo/actions`.
2. Select "Z6G4 TPC-H SF=1 cell-diff (DRAFT)".
3. Click "Run workflow".
4. Set `sf1_dir` (default `/tmp/tpch-sf1`) and `backend` (default
   `tpch_data_gen`).
5. Monitor the run in the Actions tab.

## Open follow-up issues

The two script bugs surfaced by the dev-machine attempt
(`docs/releases/v3.12.0/evidence/v312-58/issue-4540-sf1-cell-diff.md`)
have been filed as P0 follow-ups:

1. **Issue #4548** — `scripts/generate_tpch_data.sh` is missing
   `-p sqlrustgo-bench` flag. Without it, the script cannot invoke
   the in-process `tpch_data_gen` example. P0 (blocks GA-5
   promotion_to_GA_requires evidence capture).
2. **Issue #4549** — `scripts/tpch_sf1_baseline.sh` row-count
   validator has zero tolerance. Should allow a small tolerance
   (e.g., ±0.05%) to accommodate the in-process generator's rounding
   behavior. P0 (blocks GA-5 promotion_to_GA_requires evidence
   capture).

Until #4548 and #4549 are resolved, the dev-machine path in §1
above remains BLOCKED for any operator using the in-process backend.
The CI/Z6G4 path here uses `dbgen` directly (via
`/home/openclaw/...`) and is unaffected, but the same `tpch_data_gen`
backend is also blocked.

## Anti-fabrication checklist (per ADR-001)

Before posting the cell-diff:

- [ ] Every `q17_*` field references the baseline report's actual
      output, not an estimate.
- [ ] `evidence_hash` equals `git rev-parse HEAD` at the time of
      capture.
- [ ] If a step is skipped (e.g., cache warmup), the report states
      `step: SKIPPED — reason: <one-line>`.
- [ ] ADR-014 5 evidence fields are filled: source_agent,
      source_run, timestamp, evidence_hash, conflict_resolution.

## Provenance (ADR-014 5 evidence fields)

| 字段 | 值 |
|------|---|
| source_agent | claude-sonnet (Claude Code) |
| source_run | issue-4540-tpch-q17-sf1-cell-diff-20260827 |
| timestamp | 2026-08-27T22:00:00+08:00 |
| evidence_hash | local-git:`9b604a592` (post-merge of PR #4545) |
| conflict_resolution | N/A — single AI scope |

## References

- Issue #4540 (this change)
- Issue #4432 (Q17 perf acceptance boundary)
- Issue #4502 (GA-5 oracle match)
- Issue #4497 (V312-59-D umbrella)
- Issue #4548 (P0 follow-up: generate_tpch_data.sh missing `-p` flag)
- Issue #4549 (P0 follow-up: tpch_sf1_baseline.sh zero tolerance)
- `openspec/changes/issue-4540-tpch-q17-sf1-cell-diff/proposal.md`
- `openspec/changes/issue-4540-tpch-q17-sf1-cell-diff/design.md`
- `openspec/changes/issue-4540-tpch-q17-sf1-cell-diff/specs/tpch-sf1-baseline/spec.md`
- `openspec/changes/issue-4540-tpch-q17-sf1-cell-diff/specs/ga-5-tpch-sf1/spec.md`
- `.gitea/workflows/z6g4-tpch-sf1-cell-diff.yml` (DRAFT)