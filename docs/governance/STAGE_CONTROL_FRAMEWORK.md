# Stage Control Framework — Design Document

> **Status**: ACTIVE (2026-07-01)
> **Author**: claude-macmini (per 2026-07-01 governance audit 整改)
> **Refs**: issue #3667, ADR-015 (proposed), STAGE_CONFIG.yaml

## 1. Problem

Pre-2026-07-01, SQLRustGo's stage control was:

- **Version-coupled**: each version had its own RELEASE_NOTES.md / README.md with "当前阶段" line
- **Manual**: required humans to read docs, run the right gate, judge
- **Fragmented**: stage definitions scattered across:
  - `RELEASE_LIFECYCLE.md` (v1.0, 2026-03-07)
  - `BRANCH_GOVERNANCE.md`
  - `VERSION_PROMOTION_SOP.md` (v1.2, 2026-02-20)
  - `release_process.md`
  - Header comments of `check_alpha.sh`, `check_beta_gate.sh`, `check_rc_ga_gate.sh`
- **No SSOT**: "what's the current stage?" required reading 4-5 docs
- **Drift**: 4 stage gate scripts with different naming/styles, partial overlap
- **Gate count explosion**: 93 gate scripts total, many version-specific

## 2. Solution: 3-File Framework

```
docs/governance/STAGE_CONFIG.yaml       # Framework SSOT (version-agnostic)
        ↓ references
docs/releases/vX.Y.Z/STAGE.yaml         # Per-version state
        ↓ read by
scripts/gate/check_stage.sh             # Stage gate driver
        ↓ invokes
Existing gate scripts                    # Implementations
  (check_alpha.sh, check_beta_gate.sh,
   check_rc_ga_gate.sh, check_arch_invariants.sh,
   cargo build/test/clippy/fmt)
```

### 2.1 `STAGE_CONFIG.yaml` (framework SSOT)

- **Version-agnostic** — describes the framework, not a specific version
- **5 stages**: DRAFT → ALPHA → BETA → RC → GA
- **Each stage defines**:
  - `required_gates`: list of gate scripts to run
  - `required_files`: artifacts that must exist
  - `branch_pattern`: which branches belong to this stage
  - `merge_policy`: review requirements
  - `doc_artifacts`: mandatory documentation
  - `exit_criteria`: explicit checklist to advance
- **Thresholds**: coverage, clippy warnings, fmt drift, ignore test caps
- **Transitions**: rules for moving between stages

### 2.2 `docs/releases/vX.Y.Z/STAGE.yaml` (per-version state)

- **Concrete state** for a specific version
- Contains: `current_stage`, `branch`, `tags`, `blocker_issues`, `stage_history`
- Optional `thresholds_override` for version-specific exceptions
- **Updated** when transitioning between stages
- Example: v3.9.0 is in RC with 4 hardware-blocked issues

### 2.3 `scripts/gate/check_stage.sh` (driver)

- **Single command** to run the right gates for the current state
- Auto-detects version from branch name (`develop/v3.9.0` → `v3.9.0`)
- Auto-detects stage from `STAGE.yaml` or branch name
- Reads `STAGE_CONFIG.yaml` to get the gate list for that stage
- Runs each gate, reports PASS/FAIL
- Checks required files exist
- Exits 0 (PASS) / 1 (FAIL) / 2 (DRIFT)

## 3. Why this works

### 3.1 Version-agnostic

The framework (5 stages + transition rules + thresholds) is the same for
v3.9.0, v3.10.0, v4.0.0. Only the **per-version state** (current stage,
blocker issues, tags) changes. STAGE_CONFIG.yaml is the framework; STAGE.yaml
is the state.

### 3.2 No human judgment

```bash
# Before (manual):
git branch                                  # which branch?
ls docs/releases/v*/README.md                # which version's docs?
grep "当前阶段" docs/releases/v*/README.md     # what's the stage?
# (read 4-5 docs to find the right gate)
bash scripts/gate/check_beta_gate.sh         # (guessing the right script)
# (read output, judge if PASS)

# After (automated):
bash scripts/gate/check_stage.sh            # auto-detects everything
```

### 3.3 Backward compatible

The existing 4 stage gate scripts (check_alpha.sh, check_beta_gate.sh,
check_rc_ga_gate.sh, check_alpha_v380.sh) **still exist and still work**
as standalone commands. The framework just calls them from a single
entrypoint. No script deletion, no rewrite required.

### 3.4 CI integration

`.gitea/workflows/ci.yml` can call `check_stage.sh` instead of hardcoding
which gates to run for which branch. Stage is detected automatically.

```yaml
# In ci.yml:
- name: Stage gate
  run: bash scripts/gate/check_stage.sh
```

## 4. Usage

### 4.1 Daily development

