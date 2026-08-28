# Design: TPC-H Q17 SF=1 cell-diff (Issue #4540)

## Overview

This change has **no production-code surface**. It delivers a runbook,
an evidence capture path, and the orchestration needed to validate
issue #4432's perf acceptance criteria on a full SF=1 lineitem
corpus.

## Components

### 1. Dev-machine runbook (§1)

Step-by-step commands to generate SF=1 fixture on a typical dev
laptop, run the baseline, and capture the Q17 cell-diff.

#### Prerequisites

- `/tmp` ≥ 2 GB free (TPC-H SF=1 ≈ 1.2 GB).
- `cargo build --release` available (already passes per
  `c8907925c` worktree smoke check).
- Disk-cache warm-up: run a non-Q17 query once before measuring
  Q17 elapsed (cold-cache penalty distorts the metric).

#### Commands

```bash
# 1. Generate SF=1 fixture (~20-40 min on dev laptop)
bash scripts/generate_tpch_data.sh --sf 1 \
    --output /tmp/tpch-sf1 --backend tpch_data_gen

# 2. Verify row counts (must match: region=5, nation=25, supplier=10000,
#    customer=150000, part=200000, partsupp=800000, orders=1500000,
#    lineitem=6001215)
bash scripts/generate_tpch_data.sh --sf 1 --output /tmp/tpch-sf1 --check

# 3. Run the baseline (Q17 will take up to 300s)
bash scripts/tpch_sf1_baseline.sh --sf1-dir /tmp/tpch-sf1

# 4. Capture cell-diff to evidence file (manual step)
#    See evidence/v312-58/issue-4540-sf1-cell-diff.md template.
```

### 2. CI/Z6G4 runbook (§3)

Alternate path on the production-aligned Docker runner where
`devstack-gitea-1` is hosted. Issue #4540 explicitly prefers this
path because:

- Production-aligned container (same kernel, same disk layout as
  the GA smoke run).
- ~10x faster fixture generation (NVMe-backed disk).
- Z6G4 has 168h SOAK reserved slot — same machine can back-to-back
  Q17 + 168h SOAK without disk-state churn.

#### Pre-conditions

- Z6G4 access via project VPN.
- `devstack-gitea-1` reachable at `192.168.0.252:3000`.
- `/tmp/tpch-sf1` already prepared by issue-4502 fix-up (cross-link
  with the GA-5 evidence chain).

#### Commands

```bash
# 1. SSH into Z6G4 (CI runner)
ssh z6g4-runner

# 2. Sync to v3.12.0 RC HEAD
cd /work/openclaw/sqlrustgo
git fetch origin
git checkout develop/v3.12.0
git pull --recurse-submodules

# 3. Generate SF=1 fixture
bash scripts/generate_tpch_data.sh --sf 1 \
    --output /tmp/tpch-sf1 --backend tpch_data_gen

# 4. Run baseline + capture
bash scripts/tpch_sf1_baseline.sh --sf1-dir /tmp/tpch-sf1

# 5. Upload report to Gitea issue #4432
gh-issue-post.sh #4432 evidence/v312-58/issue-4540-sf1-cell-diff.md
```

### 3. Evidence capture

#### File path

`evidence/v312-58/issue-4540-sf1-cell-diff.md`

#### Required fields (per ADR-001 G-04 / G-08)

| 字段 | 值 | 来源 |
|------|---|------|
| `host` | `z6g4-runner` or dev laptop hostname | `hostname` |
| `os_kernel` | `Linux 6.x` or `Darwin 25.x` | `uname -r` |
| `disk_type` | `NVMe` (Z6G4) or `Apple SSD` (dev) | `diskutil info /tmp` |
| `sf1_lineitem_rows` | `6001215` | `wc -l /tmp/tpch-sf1/lineitem.tbl` |
| `q17_elapsed_seconds` | `<= 300` (target) | `scripts/tpch_sf1_baseline.sh` output |
| `q17_row_count` | `1` | baseline output |
| `q17_sha256` | `595003bfd…` | baseline output |
| `verdict` | PASS / DEFERRED-to-v3.13 | decision based on elapsed |
| `evidence_hash` | `git rev-parse HEAD` | local-git |
| `source_run` | `issue-4540-tpch-q17-sf1-cell-diff-20260827` | this change |

### 4. Issue closure path

If `q17_elapsed_seconds <= 300`:

1. Post evidence to issue #4432 via Gitea API.
2. Link #4432 → #4540 with `[closes #4540]` in the comment.
3. Update `docs/releases/v3.12.0/STAGE.yaml` `promotion_to_GA_requires`
   progress tracker (one of GA-5 acceptance criteria done).

If `q17_elapsed_seconds > 300`:

1. Post evidence to issue #4432 with verdict DEFERRED-to-v3.13.
2. Update `docs/releases/v3.13.0/SCOPE_TABLE_v3.13.md` with the new
   perf gap (currently scoped under #4426 master).
3. #4540 stays OPEN; the runbook remains valid for v3.13 re-verification.

## Non-design

This change does **not** redesign the Q17 query plan. The
decorrelation wire-up (commit `d705176ef`) is already shipped. The
design boundary is: "produce runtime evidence on SF=1 fixture
(whether good or bad), don't touch the query plan".

## Failure modes

| 模式 | 缓解 |
|------|------|
| tpch_data_gen OOM on dev laptop (8 GB cap) | Use `--backend dbgen` (out-of-process, no Rust memory cap) |
| `/tmp` runs out of disk mid-generation | Switch to `/Users/liying/work/tpch-sf1` with `--output` override |
| Q17 timed out on cold cache | Run Q1 first to warm file_storage, then re-run Q17 |
| Z6G4 SSH unreachable | Fall back to dev machine; record evidence with `host=dev-laptop` |