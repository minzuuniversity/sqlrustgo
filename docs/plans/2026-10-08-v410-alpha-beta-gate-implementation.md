# v4.1.0 Alpha-to-Beta Gate Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Publish an executable v4.1.0 Alpha-to-Beta test and development program, create its Gitea tracking issues, and bind every promotion criterion to measurable evidence.

**Architecture:** Keep `STAGE.yaml` as the stage SSOT while splitting operational ownership across `TEST_PLAN.md`, `DEV_PLAN.md`, a new gate plan, and `ISSUES_PLAN.md`. Use one master Gitea issue plus five child issues; existing implementation issues remain dependencies and are not duplicated.

**Tech Stack:** Markdown, Bash gate commands, Cargo test tooling, Gitea REST API, Git.

---

### Task 1: Create the Gitea issue hierarchy

**Files:**
- Read: `docs/releases/v4.1.0/ISSUES_PLAN.md`
- Read: `docs/releases/v4.1.0/STAGE.yaml`

**Step 1: Re-read all open v4.1.0 issues**

Query the 252 Gitea API and confirm that no existing issue already owns each
proposed child scope.

**Step 2: Create the five child issues**

Create issues for correctness convergence, fail-closed CI, workload regression,
SOAK/fault injection, and performance regression gating. Include priority,
dependencies, exact commands, quantitative acceptance criteria, and the
mandatory PR-based closure contract.

**Step 3: Create the master issue**

Link the five child issues and existing dependencies `#5099`, `#5057`, `#5025`,
`#5102`, and `#5103`. Define the same-commit final audit requirement.

**Step 4: Record returned issue IDs**

Keep the IDs for Tasks 2-5. Do not close or modify existing issues.

### Task 2: Rewrite the v4.1.0 test plan

**Files:**
- Modify: `docs/releases/v4.1.0/TEST_PLAN.md`

**Step 1: Replace inherited-only scope with a v4.1.0-specific layered model**

Define unit/contract, deterministic concurrency, wire/E2E, fault/SOAK, and
performance layers.

**Step 2: Define mandatory suites and quantitative thresholds**

Include transaction invariants, BustubX-EDU workload replay, SQL corpus/oracle,
ignore integrity, coverage, format, clippy, and SOAK duration gates.

**Step 3: Define evidence schema**

Require commit, command, exit code, counts, timestamp, agent/run provenance,
hash, and durable artifact path.

### Task 3: Publish the Alpha-to-Beta gate plan

**Files:**
- Create: `docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md`

**Step 1: Define hard-gate inventory**

Assign IDs `AB-01` through `AB-10` covering stage consistency, build quality,
test integrity, transaction correctness, workload compatibility, coverage,
SOAK readiness, performance baseline, security, and issue closure.

**Step 2: Define review workflow**

Specify preflight, execution, artifact validation, independent review, and
promotion decision steps.

**Step 3: Define fail-closed semantics**

Treat missing, stale, ignored, filtered, malformed, or non-zero evidence as
FAIL. Prohibit conditional Beta promotion for P0 correctness or gate integrity.

### Task 4: Update the development plan

**Files:**
- Modify: `docs/releases/v4.1.0/DEV_PLAN.md`

**Step 1: Replace stale completed/pending claims**

State the current Alpha remediation objective without claiming current gates
pass.

**Step 2: Add execution waves**

Wave A: correctness and CI; Wave B: regression system; Wave C: SOAK and
performance; Wave D: promotion audit.

**Step 3: Add ownership and dependency rules**

Bind every wave to the new issue IDs and existing implementation dependencies.

### Task 5: Update the issue plan

**Files:**
- Modify: `docs/releases/v4.1.0/ISSUES_PLAN.md`

**Step 1: Add a dated Alpha-to-Beta program section**

Record the master and five child issue IDs, priority, dependencies, and closure
criteria without rewriting historical issue tables.

**Step 2: Add status freshness rules**

State that issue state comes from Gitea and that historical tables are audit
records, not current closure evidence.

### Task 6: Verify and commit

**Files:**
- Verify all changed Markdown files.

**Step 1: Run static documentation checks**

Run:

```bash
git diff --check
bash scripts/gate/check_docs_links.sh --all
bash scripts/gate/check_docs_consistency.sh
```

Expected: capture actual exit codes; do not claim PASS for failures.

**Step 2: Run gate-integrity checks relevant to the claims**

Run:

```bash
bash scripts/gate/check_gate_test_integrity.sh
bash scripts/gate/check_anti_ignore_gate.sh
```

Expected: capture actual output. Existing failures remain blockers and must be
recorded, not hidden.

**Step 3: Review issue links and exact IDs**

Verify every new Gitea issue URL resolves and every dependency ID is correct.

**Step 4: Run GitNexus change detection**

Run `gitnexus_detect_changes` for the worktree. If the repository remains
unregistered, record that limitation and use `git diff --stat` plus link checks
as the scope evidence.

**Step 5: Commit the release documentation**

```bash
git add docs/releases/v4.1.0 docs/plans/2026-10-08-v410-alpha-beta-gate-implementation.md
git commit -m "docs(v4.1.0): define Alpha-to-Beta gates and work program"
```

Do not push while mandatory hard gates are failing unless the human architect
explicitly authorizes the documented exception.
