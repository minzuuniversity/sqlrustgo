# V312-F-6 Design: DEFERRED Items Documentation

## Boundary

**V312-13 DONE**:
- Wire main path: binary row parsing, COM_QUERY, COM_STMT_PREPARE/EXECUTE
- 22 typed-wrappers tests (1 fail post-F-1 fix)
- 37 e2e_wire_protocol tests (9 fail post-F-2 fix)

**V312-24 DEFERRED (tracked in #3959)**:
- LOAD DATA SF=1 server-side execution
- LOAD DATA SF=10 server-side execution
- TLS handshake (server-side)
- zlib compression (server-side)
- COM_RESET_CONNECTION server-side (post-F-1 fix removes from DEFERRED)

## Documentation Updates

Each evidence file gets a `## Done/Deferred Boundary` section that
explicitly lists what is IN this report (V312-13) vs what is tracked
in #3959 (V312-24 deferred items).