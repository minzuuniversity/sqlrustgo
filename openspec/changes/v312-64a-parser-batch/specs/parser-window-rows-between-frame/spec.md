# parser-window-rows-between-frame

## Purpose

Window functions `SUM|COUNT|AVG(expr) OVER (ORDER BY col [ROWS|RANGE BETWEEN ...])` MUST support frame definitions. The minimum implementation MUST handle `ROWS BETWEEN a PRECEDING AND b FOLLOWING` plus moving sum/count/avg.

## ADDED Requirements

### Requirement: parser must accept ROWS BETWEEN frame syntax

The parser MUST accept an optional frame definition after the ORDER BY clause inside OVER: `ROWS|RANGE BETWEEN a PRECEDING AND b FOLLOWING`, `ROWS|RANGE [a] PRECEDING`, `ROWS|RANGE UNBOUNDED PRECEDING`.

#### Scenario: ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING

GIVEN sql `SELECT SUM(val) OVER (ORDER BY id ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) FROM t`
WHEN parser parses OVER clause
THEN frame equals Some(WindowFrame { kind: Rows, start: Preceding(1), end: Following(1) }); no Parse error.

#### Scenario: ROWS UNBOUNDED PRECEDING

GIVEN sql `SELECT SUM(val) OVER (ORDER BY id ROWS UNBOUNDED PRECEDING)`
WHEN parser parses
THEN frame equals Some(WindowFrame { kind: Rows, start: UnboundedPreceding, end: None }).

### Requirement: moving sum must be correct

ROWS BETWEEN a PRECEDING AND b FOLLOWING MUST compute the correct moving sum/count/avg per output row.

#### Scenario: moving sum expected values

GIVEN table t with rows (id=1,val=10),(2,20),(3,30),(4,40),(5,50)
WHEN `SELECT SUM(val) OVER (ORDER BY id ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) FROM t`
THEN output is (1→30),(2→60),(3→90),(4→120),(5→90).

### Requirement: no-frame default is RANGE UNBOUNDED PRECEDING (regression)

The existing default running-total behaviour MUST be preserved when no frame is specified.

#### Scenario: running total

GIVEN `SELECT SUM(val) OVER (ORDER BY id)`
WHEN executed
THEN output is the running totals (10,30,60,100,150).

## Acceptance Criteria

- WindowSpecification gains a `frame: Option<WindowFrame>` field
- WindowFrame / FrameBound / BoundKind structures are added
- parse_over_clause consumes the optional frame
- executor implements moving sum/count/avg
- parser unit test `parse_window_rows_between` PASS
- integration test `window_rows_between_sum_moving` PASS (CLI)