# parser-instead-of-trigger

## Purpose

`CREATE TRIGGER name INSTEAD OF INSERT|UPDATE|DELETE ON view FOR EACH ROW BEGIN ... END` MUST be accepted by the parser (PostgreSQL/SQLite standard, used for updatable views via triggers).

## ADDED Requirements

### Requirement: parser must accept INSTEAD OF trigger timing

The parser MUST treat `INSTEAD OF` as a valid trigger timing alongside BEFORE / AFTER.

#### Scenario: INSTEAD OF UPDATE ON view

GIVEN sql `CREATE TRIGGER inst_upd INSTEAD OF UPDATE ON v FOR EACH ROW BEGIN UPDATE t SET val=NEW.val WHERE id=OLD.id; END`
WHEN parser parses CREATE TRIGGER
THEN timing field equals "INSTEAD OF", events contains "UPDATE", table_name equals "v", body contains the BEGIN/END block contents. No "Expected BEFORE or AFTER" error is raised.

### Requirement: BEFORE / AFTER timing regression

The parser MUST continue to accept BEFORE / AFTER timing tokens.

#### Scenario: BEFORE INSERT

GIVEN sql `CREATE TRIGGER t1 BEFORE INSERT ON t FOR EACH ROW BEGIN ...`
WHEN parser parses
THEN timing field equals "BEFORE" (regression).

## Acceptance Criteria

- parser unit test `parse_trigger_instead_of_view` PASS
- parser unit test `parse_trigger_before_after_regression` PASS
- integration test `create_trigger_instead_of_view` PASS
- Token::Instead is added to the lexer keyword map and recognised case-insensitively