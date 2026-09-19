# File Governance (v4.0.0)

> **Status**: binding rule for all commits to `develop/v3.12.0`, `develop/v4.0.0`, `main`, `release/v3.12.0`.
> **Origin**: user rule 2026-09-08 (extended 2026-09-11).
> **Enforcement**: `.github/workflows/gate-no-log-tbl-json.yml` + `scripts/gate/check_no_log_tbl_json.sh` + Gitea branch protection `protected_file_patterns: *.log,*.tbl,*.json`.

## 1. Forbidden file types

The following file extensions MUST NOT be committed to the repository:

| Pattern | Why | Allowed exception |
|---|---|---|
| `*.log` | Test runs dump server.log / sysbench.log / cargo.log / sql log | none |
| `*.tbl` | Data dump files (TPC-H fixtures in `tests/data/tpch-sf1/` excepted) | `tests/data/tpch-sf1/{nation,region}.tbl` (tiny fixtures) |
| `*.json` | Evidence bundles tend to be megabytes | small config only (see §2) |
| `*.db` | Embedded sqlite dumps | none |
| `*.wal` | SQLite write-ahead-log fragments | none |
| `*.html` | HTML evidence pages render huge | none |
| `*.tar.gz`, `*.zip` | Archive artifacts | none |
| `*.bin` | Binary evidence | `docs/formal/PROOF_*_mvcc_toctou_TTrace_*.bin` (5 TLA+ proof artifacts) |
| `*.pptx`, `*.docx`, `*.pdf` | Documentation output (regenerable) | none |
| `*.profraw` | Profile data | none |
| `*.tsv`, `*.dat`, `*.csv`, `*.out`, `*.tsv.gz` | Test result dumps | none |

## 2. JSON allowlist

JSON files matching these patterns ARE allowed (small configuration files only):

| Pattern | Reason |
|---|---|
| `Cargo.toml` | Build config (not actually JSON, but listed for reference) |
| `package.json`, `tsconfig.json`, `tsconfig.*.json`, `package-lock.json` | npm / node config |
| `*.config.json` | Generic small config (e.g. `.eslintrc.json`, `.prettierrc.json`, `!*config.json`) |
| `docs/governance/*.json` | Governance rule definitions |
| `docs/releases/*/manifest.json` | Release manifests (small, summary only) |
| `docs/releases/*/evidence/*/summary.json` | Gate summary files (small) |

Anything matching `*.json` NOT in the allowlist MUST NOT be committed.

## 3. File size limit

| Limit | Reason |
|---|---|
| 100 MB per file | GitHub rejects >100 MB blobs; multi-MB evidence should be split or compressed |
| 1 MB per `*.json` | Most useful gate evidence fits well below 1 MB |

## 4. Why these rules?

- **Reproducibility**: A clean working tree of `git ls-files` should produce the same CI artifact on every machine.
- **PR hygiene**: A reviewer should be able to inspect any PR diff without scrolling past 251 MB of test output.
- **Multi-remote sync**: GitHub has a 100 MB hard cap and 5 GB soft cap. LFS is not configured for sqlrustgo.
- **GA evidence discipline**: `docs/releases/*/evidence/` directories should contain only summary JSONs + referenced log paths in `manifest.json`. Raw logs are gitignored and live only on CI runners.

## 5. Enforcement layers

1. **`.gitignore`** (preventive) — ignores forbidden extensions at `git add` time.
2. **Local gate script** (`scripts/gate/check_no_log_tbl_json.sh`) — runs on each commit during dev; exit 1 on violation.
3. **CI workflow** (`.github/workflows/gate-no-log-tbl-json.yml`) — runs on every PR and push to protected branches; blocks merge.
4. **Gitea branch protection** (`protected_file_patterns: *.log,*.tbl,*.json`) — prevents push of protected-file paths on `.252` and `.250`.

## 6. Bypass / exception process

If a legitimately-needed file falls into the forbidden set:

1. Open a discussion issue with `governance` label explaining the need.
2. Get approval from the release owner (per WP) — usually the WP responsible for the affected feature.
3. Either:
   - Add to the allowlist in `FILE_GOVERNANCE.md` and the gate script's `ALLOWLIST_REGEX`, OR
   - Move to an external object store (S3-like, or `.git/lfs`) and reference by URL.

## 7. Cleanup history (2026-09-09 / 2026-09-10)

Historical commits (before 2026-09-08) contained forbidden files. Cleanup was performed via:

- `git filter-repo --invert-paths --paths-from-file <forbidden-list>` on all 4 main branches
- `git gc --aggressive --prune=now` on `.252`, `.250`, local
- All 5 remotes (`openclaw/sqlrustgo` on `.252`/`.250`, `BreavHeart/sqlrustgo` on `gitcode`, `yinglichina/sqlrustgo` on `gitee`, `yinglichina8848/sqlrustgo-public` on `github`) are now at SHAs `3422c039f1` (v3.12.x) / `d6ef1dd00f` (v4.0.0) with no reachable forbidden paths in HEAD tree.

12 legitimate-forbidden paths are preserved as exceptions (see §1 and §2).

## 8. Operational check

```bash
# Quick verification
scripts/gate/check_no_log_tbl_json.sh HEAD

# Should produce:
#   Checking HEAD for log/tbl/json violations and file size...
#   PASS: no files changed in HEAD
```

For full history reachability check:

```bash
git rev-list --objects --all | \
    git cat-file --batch-check='%(objecttype) %(objectname) %(rest)' | \
    grep '^blob ' | \
    awk '{print $3}' | \
    grep -E '\.(log|tbl|json|db|wal|html|tar\.gz|zip|bin|pptx|profraw|docx|tsv|dat|csv|out)$' | \
    grep -v -F -f /tmp/forbidden_allowlist.txt
```

(Last verified clean: 2026-09-11.)
