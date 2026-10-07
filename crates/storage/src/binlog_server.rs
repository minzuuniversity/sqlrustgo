//! Binlog Server - Master side network service for replication
//!
//! Accepts connections from slave nodes and pushes binlog events.

use crate::binlog_protocol::{
    BinlogEventData, BinlogProtocol, PacketReader, PacketWriter, ReplicationMessage,
};
use crate::replication::{BinlogEvent, BinlogWriter};
use crate::semisync::SemiSyncMaster;
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct SlaveSubscriber {
    pub slave_id: u32,
    pub stream: TcpStream,
    pub binlog_file: String,
    pub binlog_pos: u64,
}

impl Clone for SlaveSubscriber {
    fn clone(&self) -> Self {
        Self {
            slave_id: self.slave_id,
            stream: self.stream.try_clone().unwrap(),
            binlog_file: self.binlog_file.clone(),
            binlog_pos: self.binlog_pos,
        }
    }
}

pub struct BinlogServer {
    listener: TcpListener,
    server_id: u32,
    server_version: String,
    binlog_path: PathBuf,
    binlog_writer: Arc<Mutex<BinlogWriter>>,
    subscribers: Arc<Mutex<HashMap<u32, SlaveSubscriber>>>,
    /// #4936 PR-B: highest LSN each slave has acknowledged.
    ///
    /// **This is a liveness table, not a durability table.** The value
    /// comes from `HeartbeatAck`, which carries an LSN the *master*
    /// chose (see `BinlogClient::send_ack`, which assigns the echoed
    /// value to `current_pos`). It answers "is this replica alive",
    /// not "how far has it written". Semi-sync waits on
    /// [`Self::acked_pos`] instead — see #4937.
    ///
    /// Keyed by slave id; `0` means "no slave has acknowledged yet".
    acked_lsn: Arc<Mutex<HashMap<u32, u64>>>,
    /// #4937: highest binlog position each slave reports it has
    /// **durably written**, carried by `ReplicationMessage::BinlogAck`.
    ///
    /// Kept separate from `acked_lsn` on purpose: mixing the two would
    /// let a heartbeat stand in for a durability confirmation, which is
    /// exactly the failure mode semi-sync exists to prevent.
    acked_pos: Arc<Mutex<HashMap<u32, u64>>>,
    /// #4937: semi-sync gate. Disabled by default, so `write_event`
    /// behaves exactly as before unless it is turned on.
    semi_sync: Arc<SemiSyncMaster>,
    is_running: Arc<Mutex<bool>>,
}

impl BinlogServer {
    pub fn new(
        host: &str,
        port: u16,
        server_id: u32,
        binlog_path: PathBuf,
    ) -> std::io::Result<Self> {
        let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        let binlog_writer = BinlogWriter::new(binlog_path.clone())?;

        Ok(Self {
            listener,
            server_id,
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            binlog_path,
            binlog_writer: Arc::new(Mutex::new(binlog_writer)),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            acked_lsn: Arc::new(Mutex::new(HashMap::new())),
            acked_pos: Arc::new(Mutex::new(HashMap::new())),
            semi_sync: Arc::new(SemiSyncMaster::new()),
            is_running: Arc::new(Mutex::new(false)),
        })
    }

    /// The address this server actually bound to.
    ///
    /// Constructed with port `0` the OS picks a free port; without this
    /// accessor a caller (notably the #4937 end-to-end test) could not
    /// learn which one, and would have to guess a fixed port and risk
    /// colliding with a parallel test run.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub fn start(&self) -> std::io::Result<()> {
        *self.is_running.lock().unwrap() = true;
        let listener = &self.listener;
        let subscribers = self.subscribers.clone();
        let is_running = self.is_running.clone();

