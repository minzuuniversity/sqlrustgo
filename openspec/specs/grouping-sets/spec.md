# grouping-sets Specification

## Purpose
TBD - created by archiving change fix-v313-97-4679-grouping-sets. Update Purpose after archive.
## Requirements
### Requirement: GROUP BY accepts GROUPING SETS((...),(...),...) clause

The SQL parser SHALL accept `GROUP BY GROUPING SETS((expr[, ...]), (expr[, ...]), ...)`
in place of the standard group-by expression list. Each inner `Vec<Expression>`
records one grouping set; an empty set `()` is the SQL:1999 grand-total
shorthand. Set columns are unioned (deduped) into the SelectStatement's
`group_by` field so the main aggregation pass groups by every column that
appears in any set. The original set list is preserved in
`SelectStatement.grouping_sets` for downstream fan-out.

#### Scenario: GROUPING SETS with multiple sets and grand total

Given `SELECT grp, SUM(val) FROM t GROUP BY GROUPING SETS((grp), ())`
on a table with rows `(1,10)`, `(1,20)`, `(2,30)`, the parser succeeds
and the resulting SelectStatement has `group_by = [grp]` (union of sets)
and `grouping_sets = [[grp], []]`.

#### Scenario: GROUPING SETS with single set

Given `SELECT grp, SUM(val) FROM t GROUP BY GROUPING SETS((grp))`, the
parser succeeds and `group_by = [grp]`, `grouping_sets = [[grp]]`.

#### Scenario: GROUPING SETS with only the grand-total set

Given `SELECT SUM(val) FROM t GROUP BY GROUPING SETS(())`, the parser
succeeds and `grouping_sets = [[]]`. `group_by` is empty (the empty set
contributes no columns).

#### Scenario: GROUPING SETS with two columns

Given `SELECT a, b, SUM(v) FROM t GROUP BY GROUPING SETS((a, b), (a),
(b), ())`, the parser succeeds with `grouping_sets` containing four
sets and `group_by` containing the union `[a, b]`.

#### Scenario: GROUPING SETS without SETS keyword after GROUPING errors

Given `SELECT ... GROUP BY GROUPING (...)`, the parser rejects with
`Expected SETS after GROUPING`.

### Requirement: Executor MUST fan out union-group rows to one row per set

For each `group_by` row produced by the main aggregation pass, the
executor MUST emit one row per set in `grouping_sets`. The set's columns
MUST appear in their original `group_by` positions; missing columns
MUST be padded with `Value::Null`. The aggregate tail MUST be identical
across all rows fanned from the same group. Empty sets (`()`) MUST emit
one grand-total row whose aggregates MUST recompute over the entire
input row set.

#### Scenario: GROUPING SETS((grp), ()) on two-group input

Given rows `(1,10), (1,20), (2,30)` and
`SELECT grp, SUM(val) FROM t GROUP BY GROUPING SETS((grp), ())`, the
result is `(1,30)`, `(2,30)`, `(NULL,60)`.

#### Scenario: GROUPING SETS((grp)) on two-group input

Given rows `(1,10), (2,20)` and
`SELECT grp, SUM(val) FROM t GROUP BY GROUPING SETS((grp))`, the
result is `(1,10)`, `(2,20)`.

#### Scenario: GROUPING SETS(()) alone

Given any aggregate query with `GROUP BY GROUPING SETS(())`, the result
is a single grand-total row whose aggregates are computed over the
entire input.

