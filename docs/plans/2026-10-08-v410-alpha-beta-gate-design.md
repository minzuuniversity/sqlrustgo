# v4.1.0 Alpha-to-Beta Gate Design

> **Date**: 2026-10-08
> **Status**: APPROVED by user
> **Scope**: Documentation, gate definition, and Gitea issue decomposition only
> **Assessed base**: `gitea252/develop/v4.1.0` at `5fdea59f3420ec7713d671bdbd6edc1531f2a122`

## 1. Goal

Turn the v4.1.0 assessment into an executable Alpha-to-Beta program with
machine-verifiable gates, independent issue closure criteria, and one release
documentation source of truth for tests and acceptance.

This work does not claim that any gate currently passes. It defines what must
pass before promotion and how the evidence must be produced.

## 2. Structure

The program uses one master issue and five independently verifiable child
issues:

1. Per-connection transaction and session correctness.
2. Fail-closed CI and gate integrity.
3. Multi-connection, BustubX-EDU, and model-invariant regression coverage.
4. SOAK recovery, fault injection, and duration ladder.
5. Performance baseline and automatic regression gate.

Existing issues `#5099`, `#5057`, `#5025`, `#5102`, and `#5103` remain the
authoritative implementation dependencies. New issues must link to them rather
than duplicate their scope.

## 3. Documentation Ownership

| Document | Ownership |
|---|---|
| `TEST_PLAN.md` | Test layers, commands, thresholds, evidence schema |
| `DEV_PLAN.md` | Execution order, dependency graph, stage ownership |
| `ALPHA_TO_BETA_GATE_PLAN.md` | Hard-gate review procedure and acceptance rules |
| `ISSUES_PLAN.md` | Issue IDs, dependency links, and closure conditions |

`STAGE.yaml` remains the stage SSOT. This change does not edit its current
stage or claim a promotion. A later promotion PR must update it only after the
hard gates produce fresh evidence.

## 4. Gate Model

Every Alpha-to-Beta gate is fail-closed and must provide:

- exact commit SHA and branch;
- exact command and exit code;
- test counts including ignored and filtered tests;
- machine-readable summary;
- timestamp, source agent, source run, and evidence hash;
- an artifact path committed under the v4.1.0 evidence directory or a durable
  CI artifact URL.

An absent artifact, skipped required test, stale SHA, workflow parse failure,
or missing command result is a FAIL. Textual review is never a substitute for
execution.

## 5. Stage Boundary

Alpha owns correctness and gate trustworthiness. Beta owns broader workload,
stability, and performance qualification. RC/GA retain the 168-hour SOAK and
final release audit.

The Alpha-to-Beta promotion is blocked while any of the following is true:

- a P0 data-loss or transaction-isolation issue is open;
- the concurrent transaction invariant suite reproduces data loss;
- format, clippy, build, or required CI workflows fail;
- test registry and source-tree ignore counts differ;
- required gate tests are ignored, filtered, or not executed;
- the SOAK state is undocumented or falsely described;
- coverage, SQL corpus, or oracle evidence is missing or stale.

## 6. Issue Closure Contract

Each child issue closes only through a merged PR. The PR must contain or link
the required tests, fresh execution evidence, and the target branch commit.
The master issue closes only after every child issue is closed and a final
Alpha-to-Beta audit confirms all hard gates against the same commit.

## 7. Non-Goals

- No feature implementation in this documentation PR.
- No stage promotion or release tag.
- No claim that the current CI, SOAK, coverage, or performance gate passes.
- No duplicate issue for work already tracked by `#5099`, `#5057`, `#5025`,
  `#5102`, or `#5103`.
