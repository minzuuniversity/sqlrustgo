# SQLRustGo

> **当前开发线**: `develop/v3.12.0`
> **当前证据快照**: `542192641` on 252 Gitea, refreshed 2026-09-08T04:33:08+08:00
> **发布状态**: v3.12.0 remains **RC / GA preparation**. Do not claim GA until the release gate evidence is refreshed at final HEAD.
> **可信状态入口**: [docs/releases/v3.12.0/STAGE.yaml](docs/releases/v3.12.0/STAGE.yaml)

SQLRustGo is a Rust SQL database project with a MySQL-style server, embedded
SQL execution paths, storage/transaction components, and release-stage
governance for evidence-based quality gates.

The current v3.12.0 line is scoped as a controlled database foundation for GMP
internal-audit retrieval workloads: relational storage, document chunks,
audit/evidence records, internal vector retrieval, and SQL-backed graph
projection. It is not a general-purpose MySQL replacement, standalone vector
database, or standalone graph database.

## Current Status

| Area | Status | Evidence boundary |
|---|---|---|
| Canonical remote | 252 Gitea | `origin/develop/v3.12.0` fetched and fast-forwarded to `542192641` on 2026-09-08. |
| v3.12.0 stage | RC | `STAGE.yaml` still records `current_stage: "RC"`. |
| GA gate | Not clean | Checked-in `evidence/v312-59/ga_gate_report.json` is `mode=full` but `verdict=FAIL`, `totals.blockers=9`, at stale commit `65ef5bea`. |
| Open Gitea issues | 3 open | #4846 CHAR(n) comparison, #4847 transaction semantics, #4848 `ALTER TABLE ... RENAME COLUMN`. |
| Product claim | Controlled GMP/internal-audit workload | Broader SQL/MySQL/SQLite teaching compatibility must remain bounded until open issues and RC-GA gates close. |

The root README is intentionally concise. Release evidence, issue triage, and
gate details live under [docs/releases/v3.12.0](docs/releases/v3.12.0/README.md).

## Quick Start

```bash
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo

cargo build --all-features
cargo test --all-features

cargo run --bin sqlrustgo-mysql-server -- serve --host 127.0.0.1 --port 3307
```

For the sqlite-like teaching CLI:

```bash
cargo run --bin sqlrustgo -- sqlite --batch path/to/database.sqlrg < script.sql
```

## Architecture

```text
SQL text / MySQL wire / sqlite-like CLI
        |
Parser -> Planner -> Optimizer -> Executor
        |
Catalog / Storage / Transaction / WAL / MVCC
        |
GMP documents / chunks / audit trail / relations / embeddings
        |
Hybrid retrieval / internal vector retrieval / SQL-backed graph projection
```

Workspace crates are under [crates](crates/). Integration and compatibility
tests are under [tests](tests/).

## Release Discipline

SQLRustGo release documents follow evidence-first governance:

- A document claim is not execution evidence.
- PASS/GA claims require the actual command output, commit, timestamp, and
  evidence artifact.
- Open issues can be scoped out only with explicit release-claim downgrade.
- Issue closure requires merged PR evidence and relevant verification.

Core policies:

- [ADR-001 Truthfulness Framework](docs/governance/adr/ADR-001-truthfulness-framework.md)
- [Anti-Fabrication Policy](docs/governance/ANTI_FABRICATION_POLICY.md)
- [Gate Conditions](docs/governance/GATE_CONDITIONS.md)
- [Issue Closing Verification](docs/governance/ISSUE_CLOSING_VERIFICATION.md)
- [Document Correction Rules](docs/governance/DOC_CHECK_CORRECTION_RULES.md)

## Development Checks

Run focused checks for the code you changed, then run the broader gates required
by the target release stage.

```bash
cargo fmt --check --all
cargo clippy --all-features -- -D warnings
cargo test --all-features

bash scripts/gate/check_docs_links.sh
bash scripts/gate/check_docs_links_v312.sh
bash scripts/gate/check_docs_consistency_v312.sh
```

For v3.12.0 GA readiness, the final aggregate must be regenerated at the final
release commit:

```bash
bash scripts/gate/check_ga_v3.12.0.sh --full
```

Do not use old gate reports or historical issue counts as a substitute for a
fresh GA decision.

## Key Documents

| Document | Purpose |
|---|---|
| [v3.12.0 README](docs/releases/v3.12.0/README.md) | Current v3.12.0 scope, blockers, and GA preparation status. |
| [v3.12.0 STAGE](docs/releases/v3.12.0/STAGE.yaml) | Stage SSOT and promotion requirements. |
| [v3.12.0 GA Gate Report](docs/releases/v3.12.0/GA_GATE_REPORT.md) | GA verdict map and evidence boundaries. |
| [v3.12.0 Release Checklist](docs/releases/v3.12.0/RELEASE_CHECKLIST.md) | Final RC-to-GA action checklist. |
| [RC-GA Triage Plan](docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md) | Issue classification and new RC-GA gate plan. |
| [Claim Downgrade Manifest](docs/releases/v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md) | Release-claim exclusions and closure ledger. |
| [v3.12.0 Test Plan](docs/releases/v3.12.0/TEST_PLAN.md) | Test layers, gates, and evidence requirements. |
| [CHANGELOG](CHANGELOG.md) | Historical version changes. |

## Version Positioning

| Version | Stage / role | Boundary |
|---|---|---|
| v3.12.0 | RC / GA preparation | GMP internal-audit retrieval database; GA blocked until current gates and open issues are resolved or explicitly scoped. |
| v3.11.0 | GA | Controlled/simple production candidate; not a full MySQL 5.7 replacement. |
| v3.10.0 and earlier | Historical | Useful for tracing feature evolution, not current release evidence. |

## License

MIT License. See [LICENSE](LICENSE).
