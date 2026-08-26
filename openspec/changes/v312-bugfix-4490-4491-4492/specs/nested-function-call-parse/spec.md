## ADDED Requirements

### Requirement: single-argument nested function call parses successfully
The general `Identifier(args)` function-call parsing path in `crates/parser/src/parser.rs` MUST correctly parse expressions where a single function-call argument is itself a function call.

For example, `SELECT year(now())`, `SELECT foo(bar())`, and `SELECT foo(x, bar())` MUST all parse without error.

#### Scenario: nested function as only argument
- **WHEN** parsing `SELECT year(now())`
- **THEN** parsing succeeds and the AST contains `FunctionCall("YEAR", [FunctionCall("NOW", [])])`

#### Scenario: nested function as first of multiple arguments
- **WHEN** parsing `SELECT foo(bar(), x)`
- **THEN** parsing succeeds and the AST contains `FunctionCall("FOO", [FunctionCall("BAR", []), Identifier("x")])`

#### Scenario: nested function as last of multiple arguments
- **WHEN** parsing `SELECT foo(x, bar())`
- **THEN** parsing succeeds and the AST contains `FunctionCall("FOO", [Identifier("x"), FunctionCall("BAR", [])])`

#### Scenario: zero-argument outer call still parses
- **WHEN** parsing `SELECT foo()`
- **THEN** parsing succeeds (existing behavior preserved)

#### Scenario: non-nested single argument still parses
- **WHEN** parsing `SELECT foo(x)` or `SELECT foo(1)`
- **THEN** parsing succeeds (existing behavior preserved)

### Requirement: regression — CAST(... AS TYPE) parsing preserved
The fix to the general `Identifier(args)` path MUST NOT break the CAST special form (`CAST(expr AS TYPE)`). Both `CAST(1 AS INTEGER)` and `CAST(SUBSTR(x, 1, 4) AS INTEGER)` MUST continue to parse correctly.

#### Scenario: plain CAST
- **WHEN** parsing `SELECT CAST(1 AS INTEGER)`
- **THEN** parsing succeeds

#### Scenario: CAST with nested expression
- **WHEN** parsing `SELECT CAST(SUBSTR(x, 1, 4) AS INTEGER)`
- **THEN** parsing succeeds