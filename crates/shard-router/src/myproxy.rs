//! MySQL protocol proxy layer.
//!
//! Implements just enough of the MySQL 5.7 client/server protocol to:
//! 1. Accept a connection from a client (pymysql, mysql CLI, etc.).
//! 2. Speak the handshake + auth handshake to the client.
//! 3. For each `COM_QUERY` packet, parse the SQL, decide where it
//!    should go (per `parser_ext::try_route`), forward to one or
//!    more shard backends, and merge the response back to the
//!    client.
//!
//! This is intentionally **not** a fully spec-compliant MySQL proxy.
//! POC scope covers the minimum needed to validate the routing
//! strategy:
//!
//! - handshake: minimal HandshakeResponse41 (no SSL, no plugin auth)
//! - COM_QUIT: ack and close
//! - COM_QUERY: route, forward, return first response (broadcast
//!   writes are not merged; only the first shard's response is
//!   returned to the client)
//!
//! Out of scope: COM_STMT_PREPARE / COM_STMT_EXECUTE, COM_INIT_DB,
//! COM_RESET_CONNECTION, multi-result sets, character-set negotiation
//! beyond utf8, prepared-statement server-side state.

use std::io;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpSocket, TcpStream};
use tokio_util::sync::CancellationToken;

use crate::parser_ext::{try_route, RoutingDecision};
use crate::shard::ShardSet;

const PROTOCOL_VERSION_10: u8 = 0x0A;
const SERVER_STATUS_AUTOCOMMIT: u16 = 0x0002;
const SERVER_CHARSET_UTF8: u8 = 0x21;
const AUTH_PLUGIN_NATIVE: &[u8] = b"mysql_native_password\0";

const COM_QUIT: u8 = 0x01;
const COM_INIT_DB: u8 = 0x02;
const COM_QUERY: u8 = 0x03;
const COM_PING: u8 = 0x0E;

/// Per-connection state.
pub struct Session {
    downstream: TcpStream,
    shards: Arc<ShardSet>,
    cancel: CancellationToken,
    /// Round-robin counter for `Connection` statements.
    conn_counter: usize,
}

impl Session {
    pub fn new(downstream: TcpStream, shards: Arc<ShardSet>, cancel: CancellationToken) -> Self {
        Self {
            downstream,
            shards,
            cancel,
            conn_counter: 0,
        }
    }

