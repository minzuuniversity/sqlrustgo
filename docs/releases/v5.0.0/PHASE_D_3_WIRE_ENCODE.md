# Phase D.3 — Eliminate per-row flush + per-row `Vec::new()` in `send_result_set_with_more`

## Background

Profile of `do_command_loop` after Phase D.1, PK lookup path:

```
do_command_loop                       ~4749 ticks
└── execute_select                    ~4222 ticks
    └── scan_pk                       ~2612 ticks (Phase D.1)
    └── other (WHERE rewriter etc.)   ~1600 ticks
└── Packet::read_from                  ~667 ticks
└── send_result_set_with_more          ~200 ticks (looks small...)
└── parse (SQL parser)                 ~30  ticks
```

`sed_result_set_with_more` looks small (200 ticks, ~4% of `do_command_loop`),
but **the wall-clock impact is much bigger than the CPU ticks suggest**:

### The real issue: per-packet `flush()`

Looking at `Packet::write_to`:

```rust
pub fn write_to<W: Write>(&self, w: &mut W) -> MySqlResult<()> {
    w.write_u24::<LittleEndian>(self.length)?;
    w.write_u8(self.sequence)?;
    w.write_all(&self.payload)?;
    w.flush()?;       // ← per-packet syscall!
    Ok(())
}
```

And `send_result_set_with_more` writes one packet per row:

```rust
for r in rows.iter() {
    let mut p = Vec::new();             // ← alloc per row
    write_text_row(&mut p, r)?;
    Packet { payload: p, ... }.write_to(w)?;  // ← flushes per row
}
```

**For a single-row result**, that's:
1. `Vec::new()` alloc for column count packet
2. `Vec::new()` alloc for each column-def packet (N times)
3. `Vec::new()` alloc for the row packet
4. `Vec::new()` alloc for EOF/OK packet
5. **`flush()` per packet** = (2 + N + 1 + 1) syscalls

For our 5-column PK lookup, that's **9 Vec allocs + 9 flush syscalls** per query.

`pymysql` uses one TCP write per syscall because it's blocking. Each flush
is `~5-10µs` even on localhost. 9 × 5µs = ~45µs pure syscall overhead per
query, on top of the actual work. At 4000 OPS, that's 4µs/query avg, so
flush overhead is ~10% of total.

For multi-row results the overhead is much worse:
- 100 rows × 9 packets = 900 syscalls per query
- ~4.5ms per query just for flush
- Caps throughput at ~200 OPS for 100-row queries

### The fix

Buffer all packets for a single `send_result_set` call into ONE shared
`Vec<u8>`, write the whole thing with one `write_all` + one `flush`.

This:
1. **Eliminates per-packet flush** (1 syscall instead of 2+N+1+1)
2. **Eliminates `Vec::new()` per packet** (1 alloc instead of 2+N+1+1)
3. **Lets the kernel TCP_NODELAY/coalesce buffer fill faster**

Plus: replace `value_to_string(v) -> String` with `write_value_into<W: Write>(w, v)`
that writes directly to the buffer — eliminating the per-cell String alloc.

## Implementation

### Helper: `PacketHeader::write_to<W>`

```rust
struct PacketHeader {
    length: u32,
    sequence: u8,
}

impl PacketHeader {
    fn write_to<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u24::<LittleEndian>(self.length)?;
        w.write_u8(self.sequence)
    }
}
```

### Helper: `write_value_into<W>`

