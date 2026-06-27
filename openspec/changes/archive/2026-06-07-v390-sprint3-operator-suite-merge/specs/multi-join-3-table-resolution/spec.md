## ADDED Requirements

### Requirement: Multi-table JOIN column resolution via qualifier
The system SHALL resolve column references in JOIN ON clauses using the table qualifier when present, falling back to bare-name lookup only when no qualifier is given.

#### Scenario: 3-table chain with mixed qualifiers
- **WHEN** executing `SELECT a.x, b.y, c.z FROM a, b, c WHERE a.id = b.a_id AND b.id = c.b_id`
- **THEN** the second ON clause SHALL resolve `b.id` to the `b.id` column in the accumulated join (not `a.id`)
- **AND** result rows SHALL match the expected count (not 0)

#### Scenario: Qualified lookup preferred over bare lookup
- **WHEN** accumulated info contains BOTH `a.id` and `b.id` (both named `id`)
- **AND** JOIN references `b.id`
- **THEN** system SHALL use the qualified `b.id`, not the first-found `a.id`

#### Scenario: Bare lookup still works for 1-table queries
- **WHEN** single-table query references `id` without qualifier
- **THEN** system SHALL resolve to the unique `id` column (no ambiguity)

#### Scenario: Accumulated form `*b.id` also resolves
- **WHEN** column metadata uses the `*b.id` accumulated form
- **THEN** system SHALL treat it as `b.id` for lookup

### Requirement: Multi-join fix does not regress 2-table joins
The system SHALL preserve all existing 2-table JOIN behavior (no performance regression, no semantic change for unambiguous 2-table joins).

#### Scenario: 2-table inner join still returns correct rows
- **WHEN** 2-table JOIN with unambiguous columns
- **THEN** result SHALL be identical to pre-fix output
