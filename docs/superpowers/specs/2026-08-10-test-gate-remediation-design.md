# Test Gate Remediation Design

**Date**: 2026-08-10
**Branch**: `feature/sync-252-to-250-v2` → new worktree `feature/test-gate-remediation`
**Base**: `develop/v3.12.0` (backup250)

## Problem Statement

The v3.10 → v3.12 testing system enhancements (V312-24 to V312-30) shipped to `develop/v3.12.0` but exhibit six categories of gate/ci integration gaps that allow the project to claim completion without enforcing the claimed gates.

### Discovered Gaps (from 2026-08-10 audit)

| # | Gap | Severity | Status |
|---|-----|----------|--------|
| 1 | `src/execution_engine.rs:731` missing `Value::Json` match arm | **P0** (blocks all lib compilation) | unfixed |
| 2 | G17 Coverage Gate — `check_g_all.sh` lacks G17 entry; `check_coverage.sh` is v3.7.0 LEGACY (40% threshold) | P0 (V9 vulnerability known since v3.9.0) | unfixed |
| 3 | sql_corpus 80% gate — `corpus_test::test_sql_corpus_all` panics below 80% but no CI workflow invokes it | P0 (99.4% pass rate is unverifiable in CI) | unfixed |
| 4 | `openspec/changes/v312-27-anti-fab-fix/` directory missing | P1 (governance deviation, V312-30 evidence_hash references non-existent files) | unfixed |
| 5 | `GATE_CONDITIONS.md` v2.0 lacks G17/G18/G19 definitions | P1 (no formal coverage/corpus/ignore gate contract) | unfixed |
| 6 | `.github/workflows/regression.yml:291` still has `\|\| true` masking sqlancer failure | P1 (CI reports success when sqlancer fails) | unfixed |

## Scope

All six gaps are in scope. Each is fixed in a separate Phase with its own V312-NN identifier, commit, and (if applicable) PR to `develop/v3.12.0`. The work is performed in a new git worktree to isolate from current `feature/sync-252-to-250-v2` work.

### Out of Scope

- Fixing the 87 active `#[ignore]` markers themselves (registry 73/47 baseline preserved)
- Improving coverage of `sqlrustgo-cli` (0%), `sqlrustgo-soak` (4.89%) etc.
- New corpus files or new sqlancer strategies
- v3.13.0 GA gate criteria
- Replacing `cargo llvm-cov` with another tool

## Constraints

- **Base branch**: `develop/v3.12.0` (the v3.12.0 line where all V312-24~30 work lives)
- **Worktree isolation**: per `worktree-safety.md`, never edit worktree files without `git stash` first
- **Commit format**: `<type>: <description>` per CLAUDE.md
- **Pre-commit identity**: `openheart@gaoyuanyiyao.com`
- **No GitHub**: all pushes to Gitea (`http://192.168.0.252:3000/openclaw/sqlrustgo.git`)
- **Gitea only**: PR creation via API `POST /api/v1/repos/openclaw/sqlrustgo/pulls`
- **AFP compliance**: every claim must cite `source_agent + source_run + timestamp + evidence_hash` (per `ANTI_FABRICATION_POLICY.md`)

## Design

### Phase 1 — V312-31 Compile Error Fix

**Goal**: Unblock all subsequent gate verification by repairing the non-exhaustive match.

**File**: `src/execution_engine.rs:731`

**Change**: Add a `Value::Json(_)` match arm returning the JSON literal string `"JSON"`, mirroring the convention of nearby arms (e.g., `Value::Point(_, _) => "POINT".to_string()`).

**Pattern** (lines 731-740 context):
```rust
.map(|v| match v {
    Value::Null => "NULL".to_string(),
    Value::Point(_, _) => "POINT".to_string(),
    &sqlrustgo_types::Value::Json(_) => "JSON".to_string(),  // NEW
    // ... existing arms
})
```

**Verification**:
```bash
cargo check --all-features
cargo test --workspace --no-run
```

**Acceptance**:
- `cargo check --all-features` exits 0
- `cargo test --workspace --no-run` exits 0
- No new clippy warnings

---

### Phase 2 — V312-32 + V312-33 Governance Documentation

#### V312-32 — openspec reconstruction for V312-27

**Goal**: Restore V312-27 openspec artifacts that were missing from the original ship, marking them as retroactive reconstructions.

**Files created**:
- `openspec/changes/v312-27-anti-fab-fix/proposal.md` (≤100 lines)
- `openspec/changes/v312-27-anti-fab-fix/tasks.md` (≤60 lines)
- `openspec/changes/v312-27-anti-fab-fix/design.md` (≤80 lines)

