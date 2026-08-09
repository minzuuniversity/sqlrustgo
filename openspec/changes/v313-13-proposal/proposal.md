# v313-13: LIMIT/OFFSET with ORDER BY

## Summary
Fix LIMIT/OFFSET handling with complex ORDER BY expressions.

## Motivation
V312-11: order__test_limit.test fails due to incorrect LIMIT/OFFSET behavior.

## Scope
- LIMIT/OFFSET with ORDER BY semantics
- OFFSET without LIMIT

## Owner
openclaw

## Expiry
2027-06-30