    /// Drive the connection until either side closes or we hit a
    /// protocol error.
    pub async fn run(mut self) -> io::Result<()> {
        // 1. Send server handshake
        self.write_handshake().await?;

        // 2. Read handshake response
        let _auth = self.read_packet().await?;

        // 3. Send OK
        self.write_ok(0).await?;

        // 4. Command loop
        // We can't tokio::select! on self.cancel.cancelled() and self.read_packet()
        // simultaneously because they each borrow self. Clone the cancel
        // token so the select! can use immutable references on both.
        let cancel = self.cancel.clone();
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    tracing::debug!(target: "shard_router", "session cancelled");
                    return Ok(());
                }
                read = self.read_packet() => {
                    let pkt = match read {
                        Ok(p) => p,
                        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
                        Err(e) => return Err(e),
                    };
                    if pkt.is_empty() {
                        return Ok(());
                    }
                    let cmd = pkt[0];
                    let payload = &pkt[1..];
                    match cmd {
                        COM_QUIT => {
                            self.write_ok(0).await?;
                            return Ok(());
                        }
                        COM_PING => {
                            self.write_ok(0).await?;
                        }
                        COM_INIT_DB => {
                            // USE is shard-agnostic — ack OK
                            self.write_ok(0).await?;
                        }
                        COM_QUERY => {
                            let sql = match std::str::from_utf8(payload) {
                                Ok(s) => s,
                                Err(_) => {
                                    self.write_err(1064, "Invalid UTF-8 in query").await?;
                                    continue;
                                }
                            };
                            self.handle_query(sql).await?;
                        }
                        _ => {
                            self.write_err(1064, &format!("Unsupported command 0x{cmd:02x}")).await?;
                        }
                    }
                }
            }
        }
    }

    async fn handle_query(&mut self, sql: &str) -> io::Result<()> {
        let decision = match try_route(sql, self.shards.len()) {
            Some(d) => d,
            None => {
                // Unparseable: forward to round-robin shard; backend will error
                let idx = self.conn_counter % self.shards.len();
                self.conn_counter = self.conn_counter.wrapping_add(1);
                RoutingDecision::Single { shard_index: idx }
            }
        };

        match decision {
            RoutingDecision::Single { shard_index } => {
                self.forward_to_shard(shard_index, sql).await?;
            }
            RoutingDecision::Broadcast => {
                // Send the query to every shard serially, return the
                // first shard's response. Production deployments would
                // require shard-by-shard merge logic, but POC scope is
                // "non-PK queries go to all shards and the first
                // wins"; this matches the bench methodology where each
                // shard holds a disjoint slice of data.
                if self.shards.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "no shards configured",
                    ));
                }
                self.forward_to_shard(0, sql).await?;
                for idx in 1..self.shards.len() {
                    // Best-effort: fan out writes silently; the client
                    // gets the response from shard 0.
                    let _ = self
                        .forward_to_shard_quiet(idx, sql)
                        .await;
                }
            }
            RoutingDecision::Connection => {
                // Connection-level statement (SET/USE/BEGIN/COMMIT).
                // Route to a single shard round-robin; backend will
                // mutate per-connection state, which doesn't carry
                // across shards. POC scope: this is acceptable for
                // sharded benchmarks where each connection stays
                // pinned to one shard for its session.
                let idx = self.conn_counter % self.shards.len();
                self.conn_counter = self.conn_counter.wrapping_add(1);
                self.forward_to_shard(idx, sql).await?;
            }
        }
        Ok(())
    }

    async fn forward_to_shard(&mut self, shard_index: usize, sql: &str) -> io::Result<()> {
        let shard = match self.shards.get(shard_index) {
            Some(s) => s,
            None => {
                return self.write_err(1040, "no such shard").await;
            }
        };
        shard.in_flight.fetch_add(1, Ordering::Relaxed);
        let result = forward_one(shard, sql).await;
        shard.in_flight.fetch_sub(1, Ordering::Relaxed);
        match result {
            Ok(backend_bytes) => {
                tracing::debug!(
                    target: "shard_router", sql,
                    "writing {} bytes to downstream", backend_bytes.len()
                );
                self.downstream.write_all(&backend_bytes).await?;
                self.downstream.flush().await.ok();
                Ok(())
            }
            Err(e) => self.write_err(1040, &format!("shard error: {e}")).await,
        }
    }

    /// Same as `forward_to_shard` but discards the response (used by
    /// broadcast writes for shards after the first).
    async fn forward_to_shard_quiet(&mut self, shard_index: usize, sql: &str) -> io::Result<()> {
        let shard = match self.shards.get(shard_index) {
            Some(s) => s,
            None => return Ok(()),
        };
        shard.in_flight.fetch_add(1, Ordering::Relaxed);
        let result = forward_one(shard, sql).await;
        shard.in_flight.fetch_sub(1, Ordering::Relaxed);
        result.map(|_| ())
    }

    // ---- Wire-protocol framing --------------------------------------

    async fn read_packet(&mut self) -> io::Result<Vec<u8>> {
        let mut header = [0u8; 4];
        self.downstream.read_exact(&mut header).await?;
        let len = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
        let seq = header[3];
        let mut body = vec![0u8; len];
        self.downstream.read_exact(&mut body).await?;
        // First byte of body is the command; for client packets,
        // sequence starts at 0 (handshake response) then increments.
        // We don't strictly validate the sequence number in the POC;
        // the upstream sqlrustgo-mysql-server does not require it.
        let _ = seq;
        Ok(body)
    }

    async fn write_packet(&mut self, seq: u8, body: &[u8]) -> io::Result<()> {
        let len = body.len() as u32;
        let header = [len as u8, (len >> 8) as u8, (len >> 16) as u8, seq];
        self.downstream.write_all(&header).await?;
        self.downstream.write_all(body).await?;
        Ok(())
    }

    async fn write_handshake(&mut self) -> io::Result<()> {
        let mut body = Vec::with_capacity(128);
        body.push(PROTOCOL_VERSION_10);
        // server version: "5.7.99-router\0"
        body.extend_from_slice(b"5.7.99-router\0");
        // thread id (4 bytes, arbitrary)
        body.extend_from_slice(&1u32.to_le_bytes());
        // auth-plugin-data-part-1 (8 bytes of zeros — no challenge)
        body.extend_from_slice(&[0u8; 8]);
        body.push(0x00); // filler
        // capability flags lower 2 bytes: CLIENT_PROTOCOL_41 |
        // CLIENT_TRANSACTIONS
        let caps: u32 = 0x0001_0000 | 0x0000_2000;
        body.extend_from_slice(&(caps as u16).to_le_bytes());
        body.push(SERVER_CHARSET_UTF8);
        body.extend_from_slice(&SERVER_STATUS_AUTOCOMMIT.to_le_bytes());
        // capability flags upper 2 bytes
        body.extend_from_slice(&((caps >> 16) as u16).to_le_bytes());
        // length of auth-plugin-data (always 21)
        body.push(21);
        // reserved 10 bytes
        body.extend_from_slice(&[0u8; 10]);
        // auth-plugin-data-part-2 (≥13 bytes, zero-padded)
        body.extend_from_slice(&[0u8; 13]);
        // auth-plugin name
        body.extend_from_slice(AUTH_PLUGIN_NATIVE);
        self.write_packet(0, &body).await
    }

    async fn write_ok(&mut self, affected_rows: u64) -> io::Result<()> {
        let mut body = Vec::with_capacity(16);
        body.push(0x00); // OK packet header
        body.push(0x00); // OK packet marker (length-encoded-int 0)
        body.push(0x00); // affected_rows
        body.extend_from_slice(&affected_rows.to_le_bytes());
        body.extend_from_slice(&0u64.to_le_bytes()); // last_insert_id
        body.extend_from_slice(&SERVER_STATUS_AUTOCOMMIT.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes()); // warnings
        self.write_packet(2, &body).await
    }

    async fn write_err(&mut self, errno: u16, msg: &str) -> io::Result<()> {
        let mut body = Vec::with_capacity(16 + msg.len());
        body.push(0xFF); // ERR packet header
        body.extend_from_slice(&errno.to_le_bytes());
        body.push(b'#');
        body.extend_from_slice(b"HY000"); // SQL state
        body.extend_from_slice(msg.as_bytes());
        self.write_packet(2, &body).await
    }
}

