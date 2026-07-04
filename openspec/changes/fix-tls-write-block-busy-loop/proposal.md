## Why

After PR #3697 and #3699, `sysbench --mysql-ssl=off` against `sqlrustgo-mysql-server` still fails with `FATAL: mysql_stmt_prepare() failed — MySQL error: 2000 "Unknown or undefined error code"`.

The error occurs only over TLS. Packet capture confirms the MariaDB Connector/C 3.x advertises `CLIENT_SSL` even when `--mysql-ssl=off` is passed, so all COM_STMT_PREPARE exchanges go through the TLS code path. All unit tests (non-TLS, using `MySqlTestClient`) pass.

Root cause: the `TlsStream::write()` and `flush()` impls (modified in PR #3694 to fix a different hang) spin in a tight loop on `WouldBlock`. The kernel socket buffer is full, so `complete_io()` keeps returning `WouldBlock`, the CPU burns at 100%, and neither side makes progress. The MariaDB Connector/C times out or receives corrupted/partial data, producing error 2000.

`Read::read()` and `flush_pending()` already break on `WouldBlock`. The write path was inconsistent.
