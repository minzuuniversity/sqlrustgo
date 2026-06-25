## ADDED Requirements

### Requirement: TPC-H 22-query root cause board
The system SHALL maintain a documented classification of all 22 TPC-H queries by root cause category, enabling systematic fix-by-operator methodology.

#### Scenario: Five root cause categories defined
- **WHEN** reading `docs/audit/status/2026-06-07-tpch-root-cause-board-v390.md`
- **THEN** the board SHALL classify all 22 queries into exactly 5 buckets:
  - **Aggregate**: Q01, Q05, Q07, Q08, Q17
  - **Multi-Join**: Q03, Q10, Q18
  - **EXISTS**: Q20, Q21
  - **Date Filter**: Q06, Q19
  - **Fixture (data)**: Q14

#### Scenario: Each query has a fix track assignment
- **WHEN** a query is classified
- **THEN** it SHALL be assigned to a Sprint (Sprint 1-6) with an owner and completion target

#### Scenario: Board updates on each Sprint completion
- **WHEN** a Sprint closes a query's bug
- **THEN** the board SHALL reflect status (OPEN/IN_PROGRESS/CLOSED) with PR reference
