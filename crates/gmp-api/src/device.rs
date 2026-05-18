use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub protocol: Protocol,
    pub endpoint: String,
    pub status: DeviceStatus,
    pub last_reading: Option<DeviceReading>,
    pub last_connected: Option<DateTime<Utc>>,
    pub metadata: DeviceMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceMetadata {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub firmware_version: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    Sensor,
    Controller,
    PLC,
    Scale,
    Thermometer,
    PHMeter,
    ConductivityMeter,
    PressureSensor,
    FlowMeter,
    Other,
}

impl DeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sensor => "sensor",
            Self::Controller => "controller",
            Self::PLC => "plc",
            Self::Scale => "scale",
            Self::Thermometer => "thermometer",
            Self::PHMeter => "ph_meter",
            Self::ConductivityMeter => "conductivity_meter",
            Self::PressureSensor => "pressure_sensor",
            Self::FlowMeter => "flow_meter",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Protocol {
    OPCUA,
    MQTT,
    ModbusTCP,
    ModbusRTU,
    HTTP,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OPCUA => "OPC UA",
            Self::MQTT => "MQTT",
            Self::ModbusTCP => "Modbus TCP",
            Self::ModbusRTU => "Modbus RTU",
            Self::HTTP => "HTTP",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DeviceStatus {
    Online,
    Offline,
    Error,
    Maintenance,
    Connecting,
}

impl DeviceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Error => "error",
            Self::Maintenance => "maintenance",
            Self::Connecting => "connecting",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceReading {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub unit: String,
    pub quality: ReadingQuality,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ReadingQuality {
    Good,
    Uncertain,
    Bad,
    Initial,
    LastKnown,
}

impl ReadingQuality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Uncertain => "uncertain",
            Self::Bad => "bad",
            Self::Initial => "initial",
            Self::LastKnown => "last_known",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcuaSubscription {
    pub id: String,
    pub device_id: String,
    pub node_id: String,
    pub sampling_interval: u32,
    pub queue_size: u32,
    pub monitoring_mode: MonitoringMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MonitoringMode {
    Disabled,
    Sampling,
    Reporting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub broker_url: String,
    pub client_id: String,
    pub topic: String,
    pub qos: MqttQos,
    pub retain: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MqttQos {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusConfig {
    pub host: String,
    pub port: u16,
    pub slave_id: u8,
    pub timeout_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEvent {
    pub id: String,
    pub device_id: String,
    pub event_type: DeviceEventType,
    pub severity: EventSeverity,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeviceEventType {
    ConnectionLost,
    ConnectionRestored,
    ThresholdExceeded,
    DeviceError,
    MaintenanceStarted,
    MaintenanceCompleted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EventSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

pub struct DeviceService {
    devices: HashMap<String, Device>,
    subscriptions: HashMap<String, OpcuaSubscription>,
    readings: HashMap<String, Vec<DeviceReading>>,
    events: Vec<DeviceEvent>,
}

impl Default for DeviceService {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceService {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            subscriptions: HashMap::new(),
            readings: HashMap::new(),
            events: Vec::new(),
        }
    }

    pub fn register_device(&mut self, device: Device) -> Result<String, DeviceError> {
        let id = device.id.clone();
        if self.devices.contains_key(&id) {
            return Err(DeviceError::DeviceAlreadyExists);
        }
        self.devices.insert(id.clone(), device);
        Ok(id)
    }

    pub fn list_devices(&self, protocol: Option<Protocol>) -> Vec<&Device> {
        self.devices
            .values()
            .filter(|d| protocol.map_or(true, |p| d.protocol == p))
            .collect()
    }

    pub fn get_device(&self, id: &str) -> Option<&Device> {
        self.devices.get(id)
    }

    pub fn get_device_mut(&mut self, id: &str) -> Option<&mut Device> {
        self.devices.get_mut(id)
    }

    pub fn update_device_status(&mut self, id: &str, status: DeviceStatus) -> Result<(), DeviceError> {
        let device = self.devices.get_mut(id).ok_or(DeviceError::DeviceNotFound)?;
        device.status = status;
        if status == DeviceStatus::Online {
            device.last_connected = Some(Utc::now());
        }
        Ok(())
    }

    pub fn record_reading(&mut self, device_id: &str, reading: DeviceReading) -> Result<(), DeviceError> {
        if !self.devices.contains_key(device_id) {
            return Err(DeviceError::DeviceNotFound);
        }
        self.readings
            .entry(device_id.to_string())
            .or_insert_with(Vec::new)
            .push(reading.clone());

        let device = self.devices.get_mut(device_id).unwrap();
        device.last_reading = Some(reading);
        Ok(())
    }

    pub fn get_readings(
        &self,
        device_id: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<&DeviceReading>, DeviceError> {
        let readings = self.readings.get(device_id).ok_or(DeviceError::DeviceNotFound)?;

        Ok(readings
            .iter()
            .filter(|r| {
                let time_match = match (from, to) {
                    (Some(f), Some(t)) => r.timestamp >= f && r.timestamp <= t,
                    (Some(f), None) => r.timestamp >= f,
                    (None, Some(t)) => r.timestamp <= t,
                    (None, None) => true,
                };
                time_match
            })
            .collect())
    }

    pub fn subscribe_opcua(
        &mut self,
        device_id: &str,
        node_id: String,
        sampling_interval: u32,
    ) -> Result<String, DeviceError> {
        if !self.devices.contains_key(device_id) {
            return Err(DeviceError::DeviceNotFound);
        }

        let subscription = OpcuaSubscription {
            id: Uuid::new_v4().to_string(),
            device_id: device_id.to_string(),
            node_id,
            sampling_interval,
            queue_size: 1,
            monitoring_mode: MonitoringMode::Reporting,
        };

        let id = subscription.id.clone();
        self.subscriptions.insert(id.clone(), subscription);
        Ok(id)
    }

    pub fn unsubscribe_opcua(&mut self, subscription_id: &str) -> Result<(), DeviceError> {
        self.subscriptions
            .remove(subscription_id)
            .ok_or(DeviceError::SubscriptionNotFound)?;
        Ok(())
    }

    pub fn record_event(&mut self, event: DeviceEvent) {
        self.events.push(event);
    }

    pub fn get_events(
        &self,
        device_id: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Vec<&DeviceEvent> {
        self.events
            .iter()
            .filter(|e| {
                let device_match = device_id.map_or(true, |id| e.device_id == id);
                let time_match = match (from, to) {
                    (Some(f), Some(t)) => e.timestamp >= f && e.timestamp <= t,
                    (Some(f), None) => e.timestamp >= f,
                    (None, Some(t)) => e.timestamp <= t,
                    (None, None) => true,
                };
                device_match && time_match
            })
            .collect()
    }

    pub fn acknowledge_event(&mut self, event_id: &str) -> Result<(), DeviceError> {
        let event = self
            .events
            .iter_mut()
            .find(|e| e.id == event_id)
            .ok_or(DeviceError::EventNotFound)?;
        event.acknowledged = true;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("Device not found")]
    DeviceNotFound,
    #[error("Device already exists")]
    DeviceAlreadyExists,
    #[error("Subscription not found")]
    SubscriptionNotFound,
    #[error("Event not found")]
    EventNotFound,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Read timeout")]
    ReadTimeout,
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl From<DeviceError> for String {
    fn from(err: DeviceError) -> Self {
        err.to_string()
    }
}
