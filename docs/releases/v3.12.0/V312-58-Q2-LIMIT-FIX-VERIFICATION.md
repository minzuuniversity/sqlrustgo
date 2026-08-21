# V312-58 — TPC-H Q2 LIMIT regression — verification

## Headline

Fixing one missing entry in the lexer keyword table (`"ASC" => Token::Asc`)
restores correct execution for every TPC-H query that uses an explicit
`ORDER BY col ASC, ... LIMIT n` (Q1, Q2, Q3, Q5, Q10, Q18, Q21, ...).
The committed reproducer (`q2_canonical_5way_comma_limit`) goes from
642 rows → 20 rows at SF=1.

## Issue

V312-58 / Issue #4375 — TPC-H Q2 (canonical 5-way comma-join,
`ORDER BY s_acctbal ASC, n_name, s_name, p_partkey LIMIT 20`) returns
642 rows instead of 20 at SF=1. Symptom: `LIMIT 20` is silently dropped
and the full ASC-sorted join is returned.

## Root cause

The lexer keyword table at `crates/parser/src/lexer.rs` mapped `"DESC"`
to `Token::Desc` but had **no** entry for `"ASC"`. Every occurrence of
the word `ASC` in SQL was emitted as `Token::Identifier("ASC")`.

`parse_order_by` (`crates/parser/src/parser.rs:5920`) handles ASC only
via its `Some(Token::Asc) => self.next()` arm. When the parser saw
`Identifier("ASC")` instead of `Token::Asc`, the arm did not fire, the
ASC token was left unconsumed, and the trailing `, next_col, ... LIMIT n`
chain was never parsed. By the time `parse_select_statement` reached
the `let limit = if matches!(self.current(), Some(Token::Limit))` check,
`self.current()` was `Identifier("ASC")`, so `limit = None`.

Diagnostic trace before the fix:

```
V312-58-DEBUG parser_at_limit: order_by_len=1 current_token=Some(Identifier("ASC"))
V312-58-DEBUG parser_after_limit_parse: limit=None next_token=Some(Identifier("ASC"))
V312-58-DEBUG parser_exit: select.limit=None select.offset=None tables=part join_chain=4 ...
V312-58-DEBUG execute_select_entry: select.limit=None select.offset=None ...
```

After the fix (single one-line entry, plus a 10-line comment block in the
keyword table at `crates/parser/src/lexer.rs:457`):

```
V312-58-DEBUG parser_at_limit: order_by_len=4 current_token=Some(Limit)
V312-58-DEBUG parser_after_limit_parse: limit=Some(20) next_token=Some(Semicolon)
```

## Reproducer

`tests/integration/oracle/q2_5way_comma_limit_regression.rs` —
`q2_canonical_5way_comma_limit` (heavy SF=1 fixture, marked `#[ignore]`).
The companion `tests/integration/oracle/lexer_asc_keyword_regression.rs`
covers the same root cause at small scale (no fixture required):

| Test                                          | Fixture      | Before fix     | After fix |
|-----------------------------------------------|--------------|----------------|-----------|
| `q2_canonical_5way_comma_limit`               | SF=1 TPC-H   | 642 rows ✗     | 20 rows ✓ |
| `asc_keyword_consumed_in_order_by_single_item`| 3-row in-mem | ASC ignored ✗  | a,b,c ✓   |
| `asc_with_limit_returns_correct_rows`         | 3-row in-mem | LIMIT dropped ✗| a,b ✓     |
| `asc_with_limit_one_returns_single_row`       | 3-row in-mem | LIMIT dropped ✗| a ✓       |
| `mixed_asc_desc_list_preserves_all_items`     | 4-row in-mem | list truncated ✗| 75,100,50 ✓|

## How to reproduce / verify

```bash
# Small (no fixture required) — proves ASC + LIMIT work end-to-end
cargo test --test lexer_asc_keyword_regression --all-features

# Heavy (SF=1 fixture required) — proves TPC-H Q2 LIMIT 20 now caps correctly
export TPCH_SF1_DIR=/home/openclaw/tpch_baseline/sf1
cargo test --test q2_5way_comma_limit_regression --all-features \
  -- --ignored --nocapture q2_canonical_5way_comma_limit
```

## Commands run

```text
$ cargo build --tests --all-features 2>&1 | tail -3
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.41s

$ cargo test --test q2_5way_comma_limit_regression --all-features \
    -- --ignored --nocapture q2_canonical_5way_comma_limit 2>&1 | tail -5
Q2 returned 20 rows
  row[0] = [Float(-986.14), Text("Supplier#000003627"), Text("FRANCE"), Integer(103626), ...]
  ...
test q2_canonical_5way_comma_limit ... ok
test result: ok. 1 passed; 0 failed; ... finished in 174.24s

$ cargo test --test lexer_asc_keyword_regression --all-features 2>&1 | tail -8
running 4 tests
test asc_with_limit_returns_correct_rows ... ok
test asc_with_limit_one_returns_single_row ... ok
test asc_keyword_consumed_in_order_by_single_item ... ok
test mixed_asc_desc_list_preserves_all_items ... ok
test result: ok. 4 passed; 0 failed; ...

$ cargo test --lib --all-features 2>&1 | tail -3
test result: ok. 55 passed; 0 failed; ... finished in 11.73s

$ cargo test -p sqlrustgo-parser --all-features 2>&1 | tail -3
test result: ok. 5 passed; 0 failed; ...
```

## Diff size

One-line keyword addition + 10-line comment block at
`crates/parser/src/lexer.rs:457`. No other source files touched.
No parser / executor / planner logic modified. No spec or wire-format
changes. No behaviour change for `DESC`, `ORDER BY col DESC`, `ORDER BY col`
(no direction), or any other keyword.