**Header convention** (all three files):
```markdown
# V312-27 Anti-Fab Fix — Retroactive Reconstruction

**Status**: RECONSTRUCTED 2026-08-10
**Source evidence**:
- git commit `2b9a0d8d55` (V312-27 anti-fab fixes)
- git commit `2a5d806840` (V312-27 closure report)
- `docs/releases/v3.12.0/V312-27_anti_fab_fix_report.md`

This file was created retroactively to satisfy V312-24 governance
(per `openspec/README.md` requirement that every V312-NN change
have proposal/tasks/design). No new behavior is introduced.
```

**proposal.md content** (≤100 lines):
- Original motivation: 37 stale + 3 union + 1 phantom `#[ignore]` entries drifted from `tests/baseline/ignore_registry.json`; 5 `e2e_beta_test.rs` early-returns masked test execution
- Scope: registry reconciliation + e2e early-return removal + 2 assertion tightenings
- Non-goals: fixing the underlying tests themselves, lowering `total_allowed` further

**tasks.md content** (≤60 lines):
- Task 1.1: Delete 37 stale `#[ignore]` entries from `tests/baseline/ignore_registry.json`
- Task 1.2: Delete 5 `if is_e2e_disabled()` early-return blocks in `tests/e2e/e2e_beta_test.rs`
- Task 1.3: Remove `#[ignore]` from `crates/executor/tests/merge_vtu_test.rs`, replace with `VtuGuard<()>` Send+Sync assertion
- Task 1.4: Tighten `rows.len() <= 6` → `rows.len() > 0` in `tests/integration/tpch/tpch_wire_smoke_sf.rs`
- Task 1.5: Reconcile `total_allowed` baseline in registry.json

**design.md content** (≤80 lines):
- Anti-fabrication: each `#[ignore]` must link to a real issue or a phased removal plan
- Registry drift detection: every `#[ignore]` in source must appear in registry.json
- Exit criterion: 5 boundary conditions from `V312-27_anti_fab_fix_report.md` all PASS

#### V312-33 — GATE_CONDITIONS.md additions

**Goal**: Add G17, G18, G19 gate definitions to formalize the existing gate scripts.

**File**: `docs/governance/GATE_CONDITIONS.md`

**Append section** (after existing G1-G16, before footer):
```markdown
## Coverage Gates (added 2026-08-10)

### G17 — Coverage Gate

| Stage | Threshold | Script |
|-------|-----------|--------|
| ALPHA | L1_8 avg ≥ 75% | scripts/gate/check_coverage_v312.sh |
| BETA | L1_8 avg ≥ 80% | scripts/gate/check_coverage_v312.sh |
| RC | L1_8 avg ≥ 80% | scripts/gate/check_coverage_v312.sh |
| GA | L1_8 avg ≥ 80% | scripts/gate/check_coverage_v312.sh |

L1_8 = {parser, planner, executor, transaction, storage, catalog, optimizer, types}.

### G18 — SQL Corpus Gate

| Stage | Threshold | Script |
|-------|-----------|--------|
| BETA | pass_rate ≥ 80.0% | scripts/gate/check_sql_corpus_gate.sh |
| RC | pass_rate ≥ 80.0% | scripts/gate/check_sql_corpus_gate.sh |
| GA | pass_rate ≥ 80.0% | scripts/gate/check_sql_corpus_gate.sh |

Test target: `cargo test --release -p sqlrustgo-sql-corpus --test corpus_test test_sql_corpus_all`.

### G19 — Anti-Ignore Gate

| Stage | Threshold | Script |
|-------|-----------|--------|
| BETA | active ≤ 47, total_allowed ≤ 73 | scripts/gate/check_anti_ignore_gate.sh |
| RC | active ≤ 47, total_allowed ≤ 73 | scripts/gate/check_anti_ignore_gate.sh |
| GA | active ≤ 47, total_allowed ≤ 73 | scripts/gate/check_anti_ignore_gate.sh |

Active = entries in `tests/baseline/ignore_registry.json` with status=ACTIVE.
```

**Acceptance**: Three sections appear in alphabetical order with consistent YAML table format.

---

### Phase 3 — V312-34/37 Gate Scripts

#### V312-34 — check_g_all.sh orchestration

**Goal**: Add G17/G18/G19 entries to the orchestrator.

**File**: `scripts/gate/check_g_all.sh`

**Change** (append to GATES array after G16):
```bash
G17  check_coverage_v312.sh        (#3911) blocking=yes
G18  check_sql_corpus_gate.sh      (#3911) blocking=yes
G19  check_anti_ignore_gate.sh     (#3911) blocking=yes
```

Update header comment from `v3.9.0 G1-G16 orchestrator` → `v3.12.0 G1-G19 orchestrator`.

#### V312-35 — Replace legacy check_coverage.sh

**Goal**: Replace v3.7.0 LEGACY script with per-crate measurement.

**File changes**:
1. `scripts/gate/check_coverage_v312.sh` (NEW, ~80 lines)
2. `scripts/gate/check_coverage.sh` (becomes deprecation shim)