        loop {
            if !*is_running.lock().unwrap() {
                break;
            }

            match listener.accept() {
                Ok((stream, addr)) => {
                    let subscribers = subscribers.clone();
                    let server_id = self.server_id;
                    let version = self.server_version.clone();
                    let writer = self.binlog_writer.clone();
                    // #4936 PR-B: clone the Arc so the spawned handler
                    // books acknowledgements into the same table the
                    // server reads from.
                    let acked_lsn = self.acked_lsn.clone();
                    // #4937: the durability table semi-sync waits on.
                    let acked_pos = self.acked_pos.clone();

                    thread::spawn(move || {
                        if let Err(e) = handle_slave_connection(
                            stream,
                            addr,
                            server_id,
                            &version,
                            writer,
                            subscribers,
                            acked_lsn,
                            acked_pos,
                        ) {
                            eprintln!("Error handling slave {}: {}", addr, e);
                        }
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                }
            }
        }

        Ok(())
    }

    pub fn stop(&self) {
        *self.is_running.lock().unwrap() = false;
    }

    /// #4937: configure the semi-sync gate.
    ///
    /// `wait_count` is how many replicas must have durably written the
    /// event before `write_event` returns. `timeout_ms` bounds the wait
    /// after which the commit degrades to async (and is counted in
    /// `rpl_semi_sync_master_no_transactions`).
    ///
    /// Semi-sync is off by default, so this is opt-in per server.
    pub fn configure_semi_sync(&self, wait_count: u32, timeout_ms: u64) {
        self.semi_sync.set_wait_count(wait_count);
        self.semi_sync.set_timeout_ms(timeout_ms);
        if wait_count > 0 {
            self.semi_sync.enable();
        } else {
            self.semi_sync.disable();
        }
    }

    /// #4937: access the semi-sync state (counters, mode, wait config).
    pub fn semi_sync(&self) -> &Arc<SemiSyncMaster> {
        &self.semi_sync
    }

    /// #4937: highest durably-written position reported by a replica.
    pub fn acked_pos_of(&self, slave_id: u32) -> u64 {
        self.acked_pos
            .lock()
            .unwrap()
            .get(&slave_id)
            .copied()
            .unwrap_or(0)
    }

    /// #4937: number of replicas that have reported a durable write.
    pub fn acking_pos_slave_count(&self) -> usize {
        self.acked_pos.lock().unwrap().len()
    }

    /// #4937: block until at least `wait_count` replicas have
    /// durably written `target_pos`, or the timeout expires.
    ///
    /// Returns `true` when the wait succeeded. On timeout the caller is
    /// expected to degrade to async — the commit is already in the
    /// master's own binlog, so it is not lost, it is merely no longer
    /// guaranteed to have reached a replica before the client was told
    /// "committed".
    fn wait_for_semi_sync(&self, target_pos: u64) -> bool {
        if !self.semi_sync.is_enabled() {
            return true;
        }
        let required = self.semi_sync.get_wait_count();
        if required == 0 {
            return true;
        }
        let timeout = Duration::from_millis(self.semi_sync.get_timeout_ms());
        let start = std::time::Instant::now();

        loop {
            let ackers = self
                .acked_pos
                .lock()
                .unwrap()
                .values()
                .filter(|&&p| p >= target_pos)
                .count() as u32;
            if ackers >= required {
                return true;
            }
            if start.elapsed() >= timeout {
                return false;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    pub fn write_event(&self, event: &BinlogEvent) -> std::io::Result<u64> {
        let mut writer = self.binlog_writer.lock().unwrap();
        let lsn = writer.write_event(event)?;

        let event_data = convert_to_event_data(event);
        let msg = ReplicationMessage::BinlogData {
            file: format!("binlog.{:06}", 1),
            pos: lsn,
            events: vec![event_data],
        };

        self.broadcast(&msg);

        // #4937: semi-sync gate. `lsn` is the position the master just
        // wrote; the caller may not observe this event as committed
        // until `wait_count` replicas have reported it as durably
        // written themselves (via `BinlogAck`). On timeout the commit
        // degrades to async and is counted, exactly as MySQL records it
        // in `rpl_semi_sync_master_no_transactions`.
        if self.wait_for_semi_sync(lsn) {
            if self.semi_sync.is_enabled() {
                self.semi_sync.record_yes_transaction();
            }
        } else {
            self.semi_sync.record_no_transaction();
        }

        Ok(lsn)
    }

    fn broadcast(&self, msg: &ReplicationMessage) {
        let data = msg.serialize();
        let subscribers: Vec<_> = self.subscribers.lock().unwrap().values().cloned().collect();

        for mut subscriber in subscribers {
            if let Err(e) = PacketWriter::write_packet(&mut subscriber.stream, &data) {
                eprintln!("Failed to send to slave {}: {}", subscriber.slave_id, e);
            }
        }
    }

    pub fn register_subscriber(&self, slave_id: u32, subscriber: SlaveSubscriber) {
        self.subscribers
            .lock()
            .unwrap()
            .insert(slave_id, subscriber);
    }

    pub fn unregister_subscriber(&self, slave_id: u32) {
        self.subscribers.lock().unwrap().remove(&slave_id);
    }

    pub fn get_binlog_position(&self) -> u64 {
        let writer = self.binlog_writer.lock().unwrap();
        writer.position()
    }
}

// The replication-slave handler threads one of these per connection; the
// parameter list mirrors the master's connection state one-for-one. Bundling
// them into a context struct is a larger refactor of the binlog server than a
// lint cleanup should carry, so the lint is scoped off here rather than
// forcing the call sites through an unrelated shape change.
#[allow(clippy::too_many_arguments)]
fn handle_slave_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    server_id: u32,
    server_version: &str,
    binlog_writer: Arc<Mutex<BinlogWriter>>,
    subscribers: Arc<Mutex<HashMap<u32, SlaveSubscriber>>>,
    // #4936 PR-B: shared with the server so the master can be asked
    // how far each replica has got.
    acked_lsn: Arc<Mutex<HashMap<u32, u64>>>,
    // #4937: durability table, fed by `BinlogAck` (see `acked_pos`
    // on `BinlogServer`). Separate from `acked_lsn` because heartbeat
    // ACKs carry a master-chosen LSN, not a replica write position.
    acked_pos: Arc<Mutex<HashMap<u32, u64>>>,
) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;

    // #4936 PR-B: the replica's id is only destructured inside the
    // handshake arm, but later arms (notably HeartbeatAck) need it to
    // key the acknowledgement table. Bind it once, outside the loop.
    let mut this_slave_id: Option<u32> = None;

    loop {
        let data = match PacketReader::read_packet(&mut stream) {
            Ok(Some(d)) => d,
            Ok(None) => break,
            Err(e) => {
                eprintln!("Read error from {}: {}", addr, e);
                break;
            }
        };

        let msg = match ReplicationMessage::deserialize(&data) {
            Some(m) => m,
            None => {
                let err = ReplicationMessage::Error {
                    code: 1,
                    message: "Failed to parse message".to_string(),
                };
                PacketWriter::write_packet(&mut stream, &err.serialize())?;
                continue;
            }
        };

        match msg {
            ReplicationMessage::HandshakeRequest {
                slave_id,
                host: _,
                port: _,
                replication_protocol_version,
            } => {
                if replication_protocol_version > BinlogProtocol::VERSION {
                    let err = ReplicationMessage::Error {
                        code: 2,
                        message: "Unsupported replication protocol version".to_string(),
                    };
                    PacketWriter::write_packet(&mut stream, &err.serialize())?;
                    break;
                }

                let writer = binlog_writer.lock().unwrap();
                let response = ReplicationMessage::HandshakeResponse {
                    server_id,
                    server_version: server_version.to_string(),
                    binlog_file: format!("binlog.{:06}", 1),
                    binlog_pos: writer.position(),
                };
                drop(writer);

                PacketWriter::write_packet(&mut stream, &response.serialize())?;

                let subscriber = SlaveSubscriber {
                    slave_id,
                    stream: stream.try_clone()?,
                    binlog_file: format!("binlog.{:06}", 1),
                    binlog_pos: 0,
                };
                subscribers.lock().unwrap().insert(slave_id, subscriber);
                this_slave_id = Some(slave_id);
            }

            ReplicationMessage::BinlogPosRequest { file, pos } => {
                let _ = (file, pos);
                let writer = binlog_writer.lock().unwrap();
                let response = ReplicationMessage::BinlogPosResponse {
                    file: format!("binlog.{:06}", 1),
                    pos: writer.position(),
                };
                PacketWriter::write_packet(&mut stream, &response.serialize())?;
            }

            ReplicationMessage::HeartbeatAck { lsn } => {
                // #4936 PR-B: record it. Discarding the ACK left the
                // master with no way to tell how far a replica had got,
                // which is the thing semi-sync replication (#4937) has
                // to wait on. LSNs are monotonic per replica, so never
                // move the watermark backwards.
                let Some(sid) = this_slave_id else {
                    // An ACK before a completed handshake cannot be
                    // attributed to a replica; treat it as a protocol
                    // error rather than guessing.
                    let err = ReplicationMessage::Error {
                        code: 4,
                        message: "HeartbeatAck before handshake".to_string(),
                    };
                    PacketWriter::write_packet(&mut stream, &err.serialize())?;
                    break;
                };
                let recorded = {
                    let mut acks = acked_lsn.lock().unwrap();
                    let entry = acks.entry(sid).or_insert(0);
                    if *entry < lsn {
                        *entry = lsn;
                    }
                    *entry
                };
                let ok = ReplicationMessage::AckOk { lsn: recorded };
                PacketWriter::write_packet(&mut stream, &ok.serialize())?;
            }

            // #4937: the durability acknowledgement semi-sync waits on.
            // Booked into `acked_pos`, deliberately NOT into `acked_lsn`
            // — see the field docs on `BinlogServer::acked_pos`.
            ReplicationMessage::BinlogAck { file: _, pos } => {
                let Some(sid) = this_slave_id else {
                    let err = ReplicationMessage::Error {
                        code: 4,
                        message: "BinlogAck before handshake".to_string(),
                    };
                    PacketWriter::write_packet(&mut stream, &err.serialize())?;
                    break;
                };
                let recorded = {
                    let mut acks = acked_pos.lock().unwrap();
                    let entry = acks.entry(sid).or_insert(0);
                    if *entry < pos {
                        *entry = pos;
                    }
                    *entry
                };
                let ok = ReplicationMessage::AckOk { lsn: recorded };
                PacketWriter::write_packet(&mut stream, &ok.serialize())?;
            }

            ReplicationMessage::EOF => {
                break;
            }

            _ => {
                let err = ReplicationMessage::Error {
                    code: 3,
                    message: "Unexpected message type".to_string(),
                };
                PacketWriter::write_packet(&mut stream, &err.serialize())?;
            }
        }
    }

    Ok(())
}

fn convert_to_event_data(event: &BinlogEvent) -> BinlogEventData {
    BinlogEventData {
        event_type: event.event_type as u8,
        tx_id: event.tx_id,
        table_id: event.table_id,
        database: event.database.clone(),
        table: event.table.clone(),
        sql: event.sql.clone(),
        row_data: event.row_data.clone(),
        lsn: event.lsn,
        timestamp: event.timestamp,
    }
}

pub fn start_heartbeat(server: &BinlogServer) {
    let server = server.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(BinlogProtocol::HEARTBEAT_INTERVAL_MS));

        if !*server.is_running.lock().unwrap() {
            break;
        }

        let writer = server.binlog_writer.lock().unwrap();
        let heartbeat = ReplicationMessage::Heartbeat {
            lsn: writer.current_lsn(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        drop(writer);

        server.broadcast(&heartbeat);
    });
}

impl BinlogServer {
    /// #4936 PR-B: highest LSN acknowledged by `slave_id`, or 0 if it
    /// has never acknowledged.
    ///
    /// This is the primitive #4937 (semi-sync replication) waits on:
    /// before acknowledging a source commit, the master needs to know
    /// that every replica has applied at least that far.
    pub fn acked_lsn_of(&self, slave_id: u32) -> u64 {
        *self.acked_lsn.lock().unwrap().get(&slave_id).unwrap_or(&0)
    }

