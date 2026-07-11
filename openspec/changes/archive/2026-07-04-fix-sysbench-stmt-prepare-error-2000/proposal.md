## Why

The internal repro `tests/stmt_execute_repro.rs::repro_stmt_execute_returns_malformed_packet`
was failing: `COM_STMT_PREPARE` → `COM_STMT_EXECUTE` parameterised SELECT returned
**0 rows** instead of the expected 1.

Diagnostic output captured during investigation:
```
DBG EXECUTE pre:  raw_sql="SELECT v FROM repro_t WHERE id = ?"
DBG EXECUTE post: final_sql="SELECT v FROM repro_t WHERE id = 3" params=[([51], true)]
DBG EXECUTE result: rows=1 first_row=Some([Text("row_3")]) stmt_col_count=1
```

Engine returns 1 row correctly but the response is mis-parsed by the client.
Root cause: `write_binary_row` produced `(row.len()+9)/8 = 2` bytes of null bitmap
for a 1-column row (spec requires `(cols+7)/8 = 1` byte).
