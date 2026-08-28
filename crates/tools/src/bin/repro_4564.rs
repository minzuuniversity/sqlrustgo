#![allow(dead_code)]
//! repro_4564 — Reproduce issue #4564 (sysbench 8-worker 4/8 TLS-handshake stall).
//!
//! Mimics sysbench's per-worker wire flow:
//!   1. TCP connect
//!   2. TLS handshake (rustls client)
//!   3. Read MySQL server handshake packet
//!   4. Send handshake response over TLS
//!   5. Authenticate (skip_auth = no password)
//!   6. Optionally send a probe query (SELECT 1)
//!
//! Measures per-stage latency and per-worker success/failure across N
//! concurrent workers. All workers spawn at a Barrier so connections
//! open simultaneously, mirroring sysbench oltp_read_write behavior.
//!
//! Usage:
//!   repro_4564 --threads 8 --probe-query "SELECT 1" --hold-secs 35 \
//!              --host 127.0.0.1 --port 3400
//!
//! Output: CSV (stdout) of per-worker (worker_id,stage,latency_us,status)
//! tuples + per-worker summary on stderr.

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use rustls::client::danger::{ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, Stream};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(name = "repro_4564", about = "Reproduce #4564 4/8 TLS-handshake stall")]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 3400)]
    port: u16,
    #[arg(long, default_value_t = 8)]
    threads: usize,
    /// Skip TLS handshake entirely (test pure TCP path).
    #[arg(long)]
    no_tls: bool,
    /// Send this query after auth, then close.
    #[arg(long, default_value = "SELECT 1")]
    probe_query: String,
    /// Hold the connection open this many seconds after auth before closing.
    /// Default 35 = sysbench worker init timeout.
    #[arg(long, default_value_t = 35)]
    hold_secs: u64,
    /// Skip sending the probe query; just open + auth + hold.
    #[arg(long)]
    no_probe: bool,
    /// User name to send in handshake response.
    #[arg(long, default_value = "root")]
    user: String,
}

#[derive(Clone, Copy, Debug)]
enum Stage {
    TcpConnect,
    TlsHandshake,
    MysqlHandshakeRead,
    HandshakeRespSend,
    AuthOk,
    ProbeQuery,
}

fn stage_name(s: Stage) -> &'static str {
    match s {
        Stage::TcpConnect => "tcp_connect",
        Stage::TlsHandshake => "tls_handshake",
        Stage::MysqlHandshakeRead => "mysql_handshake_read",
        Stage::HandshakeRespSend => "handshake_resp_send",
        Stage::AuthOk => "auth_ok",
        Stage::ProbeQuery => "probe_query",
    }
}

/// Read MySQL packet: 3-byte length LE + 1-byte seq + body.
fn read_packet<R: Read>(r: &mut R) -> Result<(u32, u8, Vec<u8>)> {
    let mut hdr = [0u8; 4];
    r.read_exact(&mut hdr).context("read packet header")?;
    let len = u32::from_le_bytes([hdr[0], hdr[1], hdr[2], 0]);
    let seq = hdr[3];
    let mut body = vec![0u8; len as usize];
    r.read_exact(&mut body).context("read packet body")?;
    Ok((len, seq, body))
}

/// Write a MySQL packet: 3-byte length LE + 1-byte seq + body.
fn write_packet<W: Write>(w: &mut W, seq: u8, body: &[u8]) -> Result<()> {
    let len = body.len() as u32;
    w.write_all(&len.to_le_bytes()[..3])?;
    w.write_all(&[seq])?;
    w.write_all(body)?;
    w.flush()?;
    Ok(())
}

/// Stub verifier that accepts any certificate (server uses self-signed).
#[derive(Debug)]
struct NoCertVerifier;
impl ServerCertVerifier for NoCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

fn insecure_tls_config() -> Arc<ClientConfig> {
    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertVerifier))
        .with_no_client_auth();
    Arc::new(config)
}

