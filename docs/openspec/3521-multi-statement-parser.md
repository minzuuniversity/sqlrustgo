# ISSUE 3521: [parser] mysql-client 8.0.46 multi-statement: COM_QUERY rejects `;`-separated statements

## Status
**Closed** - Fixed in PR #3524

## Background

MySQL clients like mysql-client 8.0.46 send multiple statements in a single COM_QUERY packet, separated by semicolons (e.g., `SELECT 1; SELECT 2`). The parser was unable to handle these multi-statement queries, returning ER_PARSE_ERROR before the wire layer could form a response.

## Root Cause

The `parse()` function in `sqlrustgo-parser` only parsed a single statement and did not handle semicolon-separated statements. When mysql-client sent `SELECT 1; SELECT 2`, the parser attempted to parse the entire string as one statement and failed.

## Solution

Added a new `parse_statements()` function that:

1. Tokenizes the full SQL input
2. Splits tokens by semicolons at the top level (respecting parentheses depth and string literals)
3. Parses each batch of tokens as a separate statement
4. Returns a `Vec<Statement>`

Modified the MySQL server's COM_QUERY handler to use `parse_statements()` and execute each statement sequentially, sending results for each.

### Files Changed

- `crates/parser/src/parser.rs`: Added `parse_statements()` function
- `crates/parser/src/lib.rs`: Exported `parse_statements`
- `crates/mysql-server/src/lib.rs`: Use `parse_statements` for COM_QUERY handling

## Acceptance Criteria

- [x] `mysql -e "SELECT 1; SELECT 2"` returns two result sets
- [x] `mysql -e "SELECT 1; SELECT 2; SELECT 3"` returns three result sets
- [x] Single statements continue to work as before
- [x] Statements with parentheses are handled correctly (semicolons inside `VALUES(...)` don't split)
- [x] Statements with string literals are handled correctly

## Test Results

```
$ mysql --no-defaults -h 127.0.0.1 -P 3457 -u root --ssl-mode=DISABLED -e "SELECT 1; SELECT 2"
+---+
| 1 |
+---+
| 1 |
+---+
| 2 |
+---+
| 2 |
```

Both SELECT statements execute and return results successfully.