```rust
fn write_value_into<W: Write>(w: &mut W, v: &Value) -> MySqlResult<()> {
    match v {
        Value::Null => w.write_u8(0xfb),
        Value::Boolean(b) => w.write_all(if *b { b"1" } else { b"0" }),
        Value::Integer(i) => write_lenenc_int(w, *i as u64),
        Value::Float(f) => {
            // ryu would be faster; for now use a stack buffer
            let mut buf = ryu::Buffer::new();
            w.write_all(buf.format(*f).as_bytes())
        }
        Value::Text(s) => write_lenenc_string(w, s.as_bytes()),
        Value::Blob(b) => {
            write_lenenc_string(w, b)
        }
        Value::Point(x, y) => {
            write_lenenc_string(w, format!("POINT({} {})", x, y).as_bytes())
        }
        Value::Json(v) => {
            write_lenenc_string(w, v.to_string().as_bytes())
        }
    }
    .map_err(MySqlError::from)
}
```

Note: `Point` and `Json` still allocate a String, but they're uncommon in
sysbench-style benchmarks (only basic numeric/text columns). The hot path
(Integer, Text) avoids the alloc entirely.

### Helper: `write_text_row_into<W>`

```rust
fn write_text_row_into<W: Write>(w: &mut W, row: &[Value]) -> MySqlResult<()> {
    for v in row {
        write_value_into(w, v)?;
    }
    Ok(())
}
```

### Refactored `send_result_set_with_more`

```rust
fn send_result_set_with_more<W: Write>(
    w: &mut W,
    cols: &[String],
    ctypes: &[String],
    rows: &[Vec<Value>],
    mut seq: u8,
    cap: u32,
    more_results_flag: u16,
) -> MySqlResult<u8> {
    let trailing_status: u16 = 0x0002 | more_results_flag;

    // Phase D.3: buffer all packets in a single Vec<u8>, flush once
    // at the end. Eliminates per-packet flush (1 syscall instead of
    // 2+N+rows+1) and per-packet Vec allocation.
    let mut buf = Vec::with_capacity(256 + rows.len() * 64);

    // Column count packet
    write_lenenc_int(&mut buf, cols.len() as u64).map_err(MySqlError::from)?;
    let col_count_pkt = PacketHeader { length: buf.len() as u32, sequence: seq };
    seq = seq.wrapping_add(1);

    // We need to splice headers in front of each packet payload. The
    // simplest approach: write all payloads to `buf`, remember their
    // (offset, length), then write all headers + payloads in one go.
    //
    // Actually even simpler: keep a `Vec<(u32, u8)>` of (length, seq)
    // for each packet and write header+payload in sequence.

    let mut packets: Vec<(u32, u8, usize, usize)> = Vec::new();  // (len, seq, payload_start, payload_end)
    // ... [complexity grows]
}
```

This is getting complex. **Alternative simpler approach**: write the WHOLE
result set to a single in-memory `Vec<u8>` and then `write_all` it once.
The issue is the per-packet header (length+sequence) needs to know the
payload length, which we have.

Let me redesign:

