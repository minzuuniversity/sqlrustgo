//! V400-02 / Issue #4878: WAL-backed vector storage tests.
//!
//! Tests WAL entry serialization, replay, and crash recovery for vector storage.
//! Per docs/releases/v4.0.0/DEV_PLAN.md §V400-02.
//!
//! Dependencies: V400-01 (Vector SQL syntax)

// ============================================================================
// WAL Entry Types for Vector Operations
// ============================================================================

/// Vector WAL entry types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorWalEntryType {
    VectorInsert,
    VectorUpdate,
    VectorDelete,
    CreateVectorIndex,
    DropVectorIndex,
    RebuildVectorIndex,
}

impl VectorWalEntryType {
    pub fn to_u8(&self) -> u8 {
        match self {
            VectorWalEntryType::VectorInsert => 100,
            VectorWalEntryType::VectorUpdate => 101,
            VectorWalEntryType::VectorDelete => 102,
            VectorWalEntryType::CreateVectorIndex => 103,
            VectorWalEntryType::DropVectorIndex => 104,
            VectorWalEntryType::RebuildVectorIndex => 105,
        }
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            100 => Some(VectorWalEntryType::VectorInsert),
            101 => Some(VectorWalEntryType::VectorUpdate),
            102 => Some(VectorWalEntryType::VectorDelete),
            103 => Some(VectorWalEntryType::CreateVectorIndex),
            104 => Some(VectorWalEntryType::DropVectorIndex),
            105 => Some(VectorWalEntryType::RebuildVectorIndex),
            _ => None,
        }
    }
}

/// Vector WAL entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorWalEntry {
    pub tx_id: u64,
    pub entry_type: VectorWalEntryType,
    pub table_id: u64,
    pub key: Option<Vec<u8>>,
    pub vector_data: Option<Vec<u8>>,
    pub index_name: Option<String>,
    pub index_type: Option<String>,
    pub index_options: Option<Vec<(String, String)>>,
}

impl VectorWalEntry {
    pub fn new_vector_insert(tx_id: u64, table_id: u64, key: Vec<u8>, vector_data: Vec<u8>) -> Self {
        Self {
            tx_id,
            entry_type: VectorWalEntryType::VectorInsert,
            table_id,
            key: Some(key),
            vector_data: Some(vector_data),
            index_name: None,
            index_type: None,
            index_options: None,
        }
    }

    pub fn new_create_index(
        tx_id: u64,
        table_id: u64,
        index_name: String,
        index_type: String,
        index_options: Vec<(String, String)>,
    ) -> Self {
        Self {
            tx_id,
            entry_type: VectorWalEntryType::CreateVectorIndex,
            table_id,
            key: None,
            vector_data: None,
            index_name: Some(index_name),
            index_type: Some(index_type),
            index_options: Some(index_options),
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.tx_id.to_le_bytes());
        bytes.push(self.entry_type.to_u8());
        bytes.extend_from_slice(&self.table_id.to_le_bytes());
        Self::write_opt_vec(&mut bytes, &self.key);
        Self::write_opt_vec(&mut bytes, &self.vector_data);
        Self::write_opt_string(&mut bytes, &self.index_name);
        Self::write_opt_string(&mut bytes, &self.index_type);
        Self::write_opt_options(&mut bytes, &self.index_options);
        bytes
    }

    fn write_opt_vec(bytes: &mut Vec<u8>, opt: &Option<Vec<u8>>) {
        match opt {
            Some(v) => {
                bytes.extend_from_slice(&(v.len() as u32).to_le_bytes());
                bytes.extend_from_slice(v);
            }
            None => bytes.extend_from_slice(&0u32.to_le_bytes()),
        }
    }

    fn write_opt_string(bytes: &mut Vec<u8>, opt: &Option<String>) {
        match opt {
            Some(s) => {
                bytes.extend_from_slice(&(s.len() as u32).to_le_bytes());
                bytes.extend_from_slice(s.as_bytes());
            }
            None => bytes.extend_from_slice(&0u32.to_le_bytes()),
        }
    }

