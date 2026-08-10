# v313-09: SQLLogicTest INSERT Fixes

## Summary
Fix INSERT-related SQLLogicTest failures: row count correctness, invalid INSERT rejection.

## Motivation
V312-11 smoke baseline shows INSERT returning wrong row count and invalid INSERT not properly rejected.

## Scope
- INSERT returning correct row count
- Invalid INSERT with special characters properly rejected

## Owner
openclaw

## Expiry
2027-06-30
