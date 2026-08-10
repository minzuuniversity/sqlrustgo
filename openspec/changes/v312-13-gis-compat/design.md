# V312-13 #3900 GIS Compat Fix — Design

## Root cause

`src/execution_engine.rs:731` has a match on `Value` to infer column type from a sample row. The match arms are:
- `Value::Integer(_)` → "INTEGER"
- `Value::Float(_)` → "FLOAT"
- `Value::Text(_)` → "TEXT"
- `Value::Null` → "TEXT"
- `Value::Blob(_)` → "BLOB"
- `Value::Boolean(_)` → "BOOLEAN"
- `Value::Point(_, _)` → "POINT"

`Value::Json(_)` (added in V312-16) is not in the list, causing the `cargo build -p sqlrustgo-mysql-server` to fail.

## Fix

Add the missing arm:

```rust
&sqlrustgo_types::Value::Json(_) => "JSON".to_string(),
```

The match is on `&Value` so the `&` prefix is needed (consistent with the suggestion in compiler output).

## Verification

```bash
$ cargo build -p sqlrustgo-mysql-server
# Should exit 0 (currently fails with E0004)

$ bash scripts/gate/check_anti_fabrication.sh
# Should exit 0 or show only pre-existing failures

$ cargo test -p sqlrustgo-mysql-server --test wire_smoke_mysql_cli
# 11/11 PASS

$ bash scripts/gate/check_load_data_infile.sh
# 4/4 PASS
```

## Other Value::Json usages (out of scope for this PR)

The following files likely also have `Value::Json` non-exhaustive matches and should be addressed in follow-up #3959 (V312-24):
- `crates/executor/src/...` (storage-side matches — already fixed in PR #3982)
- `crates/parser/src/...` (parser AST — no Json match needed)
- `crates/mysql-server/src/...` (MySQL protocol encoding — to verify)
- `src/execution_engine.rs` (this PR fixes one)