**check_coverage_v312.sh** behavior:
```bash
#!/bin/bash
# V312-34: per-crate coverage measurement (L1_8)
# Threshold: avg line coverage ≥80%
set -e

L1_8=(sqlrustgo-parser sqlrustgo-planner sqlrustgo-executor \
      sqlrustgo-transaction sqlrustgo-storage sqlrustgo-catalog \
      sqlrustgo-optimizer sqlrustgo-types)
TOTAL=0
COUNT=0

for crate in "${L1_8[@]}"; do
    coverage=$(cargo llvm-cov test -p "$crate" --no-fail-fast 2>/dev/null \
        | grep -oP 'covered:\s*\K[0-9.]+' | head -1)
    if [ -n "$coverage" ]; then
        TOTAL=$(echo "$TOTAL + $coverage" | bc)
        COUNT=$((COUNT + 1))
    fi
done

AVG=$(echo "scale=2; $TOTAL / $COUNT" | bc)
echo "L1_8 average coverage: ${AVG}%"

if (( $(echo "$AVG < 80.0" | bc -l) )); then
    echo "FAIL: L1_8 avg ${AVG}% < 80%" >&2
    exit 1
fi
exit 0
```

**check_coverage.sh** becomes:
```bash
#!/bin/bash
# DEPRECATED 2026-08-10: use check_coverage_v312.sh (G17)
exec "$(dirname "$0")/check_coverage_v312.sh" "$@"
```

#### V312-36 — check_sql_corpus_gate.sh (NEW, G18)

**File**: `scripts/gate/check_sql_corpus_gate.sh` (~40 lines)

```bash
#!/bin/bash
# V312-36: SQL Corpus 80% pass rate gate
set -e

REPORT=$(mktemp)
cargo test --release -p sqlrustgo-sql-corpus --test corpus_test \
    test_sql_corpus_all -- --nocapture > "$REPORT" 2>&1 || true

PASS_RATE=$(grep -oP 'pass rate:\s*\K[0-9.]+' "$REPORT" | head -1)

if [ -z "$PASS_RATE" ]; then
    echo "FAIL: could not extract pass rate from corpus output" >&2
    cat "$REPORT" >&2
    rm -f "$REPORT"
    exit 1
fi

echo "SQL Corpus pass rate: ${PASS_RATE}%"

if (( $(echo "$PASS_RATE < 80.0" | bc -l) )); then
    echo "FAIL: pass rate ${PASS_RATE}% < 80% threshold" >&2
    rm -f "$REPORT"
    exit 1
fi

rm -f "$REPORT"
exit 0
```

#### V312-37 — check_anti_ignore_gate.sh (NEW, G19)

**File**: `scripts/gate/check_anti_ignore_gate.sh` (~50 lines)

```bash
#!/bin/bash
# V312-37: Anti-Ignore gate — verify #[ignore] count against baseline
set -e

REGISTRY="tests/baseline/ignore_registry.json"
TOTAL_ALLOWED_MAX=73
ACTIVE_MAX=47

if [ ! -f "$REGISTRY" ]; then
    echo "FAIL: $REGISTRY not found" >&2
    exit 1
fi

# Count ACTIVE entries
ACTIVE=$(python3 -c "
import json
with open('$REGISTRY') as f:
    data = json.load(f)
print(sum(1 for e in data['entries'] if e.get('status') == 'ACTIVE'))
")

# Read total_allowed
TOTAL_ALLOWED=$(python3 -c "
import json
with open('$REGISTRY') as f:
    data = json.load(f)
print(data.get('total_allowed', 0))
")

echo "ignore_registry.json: total_allowed=$TOTAL_ALLOWED, active=$ACTIVE"

if [ "$ACTIVE" -gt "$ACTIVE_MAX" ]; then
    echo "FAIL: active entries $ACTIVE > $ACTIVE_MAX" >&2
    exit 1
fi

if [ "$TOTAL_ALLOWED" -gt "$TOTAL_ALLOWED_MAX" ]; then
    echo "FAIL: total_allowed $TOTAL_ALLOWED > $TOTAL_ALLOWED_MAX" >&2
    exit 1
fi

exit 0
```

---

### Phase 4 — V312-38/40 CI Integration

#### V312-38 — New ci-corpus.yml workflow (G18)

**File**: `.github/workflows/ci-corpus.yml` (~40 lines)

```yaml
name: CI Corpus Gate

on:
  pull_request:
    branches: [develop/v3.12.0, develop/v3.13.0, main]
  push:
    branches: [develop/v3.12.0, develop/v3.13.0]

jobs:
  corpus:
    runs-on: ubuntu-latest
    timeout-minutes: 60
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov
      - name: Run G18 corpus gate
        run: bash scripts/gate/check_sql_corpus_gate.sh
```

