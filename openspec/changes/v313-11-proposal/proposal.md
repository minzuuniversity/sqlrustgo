# v313-11: UPDATE Constraint Enforcement

## Summary
Implement UPDATE constraint checking (NOT NULL, CHECK, etc).

## Motivation
V312-11: UPDATE statement expected to fail due to constraint violation but succeeded.

## Scope
- NOT NULL constraint checking on UPDATE
- CHECK constraint on UPDATE
- Other constraint enforcement during UPDATE

## Owner
openclaw

## Expiry
2027-06-30
