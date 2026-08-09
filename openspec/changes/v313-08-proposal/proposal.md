# v313-08: SQLLogicTest Parser Fixes

## Summary
Fix parser-level SQLLogicTest failures in V312-11 baseline: ALTER TABLE PARTITIONED BY, case-insensitive keywords, UPDATE constraint syntax, CREATE TABLE AS.

## Motivation
V312-11 established a smoke baseline with 16 test files, 6 passing. The remaining 10 failures include several parser-level issues that block full corpus integration.

## Scope
- ALTER TABLE SET/DROP PARTITIONED BY syntax support
- Case-insensitive ALTER TABLE keywords
- UPDATE with constraint syntax (con1)
- CREATE TABLE AS SELECT semantics

## Owner
openclaw

## Expiry
2027-06-30

## Tasks
See tasks.md