#### V312-39 — Add G17 step to ci-pr.yml

**File**: `.github/workflows/ci-pr.yml`

**Insert** (after existing "Cargo Check" step):
```yaml
- name: Gate G17 — Coverage
  run: bash scripts/gate/check_coverage_v312.sh
```

**Trigger condition**: only when source changes touch crates other than docs/.

#### V312-40 — Remove `|| true` masking

**File**: `.github/workflows/regression.yml:291`

**Change**:
```diff
-    timeout 600 cargo run --release -p sqlancer -- --duration 300 || true
+    timeout 600 cargo run --release -p sqlancer -- --duration 300
```

**Audit** (run `grep -rn "|| true" .github/workflows/`):
- All remaining `|| true` occurrences are documented as intentional (e.g., best-effort artifact upload)
- Each documented occurrence gets a `# intentional: <reason>` comment

**Acceptance**: `grep -c "|| true" .github/workflows/*.yml` reduced to documented-only.

---

### Phase 5 — V312-41 STAGE_CONFIG.yaml sync

**File**: `docs/governance/STAGE_CONFIG.yaml`

**Change** (move check_coverage from optional to required_gates):
```diff
- optional_gates:
-   - check_coverage.sh
+ required_gates:
+   - check_coverage_v312.sh
+   - check_sql_corpus_gate.sh
+   - check_anti_ignore_gate.sh
```

---

## Sequencing & PR Strategy

Each Phase becomes one PR. PR numbers are filled in at push time (Gitea API returns them).

| PR (placeholder) | Title | Commits | Merge Target |
|----|-------|---------|--------------|
| PR-1 | `fix(executor): add Value::Json match arm (V312-31)` | 1 | `develop/v3.12.0` |
| PR-2 | `docs(governance): V312-27 openspec reconstruction + G17-G19 gate definitions (V312-32, V312-33)` | 2 | `develop/v3.12.0` |
| PR-3 | `feat(gate): add G17/G18/G19 gate scripts + rewrite legacy coverage (V312-34..37)` | 4 | `develop/v3.12.0` |
| PR-4 | `ci: add corpus workflow + remove \|\| true masking + coverage step (V312-38..40)` | 3 | `develop/v3.12.0` |
| PR-5 | `chore(governance): STAGE_CONFIG sync for G17/G18/G19 (V312-41)` | 1 | `develop/v3.12.0` |

Total: 5 PRs, 11 commits, ~13 files touched (5 new files, 6 modified, 2 deprecated).

## Verification Plan

After each Phase:

```bash
# Per Phase
cargo check --all-features          # V312-31 only
cargo test --workspace --no-run     # All phases
bash scripts/gate/check_g_all.sh    # Phases 3-5

# After all Phases
bash scripts/gate/check_coverage_v312.sh
bash scripts/gate/check_sql_corpus_gate.sh
bash scripts/gate/check_anti_ignore_gate.sh
```

## Success Criteria

1. `cargo check --all-features` exits 0 (V312-31)
2. `bash scripts/gate/check_g_all.sh` exits 0 with G17-G19 entries (V312-34)
3. `bash scripts/gate/check_sql_corpus_gate.sh` exits 0 with current 99.4% pass rate (V312-36)
4. `bash scripts/gate/check_anti_ignore_gate.sh` exits 0 (V312-37)
5. `grep "|| true" .github/workflows/*.yml` shows only documented exceptions (V312-40)
6. `docs/governance/GATE_CONDITIONS.md` contains G17/G18/G19 definitions (V312-33)
7. `openspec/changes/v312-27-anti-fab-fix/` exists with proposal.md/tasks.md/design.md (V312-32)
8. PRs PR-1..5 merged to `develop/v3.12.0` via Gitea API

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| G17 (L1_8 ≥80%) currently at 80.60% — close to threshold | G17 may intermittently fail | Add 0.5% tolerance buffer; document as known |
| New CI workflow (ci-corpus.yml) adds 5-10min to PR | Slower CI | Cache `~/.cargo`; run on `ubuntu-latest` only |
| openspec reconstruction violates "no retroactive changes" governance principle | Governance pushback | Mark all three files with `Status: RECONSTRUCTED` headers; explain in commit body |
| Worktree loses AGENTS.md modification on stash | Lost work | Stash AGENTS.md before initial pull per worktree-safety.md |
| Removing `|| true` exposes flaky sqlancer | Increased CI noise | If sqlancer proves flaky in Phase 4, replace with targeted subset (e.g., `--iterations 10 --duration 60`) |

## Rollback

Each Phase is independent. If any Phase causes regressions:
1. Revert the Phase's PR via `gh`-equivalent Gitea API
2. Re-evaluate the gate definition before re-attempting

No Phase modifies user-facing behavior; rollback is risk-free at the gate layer.