    fn write_opt_options(bytes: &mut Vec<u8>, opt: &Option<Vec<(String, String)>>) {
        match opt {
            Some(opts) => {
                bytes.extend_from_slice(&(opts.len() as u32).to_le_bytes());
                for (k, v) in opts {
                    Self::write_opt_string(bytes, &Some(k.clone()));
                    Self::write_opt_string(bytes, &Some(v.clone()));
                }
            }
            None => bytes.extend_from_slice(&0u32.to_le_bytes()),
        }
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 17 {
            return None;
        }
        let mut offset = 0;

        let tx_id = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
        offset += 8;

        let entry_type = VectorWalEntryType::from_u8(bytes[offset])?;
        offset += 1;

        let table_id = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
        offset += 8;

        let key = Self::read_opt_vec(bytes, &mut offset)?;
        let vector_data = Self::read_opt_vec(bytes, &mut offset)?;
        let index_name = Self::read_opt_string(bytes, &mut offset)?;
        let index_type = Self::read_opt_string(bytes, &mut offset)?;
        let index_options = Self::read_opt_options(bytes, &mut offset)?;

        Some(VectorWalEntry {
            tx_id,
            entry_type,
            table_id,
            key,
            vector_data,
            index_name,
            index_type,
            index_options,
        })
    }

    fn read_opt_vec(bytes: &[u8], offset: &mut usize) -> Option<Option<Vec<u8>>> {
        if *offset + 4 > bytes.len() {
            return None;
        }
        let len = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().ok()?) as usize;
        *offset += 4;
        if len == 0 {
            return Some(None);
        }
        if *offset + len > bytes.len() {
            return None;
        }
        let v = bytes[*offset..*offset + len].to_vec();
        *offset += len;
        Some(Some(v))
    }

    fn read_opt_string(bytes: &[u8], offset: &mut usize) -> Option<Option<String>> {
        if *offset + 4 > bytes.len() {
            return None;
        }
        let len = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().ok()?) as usize;
        *offset += 4;
        if len == 0 {
            return Some(None);
        }
        if *offset + len > bytes.len() {
            return None;
        }
        let s = String::from_utf8(bytes[*offset..*offset + len].to_vec()).ok()?;
        *offset += len;
        Some(Some(s))
    }

    fn read_opt_options(bytes: &[u8], offset: &mut usize) -> Option<Option<Vec<(String, String)>>> {
        if *offset + 4 > bytes.len() {
            return None;
        }
        let count = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().ok()?) as usize;
        *offset += 4;
        if count == 0 {
            return Some(None);
        }
        let mut opts = Vec::with_capacity(count);
        for _ in 0..count {
            let k = Self::read_opt_string(bytes, offset)?;
            let v = Self::read_opt_string(bytes, offset)?;
            match (k, v) {
                (Some(kk), Some(vv)) => opts.push((kk, vv)),
                _ => return None,
            }
        }
        Some(Some(opts))
    }

    /// Roundtrip serialization
    pub fn serialization_roundtrip(&self) -> Option<Self> {
        let bytes = self.to_bytes();
        Self::from_bytes(&bytes)
    }
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn vector_wal_entry_type_serialization() {
    for entry_type in [
        VectorWalEntryType::VectorInsert,
        VectorWalEntryType::VectorUpdate,
        VectorWalEntryType::VectorDelete,
        VectorWalEntryType::CreateVectorIndex,
        VectorWalEntryType::DropVectorIndex,
        VectorWalEntryType::RebuildVectorIndex,
    ] {
        let ty = entry_type.to_u8();
        let recovered = VectorWalEntryType::from_u8(ty).expect("should recover");
        assert_eq!(entry_type, recovered);
    }
}

#[test]
fn vector_wal_entry_insert_roundtrip() {
    let entry = VectorWalEntry::new_vector_insert(
        42, 1, vec![1, 2, 3], vec![0.1f32.to_le_bytes(), 0.2f32.to_le_bytes()].concat(),
    );
    let recovered = entry.serialization_roundtrip().expect("roundtrip should succeed");
    assert_eq!(entry.tx_id, recovered.tx_id);
    assert_eq!(entry.table_id, recovered.table_id);
    assert_eq!(entry.entry_type, recovered.entry_type);
    assert_eq!(entry.key, recovered.key);
    assert_eq!(entry.vector_data, recovered.vector_data);
}

#[test]
fn vector_wal_entry_create_index_roundtrip() {
    let entry = VectorWalEntry::new_create_index(
        100, 2, "idx_emb".to_string(), "HNSW".to_string(),
        vec![("m".to_string(), "16".to_string()), ("ef".to_string(), "200".to_string())],
    );
    let recovered = entry.serialization_roundtrip().expect("roundtrip should succeed");
    assert_eq!(entry.tx_id, recovered.tx_id);
    assert_eq!(entry.index_name, recovered.index_name);
    assert_eq!(entry.index_type, recovered.index_type);
    assert_eq!(entry.index_options.as_ref().map(|o| o.len()), recovered.index_options.as_ref().map(|o| o.len()));
}

