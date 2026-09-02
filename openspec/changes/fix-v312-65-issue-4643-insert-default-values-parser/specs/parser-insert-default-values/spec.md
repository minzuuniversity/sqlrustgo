## ADDED Requirements

### Requirement: Parser MUST accept `INSERT INTO <table> DEFAULT VALUES`

The parser MUST accept the standard `INSERT INTO <table> DEFAULT VALUES` syntax and emit `Statement::Insert(InsertStatement { default_values: true, values: [], select: None, .. })`. The executor MUST insert exactly one row where each column is populated from its declared `DEFAULT` expression (or `NULL` when the column has no default).

#### Scenario: basic DEFAULT VALUES inserts one row with column defaults

- **WHEN** the executor runs:
  1. `CREATE TABLE t(id INT, val INT DEFAULT 100, name VARCHAR(20) DEFAULT 'hello')`
  2. `INSERT INTO t DEFAULT VALUES`
  3. `SELECT * FROM t`
- **THEN** the SELECT MUST return exactly one row with `id = NULL`, `val = 100`, `name = 'hello'`

#### Scenario: DEFAULT VALUES on a table without any DEFAULT clauses

- **WHEN** the executor runs `CREATE TABLE t(a INT, b TEXT); INSERT INTO t DEFAULT VALUES; SELECT * FROM t`
- **THEN** the SELECT MUST return exactly one row with `a = NULL`, `b = NULL`

#### Scenario: DEFAULT VALUES is case-insensitive

- **WHEN** the parser is given `insert into t default values`
- **THEN** the parser MUST accept it and emit `Statement::Insert` with `default_values: true`

### Requirement: Parser MUST reject malformed `INSERT ... DEFAULT` (missing VALUES)

The parser MUST still surface a clear parse error if the user writes `INSERT INTO t DEFAULT` without the trailing `VALUES` keyword.

#### Scenario: `DEFAULT` without trailing `VALUES`

- **WHEN** the parser is given `INSERT INTO t DEFAULT`
- **THEN** `parse(...)` MUST return `Err` containing `"VALUES"`

### Requirement: Executor MUST surface a clear runtime error on other binder failures

`INSERT ... DEFAULT VALUES` MUST NOT silently insert a row with all NULLs when the binder detects a real problem (e.g. the table does not exist). The binder error path is unchanged from `INSERT VALUES` — the only new behaviour is the no-row-into-one-row expansion in `execute_insert`.

#### Scenario: DEFAULT VALUES on a non-existent table

- **WHEN** the executor runs `INSERT INTO nonexistent DEFAULT VALUES`
- **THEN** the executor MUST return `Err` with a binder/storage error indicating the table does not exist
