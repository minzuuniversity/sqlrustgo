# parser-like-escape-multi-char Specification

## Purpose
TBD - created by archiving change fix-v312-70-issue-4673-like-escape-multi-char. Update Purpose after archive.
## Requirements
### Requirement: Parser MUST accept any string literal after ESCAPE (use first character)

The parser MUST accept `LIKE pattern ESCAPE 'string'` where the escape string is a string literal of any length. The first character of the string is used as the escape character (matching MySQL/PostgreSQL semantics). A non-string-literal token after `ESCAPE` MUST surface a clear parse error.

#### Scenario: escape with 2-character string (backslashes)

- **WHEN** the parser is given `SELECT * FROM t WHERE name LIKE 'a\\%bc' ESCAPE '\\'` (SQL source after shell escape: `ESCAPE '\\'`)
- **THEN** `parse(...)` returns `Ok(...)` and the `Like` expression's escape field is `Some('\\')` (a single backslash)

#### Scenario: escape with 1-character string

- **WHEN** the parser is given `SELECT * FROM t WHERE name LIKE 'a\\%bc' ESCAPE 'x'`
- **THEN** `parse(...)` returns `Ok(...)` and the `Like` expression's escape field is `Some('x')`

#### Scenario: not-like with escape

- **WHEN** the parser is given `SELECT * FROM t WHERE name NOT LIKE 'a\\%bc' ESCAPE '\\'`
- **THEN** `parse(...)` returns `Ok(...)` and the `NotLike` expression's escape field is `Some('\\')`

#### Scenario: non-string-literal after ESCAPE

- **WHEN** the parser is given `SELECT * FROM t WHERE name LIKE 'a%bc' ESCAPE 42` (an integer, not a string)
- **THEN** `parse(...)` returns `Err` with a message mentioning "ESCAPE"

