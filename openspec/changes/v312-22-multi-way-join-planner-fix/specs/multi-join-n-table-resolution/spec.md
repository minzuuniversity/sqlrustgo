# multi-join-n-table-resolution Specification

## Purpose

Extend the multi-join column resolution spec to handle N-table joins (4-7+)
without silent `None` returns from `try_comma_join_hash_chain`. The previous
spec only covered the 3-table case; this delta adds scenarios for star-schema
joins like TPC-H Q7/Q9 (where one table is a hub with multiple spokes).

## Requirements

### Requirement: N-table JOIN chain construction is sound

The system SHALL construct a complete chain through all tables referenced by a
multi-way JOIN (comma-join + WHERE-extracted equi-join) whenever the join
graph is connected.

#### Scenario: 6-table chain with hub-and-spoke topology

- **WHEN** executing `SELECT ... FROM a, b, c, d, e, f WHERE a.id=b.a_id AND c.id=b.c_id AND d.id=c.d_id AND e.id=d.e_id AND f.id=e.f_id`
- **AND** the join graph is connected via equality predicates
- **THEN** `try_comma_join_hash_chain` SHALL produce a chain covering all 6 tables
- **AND** result rows SHALL match the expected count (not 0)

#### Scenario: 7-table TPC-H Q9 chain

- **WHEN** executing TPC-H Q9 (`n, p, s, ps, o, l` joined via `supplier.nationkey=nation.nationkey`, `lineitem.suppkey=supplier.suppkey`, `lineitem.partkey=part.partkey`, `partsupp.partkey=part.partkey`, `partsupp.suppkey=supplier.suppkey`, `lineitem.orderkey=orders.orderkey`)
- **THEN** the chain SHALL cover all 6 tables (note Q9 has 6 distinct tables, not 7)
- **AND** result rows SHALL be non-zero on SF=1

#### Scenario: Greedy cannot get stuck

- **WHEN** the greedy chain builder would have terminated early (chain_order.len() < join_tables.len()) on previous code
- **THEN** the new DFS-based chain builder SHALL backtrack and find an alternative ordering
- **AND** SHALL NOT return `None` for any connected join graph

#### Scenario: Disconnected graph is reported honestly

- **WHEN** a join references tables that have no equality predicate connecting them
- **THEN** the planner SHALL return `None` and emit a diagnostic naming the unreachable tables
- **AND** SHALL NOT panic or return an incomplete chain silently

### Requirement: Multi-join fix does not regress 2-table / 3-table joins

The system SHALL preserve all existing 2-table and 3-table JOIN behavior
(no performance regression, no semantic change for unambiguous N≤3 joins).

#### Scenario: 3-table inner join still returns correct rows

- **WHEN** 3-table JOIN with unambiguous columns
- **THEN** result SHALL be identical to pre-fix output

#### Scenario: 2-table inner join still returns correct rows

- **WHEN** 2-table JOIN with unambiguous columns
- **THEN** result SHALL be identical to pre-fix output