fn run_worker(args: Arc<Args>, worker_id: usize, barrier_start: Instant) -> Result<()> {
    let t_total_start = Instant::now();
    let offset_from_barrier = t_total_start.duration_since(barrier_start);
    eprintln!(
        "[w{:02}] start (offset from barrier: {:?})",
        worker_id, offset_from_barrier
    );

    let t = Instant::now();
    let tcp = TcpStream::connect((args.host.as_str(), args.port))
        .with_context(|| format!("TCP connect to {}:{}", args.host, args.port))?;
    tcp.set_nodelay(true).ok();
    let tcp_connect_us = t.elapsed().as_micros();
    println!(
        "{},w{:02},{},{},ok",
        worker_id,
        worker_id,
        stage_name(Stage::TcpConnect),
        tcp_connect_us
    );

    if args.no_tls {
        return run_mysql_handshake_plain(tcp, args, worker_id);
    }

    let mut tcp = tcp;
    // ── Read plaintext MySQL handshake packet first (sent by server
    // before it knows whether we'll do STARTTLS) ──
    let t = Instant::now();
    let (_plen, _pseq, body) = read_packet(&mut tcp).context("read plaintext server handshake")?;
    if body.is_empty() || body[0] != 0x0a {
        return Err(anyhow!(
            "not a server handshake packet (body[0]=0x{:x}, len={})",
            body.first().copied().unwrap_or(0),
            body.len()
        ));
    }
    let plaintext_hs_us = t.elapsed().as_micros();
    println!(
        "{},w{:02},plaintext_handshake,{},ok",
        worker_id, worker_id, plaintext_hs_us
    );
    // ── Now send STARTTLS (32-byte SSL request) over plaintext ──
    let t = Instant::now();
    let ssl_cap: u32 = 0x0801_a88d; // CLIENT_SSL (0x800) + typical cap
    let mut ssl_req: Vec<u8> = Vec::new();
    ssl_req.extend_from_slice(&ssl_cap.to_le_bytes());
    ssl_req.extend_from_slice(&0x00000040_u32.to_le_bytes());
    ssl_req.extend_from_slice(&[0u8; 24]);
    write_packet(&mut tcp, 0, &ssl_req).context("write STARTTLS request")?;
    let starttls_us = t.elapsed().as_micros();
    println!(
        "{},w{:02},starttls_req,{},ok",
        worker_id, worker_id, starttls_us
    );
    let server_name =
        ServerName::try_from(args.host.clone()).map_err(|_| anyhow!("invalid server name"))?;
    let config = insecure_tls_config();
    let mut conn = ClientConnection::new(config, server_name)
        .map_err(|e| anyhow!("ClientConnection::new: {}", e))?;
    let mut tls = Stream::new(&mut conn, &mut tcp);
    let tls_handshake_us = t.elapsed().as_micros();
    println!(
        "{},w{:02},{},{},ok",
        worker_id,
        worker_id,
        stage_name(Stage::TlsHandshake),
        tls_handshake_us
    );
    // Skip reading MySQL handshake over TLS — we already read it in
    // plaintext above. Next: send full handshake response over TLS.

    let t = Instant::now();
    let cap: u32 = 0x0001_a88d;
    let mut payload: Vec<u8> = Vec::new();
    payload.extend_from_slice(&cap.to_le_bytes());
    payload.extend_from_slice(&0x00000040_u32.to_le_bytes());
    payload.push(33);
    payload.extend_from_slice(&[0u8; 23]);
    payload.extend_from_slice(args.user.as_bytes());
    payload.push(0);
    payload.push(0);
    payload.extend_from_slice(b"sbtest");
    payload.push(0);
    write_packet(&mut tls, 1, &payload).context("write handshake response over TLS")?;
    let handshake_resp_send_us = t.elapsed().as_micros();
    println!(
        "{},w{:02},{},{},ok",
        worker_id,
        worker_id,
        stage_name(Stage::HandshakeRespSend),
        handshake_resp_send_us
    );

    let t = Instant::now();
    let (_len2, _seq2, body2) = read_packet(&mut tls).context("read auth response over TLS")?;
    if body2.is_empty() {
        return Err(anyhow!("empty auth response"));
    }
    let auth_ok_us = t.elapsed().as_micros();
    let auth_ok_status = if body2[0] == 0x00 { "ok" } else { "fail" };
    println!(
        "{},w{:02},{},{},{}",
        worker_id,
        worker_id,
        stage_name(Stage::AuthOk),
        auth_ok_us,
        auth_ok_status
    );
    if body2[0] != 0x00 {
        return Err(anyhow!(
            "auth failed: {:02x?}",
            &body2[..body2.len().min(64)]
        ));
    }

    if args.hold_secs > 0 {
        std::thread::sleep(Duration::from_secs(args.hold_secs));
    }

    if !args.no_probe {
        let t = Instant::now();
        write_packet(&mut tls, 0, args.probe_query.as_bytes())
            .context("write probe query over TLS")?;
        let (_l, _s, resp) = read_packet(&mut tls).context("read probe response")?;
        let probe_query_us = t.elapsed().as_micros();
        let probe_status = if !resp.is_empty() { "ok" } else { "fail" };
        println!(
            "{},w{:02},{},{},{}",
            worker_id,
            worker_id,
            stage_name(Stage::ProbeQuery),
            probe_query_us,
            probe_status
        );
    }

    let t_total = t_total_start.elapsed();
    eprintln!(
        "[w{:02}] done (total = {:?}, from barrier = {:?})",
        worker_id,
        t_total,
        barrier_start.elapsed()
    );
    Ok(())
}

