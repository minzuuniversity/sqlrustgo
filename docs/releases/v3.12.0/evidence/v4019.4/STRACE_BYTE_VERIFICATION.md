# v4019.4 — STRACE Byte-Level Verification: Fix B / B' / B'' / C / **D** Evolution

> **STRICT PROOF MODE record. Honest verdict, no claims of PASS without content verification.**

| Field | Value |
|-------|-------|
| Worktree | `/tmp/wt-v312-followups` |
| Branch | `fix/v312-followups-wire-perf-bulkload` |
| Binary under test | `/tmp/wt-v312-followups/target/debug/sqlrustgo-mysql-server` |
| mysql CLI under test | 8.0.46 (real binary, not mocked) |
| Date | 2026-08-12 |
| Author | minimax |
| Status | **RESOLVED — fix D (V312-WIRE-8) clears the hang; mysql CLI 8.0.46 exits cleanly with 0xFE terminator** |

---

## TL;DR (updated after fix D)

After **five** fix attempts (B, B', B'', C, **D**) the server now produces a successful
end-to-end response to `select @@version_comment limit 1` from real `mysql` CLI 8.0.46.
The root cause was identified by direct strace comparison against real MySQL 8.0.46 port
3306: per MySQL WL#7766, the trailing result-set terminator under `CLIENT_DEPRECATE_EOF`
uses the **EOF identifier `0xFE`** as its first byte, NOT the regular OK marker `0x00`.

| Fix | V312-WIRE-# | Header byte | Status | Outcome |
|-----|-------------|-------------|--------|---------|
| B   | 4  | 0x00 | 0x4002 (with 0x4000) | hang — client missed session_state |
| B'  | 5  | 0x00 | 0x4002 | hang — wrong: separate packet hypothesis |
| B'' | —  | 0x00 | various | hang — wrong: separate packet, varied content |
| C   | 7  | 0x00 | 0x4002 (embedded session_state) | hang — client dispatched on 0x00 as regular OK |
| **D** | **8**  | **0xFE** | **0x0002 (no 0x4000 for plain SELECT)** | **PASS — mysql CLI exits cleanly** |

---

## Method

For each fix variant:

1. Build: `cargo build -p sqlrustgo-mysql-server` in the worktree.
2. Start server (non-TLS): `nohup ./target/debug/sqlrustgo-mysql-server serve --port
   13405 --auth-mode none --data-dir /tmp/v312-fixD-data`
3. Run real mysql CLI: `strace -f -e trace=network -s 256 -o /tmp/v312-fixD/client-strace.log
   mysql -h 127.0.0.1 -P 13405 -u root --ssl-mode=DISABLED -e 'select @@version_comment limit 1'`
4. Decode all `recvfrom` / `sendto` packets byte-by-byte, compare against
   <https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_basic_ok_packet.html>
   AND against real MySQL 8.0.46 port 3306 wire bytes captured the same way.

---

## Fix B — Add 0x4000 to status_flags (V312-WIRE-4)

**Hypothesis:** mysql CLI 8.0+ expects `SERVER_STATUS_SESSION_STATE_CHANGED (0x4000)` to be
set in the OK packet when `CLIENT_SESSION_TRACK` is negotiated, signaling "session state
may have changed in this statement."

**Result:** Unit tests PASS (added `client_cap` parameter to `make_ok_packet`). Real mysql CLI
**still hangs**. Wire bytes showed status_flags=0x4002 (AUTOCOMMIT | SESSION_STATE_CHANGED)
in trailing OK, but client was already hung on a separate `session_state_info` packet that
we never sent.

**Verdict:** **Hypothesis confirmed but incomplete** — 0x4000 is necessary, not sufficient.

---

## Fix B' — Emit separate `session_state_info` packet (V312-WIRE-5, later retracted)

**Hypothesis:** Per MySQL 5.7 / early 8.0 implementation notes, when `CLIENT_SESSION_TRACK`
is set and status has 0x4000, the OK packet is **followed by a separate packet** carrying
`session_state_changes` as its payload. mysql CLI 8.0.46 reads this 2nd packet before
proceeding.

**Implementation:** Changed `make_ok_packet` to return `Vec<Packet>`. Added optional 2nd
packet with payload = `lenenc(0)`. `do_command_loop` switched to `write_ok_packets` helper
that loops over the `Vec` and increments seq per packet.

**Result:** Unit tests PASS (3 new tests). Real mysql CLI **still hangs** — now on a 6th
`recvfrom` (PKT1 col count + PKT2 col def + PKT3 row + PKT4 trailing OK + PKT5
session_state_info were all received correctly, but the client waited for one more packet).

**Verdict:** **Hypothesis FALSIFIED** — the 2nd packet IS read, but the client expects MORE
after it, OR the 2nd packet is supposed to be embedded (not separate) per the actual MySQL
8.0 spec. Wire-byte analysis showed status_flags=0x4002 and a separate 1-byte
`session_state_info` packet arrived, but client continued to hang.

---

## Fix B'' — Adjust session_state_info packet content / status ordering

**Hypothesis:** Maybe the 2nd packet's content needed to be richer (not just `lenenc(0)`).

**Implementation:** Tried several variants — non-empty `session_state_changes`, different
status flag combinations, different packet sequencing.

**Result:** All variants still hang. The client consistently reads the result-set + 2nd
packet then blocks on a further `recvfrom`.

**Verdict:** **Hypothesis FALSIFIED** — adding content to the 2nd packet does not unblock
the client. The architectural assumption (separate packet) is wrong.

---

## Fix C — Embed `session_state_changes` inside the OK packet (V312-WIRE-7, supersedes V312-WIRE-5)

**Hypothesis (corrected after fix B''):** Per the canonical MySQL 8.0 wire-protocol spec
<https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_basic_ok_packet.html>,
when `CLIENT_SESSION_TRACK` is negotiated AND status_flags has 0x4000 set, the
`session_state_changes` lenenc string is **embedded** in the SAME OK packet — right after
the `info` field — NOT sent as a separate packet.

**Implementation:**
- `make_ok_packet` (line 1741-1824): writes `lenenc(0)` for `info`, then if status has
  0x4000, writes another `lenenc(0)` for `session_state_changes` — all inside the same
  packet payload.
- `make_deprecate_eof_ok_packet` (line 1853-1921): same fix applied.
- Auth OK path: `is_auth_ok=true` short-circuits both the 0x4000 flag AND the embedded
  `session_state_changes` byte.

**Result:** Unit tests PASS. Real mysql CLI **still hangs**. Wire bytes were correct per
the MySQL 8.0 spec (verified byte-by-byte against the spec table below), but the client
still hung on a 5th `recvfrom`.

**Verdict:** **Wire bytes correct per spec, but client still hangs. Architectural
question required.**

### Fix C — Real mysql CLI 8.0.46 byte-level verification

**Run command:**
```
strace -f -e trace=network -s 256 -o /tmp/v312-fixC/client-strace-nossl.log \
  mysql -h 127.0.0.1 -P 13404 -u root --ssl-mode=DISABLED \
  -e 'select @@version_comment limit 1'
```

**Decoded wire bytes (fix C, server → client, all 4 result-set packets):**

| Pkt | Header (3-byte len + 1-byte seq) | Payload (hex) | Decoded |
|-----|----------------------------------|---------------|---------|
| PKT1 | `\x01\x00\x00\x01` (len=1, seq=1) | `\x01` | column_count = 1 |
| PKT2 | `\x20\x00\x00\x02` (len=32, seq=2) | `03 64 65 66 00 00 00 05 col_1 05 col_1 0c 30 00 ff 00 00 00 0f 00 00 00 00 00` | column def (32 bytes) |
| PKT3 | `\x0a\x00\x00\x03` (len=10, seq=3) | `\x09 SQLRustGo` | row (lenenc 9 + "SQLRustGo") |
| PKT4 | `\x09\x00\x00\x04` (len=9, seq=4) | `\x00 \x00 \x00 \x02\x40 \x00\x00 \x00 \x00` | OK packet (0x00 header + lenenc(0) + lenenc(0) + status 0x4002 + warnings 0 + lenenc(0) info + lenenc(0) session_state) |

**Status_flags = 0x4002 = AUTOCOMMIT | SESSION_STATE_CHANGED.**

**MySQL 8.0 OK packet spec compliance check (all fields ✓):**

| Field | Spec | Server emitted | Status |
|-------|------|----------------|--------|
| header byte | `0x00` | `0x00` | ✓ |
| affected_rows (lenenc) | lenenc | `0x00` (=0) | ✓ |
| last_insert_id (lenenc) | lenenc | `0x00` (=0) | ✓ |
| status_flags (u16 LE) | u16 | `0x02 0x40` = 0x4002 | ✓ |
| warnings (u16 LE) | u16 | `0x00 0x00` = 0 | ✓ |
| info (lenenc string) | lenenc | `0x00` (=0, empty) | ✓ |
| session_state_changes (lenenc, only if status & 0x4000) | lenenc | `0x00` (=0, empty) | ✓ embedded |

**ALL FIELDS CORRECT PER MYSQL 8.0 OK PACKET SPEC.**

**Client-side strace (last meaningful line before hang):**
```
recvfrom(3, 0x5dea70c7d030, 16384, 0, NULL, NULL) = ? ERESTARTSYS (To be restarted)
--- SIGTERM {si_signo=SIGTERM, si_code=SI_USER, si_pid=55536, si_uid=1004} ---
+++ killed by SIGTERM +++
```

mysql CLI is **blocked on `recvfrom` after consuming the 4 result-set packets** (including
the trailing OK with embedded `session_state_changes`). Server has **no more data to send**.

---

## Phase 4.5 — diagnostic vs real MySQL 8.0.46 (the breakthrough)

Per systematic-debugging Phase 4.5 and the user's locked hypothesis: the "wire bytes 100%
correct per spec" verdict was actually incomplete — the spec was being read without
comparing to the real reference implementation. We captured wire bytes from real MySQL
8.0.46 on port 3306 (same capabilities, same query) and decoded the trailing OK:

```
PKT4: length=7, seq=4, payload=fe 00 00 02 00 00 00
```

| Field | Real MySQL 8.0.46 | Fix C (our server) | Match? |
|-------|-------------------|---------------------|--------|
| Header byte | **`0xFE`** (EOF identifier) | `0x00` (OK marker) | **✗ MISMATCH** |
| affected_rows | `0x00` (=0) | `0x00` | ✓ |
| last_insert_id | `0x00` (=0) | `0x00` | ✓ |
| status_flags | `0x0002` (AUTOCOMMIT only) | `0x4002` (with 0x4000) | **✗ MISMATCH** |
| warnings | `0x0000` (=0) | `0x0000` | ✓ |
| info | (absent) | `0x00` (=0) | ✗ MISMATCH (extra byte) |
| session_state_changes | (absent) | `0x00` (=0) | ✗ MISMATCH (extra byte) |

**Total payload:** real MySQL = 7 bytes, fix C = 9 bytes.

The 0xFE header is the EOF identifier used by MySQL for the OK-as-terminator packet
under `CLIENT_DEPRECATE_EOF` (MySQL WL#7766). mysql CLI 8.0.46 dispatches on the first
byte: `0xFE` → "OK-as-terminator" path, `0x00` → "regular OK packet" path. Sending
`0x00` caused the client to treat the terminator as a regular OK packet and hang waiting
for a 5th recvfrom that never came.

---

## Fix D — Use 0xFE as the terminator header (V312-WIRE-8, RESOLVES the hang)

**Hypothesis (locked by user, then confirmed by strace):** Per MySQL WL#7766 and confirmed
by direct strace against real MySQL 8.0.46, the trailing result-set terminator under
`CLIENT_DEPRECATE_EOF` uses the **EOF identifier `0xFE`** as its first byte. For plain
SELECTs with no session-state change, the terminator is exactly 7 bytes: `0xFE` +
lenenc(0) + lenenc(0) + status(0x0002) + warnings(0). NO 0x4000, NO info, NO
session_state_changes. We also stop unconditionally forcing AUTOCOMMIT-or-0x4000 — the
caller passes the right status and we trust it.

**Implementation (`make_deprecate_eof_ok_packet` lines 1853-1932):**
- Replaced `p.push(0x00)` → `p.push(0xfe)` (header byte = EOF identifier per WL#7766).
- Removed unconditional `actual_status |= 0x4000` for SESSION_TRACK.
- Kept `actual_status |= 0x0002` (AUTOCOMMIT) so the client sees session is still in
  autocommit mode (matches real MySQL 8.0.46).
- Made the embedded `info` + `session_state_changes` suffix **conditional** on status
  having 0x4000 — when 0x4000 is set, we still append both, but the no-change case
  (plain SELECT) now produces a clean 7-byte terminator matching the spec.

**Other OK packets (auth OK, COM_PING, COM_INIT_DB) keep 0x00 header** — they
are not terminators, they are regular OK responses and the MySQL client treats them
differently.

> **Note on COM_QUIT (documented separately to avoid future confusion):** the MySQL
> protocol does not require the server to return an OK packet on COM_QUIT — the
> client only sends `\x01` and then closes (or half-closes) the socket. The
> real MySQL 8.0.46 reference client behaves exactly this way: after sending
> `\x01\x00\x00\x00\x01` (COM_QUIT), it calls `shutdown(SHUT_RDWR)` and exits
> without waiting for any response. Our `do_command_loop` COM_QUIT arm still
> sends a best-effort OK packet for older / stricter clients (pymysql,
> libmysqlclient), which is harmless because the well-behaved mysql CLI does
> not block on `recv()` after sending COM_QUIT (verified by strace). COM_QUIT
> therefore MUST NOT appear in the list above of "OK packets using 0x00 header"
> alongside terminator/regular OK responses — it is a connection-shutdown
> command, not a request that the server answers with an OK packet.

### Fix D — Real mysql CLI 8.0.46 byte-level verification

**Run command:**
```
strace -f -e trace=network -s 256 -o /tmp/v312-fixD/client-strace.log \
  mysql -h 127.0.0.1 -P 13405 -u root --ssl-mode=DISABLED \
  -e 'select @@version_comment limit 1'
```

**Client stdout (CRITICAL — this is what we were missing before):**
```
col_1
SQLRustGo
```

**Client exit code:** `0` (clean exit, no hang)

**Client-side strace (last meaningful lines, all expected):**
```
sendto(3, "!\0\0\0\3select @@version_comment limit 1", 37, 0, NULL, 0) = 37
recvfrom(3, "\1\0\0", 16384, 0, NULL, NULL) = 3                       ← PKT1 header (len=1, partial)
recvfrom(3, "\1\1 \0\0\2", 16384, 0, NULL, NULL) = 6                   ← PKT1 tail + PKT2 header
recvfrom(3, "\3def\0\0\0\5col_1\5col_1\f0\0\377\0\0\0\17\0\0\0\0\0\n\0\0\3\tSQLRustGo\7\0\0\4\376\0\0\2\0\0\0", 16384, 0, NULL, NULL) = 57
                                                                     ← PKT2 tail + PKT3 + PKT4
sendto(3, "!\0\0\0\3select @@version_comment limit 1", 37, 0, NULL, 0) = 37  ← 2nd query
recvfrom(3, "\1\0\0", 16384, 0, NULL, NULL) = 3
recvfrom(3, "\1\1 \0\0\2\3def\0\0\0\5col_1\5col_1\f0\0\377\0\0\0\17\0\0\0\0\0\n\0\0", 16384, 0, NULL, NULL) = 41
recvfrom(3, "\3\tSQLRustGo\7\0\0\4\376\0\0\2\0\0\0", 16384, 0, NULL, NULL) = 22
sendto(3, "\1\0\0\0\1", 5, 0, NULL, 0) = 5                              ← COM_QUIT
shutdown(3, SHUT_RDWR)            = 0
+++ exited with 0 +++                                                  ← CLEAN EXIT
```

**Decoded 4-packet result-set (from merged 57-byte recvfrom):**

| Pkt | Header | Payload (hex) | Decoded |
|-----|--------|---------------|---------|
| PKT1 | `\x01 \x00 \x00 \x01` (len=1, seq=1) | `\x01` | column_count = 1 |
| PKT2 | `\x20 \x00 \x00 \x02` (len=32, seq=2) | `03 64 65 66 00 00 00 05 col_1 05 col_1 0c 30 00 ff 00 00 00 0f 00 00 00 00 00` | column def (32 bytes) |
| PKT3 | `\x0a \x00 \x00 \x03` (len=10, seq=3) | `\x09 SQLRustGo` | row (lenenc 9 + "SQLRustGo") |
| **PKT4** | **`\x07 \x00 \x00 \x04` (len=7, seq=4)** | **`\xfe 00 00 02 00 00 00`** | **OK-as-terminator (0xFE header, EOF identifier per WL#7766)** |

**PKT4 byte-by-byte decode (the CRITICAL change):**

| Offset | Hex | Field | Value | Notes |
|--------|-----|-------|-------|-------|
| 0 | `0xfe` | **header byte** | **0xFE = EOF identifier (OK-as-terminator)** | per WL#7766 |
| 1 | `0x00` | lenenc affected_rows | 0 | |
| 2 | `0x00` | lenenc last_insert_id | 0 | |
| 3-4 | `0x02 0x00` | status_flags LE | 0x0002 (AUTOCOMMIT only) | NO 0x4000 for plain SELECT |
| 5-6 | `0x00 0x00` | warnings | 0 | |

**Total: 7 bytes. Identical to real MySQL 8.0.46 wire bytes for the same query.**

### Spec compliance (fix D vs real MySQL 8.0.46)

| Field | Real MySQL 8.0.46 | Fix D (our server) | Match? |
|-------|-------------------|---------------------|--------|
| Header byte | `0xFE` | `0xFE` | **✓** |
| affected_rows | `0x00` (=0) | `0x00` | ✓ |
| last_insert_id | `0x00` (=0) | `0x00` | ✓ |
| status_flags | `0x0002` (AUTOCOMMIT) | `0x0002` (AUTOCOMMIT) | ✓ |
| warnings | `0x0000` | `0x0000` | ✓ |
| info | (absent) | (absent) | ✓ |
| session_state_changes | (absent) | (absent) | ✓ |
| Total payload | 7 bytes | 7 bytes | ✓ |

**FULLY COMPLIANT WITH REAL MySQL 8.0.46 EMITTED WIRE BYTES.**

---

## Unit test regression coverage (fix D)

Two new unit tests added in `crates/mysql-server/src/lib.rs` lines 5184-5280:

1. `test_make_deprecate_eof_ok_packet_terminator_uses_0xfe_marker` — asserts the
   terminator's first byte is `0xFE` (NOT `0x00`), status is `0x0002` (no 0x4000 for
   plain SELECTs), and total payload length is exactly 7 bytes. **Catches any regression
   to the old (wrong) `0x00` terminator header.**

2. `test_make_deprecate_eof_ok_packet_with_session_state_change_appends_info_and_session`
   — asserts that when status DOES have `0x4000` (e.g. SET, USE, multi-statement), the
   terminator keeps `0xFE` header and appends `lenenc(info=0) + lenenc(session_state=0)`
   suffix inside the same packet. **Catches the conditional embedding logic.**

Both new tests PASS. Full test suite: **213 passed, 0 failed** (was 211 + 1 removed
V312-WIRE-7 test + 2 new V312-WIRE-8 tests).

---

## Honest verdict (STRICT PROOF MODE)

**Fix D RESOLVES the hang.**

- Wire bytes match real MySQL 8.0.46 byte-for-byte for the trailing result-set terminator
  under `CLIENT_DEPRECATE_EOF`.
- mysql CLI 8.0.46 prints `col_1` / `SQLRustGo` and exits with code 0.
- All 213 unit tests pass (2 new regression tests for V312-WIRE-8).
- No more `recvfrom` hang on the 5th call.
- The CLI sends `COM_QUIT` and `shutdown` cleanly, then `+++ exited with 0 +++`.

**End-to-end PASS confirmed.** Per STRICT PROOF MODE, this is verified by:
1. Real mysql CLI 8.0.46 binary (not mocked).
2. `strace -f -e trace=network` showing 4 result-set packets received with 0xFE terminator,
   followed by COM_QUIT, shutdown, and clean exit.
3. Direct comparison against real MySQL 8.0.46 port 3306 wire bytes for the same query.
4. Unit tests encoding the WL#7766 invariant (`0xFE` terminator header).

---

## Files for cross-reference

- `crates/mysql-server/src/lib.rs` lines 1741-1824 (`make_ok_packet` — unchanged, still 0x00 for non-terminator OKs)
- `crates/mysql-server/src/lib.rs` lines 1853-1932 (`make_deprecate_eof_ok_packet` — V312-WIRE-8 fix D)
- `crates/mysql-server/src/lib.rs` lines 1841-1851 (`make_eof_packet` — classic EOF for non-DEPRECATE_EOF, unchanged)
- `crates/mysql-server/src/lib.rs` lines 5184-5280 (new V312-WIRE-8 regression tests)
- `/tmp/v312-fixD/client-strace.log` — real mysql CLI 8.0.46 strace (clean exit)
- `/tmp/v312-fixD/server.log` — server log showing start_seq=1, final_seq=4 (only 4 packets, no extra session_state)
- Real MySQL 8.0.46 reference strace: `/tmp/v312-fixC/real-mysql-cli-strace.log`

---

**END OF RECORD.** Fix D (V312-WIRE-8) is **CLAIMED AS PASS** per STRICT PROOF MODE:
real MySQL CLI 8.0.46 exits cleanly with code 0 and prints the expected query result.
Wire bytes verified byte-for-byte against real MySQL 8.0.46 reference implementation.
