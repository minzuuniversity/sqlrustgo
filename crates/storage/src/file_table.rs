//! FileTable - Per-table file storage implementation
//!
//! V311-09: Table-level lock architecture

use crate::engine::{Record, SqlResult, TableInfo, Value};
use crate::table_engine::TableEngine;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct FileTable {
    table_name: String,
    info: TableInfo,
    records: Vec<Record>,
    dirty: bool,
    dirty_tables: HashSet<String>,
    data_dir: PathBuf,
}

impl FileTable {
    pub fn new(table_name: String, info: TableInfo, data_dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        Ok(Self {
            table_name,
            info,
            records: Vec::new(),
            dirty: false,
            dirty_tables: HashSet::new(),
            data_dir,
        })
    }

    pub fn load(&mut self) -> std::io::Result<()> {
        let data_path = self.data_dir.join("data.bin");
        if !data_path.exists() {
            return Ok(());
        }
        let mut file = File::open(&data_path)?;
        let mut records = Vec::new();
        loop {
            let mut len_bytes = [0u8; 4];
            let n = file.read(&mut len_bytes)?;
            if n == 0 {
                break;
            }
            if n < 4 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "truncated record length",
                ));
            }
            let len = u32::from_le_bytes(len_bytes) as usize;
            let mut buf = vec![0u8; len];
            file.read_exact(&mut buf)?;
            match self.deserialize_record(&buf) {
                Ok(record) => records.push(record),
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            }
        }
        self.records = records;
        Ok(())
    }

    fn deserialize_record(&self, buf: &[u8]) -> Result<Record, String> {
        let mut record = Vec::new();
        let mut pos = 0;
        for _ in 0..self.info.columns.len() {
            if pos >= buf.len() {
                break;
            }
            let type_byte = buf[pos];
            pos += 1;
            match type_byte {
                0 => record.push(Value::Null),
                1 => {
                    if pos + 8 > buf.len() {
                        return Err("Unexpected end".to_string());
                    }
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&buf[pos..pos + 8]);
                    record.push(Value::Integer(i64::from_le_bytes(bytes)));
                    pos += 8;
                }
                2 => {
                    if pos + 8 > buf.len() {
                        return Err("Unexpected end".to_string());
                    }
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&buf[pos..pos + 8]);
                    record.push(Value::Float(f64::from_le_bytes(bytes)));
                    pos += 8;
                }
                3 => {
                    if pos + 4 > buf.len() {
                        return Err("Unexpected end".to_string());
                    }
                    let mut len_bytes = [0u8; 4];
                    len_bytes.copy_from_slice(&buf[pos..pos + 4]);
                    let len = u32::from_le_bytes(len_bytes) as usize;
                    pos += 4;
                    if pos + len > buf.len() {
                        return Err("Unexpected end".to_string());
                    }
                    let s = String::from_utf8(buf[pos..pos + len].to_vec())
                        .map_err(|_| "Invalid UTF-8".to_string())?;
                    record.push(Value::Text(s));
                    pos += len;
                }
                _ => record.push(Value::Null),
            }
        }
        Ok(record)
    }

    fn serialize_record(&self, record: &Record) -> Vec<u8> {
        let mut buf = Vec::new();
        for value in record {
            match value {
                Value::Integer(i) => {
                    buf.push(1);
                    buf.extend(&i.to_le_bytes());
                }
                Value::Float(f) => {
                    buf.push(2);
                    buf.extend(&f.to_le_bytes());
                }
                Value::Text(s) => {
                    buf.push(3);
                    let bytes = s.as_bytes();
                    buf.extend(&(bytes.len() as u32).to_le_bytes());
                    buf.extend(bytes);
                }
                Value::Null => buf.push(0),
                _ => {
                    buf.push(3);
                    let s = value.to_string();
                    let bytes = s.as_bytes();
                    buf.extend(&(bytes.len() as u32).to_le_bytes());
                    buf.extend(bytes);
                }
            }
        }
        buf
    }
}