fn run_mysql_handshake_plain(tcp: TcpStream, args: Arc<Args>, worker_id: usize) -> Result<()> {
    let mut tcp = tcp;
    let (_len, _seq, body) = read_packet(&mut tcp).context("read mysql handshake")?;
    if body.len() < 4 || body[4] != 0x0a {
        return Err(anyhow!("not a server handshake"));
    }

    let cap: u32 = 0x0001_a88d;
    let mut payload: Vec<u8> = Vec::new();
    payload.extend_from_slice(&cap.to_le_bytes());
    payload.extend_from_slice(&0x00000040_u32.to_le_bytes());
    payload.push(33);
    payload.extend_from_slice(&[0u8; 23]);
    payload.extend_from_slice(args.user.as_bytes());
    payload.push(0);
    payload.push(0);
    payload.extend_from_slice(b"sbtest");
    payload.push(0);
    write_packet(&mut tcp, 1, &payload)?;
    let (_l, _s, body2) = read_packet(&mut tcp)?;
    if body2.is_empty() || body2[0] != 0x00 {
        return Err(anyhow!("plain auth failed"));
    }
    println!("{},w{:02},plain_auth,0,ok", worker_id, worker_id);

    if args.hold_secs > 0 {
        std::thread::sleep(Duration::from_secs(args.hold_secs));
    }

    if !args.no_probe {
        write_packet(&mut tcp, 0, args.probe_query.as_bytes())?;
        let (_l, _s, resp) = read_packet(&mut tcp)?;
        println!(
            "{},w{:02},plain_probe,0,{}",
            worker_id,
            worker_id,
            if !resp.is_empty() { "ok" } else { "fail" }
        );
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    let args = Arc::new(args);
    println!("worker_id,worker_id,stage,latency_us,status");

    let barrier = Arc::new(std::sync::Barrier::new(args.threads));
    let barrier_start = Instant::now() + Duration::from_millis(500);

    let mut handles = Vec::new();
    for w in 0..args.threads {
        let args = Arc::clone(&args);
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            let res = run_worker(args, w, barrier_start);
            if let Err(e) = res {
                eprintln!("[w{:02}] ERROR: {:#}", w, e);
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    eprintln!("all workers done");
}