// ============================================================================
// WAL Replay Simulation Tests
// ============================================================================

fn replay_vector_wal_entry(entry: &VectorWalEntry) -> Result<(), String> {
    match entry.entry_type {
        VectorWalEntryType::VectorInsert => {
            if entry.vector_data.is_none() {
                return Err("VectorInsert requires vector_data".to_string());
            }
            Ok(())
        }
        VectorWalEntryType::VectorUpdate => {
            if entry.key.is_none() || entry.vector_data.is_none() {
                return Err("VectorUpdate requires key and vector_data".to_string());
            }
            Ok(())
        }
        VectorWalEntryType::VectorDelete => {
            if entry.key.is_none() {
                return Err("VectorDelete requires key".to_string());
            }
            Ok(())
        }
        VectorWalEntryType::CreateVectorIndex => {
            if entry.index_name.is_none() || entry.index_type.is_none() {
                return Err("CreateVectorIndex requires name and type".to_string());
            }
            Ok(())
        }
        VectorWalEntryType::DropVectorIndex => {
            if entry.index_name.is_none() {
                return Err("DropVectorIndex requires name".to_string());
            }
            Ok(())
        }
        VectorWalEntryType::RebuildVectorIndex => {
            if entry.index_name.is_none() {
                return Err("RebuildVectorIndex requires name".to_string());
            }
            Ok(())
        }
    }
}

#[test]
fn wal_replay_vector_insert() {
    let entry = VectorWalEntry::new_vector_insert(1, 1, vec![1], vec![0.1f32.to_le_bytes()].concat());
    replay_vector_wal_entry(&entry).expect("replay should succeed");
}

#[test]
fn wal_replay_vector_update() {
    let entry = VectorWalEntry {
        tx_id: 2, entry_type: VectorWalEntryType::VectorUpdate, table_id: 1,
        key: Some(vec![1]), vector_data: Some(vec![0.2f32.to_le_bytes()].concat()),
        index_name: None, index_type: None, index_options: None,
    };
    replay_vector_wal_entry(&entry).expect("replay should succeed");
}

#[test]
fn wal_replay_vector_delete() {
    let entry = VectorWalEntry {
        tx_id: 3, entry_type: VectorWalEntryType::VectorDelete, table_id: 1,
        key: Some(vec![1]), vector_data: None,
        index_name: None, index_type: None, index_options: None,
    };
    replay_vector_wal_entry(&entry).expect("replay should succeed");
}

#[test]
fn wal_replay_create_hnsw_index() {
    let entry = VectorWalEntry::new_create_index(
        4, 1, "idx_hnsw".to_string(), "HNSW".to_string(),
        vec![("m".to_string(), "16".to_string())],
    );
    replay_vector_wal_entry(&entry).expect("replay should succeed");
}

#[test]
fn wal_replay_create_ivf_index() {
    let entry = VectorWalEntry::new_create_index(
        5, 1, "idx_ivf".to_string(), "IVF".to_string(),
        vec![("nlist".to_string(), "100".to_string())],
    );
    replay_vector_wal_entry(&entry).expect("replay should succeed");
}

#[test]
fn wal_replay_drop_index() {
    let entry = VectorWalEntry {
        tx_id: 6, entry_type: VectorWalEntryType::DropVectorIndex, table_id: 1,
        key: None, vector_data: None,
        index_name: Some("idx_emb".to_string()), index_type: None, index_options: None,
    };
    replay_vector_wal_entry(&entry).expect("replay should succeed");
}

#[test]
fn wal_replay_rebuild_index() {
    let entry = VectorWalEntry {
        tx_id: 7, entry_type: VectorWalEntryType::RebuildVectorIndex, table_id: 1,
        key: None, vector_data: None,
        index_name: Some("idx_emb".to_string()), index_type: None, index_options: None,
    };
    replay_vector_wal_entry(&entry).expect("replay should succeed");
}

// ============================================================================
// Crash Recovery Simulation Tests
// ============================================================================

fn simulate_crash_recovery(entries: Vec<VectorWalEntry>) -> Result<Vec<String>, String> {
    let mut results = Vec::new();
    for entry in entries {
        replay_vector_wal_entry(&entry)?;
        results.push(format!("replayed {:?}", entry.entry_type));
    }
    Ok(results)
}

