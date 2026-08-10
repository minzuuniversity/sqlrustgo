# v313-14: NOT NULL Constraint on UPDATE

## Summary
Enforce NOT NULL constraint during UPDATE operations.

## Motivation
V312-11: constraints__test_not_null.test fails because NOT NULL is not enforced on UPDATE.

## Scope
- NOT NULL constraint checking on UPDATE
- Appropriate error messages

## Owner
openclaw

## Expiry
2027-06-30
