# TPC-H .tbl Fixture Data Handling

> **Status**: enforced via `.gitignore` (line 72-73) and AGENTS.md.
> **Audited**: 2026-07-01 after issue: accidental commit of 8 LFS-tracked
> TPC-H .tbl files (3.3 GB total) into `tests/data/tpch-sf001-real/` from the
> local SF=0.01 SOAK test corpus.

## Rule

**TPC-H `.tbl` fixture files must NEVER be committed to git directly.**

This applies to all scale factors:

| Path | SF | Generator | How big |
| --- | --- | --- | --- |
| `tests/data/tpch-sf001/*.tbl` | 0.001 | `scripts/gate/generate_sf001_fixture.py` | ~9 KB total |
| `tests/data/tpch-sf01/*.tbl`   | 0.01  | `scripts/gate/generate_tpch_sf01_fixture.py` | ~2 MB total |
| `tests/data/tpch-sf1/*.tbl`    | 1.0   | `scripts/generate_tpch_data.sh --sf 1` | ~1.2 GB total |
| `tests/data/tpch-sf001-real/*.tbl` | 0.001 (real dbgen output) | repo-local only | ~1 GB total |
| `data/*.tbl` | ad-hoc | `scripts/generate_tpch_data.sh` | varies |

`.gitignore` covers the first four via patterns:

```gitignore
data/*.tbl
tests/data/tpch-sf001/*.tbl
tests/data/tpch-sf01/*.tbl
tests/data/tpch-sf1/*.tbl
tests/data/tpch-sf001-real/*.tbl   # added 2026-07-01
```

## Why

1. **LFS cost.** `.gitattributes` routes `.tbl` through Git LFS so any
   `tests/data/tpch-sf1/lineitem.tbl` (1.2 GB) ends up in the LFS object
   store. Gitea at `192.168.0.252:3000` has the LFS extension tracked via
   `.gitattributes`, but the LFS server itself is not enabled
   (`POST /info/lfs/objects/batch` returns HTTP 404). Until admin enables
   `[lfs] STORAGE_TYPE = local` in `app.ini` and restarts
   `devstack-gitea-1`, any commit containing `.tbl` LFS pointers will
   **fail to clone / pull / rebase** on every other developer's machine.
2. **Determinism.** The Python generators in `scripts/gate/*` produce
   byte-identical output (fixed `random.seed()`), so the SHA-256 baselines
   captured by `tpch_hash_compare.py --capture` stay stable across
   regenerations. Checking generated output into git would force every
   branch / fork to carry an inconsistent copy.
3. **Reproducibility by intent.** When a developer needs SF=0.01 data,
   the recipe is one command. A committed copy ages out of sync with
   `ROW_COUNTS` and `expected_rows_for()` tables that already drift
   between SF=0.001 (501 lineitem rows) and SF=0.01 (6015 lineitem rows).

## How to regenerate

```bash
# SF=0.001 — for tests/data/tpch-sf001/
python3 scripts/gate/generate_sf001_fixture.py --output tests/data/tpch-sf001

# SF=0.01  — for tests/data/tpch-sf01/
python3 scripts/gate/generate_tpch_sf01_fixture.py --output tests/data/tpch-sf01

# SF=1.0   — for tests/data/tpch-sf1/  (full TPC-H dbgen or our builtin)
scripts/generate_tpch_data.sh --sf 1.0 --output tests/data/tpch-sf1

# SF=0.01 subset from SF=1 (used by SOAK runners)
scripts/soak/prepare_sf01_data.sh
```

## Validation

```bash
# Verify the generator output matches engine expectations:
scripts/generate_tpch_data.sh --sf 0.001 --check
```

## What if I really need a fixture committed?

Don't. If a regression test genuinely depends on a specific bit-exact
binary, put the data in:

- `tests/data/expected/*.json` (already present for Q1..Q22 hashes), or
- an encrypted / compressed `.tar.gz` under `tests/data/private/` with
  a generator script that re-creates the binary on test setup.

## What happened on 2026-07-01

In commit `853c1c722` ("test(stability): add SOAK tooling and wired-insert
payload regression") the SOAK agent accidentally committed 8 LFS-tracked
.tbl files under `tests/data/tpch-sf001-real/`. The commit reached local
HEAD but **could not be pushed** to `origin/develop/v3.9.0` (Gitea 252)
because the LFS endpoint returned HTTP 404.

Resolution:

1. Soft-reset local HEAD past the bad commit.
2. Repacked the useful content (SOAK scripts + Issue #3265 regression
   test) into the new commits `2554f557b` and `cb177b3a7`.
3. Added `tests/data/tpch-sf001-real/*.tbl` + `*.csv` to `.gitignore`
   to formally forbid the same mistake in future commits.

See `ISSUE-tls-select-hang.md` and `reports/SERVER_DEADLOCK_FIX_PLAN_2026-06-28.md`
for context on the SOAK work that produced these files.
