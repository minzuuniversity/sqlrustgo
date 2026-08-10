# v313-08 Tasks: SQLLogicTest Parser Fixes

## Task List

- [ ] v313-08-01: Implement ALTER TABLE SET PARTITIONED BY syntax
- [ ] v313-08-02: Implement ALTER TABLE DROP PARTITIONED BY syntax
- [ ] v313-08-03: Fix case-insensitive ALTER TABLE keywords
- [ ] v313-08-04: Support UPDATE with constraint syntax (con1)
- [ ] v313-08-05: Fix CREATE TABLE AS SELECT semantics
- [ ] v313-08-06: Add/update tests for all above fixes
- [ ] v313-08-07: Run sqllogictest gate, verify fixes

## Verification
```bash
bash scripts/gate/check_sqllogictest_v312.sh
# Target: 10+ files passing (was 6)
```
