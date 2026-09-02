# parser-instead-of-trigger Specification

## Purpose
TBD - created by archiving change v312-64a-parser-batch. Update Purpose after archive.
## Requirements
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

