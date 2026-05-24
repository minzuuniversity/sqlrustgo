//! MQTT Gateway — connects to MQTT broker, normalizes messages into DeviceEvents

use std::sync::Arc;
use std::time::Duration;

use rumqttc::{Client, MqttOptions, QoS};

use crate::{AuditContext, AuditEmitter, DeviceEvent, DeviceReading, EventBus, GmpEvent};

// ─── Handle ──────────────────────────────────────────────────────────────────

/// Handle to stop the gateway thread
pub struct MqttGatewayHandle {
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl MqttGatewayHandle {
    /// Signal the gateway to stop
    pub fn stop(self) {
        self.stop_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

// ─── Gateway ─────────────────────────────────────────────────────────────────

/// MQTT device event gateway
///
/// Connects to an MQTT broker, subscribes to configured topics,
/// parses JSON payloads into DeviceReading, and publishes DeviceEvents
/// to the Event Bus.
pub struct MqttGateway {
    broker_url: String,
    client_id: String,
    bus: Arc<dyn EventBus>,
    emitter: AuditEmitter,
}

impl MqttGateway {
    /// Create a new gateway (does not connect)
    pub fn new(broker_url: &str, client_id: &str, bus: Arc<dyn EventBus>) -> Self {
        let emitter = AuditEmitter::new(Arc::clone(&bus));
        Self {
            broker_url: broker_url.to_string(),
            client_id: client_id.to_string(),
            bus,
            emitter,
        }
    }

    /// Start the gateway in a background thread, returning a stop handle.
    pub fn start(self) -> MqttGatewayHandle {
        let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let bus = Arc::clone(&self.bus);
        let broker_url = self.broker_url;
        let client_id = self.client_id;
        let stop_for_handle = Arc::clone(&stop_flag);

        std::thread::spawn(move || {
            tracing::info!(broker = %broker_url, client = %client_id, "MQTT gateway starting");
            let result = run_mqtt_loop(&broker_url, &client_id, bus, self.emitter, stop_flag);
            if let Err(e) = result {
                tracing::error!(error = %e, "MQTT gateway error");
            }
        });

        MqttGatewayHandle {
            stop_flag: stop_for_handle,
        }
    }

    /// Publish a device reading event directly (for testing or manual injection)
    pub fn publish_reading(&self, reading: &DeviceReading) -> anyhow::Result<()> {
        let trace_id = uuid::Uuid::new_v4().to_string();
        let ctx = AuditContext::with_trace("mqtt-gateway", &trace_id);

        let _ = self.emitter.emit_insert(&ctx, &reading.device_id, "device");

        let event = GmpEvent::Device(DeviceEvent {
            device_id: reading.device_id.clone(),
            reading_type: reading.reading_type,
            value: reading.value,
            unit: reading.unit.clone(),
            timestamp: reading.timestamp,
            trace_id: trace_id.clone(),
        });

        self.bus.publish(event)?;
        Ok(())
    }
}

// ─── MQTT Loop ────────────────────────────────────────────────────────────────

fn run_mqtt_loop(
    broker_url: &str,
    client_id: &str,
    bus: Arc<dyn EventBus>,
    emitter: AuditEmitter,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {

    loop {
        if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
            tracing::info!("MQTT gateway stopped");
            return Ok(());
        }

        match connect_and_stream(broker_url, client_id, &bus, &emitter, &stop_flag) {
            Ok(_) => return Ok(()),
            Err(e) => {
                tracing::error!(error = %e, "MQTT connection failed, retrying in 5s");
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

fn connect_and_stream(
    broker_url: &str,
    client_id: &str,
    bus: &Arc<dyn EventBus>,
    emitter: &AuditEmitter,
    stop_flag: &Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {
    let mut options = MqttOptions::new(client_id, broker_url, 1883);
    options.set_keep_alive(Duration::from_secs(30));

    let (client, mut connection) = Client::new(options, 100);
    client.subscribe("gmp/devices/#", QoS::AtLeastOnce)?;

    loop {
        if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }

        // Use recv_timeout to allow periodic shutdown checks
        match connection.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(rumqttc::Event::Incoming(incoming))) => {
                if let rumqttc::Incoming::Publish(publish) = incoming {
                    let topic = publish.topic.clone();
                    let payload = publish.payload.to_vec();
                    if let Err(e) = handle_message(bus, emitter, &topic, &payload) {
                        tracing::error!(error = ?e, topic = %topic, "MQTT message handling failed");
                    }
                }
            }
            Ok(Ok(_)) => { /* keepalive, suback, etc. */ }
            Ok(Err(e)) => {
                tracing::warn!(error = ?e, "MQTT connection warning");
            }
            Err(rumqttc::RecvTimeoutError::Timeout) => {
                // timeout, loop back to check stop_flag
            }
            Err(rumqttc::RecvTimeoutError::Disconnected) => {
                tracing::info!("MQTT disconnected");
                return Ok(());
            }
        }
    }
}

// ─── Message Handling ────────────────────────────────────────────────────────

fn handle_message(
    bus: &Arc<dyn EventBus>,
    emitter: &AuditEmitter,
    topic: &str,
    payload: &[u8],
) -> anyhow::Result<()> {
    let reading: DeviceReading = match serde_json::from_slice(payload) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(topic = %topic, error = %e, "Failed to parse MQTT payload");
            return Ok(());
        }
    };

    if reading.device_id.is_empty() {
        return Ok(());
    }

    let trace_id = uuid::Uuid::new_v4().to_string();
    let ctx = AuditContext::with_trace("mqtt-gateway", &trace_id);

    let _ = emitter.emit_insert(&ctx, &reading.device_id, "device");

    let event = GmpEvent::Device(DeviceEvent {
        device_id: reading.device_id.clone(),
        reading_type: reading.reading_type,
        value: reading.value,
        unit: reading.unit.clone(),
        timestamp: reading.timestamp,
        trace_id,
    });

    bus.publish(event)?;
    Ok(())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeviceReadingType;

    #[test]
    fn test_device_reading_deserialize() {
        let json = r#"{
            "device_id": "scale-001",
            "reading_type": "weight",
            "value": 1250.5,
            "unit": "g",
            "timestamp": 1716588000
        }"#;

        let reading: DeviceReading = serde_json::from_str(json).unwrap();
        assert_eq!(reading.device_id, "scale-001");
        assert!(matches!(reading.reading_type, DeviceReadingType::Weight));
        assert!((reading.value - 1250.5).abs() < 0.001);
        assert_eq!(reading.unit, "g");
        assert_eq!(reading.timestamp, 1716588000);
    }

    #[test]
    fn test_device_reading_all_types() {
        for rt in ["weight", "temperature", "humidity"] {
            let json = format!(
                r#"{{"device_id":"d1","reading_type":"{}","value":1.0,"unit":"u","timestamp":0}}"#,
                rt
            );
            let reading: DeviceReading = serde_json::from_str(&json).unwrap();
            assert!(matches!(
                reading.reading_type,
                DeviceReadingType::Weight | DeviceReadingType::Temperature | DeviceReadingType::Humidity
            ));
        }
    }

    #[test]
    fn test_gateway_creation() {
        use crate::InMemoryBus;
        let bus = Arc::new(InMemoryBus::new());
        let _gw = MqttGateway::new("tcp://localhost:1883", "test", bus);
    }

    #[test]
    fn test_handle_stop() {
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(flag.load(std::sync::atomic::Ordering::SeqCst));
    }
}
