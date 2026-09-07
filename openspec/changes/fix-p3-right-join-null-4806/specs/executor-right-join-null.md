## ADDED Requirements

### Requirement: RIGHT JOIN unmatched row produces NULL

`RIGHT JOIN` MUST fill unmapped left-side columns with `NULL` (not an
empty string) for rows where the right-side table has no matching
left-side row.

#### Scenario: Basic unmatched RIGHT JOIN

- GIVEN:
  ```
  CREATE TABLE l (id INT, val TEXT);
  INSERT INTO l VALUES (1, 'a');
  CREATE TABLE r (id INT, val TEXT);
  INSERT INTO r VALUES (2, 'b');
  ```
- WHEN `SELECT l.*, r.* FROM l RIGHT JOIN r ON l.id = r.id`
- THEN the result has one row: `(NULL, NULL, 2, 'b')`.
- AND the column types are `(Integer, Text, Integer, Text)`.
- AND the NULL values are actual SQL NULL (verified via `IS NULL`).

### Requirement: NULL distinguished from empty string

The NULL values produced by unmatched JOIN rows MUST be distinguishable
from empty strings. `IS NULL` MUST return true for these values, and
`=` comparison with `''` MUST return false.

#### Scenario: IS NULL check

- GIVEN the same setup as above
- WHEN:
  ```
  SELECT l.id IS NULL AS is_null, l.id = '' AS is_empty
  FROM l RIGHT JOIN r ON l.id = r.id
  ```
- THEN the result is `(true, false)`.

#### Scenario: COALESCE uses NULL

- WHEN `SELECT COALESCE(l.id, 999) FROM l RIGHT JOIN r ON l.id = r.id`
- THEN the result is `999` (because l.id IS NULL triggers the
  COALESCE fallback).

### Requirement: COUNT ignores JOIN NULL values

`COUNT(col)` MUST skip rows where `col` is NULL (the result of an
unmatched JOIN row), matching standard SQL semantics.

#### Scenario: COUNT after RIGHT JOIN

- GIVEN the same setup as above (1 unmatched row)
- WHEN `SELECT COUNT(l.id) FROM l RIGHT JOIN r ON l.id = r.id`
- THEN the result is `0` (the unmatched row contributes NULL to l.id,
  which COUNT skips).

### Requirement: FULL OUTER JOIN same NULL behavior

`FULL OUTER JOIN` MUST use the same NULL semantics for both unmatched
left and unmatched right rows.

#### Scenario: FULL OUTER with unmatched on both sides

- GIVEN:
  ```
  l: (1, 'a')
  r: (2, 'b')
  ```
- WHEN `SELECT l.id, r.id FROM l FULL OUTER JOIN r ON l.id = r.id`
- THEN two rows are returned:
  - `(1, NULL)` — left-only row, r.id = NULL
  - `(NULL, 2)` — right-only row, l.id = NULL

### Requirement: Column types preserved with NULL value

A column with NULL value MUST retain its declared column type.

#### Scenario: Type preservation

- GIVEN the same setup
- WHEN:
  ```
  SELECT typeof(l.id) FROM l RIGHT JOIN r ON l.id = r.id
  ```
- THEN the result is `'integer'` (the declared type), not `'text'` or
  `'null'`.