```bash
# What stage is this version in? What gates will run?
bash scripts/gate/check_stage.sh --list

# Run gates for current state
bash scripts/gate/check_stage.sh

# Dry run (show what would run)
bash scripts/gate/check_stage.sh --dry-run

# Force a specific stage (testing)
bash scripts/gate/check_stage.sh --stage RC

# Per-version override
bash scripts/gate/check_stage.sh --version v3.9.0
```

### 4.2 Stage transition

```bash
# 1. Update STAGE.yaml: current_stage, last_transition
# 2. Run the gate
bash scripts/gate/check_stage.sh
# 3. If PASS, human approves (see merge_policy in STAGE_CONFIG.yaml)
# 4. Cut tag, commit, push
git tag v3.9.0-rcN
git commit -am "Stage transition: BETA -> RC"
git push
```

### 4.3 CI integration

```yaml
# In .gitea/workflows/ci.yml (or any CI):
- name: Stage Gate Check
  run: bash scripts/gate/check_stage.sh
```

## 5. Stage definitions

| Stage | Order | Required gates | Branch pattern | Exit criteria |
| --- | --- | --- | --- | --- |
| DRAFT | 1 | build, docs-links | develop/v*, feature/* | first alpha tag |
| ALPHA | 2 | check_alpha, arch-invariants, arch3, build, test, fmt | develop/v* | coverage ≥ 50%, all alpha issues closed |
| BETA | 3 | check_beta_gate, arch-invariants, arch3, arch-sem-debt, cross-version-debt, int-debt, build, test, fmt, clippy | develop/v*, beta/v* | all features CLOSED/DEFERRED, coverage ≥ 75%, B-F1~B-F7 PASS |
| RC | 4 | check_rc_ga_gate, arch-invariants, arch3, integration, anti-fab, full-gate, drift-not-pass, build, test, fmt, clippy | develop/v*, release/v*, rc/v* | 5 dimensions PASS, all P0 bugs closed, human signoff |
| GA | 5 | all RC gates + arch-freeze, gate-self-verification, gate-test-integrity, soak | main, release/v*, hotfix/v* | GA report signed, tag cut, merge to main |

See STAGE_CONFIG.yaml for the canonical list.

## 6. Migration path

The framework is **additive, not replacing**:

| Pre-framework | Post-framework | Notes |
| --- | --- | --- |
| `check_alpha.sh` (standalone) | `check_alpha.sh` + `check_stage.sh --stage ALPHA` | Both still work |
| `bash check_alpha_v380.sh` (v2.9.0 specific) | `check_stage.sh --version v2.9.0 --stage ALPHA` | v380-specific args removed |
| `bash check_beta_gate.sh` | `check_stage.sh --stage BETA` | Driver wraps it |
| `bash check_rc_ga_gate.sh` | `check_stage.sh --stage RC` or `--stage GA` | Mode arg respected |
| Manual `cat README.md \| grep 当前阶段` | `check_stage.sh --list` | Single source of truth |
| Per-version `RELEASE_NOTES.md 当前阶段: RC` | `STAGE.yaml current_stage: RC` (machine-readable) | YAML > prose |

Existing standalone scripts are **preserved** for backward compatibility
and emergency use. The framework is the recommended entry point.

## 7. Future work (not in this PR)

- **ADR-015**: Formal ADR for "Stage Control Framework" (current PR proposes it)
- **CI integration**: Update `.gitea/workflows/ci.yml` to call `check_stage.sh`
- **Per-version STAGE.yaml adoption**: Create STAGE.yaml for v3.8.0, v3.7.0, v2.9.0, etc. (one-time)
- **Migration of existing gate headers**: Add `STAGE_CONFIG.yaml` reference to the 4 stage gate scripts' headers
- **Deprecation**: After adoption, mark RELEASE_LIFECYCLE.md / VERSION_PROMOTION_SOP.md as "see STAGE_CONFIG.yaml"

## 8. Cross-references

- **Framework SSOT**: `docs/governance/STAGE_CONFIG.yaml`
- **Per-version state**: `docs/releases/vX.Y.Z/STAGE.yaml`
- **Driver script**: `scripts/gate/check_stage.sh`
- **Existing stage gates** (still work):
  - `scripts/gate/check_alpha.sh`
  - `scripts/gate/check_beta_gate.sh`
  - `scripts/gate/check_rc_ga_gate.sh`
  - `scripts/gate/check_alpha_v380.sh`
- **Related**:
  - `docs/governance/RELEASE_LIFECYCLE.md` (v1.0 baseline, banner-added)
  - `docs/governance/BRANCH_GOVERNANCE.md`
  - `docs/governance/VERSION_PROMOTION_SOP.md` (v1.2, banner-added)
  - `docs/governance/release_process.md` (banner-added)
  - `docs/governance/GATE_CI_CD.md`
  - `docs/governance/GATE_CONDITIONS.md`

## 9. Author

2026-07-01 governance audit 整改, claude-macmini.

Refs: #3667 (state snapshot), GOVERNANCE_COMPLIANCE_REPORT §7.