```rust
fn send_result_set_with_more<W: Write>(
    w: &mut W,
    cols: &[String],
    ctypes: &[String],
    rows: &[Vec<Value>],
    mut seq: u8,
    cap: u32,
    more_results_flag: u16,
) -> MySqlResult<u8> {
    let trailing_status: u16 = 0x0002 | more_results_flag;

    // Phase D.3: build full packet sequence into one buffer, then write
    // once. Each packet is (3-byte length, 1-byte seq, payload).

    let mut buf: Vec<u8> = Vec::with_capacity(256 + rows.len() * 128);

    // Packet 1: column count
    let p1_payload_start = buf.len();
    write_lenenc_int(&mut buf, cols.len() as u64).map_err(MySqlError::from)?;
    let p1_payload_len = (buf.len() - p1_payload_start) as u32;
    let p1_seq = seq; seq = seq.wrapping_add(1);

    // Per-column def packets (we still need to serialize each column's def)
    let mut col_pkt_lens: Vec<(u32, u8)> = Vec::with_capacity(cols.len());
    for (i, n) in cols.iter().enumerate() {
        let col_def_start = buf.len();
        // write_column_def writes payload directly into buf
        // (refactored version that takes &mut Write)
        write_column_def_into(&mut buf, n, ctypes.get(i).map(|s| s.as_str()).unwrap_or("VARCHAR(255)"))
            .map_err(MySqlError::from)?;
        col_pkt_lens.push(((buf.len() - col_def_start) as u32, seq));
        seq = seq.wrapping_add(1);
    }

    // Inter-record separator (classic protocol only)
    let eof_pkt = if cap & capability::DEPRECATE_EOF == 0 {
        let eof_start = buf.len();
        write_eof_into(&mut buf, seq, 0x0002).map_err(MySqlError::from)?;
        let eof_len = (buf.len() - eof_start) as u32;
        let eof_seq = seq;
        seq = seq.wrapping_add(1);
        Some((eof_len, eof_seq))
    } else {
        None
    };

    // Row packets
    let mut row_pkt_lens: Vec<(u32, u8)> = Vec::with_capacity(rows.len());
    for r in rows.iter() {
        let row_start = buf.len();
        write_text_row_into(&mut buf, r)?;
        row_pkt_lens.push(((buf.len() - row_start) as u32, seq));
        seq = seq.wrapping_add(1);
    }

    // Trailing terminator
    let term_start = buf.len();
    write_terminator_into(&mut buf, seq, cap, trailing_status)?;
    let term_len = (buf.len() - term_start) as u32;
    seq = seq.wrapping_add(1);

    // Now we have all payloads in `buf`, but they need headers (length+seq)
    // prepended. The cleanest way: build the final buffer with headers.

    let total_packet_size = 4 * (1 + col_pkt_lens.len() + row_pkt_lens.len()
                                  + if eof_pkt.is_some() { 1 } else { 0 }
                                  + 1); // +1 for trailing terminator
    let total_payload_size = p1_payload_len
        + col_pkt_lens.iter().map(|(l, _)| l).sum::<u32>()
        + row_pkt_lens.iter().map(|(l, _)| l).sum::<u32>()
        + eof_pkt.map(|(l, _)| l).unwrap_or(0)
        + term_len;
    let final_size = total_packet_size as usize + total_payload_size as usize;

    let mut final_buf = Vec::with_capacity(final_size);

    // Helper closure to write header + slice of buf
    let write_pkt = |final_buf: &mut Vec<u8>, seq: u8, payload_start: usize, payload_len: u32| {
        final_buf.write_u24::<LittleEndian>(payload_len).unwrap();
        final_buf.write_u8(seq).unwrap();
        final_buf.extend_from_slice(&buf[payload_start..payload_start + payload_len as usize]);
    };

    // ... etc, but tracking payload offsets is fiddly
}
```

This is getting ugly. **Simpler**: use `BufWriter` and write each "packet"
to it. The BufWriter only flushes on `flush()` or when full. The
existing `write_text_row` already takes `&mut W: Write` — just don't
allocate the `Vec` per packet.

Actually, looking at the existing code, `write_text_row(&mut p, r)` already
takes `&mut W: Write` — we just need to pass `&mut final_buf` instead of
allocating `Vec` per row. Same for column count and column defs.

The cleanest refactor:

