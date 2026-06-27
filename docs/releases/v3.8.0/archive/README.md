# Retired Binary Archive (v3.8.0+)

> **Status**: 2026-06-04 — Retired as part of the [mysql-server-canonical-entry](../SPEC-v3.8.0-001-mysql-server-canonical-entry.md) change.

This directory contains a brief index of the legacy binaries that were retired
in v3.8.0+, replaced by the canonical entry point
**`sqlrustgo-mysql-server`** and its subcommands.

## Retired Binaries

| Retired Binary | Replacement (mysql-server subcommand) | Removal Date |
|----------------|---------------------------------------|--------------|
| `sqlrustgo` (root stub) | `serve` (default) or `exec "<sql>"` | 2026-06-04 |
| `sqlrustgo-sql-cli` (REPL) | `repl` | 2026-06-04 |
| `sqlrustgo-bench` | `bench` (placeholder; full TPC-H / OLTP runner is in `crates/bench/examples/`) | 2026-06-04 |
| `sqlrustgo-bench-cli` | `bench tpc-h\|oltp\|custom` (placeholder) | 2026-06-04 |
| `sqlrustgo-gmp-cli` | `gmp retrieve\|index\|status` (placeholder; lib preserved in `crates/gmp`) | 2026-06-04 |
| `sqlrustgo-tools` | `diag doctor\|info\|stats` (placeholder; lib preserved in `crates/tools` for `backup` / `restore`) | 2026-06-04 |

## Why Archived (Not Just Deleted)

These binaries were internal-only — never published as a public API — and
were fully removed from the workspace tree. This archive index exists so that
historical doc references (e.g. CI pipelines, third-party integrations pointing
at v3.7.0-era commands) can be mapped to the new canonical entry without
requiring a `git log` archaeology session.

## Out of Scope (NOT Retired)

- **`tools/sqlrustgo-gate/`** and **`tools/graph-cli/`** — pre-existing
  graph-tool binaries (`graph-gate`, `graph-ingest`, `sqlrustgo-gate`,
  `gate`, `ingest`). They are orthogonal to the SQL server entry and
  serve the graph ingestion / RAG pipeline, not user-facing SQL execution.
  Tracked separately.

## See Also

- [SPEC-v3.8.0-001-mysql-server-canonical-entry.md](../SPEC-v3.8.0-001-mysql-server-canonical-entry.md) — full design
- [openspec/changes/mysql-server-canonical-entry/](../openspec/changes/mysql-server-canonical-entry/) — the source change
- [CHANGELOG.md](../../../../CHANGELOG.md) `## [Unreleased]` — breaking change entry
