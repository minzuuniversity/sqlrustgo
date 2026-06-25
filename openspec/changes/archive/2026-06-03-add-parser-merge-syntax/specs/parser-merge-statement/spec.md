# Spec: parser-merge-statement

## ADDED Requirements

### Requirement: Parser SHALL Recognize MERGE Keyword as Token

The lexer at `crates/parser/src/lexer.rs` MUST recognize the case-insensitive keyword `MERGE` and produce `Token::Merge`. The token MUST be defined in `crates/parser/src/token.rs` alongside other DML keywords (Insert/Update/Delete).

#### Scenario: Lexer tokenizes MERGE keyword
- **WHEN** the lexer processes the SQL `MERGE INTO target USING source ON target.id = source.id`
- **THEN** the first token MUST be `Token::Merge`
- **AND** the subsequent tokens MUST be `Token::Into`, identifier `target`, `Token::Using`, identifier `source`, `Token::On`, ...

#### Scenario: Lexer tokenizes lowercase merge keyword
- **WHEN** the lexer processes the SQL `merge into t using s on t.id = s.id`
- **THEN** the first token MUST be `Token::Merge` (case-insensitive matching)

#### Scenario: Lexer tokenizes mixed-case Merge keyword
- **WHEN** the lexer processes the SQL `Merge Into t Using s On t.id = s.id`
- **THEN** the first token MUST be `Token::Merge`

### Requirement: Parser SHALL Parse Standard SQL MERGE Statement

The parser at `crates/parser/src/parser.rs` MUST parse a complete `MERGE INTO target [AS t_alias] USING source [AS s_alias] ON <expr> [WHEN MATCHED [AND <expr>] THEN UPDATE SET ...] [WHEN NOT MATCHED [AND <expr>] THEN INSERT (...) VALUES (...)]` statement and produce a `Statement::Merge(MergeStatement)` AST node.

#### Scenario: Parse minimal MERGE with single WHEN MATCHED UPDATE
- **WHEN** the parser processes `MERGE INTO target t USING source s ON t.id = s.id WHEN MATCHED THEN UPDATE SET t.val = s.val`
- **THEN** the parser MUST return `Ok(Statement::Merge(MergeStatement{...}))`
- **AND** the AST MUST contain target_table=`"target"`, target_alias=`Some("t")`, source=Table{name:`"source"`}, source_alias=`Some("s")`, on_condition set to a binary expression comparing `t.id` and `s.id`
- **AND** `when_clauses` MUST contain exactly one `MergeWhenClause` with `is_matched=true` and `action=Update{set_clauses: [("t.val", <s.val>)]}`

#### Scenario: Parse MERGE with both WHEN MATCHED and WHEN NOT MATCHED
- **WHEN** the parser processes a MERGE statement with both `WHEN MATCHED THEN UPDATE` and `WHEN NOT MATCHED THEN INSERT`
- **THEN** `when_clauses` MUST contain 2 elements in order
- **AND** the first MUST be `is_matched=true` with Update action
- **AND** the second MUST be `is_matched=false` with Insert action

#### Scenario: Parse MERGE with WHEN NOT MATCHED BY TARGET
- **WHEN** the parser processes `MERGE INTO t USING s ON t.id = s.id WHEN NOT MATCHED THEN INSERT (id, val) VALUES (s.id, s.val)`
- **THEN** the parser MUST return `Statement::Merge(...)`
- **AND** the Insert action MUST contain columns `["id", "val"]` and values matching `s.id` and `s.val`

#### Scenario: Parse MERGE with optional AS aliases
- **WHEN** the parser processes `MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN UPDATE SET target.val = source.val`
- **THEN** the AST MUST have `target_alias=None` and `source_alias=None`

#### Scenario: Parse MERGE with subquery source
- **WHEN** the parser processes `MERGE INTO target t USING (SELECT id, val FROM other) s ON t.id = s.id WHEN NOT MATCHED THEN INSERT (id, val) VALUES (s.id, s.val)`
- **THEN** `source` MUST be `MergeSource::Subquery(Box<SelectStatement>)`
- **AND** `source_alias` MUST be `Some("s")`

#### Scenario: Reject MERGE without USING
- **WHEN** the parser processes `MERGE INTO target WHEN MATCHED THEN UPDATE SET target.val = 1`
- **THEN** the parser MUST return an error containing `"Expected USING"` or `"Unexpected token"`

#### Scenario: Reject MERGE without ON
- **WHEN** the parser processes `MERGE INTO target USING source WHEN MATCHED THEN UPDATE SET target.val = 1`
- **THEN** the parser MUST return an error containing `"Expected ON"` or `"Unexpected token"`

### Requirement: Statement Enum MUST Include Merge Variant

The `Statement` enum at `crates/parser/src/parser.rs` MUST include a `Merge(MergeStatement)` variant. The `lib.rs` MUST re-export `MergeStatement`, `MergeSource`, `MergeWhenClause`, and `MergeAction` for downstream crates (executor, planner).

#### Scenario: Statement::Merge is constructable
- **WHEN** downstream code constructs `Statement::Merge(MergeStatement { ... })`
- **THEN** the type MUST compile without errors
- **AND** downstream code MAY import the type via `sqlrustgo_parser::MergeStatement`

#### Scenario: parse_statement dispatches Merge to parse_merge
- **WHEN** the parser's `parse_statement` is called and the current token is `Token::Merge`
- **THEN** it MUST delegate to `self.parse_merge()`
- **AND** the returned `Statement` MUST be `Statement::Merge(...)`
