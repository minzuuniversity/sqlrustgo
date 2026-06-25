# Tasks

## 1. Switch hardcoded path to TempDir

In `tests/multi_statement_test.rs`, replace the `let data_dir =
Path::new("/tmp/multi_stmt_test"); std::fs::create_dir_all(&data_dir)
.unwrap();` block in `test_multi_statement_two_selects` with
`let dir = tempfile::TempDir::new().expect("...");`. Update the
`EphemeralConfig` to use `dir.path().to_path_buf()`.

The test does not need to pass `dir` to the rest of the function —
its `Drop` runs at the closing `}` of the test, after `client.quit()`
and all assertions. Bind the `dir` variable at the top of the
function so it lives for the full test scope.

Remove the now-unused `use std::path::Path;` import (it is only
referenced in the line we replaced).

## 2. Verify the test passes

Run:

```
cargo test -p sqlrustgo --test multi_statement_test
```

Expected: 15/15 pass (was 14/15 failing on the EAGAIN test).

If the test still fails, re-run with `--nocapture` and check that
the server's startup logs report the expected `${tempdir}/sqlrustgo.wal`
path and not a stale `/tmp/multi_stmt_test/sqlrustgo.wal`. A
remaining failure would point to a deeper WAL recovery issue, in
which case escalate to the next OpenSpec change rather than chasing
in this one.

## 3. Commit and push

Commit message:

```
test(multi_stmt): isolate ephemeral data dir with TempDir

multi_statement_test.rs::test_multi_statement_two_selects hardcoded
/tmp/multi_stmt_test as the ephemeral server's data dir and never
cleaned it. Stale WAL files from a prior run tripped the recovery
engine and caused the client to read an EOF (or EAGAIN) on the
handshake.

Switch to tempfile::TempDir — the same pattern already used by
show_tables_test. The TempDir is removed when the test returns, so
no /tmp litter and no cross-test state survives.
```

Push to backup250/develop/v3.9.0:

```
git push backup250 develop/v3.9.0
```

## 4. Archive the OpenSpec change

After the commit is on backup250, move the change directory under
`openspec/changes/archive/`:

```
mv openspec/changes/fix-multi-statement-data-dir-pollution \
   openspec/changes/archive/fix-multi-statement-data-dir-pollution
```

Commit the archive move separately so the project history records
the closure of the change.