```rust
fn send_result_set_with_more<W: Write>(
    w: &mut W,
    cols: &[String],
    ctypes: &[String],
    rows: &[Vec<Value>],
    mut seq: u8,
    cap: u32,
    more_results_flag: u16,
) -> MySqlResult<u8> {
    let trailing_status: u16 = 0x0002 | more_results_flag;

    // Phase D.3: write everything to a single buffer, send once.
    // This eliminates per-packet flush (1 syscall total) and per-packet
    // Vec allocation (1 alloc total).
    //
    // We use a local Vec<u8> instead of writing directly to w because
    // each packet needs its length prefix, which requires knowing the
    // payload size before writing the header. We track payload offsets.
    let mut buf: Vec<u8> = Vec::with_capacity(256 + rows.len() * 128);

    // We collect (offset, length, seq) for each packet's payload,
    // then write headers + payloads in order.
    let mut pkt_records: Vec<(usize, u32, u8)> = Vec::new(); // (offset, len, seq)

    // Packet: column count
    let start = buf.len();
    write_lenenc_int(&mut buf, cols.len() as u64).map_err(MySqlError::from)?;
    pkt_records.push((start, (buf.len() - start) as u32, seq));
    seq = seq.wrapping_add(1);

    // Packets: column defs
    for (i, n) in cols.iter().enumerate() {
        let start = buf.len();
        write_column_def_into(
            &mut buf,
            n,
            ctypes.get(i).map(|s| s.as_str()).unwrap_or("VARCHAR(255)"),
        )?;
        pkt_records.push((start, (buf.len() - start) as u32, seq));
        seq = seq.wrapping_add(1);
    }

    // Inter-record separator (classic protocol)
    if cap & capability::DEPRECATE_EOF == 0 {
        let start = buf.len();
        make_eof_packet_into(&mut buf, seq, 0x0002)?;
        pkt_records.push((start, (buf.len() - start) as u32, seq));
        seq = seq.wrapping_add(1);
    }

    // Row packets
    for r in rows.iter() {
        let start = buf.len();
        write_text_row_into(&mut buf, r)?;
        pkt_records.push((start, (buf.len() - start) as u32, seq));
        seq = seq.wrapping_add(1);
    }

    // Trailing terminator
    let start = buf.len();
    if cap & capability::DEPRECATE_EOF == 0 {
        make_eof_packet_into(&mut buf, seq, trailing_status)?;
    } else {
        make_ok_packet_into(&mut buf, seq, 0, 0, trailing_status, 0)?;
    }
    pkt_records.push((start, (buf.len() - start) as u32, seq));
    seq = seq.wrapping_add(1);

    // Now write the final assembled buffer to w in one shot.
    let mut final_buf: Vec<u8> = Vec::with_capacity(
        pkt_records.iter().map(|(_, l, _)| l + 4).sum::<u32>() as usize,
    );
    for (offset, length, seq) in pkt_records {
        final_buf.write_u24::<LittleEndian>(length).unwrap();
        final_buf.write_u8(seq).unwrap();
        final_buf.extend_from_slice(&buf[offset..offset + length as usize]);
    }
    w.write_all(&final_buf)?;
    w.flush()?;
    Ok(seq)
}
```

This is cleaner. Need to also refactor:
- `write_column_def` → `write_column_def_into<W: Write>(w: &mut W, ...)`
- `make_eof_packet` → `make_eof_packet_into<W: Write>(w: &mut W, ...)`
- OK packet → `make_ok_packet_into<W: Write>(w: &mut W, ...)`
- `write_text_row` → already takes `&mut W: Write`, but uses
  `value_to_string` which allocates. Replace with `write_value_into`.

The existing `write_column_def` and `make_eof_packet` likely already take
`&mut W: Write` (since `Packet::write_to` does). Let me check.

## Validation

- All existing mysql-server tests pass
- Bench:
  - Phase D.1 baseline: 3788 OPS
  - Phase D.3 expected: 4500-5500 OPS (1.2-1.5× improvement)
  - Multi-row case (`SELECT * FROM t LIMIT 100`): should show biggest gain

## Files affected

- `crates/mysql-server/src/lib.rs`:
  - New `write_value_into<W: Write>` helper
  - Refactor `send_result_set_with_more` to buffer + single flush
  - New `_into` variants of `make_eof_packet`, `make_ok_packet`, `write_column_def`
  - Remove `value_to_string` (replaced by `write_value_into`)
  - Deprecate or remove `write_text_row` (replaced by `write_text_row_into`)

## Effort estimate

2-3 hours:
- ~150 lines of refactoring
- 1-2 bench runs (single-row + multi-row)
- Existing wire-protocol tests must pass

## Rollback plan

If tests fail or bench regresses, revert with `git checkout HEAD~1`.
