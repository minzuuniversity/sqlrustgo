## ADDED Requirements

### Requirement: NTILE balances bucket sizes

`NTILE(n_buckets) OVER (...)` MUST distribute rows in a partition across
`n_buckets` buckets such that the bucket sizes differ by at most 1. The
larger buckets (if any) MUST appear first.

#### Scenario: Balanced distribution

- GIVEN 10 rows partitioned by NTILE(4) in some ordering
- THEN the bucket sequence is `(1,1,1,2,2,2,3,3,4,4)` — 2 buckets of
  size 3 followed by 2 buckets of size 2.

#### Scenario: Exact division

- GIVEN 8 rows partitioned by NTILE(4)
- THEN the bucket sequence is `(1,1,2,2,3,3,4,4)` — all buckets equal.

### Requirement: NTILE handles edge cases

#### Scenario: More buckets than rows

- GIVEN 3 rows and NTILE(5)
- THEN bucket sequence is `(1,2,3)` (3 buckets of size 1, remaining
  buckets are empty).

#### Scenario: Single bucket

- GIVEN 5 rows and NTILE(1)
- THEN all rows have bucket = 1.

#### Scenario: Zero buckets is an error

- WHEN `NTILE(0) OVER (...)` is invoked
- THEN the statement fails with `InvalidArgument: NTILE requires
  positive bucket count`.

#### Scenario: Negative buckets is an error

- WHEN `NTILE(-1) OVER (...)` is invoked
- THEN the statement fails with the same error.

### Requirement: NTILE respects PARTITION BY

`NTILE OVER (PARTITION BY col ORDER BY col2)` MUST compute bucket
assignments independently within each partition.

#### Scenario: Two partitions each with NTILE(2)

- GIVEN 4 rows of cat='A' and 4 rows of cat='B'
- WHEN `NTILE(2) OVER (PARTITION BY cat ORDER BY id)` is invoked
- THEN within cat='A', bucket sequence is `(1,1,2,2)`.
- AND within cat='B', bucket sequence is `(1,1,2,2)`.
- AND buckets are NOT shared across partitions.

### Requirement: NTILE ties remain in same bucket

When ORDER BY produces ties, all tied rows MUST be assigned to the same
bucket (no row is split across buckets).

#### Scenario: 6 rows, ORDER BY has 3-way tie at id=1

- GIVEN 6 rows: (1, ...), (1, ...), (1, ...), (2, ...), (3, ...), (4, ...)
  in ORDER BY id
- WHEN NTILE(2) is invoked
- THEN the 3 rows with id=1 are all in the same bucket (e.g., bucket 1).