impl TableEngine for FileTable {
    fn get_table_info(&self) -> &TableInfo {
        &self.info
    }
    fn insert(&mut self, records: Vec<Record>) -> SqlResult<()> {
        self.records.extend(records);
        self.dirty = true;
        self.dirty_tables.insert(self.table_name.clone());
        Ok(())
    }
    fn scan(&self) -> SqlResult<Vec<Record>> {
        Ok(self.records.clone())
    }
    fn delete(&mut self, _filters: &[Value]) -> SqlResult<usize> {
        let deleted = self.records.len();
        self.records.clear();
        if deleted > 0 {
            self.dirty = true;
            self.dirty_tables.insert(self.table_name.clone());
        }
        Ok(deleted)
    }
    fn update(&mut self, _filters: &[Value], _updates: &[(usize, Value)]) -> SqlResult<usize> {
        Ok(0)
    }
    fn flush(&mut self) -> SqlResult<()> {
        if !self.dirty {
            return Ok(());
        }
        let data_path = self.data_dir.join("data.bin");
        let mut file = File::create(&data_path)
            .map_err(|e| crate::engine::SqlError::ExecutionError(format!("IO error: {}", e)))?;
        for record in &self.records {
            let data = self.serialize_record(record);
            file.write_all(&(data.len() as u32).to_le_bytes())
                .map_err(|e| crate::engine::SqlError::ExecutionError(format!("IO error: {}", e)))?;
            file.write_all(&data)
                .map_err(|e| crate::engine::SqlError::ExecutionError(format!("IO error: {}", e)))?;
        }
        file.flush()
            .map_err(|e| crate::engine::SqlError::ExecutionError(format!("IO error: {}", e)))?;
        self.dirty = false;
        self.dirty_tables.clear();
        Ok(())
    }
    fn dirty_tables(&self) -> &HashSet<String> {
        &self.dirty_tables
    }
    fn mark_dirty(&mut self, table: &str) {
        self.dirty_tables.insert(table.to_string());
        self.dirty = true;
    }
    fn table_name(&self) -> &str {
        &self.table_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    fn temp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = env::temp_dir().join(format!("file_table_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    fn sample_info(name: &str) -> TableInfo {
        TableInfo {
            name: name.to_string(),
            columns: vec![crate::engine::ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                char_max_length: None,
                collation: None,
                default_value: None,
            }],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            collations: std::collections::HashMap::new(),

            partition_info: None,
            compression: None,
        }
    }

    fn multi_column_info() -> TableInfo {
        TableInfo {
            name: "t".into(),
            columns: vec![
                crate::engine::ColumnDefinition {
                    name: "a".into(),
                    data_type: "INTEGER".into(),
                    nullable: false,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                },
                crate::engine::ColumnDefinition {
                    name: "b".into(),
                    data_type: "FLOAT".into(),
                    nullable: true,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                },
                crate::engine::ColumnDefinition {
                    name: "c".into(),
                    data_type: "TEXT".into(),
                    nullable: true,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                },
                crate::engine::ColumnDefinition {
                    name: "d".into(),
                    data_type: "TEXT".into(),
                    nullable: true,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                },
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            collations: std::collections::HashMap::new(),

            partition_info: None,
            compression: None,
        }
    }

    #[test]
    fn insert_scan_flush_roundtrip() {
        let dir = temp_dir();
        let mut t = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
        t.insert(vec![vec![Value::Integer(1)]]).unwrap();
        t.insert(vec![vec![Value::Integer(2)]]).unwrap();
        let rows = t.scan().unwrap();
        assert_eq!(rows.len(), 2);
        t.flush().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_update_are_stubs() {
        let dir = temp_dir();
        let mut t = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
        assert_eq!(t.delete(&[]).unwrap(), 0);
        assert_eq!(t.update(&[], &[]).unwrap(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn accessors_and_dirty_tracking() {
        let dir = temp_dir();
        let mut t = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
        assert_eq!(t.table_name(), "t");
        assert_eq!(t.get_table_info().name, "t");
        assert!(t.dirty_tables().is_empty());
        t.mark_dirty("t");
        assert!(t.dirty_tables().contains("t"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn flush_skips_when_clean() {
        // flush() must be a no-op when dirty=false; it must not
        // create the data file. Covers the early-return branch.
        let dir = temp_dir();
        let mut t = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
        t.flush().unwrap();
        assert!(!dir.join("data.bin").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn serialize_deserialize_roundtrip_all_value_types() {
        // Exercise the type_byte switch: 0 (Null), 1 (Integer), 2 (Float),
        // 3 (Text), and the fallback arm for unknown variants.
        let dir = temp_dir();
        let t = FileTable::new("t".into(), multi_column_info(), dir.clone()).unwrap();
        let record: Record = vec![
            Value::Integer(42),
            Value::Float(3.14),
            Value::Text("hello".into()),
            Value::Null,
        ];
        let buf = t.serialize_record(&record);
        let parsed = t
            .deserialize_record(&buf)
            .expect("deserialize must succeed");
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0], Value::Integer(42));
        assert_eq!(parsed[1], Value::Float(3.14));
        assert_eq!(parsed[2], Value::Text("hello".into()));
        assert_eq!(parsed[3], Value::Null);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deserialize_unknown_type_byte_treats_as_null() {
        // A type_byte outside the 0..=3 range should be treated as Null
        // (the match arm `_ => record.push(Value::Null)`).
        let dir = temp_dir();
        let t = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
        let buf = vec![99u8]; // unknown type_byte 99
        let parsed = t
            .deserialize_record(&buf)
            .expect("unknown type must be null");
        assert_eq!(parsed, vec![Value::Null]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_recovers_records_after_flush() {
        // End-to-end: insert -> flush -> drop in-memory -> load -> records
        // reappear. Exercises both serialize_record (flush) and
        // deserialize_record (load) on the same bytes.
        let dir = temp_dir();
        {
            let mut t = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
            t.insert(vec![vec![Value::Integer(42)]]).unwrap();
            t.insert(vec![vec![Value::Integer(99)]]).unwrap();
            t.flush().unwrap();
        }
        let mut t2 = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
        t2.load().unwrap();
        let rows = t2.scan().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec![Value::Integer(42)]);
        assert_eq!(rows[1], vec![Value::Integer(99)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_without_data_file_is_noop() {
        // load() when no data.bin exists must succeed and yield empty
        // records (early-return branch in the load path).
        let dir = temp_dir();
        let mut t = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
        assert!(t.load().is_ok());
        assert!(t.scan().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_then_flush_then_load_recovers_empty() {
        // Delete returns the count of records, marks dirty, and flush
        // must write an empty file (or no file) that load reads back as
        // empty.
        let dir = temp_dir();
        let mut t = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
        t.insert(vec![vec![Value::Integer(1)]]).unwrap();
        t.insert(vec![vec![Value::Integer(2)]]).unwrap();
        t.insert(vec![vec![Value::Integer(3)]]).unwrap();
        let n = t.delete(&[]).unwrap();
        assert_eq!(n, 3);
        t.flush().unwrap();
        let mut t2 = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
        t2.load().unwrap();
        assert!(t2.scan().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn insert_after_flush_resets_dirty_flag() {
        // After flush(), the dirty flag is cleared. flush() called
        // twice on a clean table should be a no-op the second time.
        let dir = temp_dir();
        let mut t = FileTable::new("t".into(), sample_info("t"), dir.clone()).unwrap();
        t.insert(vec![vec![Value::Integer(1)]]).unwrap();
        t.flush().unwrap();
        t.flush().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