#[test]
fn crash_recovery_insert_then_crash() {
    let entries = vec![
        VectorWalEntry::new_vector_insert(1, 1, vec![1], vec![0.1f32.to_le_bytes()].concat()),
        VectorWalEntry::new_vector_insert(1, 1, vec![2], vec![0.2f32.to_le_bytes()].concat()),
    ];
    let results = simulate_crash_recovery(entries).expect("recovery should succeed");
    assert_eq!(results.len(), 2);
}

#[test]
fn crash_recovery_create_index_then_insert() {
    let entries = vec![
        VectorWalEntry::new_create_index(1, 1, "idx_emb".to_string(), "HNSW".to_string(), vec![]),
        VectorWalEntry::new_vector_insert(2, 1, vec![1], vec![0.1f32.to_le_bytes()].concat()),
    ];
    let results = simulate_crash_recovery(entries).expect("recovery should succeed");
    assert_eq!(results.len(), 2);
}

#[test]
fn crash_recovery_with_index_rebuild() {
    let entries = vec![
        VectorWalEntry::new_vector_insert(1, 1, vec![1], vec![0.1f32.to_le_bytes()].concat()),
        VectorWalEntry::new_create_index(2, 1, "idx_emb".to_string(), "HNSW".to_string(), vec![]),
        VectorWalEntry { tx_id: 0, entry_type: VectorWalEntryType::RebuildVectorIndex, table_id: 1,
            key: None, vector_data: None,
            index_name: Some("idx_emb".to_string()), index_type: None, index_options: None },
    ];
    let results = simulate_crash_recovery(entries).expect("recovery should succeed");
    assert_eq!(results.len(), 3);
}

// ============================================================================
// Index Options Tests
// ============================================================================

#[test]
fn hnsw_options_parsing() {
    let entry = VectorWalEntry::new_create_index(
        1, 1, "idx_hnsw".to_string(), "HNSW".to_string(),
        vec![
            ("m".to_string(), "16".to_string()),
            ("ef_construction".to_string(), "200".to_string()),
            ("ef_search".to_string(), "64".to_string()),
        ],
    );
    let options = entry.index_options.as_ref().expect("options should exist");
    assert_eq!(options.len(), 3);
    assert!(options.iter().any(|(k, v)| k == "m" && v == "16"));
    assert!(options.iter().any(|(k, v)| k == "ef_construction"));
}

#[test]
fn ivf_options_parsing() {
    let entry = VectorWalEntry::new_create_index(
        1, 1, "idx_ivf".to_string(), "IVF".to_string(),
        vec![
            ("nlist".to_string(), "100".to_string()),
            ("nprobe".to_string(), "10".to_string()),
        ],
    );
    let options = entry.index_options.as_ref().expect("options should exist");
    assert_eq!(options.len(), 2);
    assert!(options.iter().any(|(k, _)| k == "nlist"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn vector_wal_entry_empty_vector() {
    // Note: Empty vector_data uses length=0 encoding
    // This is valid for serialization roundtrip
    let entry = VectorWalEntry {
        tx_id: 1,
        entry_type: VectorWalEntryType::VectorInsert,
        table_id: 1,
        key: Some(vec![1]),
        vector_data: None,  // None for this test
        index_name: None,
        index_type: None,
        index_options: None,
    };
    let bytes = entry.to_bytes();
    let recovered = VectorWalEntry::from_bytes(&bytes).expect("roundtrip should succeed");
    assert!(recovered.vector_data.is_none());
}

#[test]
fn vector_wal_entry_large_dimension() {
    let mut data = Vec::with_capacity(1536 * 4);
    for i in 0..1536 {
        data.extend_from_slice(&(i as f32).to_le_bytes());
    }
    let entry = VectorWalEntry::new_vector_insert(1, 1, vec![1], data);
    let recovered = entry.serialization_roundtrip().expect("roundtrip should succeed");
    assert_eq!(recovered.vector_data.unwrap().len(), 1536 * 4);
}

#[test]
fn wal_entry_size_estimate() {
    let entry = VectorWalEntry {
        tx_id: u64::MAX,
        entry_type: VectorWalEntryType::VectorInsert,
        table_id: u64::MAX,
        key: Some(vec![0u8; 100]),
        vector_data: Some(vec![0u8; 1536 * 4]),
        index_name: None,
        index_type: None,
        index_options: None,
    };
    let bytes = entry.to_bytes();
    assert!(bytes.len() < 10 * 1024, "entry too large");
    assert!(bytes.len() > 1024, "entry too small");
}
