# Design

## Approach

Replace the hardcoded data-dir string with a `tempfile::TempDir` instance
bound to the test function's stack. The `tempfile` crate is already a
dev-dependency in the workspace root `Cargo.toml` (`tempfile = "3.25.0"`),
so no new dependency is needed.

The test currently does:

```rust
let data_dir = Path::new("/tmp/multi_stmt_test");
std::fs::create_dir_all(&data_dir).unwrap();

let config = EphemeralConfig {
    data_dir: Some(data_dir.to_path_buf()),
    ...
};
```

After the fix:

```rust
let dir = tempfile::TempDir::new().expect("create tempdir for ephemeral server");
let config = EphemeralConfig {
    data_dir: Some(dir.path().to_path_buf()),
    ...
};
// dir is bound to the function scope; its Drop removes the directory
// (along with the WAL and any table files the server wrote) when
// the test returns.
```

The `use std::path::Path;` line at the top of the test can be deleted
because no other code in the function uses `Path::new` after the
replacement.

## Why a `TempDir` (and not `std::env::temp_dir()` + a unique suffix)

`std::env::temp_dir()` plus a port-keyed or pid-keyed suffix is the
naïve alternative. The server's accept loop already computes
`sqlrustgo_ephemeral_<port>_<pid>` for the data dir when the caller
passes `data_dir: None`, but the test that hardcodes the path
**bypasses** that path and asserts a specific location. The bug here
is not the location-per-se but the **shared, never-cleaned** nature
of the directory: a stale WAL from the previous process lives there
and trips the recovery engine on startup.

`tempfile::TempDir` is the existing project pattern (used by
`show_tables_test`, the 3.8.0-era wire-protocol suite, and
`run_server_v2`'s 72h-soak harness). It gives us:

- **Fresh state** for each invocation (mkdtemp).
- **Automatic cleanup** on Drop (no manual `remove_dir_all`, no
  permanent `/tmp` litter).
- **A stable absolute path** so the server's WAL file
  (`${dir}/sqlrustgo.wal`) is created in a location the OS is
  comfortable with (long-path-tolerant filesystems on macOS).

## Test-mode-only change

The change touches only test code. No production code (`crates/*/src/*`)
is modified. The `tempfile` dev-dependency is already present, so
`cargo test -p sqlrustgo` does not pull any new transitive deps.