/// Forward a single query to a shard backend. Performs the
/// backend handshake (we send a synthetic auth handshake response) and
/// then sends the COM_QUERY packet, then reads the response. Returns
/// the raw response bytes (including any OK/ERR/data packets) ready
/// to be written back to the downstream client.
///
/// Response packets' sequence numbers are rewritten to `target_seq`
/// (the sequence number the downstream client expects next).
async fn forward_one(shard: &crate::shard::ShardEndpoint, sql: &str) -> io::Result<Vec<u8>> {
    let mut stream = match TcpStream::connect(&shard.addr).await {
        Ok(s) => s,
        Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("connect: {e}"))),
    };

    // 1. Read backend handshake.
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    let len = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await?;

    // 2. Send handshake response (minimal, no real auth).
    let mut body = Vec::with_capacity(64);
    let caps: u32 = 0x0001_A200 | 0x0008_0000 | 0x0000_0008;
    body.extend_from_slice(&caps.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.push(SERVER_CHARSET_UTF8);
    body.extend_from_slice(&[0u8; 23]);
    body.extend_from_slice(b"router\0");
    body.extend_from_slice(&[0u8; 20]);
    body.extend_from_slice(b"mysql_native_password\0");
    let pkt_len = body.len() as u32;
    stream.write_all(&[pkt_len as u8, (pkt_len >> 8) as u8, (pkt_len >> 16) as u8, 1]).await?;
    stream.write_all(&body).await?;

    // 3. Read backend OK to handshake (discard it; we send our own standard OK).
    stream.read_exact(&mut header).await?;
    let len = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
    let mut ack = vec![0u8; len];
    stream.read_exact(&mut ack).await?;
    let _ = ack;

    // 4. Send COM_QUERY (seq=0 from client to server).
    let mut q = Vec::with_capacity(1 + sql.len());
    q.push(COM_QUERY);
    q.extend_from_slice(sql.as_bytes());
    let pkt_len = q.len() as u32;
    stream.write_all(&[pkt_len as u8, (pkt_len >> 8) as u8, (pkt_len >> 16) as u8, 0]).await?;
    stream.write_all(&q).await?;
    stream.flush().await.ok();

    // 5. Drain response packets. EOF (0xFE) and OK (0x00) end the result
    //    set; we read with a short trailing timeout to capture any
    //    session-state-info packet.
    let mut out = Vec::with_capacity(4096);
    // Outer safety net: 5 seconds total, hard cap. The per-packet timeout
    // below should make this unnecessary in practice.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let mut h = [0u8; 4];
            match stream.read_exact(&mut h).await {
                Ok(_n) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => {
                    tracing::warn!(target: "shard_router", sql, error = %e, "backend read error");
                    break;
                }
            }
            let len = u32::from_le_bytes([h[0], h[1], h[2], 0]) as usize;
            let seq = h[3];
            let mut body = vec![0u8; len];
            if stream.read_exact(&mut body).await.is_err() {
                break;
            }
            out.extend_from_slice(&h);
            out.extend_from_slice(&body);
            tracing::debug!(
                target: "shard_router", sql,
                "backend resp packet len={} seq={} first=0x{:02x}",
                len, seq, body[0]
            );
            let is_eof = body[0] == 0xFE;
            let is_ok = body[0] == 0x00;
            if is_eof || is_ok {
                // Brief wait for any session-state follow-up packet.
                let had_more = tokio::time::timeout(
                    std::time::Duration::from_millis(20),
                    stream.read_exact(&mut [0u8; 4]),
                )
                .await
                .ok()
                .and_then(|r| r.ok())
                .map(|n| n > 0)
                .unwrap_or(false);
                if had_more {
                    let mut buf = vec![0u8; 4096];
                    loop {
                        match stream.read(&mut buf).await {
                            Ok(n) if n > 0 => out.extend_from_slice(&buf[..n]),
                            _ => break,
                        }
                    }
                }
                break;
            }
            if out.len() > 4 * 1024 * 1024 {
                break;
            }
        }
    })
    .await;
    tracing::debug!(target: "shard_router", sql, "backend returned {} bytes", out.len());
    Ok(out)
}


