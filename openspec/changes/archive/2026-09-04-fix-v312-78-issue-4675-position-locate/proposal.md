## Why

`sqlrustgo-cli sqlite` (HEAD, develop/v3.12.0) returns NULL for `POSITION(substr IN str)` (SQL standard) and `LOCATE(substr, str)` (MySQL compatibility) because the `eval_fn` match arm has no arms for these names and defaults to `Value::Null`. Issue #4675 reports both functions as production-blocking.

Repro:
```
$ printf 'CREATE TABLE t(id int, name VARCHAR(50));
   INSERT INTO t VALUES (1, '\''a%bc'\''), (2, '\''a_bc'\''), (3, '\''abc'\'');
   SELECT id, name, POSITION('\''bc'\'' IN name), LOCATE('\''bc'\'', name) FROM t;' \
   | sqlrustgo-cli sqlite --batch --mode csv /tmp/db
1,a%bc,,
2,a_bc,,
3,abc,,
```

Expected (SQL standard / MySQL compatibility):
```
1,a%bc,2,2   (POSITION: 1-based index of 'bc' in 'a%bc' = 2)
2,a_bc,0,0   (not found → 0)
3,abc,2,2    (POSITION: 1-based index of 'bc' in 'abc' = 2)
```

Root cause: `eval_fn` (~line 2182) defaults to `Value::Null`. POSITION and LOCATE are not registered.

## What Changes

- Add `POSITION` arm to `eval_fn` — SQL standard `POSITION(substr IN str)`:
  - Returns 1-based integer position of `substr` in `str`
  - Returns 0 if `substr` not found (SQL standard: NOT NULL, returns 0)
  - Case-sensitive (SQL standard default)
- Add `LOCATE(substr, str[, pos])` arm — MySQL compatibility:
  - `LOCATE(substr, str)` → same as `POSITION(substr IN str)` (1-based, 0 if not found)
  - `LOCATE(substr, str, pos)` → start search from byte position `pos` (1-based)
  - Case-sensitive
- No AST change, no storage/WAL impact.

## Capabilities

### New Capabilities

- `executor-string-position`: `sqlrustgo` MUST evaluate `POSITION(substr IN str)` per SQL standard and `LOCATE(substr, str[, pos])` per MySQL, returning 1-based integer position or 0 if not found.

### Modified Capabilities

- None.
