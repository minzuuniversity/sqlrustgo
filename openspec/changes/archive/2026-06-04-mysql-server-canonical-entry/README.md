# mysql-server-canonical-entry

Consolidate all SQLRustGo v3.8.0 execution paths into a single canonical entry point: the sqlrustgo-mysql-server binary. This change retires the legacy REPL (), the stub root binary, the dual HTTP server (), and other in-process execution paths. From v3.8.0 onward, every e2e, integration, and performance test must be driven through the MySQL wire protocol so that there is exactly one production execution surface to validate, benchmark, and ship.