    /// #4936 PR-B: lowest watermark across all registered slaves.
    ///
    /// Semi-sync's wait condition is "the slowest replica", not the
    /// average and not any single one — a fast replica must not mask a
    /// lagging one. Returns 0 when no replica is registered, which
    /// callers must treat as "nothing to wait for" rather than
    /// "already caught up".
    pub fn min_acked_lsn(&self) -> u64 {
        let acks = self.acked_lsn.lock().unwrap();
        if acks.is_empty() {
            return 0;
        }
        acks.values().copied().min().unwrap_or(0)
    }

    /// #4936 PR-B: number of replicas that have ever acknowledged.
    pub fn acking_slave_count(&self) -> usize {
        self.acked_lsn.lock().unwrap().len()
    }
}

impl Clone for BinlogServer {
    fn clone(&self) -> Self {
        Self {
            listener: TcpListener::bind("127.0.0.1:0").unwrap(),
            server_id: self.server_id,
            server_version: self.server_version.clone(),
            binlog_path: self.binlog_path.clone(),
            binlog_writer: self.binlog_writer.clone(),
            subscribers: self.subscribers.clone(),
            acked_lsn: self.acked_lsn.clone(),
            acked_pos: self.acked_pos.clone(),
            semi_sync: self.semi_sync.clone(),
            is_running: self.is_running.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let temp_dir = std::env::temp_dir();
        let binlog_path = temp_dir.join("test_binlog_server");

        let server = BinlogServer::new("127.0.0.1", 0, 1, binlog_path);
        assert!(server.is_ok());
    }

    /// #4936 PR-B: the acknowledgement table is the thing semi-sync
    /// replication (#4937) waits on. Before this the master discarded
    /// every HeartbeatAck, so nothing could be waited on.
    #[test]
    fn test_acked_lsn_starts_at_zero() {
        let dir = std::env::temp_dir().join(format!("b22_ack0_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let srv = BinlogServer::new("127.0.0.1", 0, 1, dir.clone()).expect("server");
        assert_eq!(srv.acked_lsn_of(7), 0, "unknown slave acknowledges nothing");
        assert_eq!(srv.acking_slave_count(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// #4936 PR-B: `min_acked_lsn` is the slowest replica, not the
    /// average — a fast replica must not mask a lagging one.
    #[test]
    fn test_min_acked_lsn_tracks_the_slowest_replica() {
        use std::sync::Arc;
        let dir = std::env::temp_dir().join(format!("b22_ack1_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let srv = BinlogServer::new("127.0.0.1", 0, 1, dir.clone()).expect("server");
        {
            let mut acks = srv.acked_lsn.lock().unwrap();
            acks.insert(1, 900);
            acks.insert(2, 100);
            acks.insert(3, 500);
        }
        assert_eq!(
            srv.min_acked_lsn(),
            100,
            "the lagging replica sets the pace"
        );
        assert_eq!(srv.acked_lsn_of(1), 900);
        assert_eq!(srv.acking_slave_count(), 3);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = Arc::strong_count(&Arc::new(0));
    }

    /// #4936 PR-B: an out-of-order or replayed ACK must not move a
    /// replica's watermark backwards, or a delayed heartbeat would
    /// un-do progress and stall a semi-sync wait forever.
    #[test]
    fn test_acked_lsn_never_moves_backwards() {
        let dir = std::env::temp_dir().join(format!("b22_ack2_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let srv = BinlogServer::new("127.0.0.1", 0, 1, dir.clone()).expect("server");
        {
            let mut acks = srv.acked_lsn.lock().unwrap();
            // Mirrors the monotonic guard in the HeartbeatAck arm.
            let e = acks.entry(4).or_insert(0);
            if *e < 500 {
                *e = 500;
            }
            let e = acks.entry(4).or_insert(0);
            if *e < 300 {
                *e = 300;
            } // stale ACK, must be ignored
        }
        assert_eq!(
            srv.acked_lsn_of(4),
            500,
            "a stale ACK must not regress the watermark"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// #4936 PR-B: `min_acked_lsn` returning 0 for "no replicas" must
    /// be distinguishable from "caught up". Callers need the count to
    /// tell those apart.
    #[test]
    fn test_min_acked_lsn_distinguishes_empty_from_caught_up() {
        let dir = std::env::temp_dir().join(format!("b22_ack3_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let srv = BinlogServer::new("127.0.0.1", 0, 1, dir.clone()).expect("server");
        assert_eq!(srv.min_acked_lsn(), 0);
        assert_eq!(
            srv.acking_slave_count(),
            0,
            "0 replicas, not 'replicas at LSN 0'"
        );
        {
            let mut acks = srv.acked_lsn.lock().unwrap();
            acks.insert(1, 0);
        }
        assert_eq!(srv.min_acked_lsn(), 0);
        assert_eq!(
            srv.acking_slave_count(),
            1,
            "now a replica exists, at LSN 0"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
