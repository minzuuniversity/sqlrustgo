## Why

`sqlrustgo-cli sqlite` rejects `LIKE 'pattern' ESCAPE 'string'` with a parse error when the escape string is more than 1 character. Issue #4673 reports this. The parser at `crates/parser/src/parser.rs:7800` and `parser.rs:7881` requires `s.len() == 1` for the escape string literal — but the lexer correctly returns the full string content (e.g. `\\` is 2 chars, `\'` is 1 char). A 1-char restriction blocks the natural SQL idiom `ESCAPE '\\'` (which is the standard way to use backslash as the escape character).

This is a P0 issue per the issue-classification report: simple real-world SQL patterns fail to parse.

## What Changes

- Change the `s.len() == 1` guard in both `LIKE` and `NOT LIKE` parsing to use the first character of the string instead. This matches MySQL/PostgreSQL behaviour where `ESCAPE '\\'` (the SQL string containing 2 chars `\\`) uses the first char (`\`) as the escape.
- Update the error message to "Expected string literal after ESCAPE" so the failure is clearer when a non-string-literal token is supplied.

No public API change. No executor change. No breaking change.

## Capabilities

### New Capabilities

- `parser-like-escape-multi-char`: `sqlrustgo_parser` MUST accept `LIKE pattern ESCAPE 'string'` (and `NOT LIKE`) where the string may be longer than 1 character; the first character is used as the escape.

### Modified Capabilities

- None.
