---
name: verify-ignore-tests
description: Verify long stability tests marked #[ignore] — run with --include-ignored, update ignore_registry.json, create spec
source: auto-skill
extracted_at: '2026-06-20T18:18:24.471Z'
---

## Procedure: Verify #[ignore] Tests

When the user asks to verify, un-ignore, or audit tests that are currently marked `#[ignore]`:

### Step 1: Identify target test files

```bash
# Find all #[ignore] markers in test files
grep -rn "#\[ignore\]" tests/*.rs crates/*/tests/*.rs crates/*/src/*.rs

# Count total ignored tests
grep -c "#\[ignore\]" tests/*.rs
```

### Step 2: Check ignore_registry.json

The file `tests/baseline/ignore_registry.json` tracks all `#[ignore]` entries. If it does not exist, create it.

```bash
ls tests/baseline/ignore_registry.json
```

### Step 3: Run tests with --include-ignored

Run each test file independently to isolate failures:

```bash
# Run specific test file with ignored tests included
cargo test --release --test <test_file_name> -- --include-ignored

# If test file has multiple groups, run with --test-threads=1 for debugging
cargo test --release --test <test_file_name> -- --include-ignored --test-threads=1
```

### Step 4: Update ignore_registry.json

- Remove entries for tests that now pass and have no `#[ignore]` markers
- Add entries for new `#[ignore]` markers discovered during grep
- Update `total_allowed` count
- Update `timestamp`

```bash
# After removing entries, verify JSON is valid
python3 -c "import json; json.load(open('tests/baseline/ignore_registry.json'))"
```

### Step 5: Remove #[ignore] markers (if tests pass)

If all tests in a file pass, remove `#[ignore]` markers from the source files.

### Step 6: Create spec documentation

Create `openspec/changes/<change-name>/specs/<capability>/spec.md` documenting:
- Test names and files
- Pass/fail status per test
- Any deferred tests with reason
- Final `#[ignore]` count

### Important Notes

- **1000 iterations may not mean 1000 seconds**: The `STABILITY_ITERATIONS` constant (1000) is simulated iterations, not wall-clock time. In-memory tests run in milliseconds.
- **72h tests may be smoke tests**: Some "72h" tests only run a few seconds as smoke tests, not actual 72-hour runs.
- **Registry entries can be stale**: Tests may have been un-ignored previously but registry not updated. Always grep the source files before trusting the registry.
- **Run tests independently**: Always use `--test <name>` to run one file at a time to isolate which tests pass/fail.
