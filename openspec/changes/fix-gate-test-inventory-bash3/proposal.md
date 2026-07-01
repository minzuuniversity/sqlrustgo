## Why

`scripts/gate/check_test_inventory.sh` uses `declare -A` (associative arrays), a feature added in **bash 4.0**. The macOS default shell is **bash 3.2.57** (last GPLv2 release, shipped with every macOS since 2007), so the gate fails immediately on every macOS runner:

```
Test files discovered:      238
scripts/gate/check_test_inventory.sh: line 47: declare: -A: invalid option
declare: usage: declare [-afFirtx] [-p] [name[=value] ...]
scripts/gate/check_test_inventory.sh: line 49: tests: unbound variable
```

This is a **process gate** (5-Principle P4 enforcement: every test file must be invoked at gate level). With the script broken, D6 (Test Inventory Gate) cannot run on any macOS machine, and CI runners using bash 3.x silently fail too. The test files are still being run by their own `[[test]]` blocks in `Cargo.toml`, but the **inventory count** (which test files exist vs. which are gate-covered) is broken.

## What Changes

- Rewrite the associative-array usage with a bash 3.2-compatible pattern: store the path→name mapping in a temp file keyed by line, then look up via `grep`. This is O(n) per lookup but the test set is ~240 files, well under any perf concern.
- Add a shebang check: if `${BASH_VERSION%%.*}` < 4, source a portable code path; otherwise use the cleaner `declare -A` form. Same script, two implementations behind a version check.
- Add a smoke test: the script must `exit 0` on bash 3.2.57 with the 238-file test set and report test counts accurately.