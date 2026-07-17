//! Append-Only Storage Engine
//!
//! V311-08: Incremental write implementation

use crate::engine::{Record, SqlError, SqlResult, StorageEngine, TableInfo, Value};
use std::collections::{BTreeMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::PathBuf;

pub struct AppendOnlyStorage {
    data_dir: PathBuf,
    tables: BTreeMap<String, TableInfo>,
    indexes: BTreeMap<String, BTreeMap<String, u64>>,
    cache: BTreeMap<String, Vec<Record>>,
    tombstones: BTreeMap<String, HashSet<String>>,
    dirty: bool,
    current_tx_id: u64,
}

impl AppendOnlyStorage {
    pub fn new(data_dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        Ok(Self {
            data_dir,
            tables: BTreeMap::new(),
            indexes: BTreeMap::new(),
            cache: BTreeMap::new(),
            tombstones: BTreeMap::new(),
            dirty: false,
            current_tx_id: 0,
        })
    }

    fn meta_path(&self, table: &str) -> PathBuf { self.data_dir.join(format!("{}.meta", table)) }
    fn log_path(&self, table: &str) -> PathBuf { self.data_dir.join(format!("{}.alog", table)) }
    fn tombstone_path(&self, table: &str) -> PathBuf { self.data_dir.join(format!("{}.tomb", table)) }

    fn serialize_record(&self, record: &Record) -> Vec<u8> {
        let mut buf = Vec::new();
        for value in record {
            match value {
                Value::Integer(i) => { buf.push(1); buf.extend(&i.to_le_bytes()); }
                Value::Float(f) => { buf.push(2); buf.extend(&f.to_le_bytes()); }
                Value::Text(s) => { buf.push(3); let bytes = s.as_bytes(); buf.extend(&(bytes.len() as u32).to_le_bytes()); buf.extend(bytes); }
                Value::Null => buf.push(0),
                _ => { buf.push(3); let s = value.to_string(); let bytes = s.as_bytes(); buf.extend(&(bytes.len() as u32).to_le_bytes()); buf.extend(bytes); }
            }
        }
        buf
    }

    fn deserialize_record(&self, buf: &[u8], num_columns: usize) -> Result<Record, String> {
        let mut record = Vec::with_capacity(num_columns);
        let mut pos = 0;
        for _ in 0..num_columns {
            if pos >= buf.len() { return Err("Unexpected end of data".to_string()); }
            let type_byte = buf[pos];
            pos += 1;
            match type_byte {
                0 => record.push(Value::Null),
                1 => {
                    if pos + 8 > buf.len() { return Err("Unexpected end of data for Integer".to_string()); }
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&buf[pos..pos + 8]);
                    record.push(Value::Integer(i64::from_le_bytes(bytes)));
                    pos += 8;
                }
                2 => {
                    if pos + 8 > buf.len() { return Err("Unexpected end of data for Float".to_string()); }
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&buf[pos..pos + 8]);
                    record.push(Value::Float(f64::from_le_bytes(bytes)));
                    pos += 8;
                }
                3 => {
                    if pos + 4 > buf.len() { return Err("Unexpected end of data for Text length".to_string()); }
                    let mut len_bytes = [0u8; 4];
                    len_bytes.copy_from_slice(&buf[pos..pos + 4]);
                    let len = u32::from_le_bytes(len_bytes) as usize;
                    pos += 4;
                    if pos + len > buf.len() { return Err("Unexpected end of data for Text".to_string()); }
                    let s = String::from_utf8(buf[pos..pos + len].to_vec()).map_err(|_| "Invalid UTF-8".to_string())?;
                    record.push(Value::Text(s));
                    pos += len;
                }
                _ => return Err(format!("Unknown type byte: {}", type_byte)),
            }
        }
        Ok(record)
    }

    fn read_log(&self, table: &str, num_columns: usize, tombstones: &HashSet<String>) -> SqlResult<Vec<Record>> {
        let log_path = self.log_path(table);
        if !log_path.exists() { return Ok(Vec::new()); }
        let mut file = File::open(&log_path).map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
        let mut records = Vec::new();
        let mut pos = 0;
        while pos < buf.len() {
            if pos + 4 > buf.len() { break; }
            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&buf[pos..pos + 4]);
            let len = u32::from_le_bytes(len_bytes) as usize;
            pos += 4;
            if pos + len > buf.len() { break; }
            let record_data = &buf[pos..pos + len];
            pos += len;
            if let Ok(record) = self.deserialize_record(record_data, num_columns) {
                let key = format!("{:?}", record.first());
                if !tombstones.contains(&key) { records.push(record); }
            }
        }
        Ok(records)
    }

    pub fn compact_table(&mut self, table: &str) -> std::io::Result<usize> {
        let log_path = self.log_path(table);
        if !log_path.exists() { return Ok(0); }
        let num_columns = match self.tables.get(table) { Some(t) => t.columns.len(), None => return Ok(0), };
        let tombstones = self.tombstones.get(table).cloned().unwrap_or_default();
        let records = self.read_log(table, num_columns, &tombstones).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let count = records.len();
        let mut new_file = File::create(&log_path)?;
        for record in &records {
            let data = self.serialize_record(record);
            new_file.write_all(&(data.len() as u32).to_le_bytes())?;
            new_file.write_all(&data)?;
        }
        new_file.flush()?;
        if let Some(tomb) = self.tombstones.get_mut(table) { tomb.clear(); }
        if let Some(cache) = self.cache.get_mut(table) { *cache = records; }
        Ok(count)
    }

    pub fn load_table(&mut self, table: &str) -> std::io::Result<()> {
        let log_path = self.log_path(table);
        if !log_path.exists() { return Ok(()); }
        let num_columns = match self.tables.get(table) { Some(t) => t.columns.len(), None => return Ok(()), };
        let tomb_path = self.tombstone_path(table);
        let tombstones: HashSet<String> = if tomb_path.exists() {
            let mut file = File::open(&tomb_path)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            contents.lines().map(|l| l.to_string()).collect()
        } else { HashSet::new() };
        let records = self.read_log(table, num_columns, &tombstones).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let mut index = BTreeMap::new();
        for (i, record) in records.iter().enumerate() {
            let key = format!("{:?}", record.first());
            index.insert(key, i as u64);
        }
        self.cache.insert(table.to_string(), records);
        self.indexes.insert(table.to_string(), index);
        self.tombstones.insert(table.to_string(), tombstones);
        Ok(())
    }

    pub fn load_all(&mut self) -> std::io::Result<()> {
        let tables: Vec<String> = self.tables.keys().cloned().collect();
        for table in tables { self.load_table(&table)?; }
        Ok(())
    }
}

impl StorageEngine for AppendOnlyStorage {
    fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
        if !self.tables.contains_key(table) { return Err(SqlError::ExecutionError(format!("Table not found: {}", table))); }
        let num_columns = self.tables.get(table).map(|t| t.columns.len()).unwrap_or(0);
        let serialized: Vec<(Vec<u8>, String, Record)> = records.iter().map(|record| {
            let data = self.serialize_record(record);
            let key = format!("{:?}", record.first());
            (data, key, record.clone())
        }).collect();
        let mut file = OpenOptions::new().create(true).append(true).open(self.log_path(table))
            .map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
        let mut offsets = Vec::new();
        for (data, _, _) in &serialized {
            let offset = file.stream_position().map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
            file.write_all(&(data.len() as u32).to_le_bytes()).map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
            file.write_all(data).map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
            offsets.push(offset);
        }
        file.flush().map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
        drop(file);
        if let Some(idx) = self.indexes.get_mut(table) {
            let cache = self.cache.entry(table.to_string()).or_insert_with(Vec::new);
            for (i, (_, key, record)) in serialized.iter().enumerate() {
                idx.insert(key.clone(), offsets[i]);
                cache.push(record.clone());
            }
        }
        self.dirty = true;
        Ok(())
    }

    fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
        let table_info = self.tables.get(table).ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))?;
        let tombstones = self.tombstones.get(table).cloned().unwrap_or_default();
        self.read_log(table, table_info.columns.len(), &tombstones)
    }

    fn delete(&mut self, table: &str, _filters: &[Value]) -> SqlResult<usize> {
        if !self.tables.contains_key(table) { return Err(SqlError::ExecutionError(format!("Table not found: {}", table))); }
        let num_columns = self.tables.get(table).map(|t| t.columns.len()).unwrap_or(0);
        let tombstones = self.tombstones.get(table).cloned().unwrap_or_default();
        let records = self.read_log(table, num_columns, &tombstones)?;
        if records.is_empty() { return Ok(0); }
        let keys: Vec<String> = records.iter().map(|r| format!("{:?}", r.first())).collect();
        let tomb_path = self.tombstone_path(table);
        { let mut file = OpenOptions::new().create(true).append(true).open(&tomb_path).map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
          for key in &keys { writeln!(file, "{}", key).map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?; }
          file.flush().map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?; }
        let tombstones = self.tombstones.entry(table.to_string()).or_insert_with(HashSet::new);
        for key in keys { tombstones.insert(key); }
        Ok(records.len())
    }

    fn delete_if(&mut self, table: &str, filter: &crate::engine::RowFilter) -> SqlResult<usize> {
        if !self.tables.contains_key(table) { return Err(SqlError::ExecutionError(format!("Table not found: {}", table))); }
        let num_columns = self.tables.get(table).map(|t| t.columns.len()).unwrap_or(0);
        let tombstones = self.tombstones.get(table).cloned().unwrap_or_default();
        let records = self.read_log(table, num_columns, &tombstones)?;
        let keys_to_delete: Vec<String> = records.iter().filter(|r| filter(r)).map(|r| format!("{:?}", r.first())).collect();
        if keys_to_delete.is_empty() { return Ok(0); }
        let tomb_path = self.tombstone_path(table);
        { let mut file = OpenOptions::new().create(true).append(true).open(&tomb_path).map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
          for key in &keys_to_delete { writeln!(file, "{}", key).map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?; }
          file.flush().map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?; }
        let tombstones = self.tombstones.entry(table.to_string()).or_insert_with(HashSet::new);
        for key in keys_to_delete { tombstones.insert(key); }
        Ok(tombstones.len())
    }

    fn update(&mut self, _table: &str, _filters: &[Value], _updates: &[(usize, Value)]) -> SqlResult<usize> { Ok(0) }
    fn update_if(&mut self, _table: &str, _filter: &crate::engine::RowFilter, _mutation: &crate::engine::RowMutation) -> SqlResult<usize> { Ok(0) }

    fn create_table(&mut self, info: &TableInfo) -> SqlResult<()> {
        self.tables.insert(info.name.clone(), info.clone());
        self.cache.insert(info.name.clone(), Vec::new());
        self.indexes.insert(info.name.clone(), BTreeMap::new());
        self.tombstones.insert(info.name.clone(), HashSet::new());
        let file = File::create(self.meta_path(&info.name)).map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
        let mut writer = std::io::BufWriter::new(file);
        serde_json::to_writer(&mut writer, info).map_err(|e| SqlError::ExecutionError(format!("JSON error: {}", e)))?;
        writer.flush().map_err(|e| SqlError::ExecutionError(format!("IO error: {}", e)))?;
        Ok(())
    }

    fn drop_table(&mut self, table: &str) -> SqlResult<()> { self.tables.remove(table); self.cache.remove(table); self.indexes.remove(table); self.tombstones.remove(table); Ok(()) }
    fn has_table(&self, table: &str) -> bool { self.tables.contains_key(table) }
    fn list_tables(&self) -> Vec<String> { self.tables.keys().cloned().collect() }
    fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> { self.tables.get(table).cloned().ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table))) }
    fn flush(&mut self) -> SqlResult<()> { self.dirty = false; Ok(()) }
    fn begin_transaction(&mut self) -> SqlResult<u64> { self.current_tx_id += 1; Ok(self.current_tx_id) }
    fn commit_transaction(&mut self) -> SqlResult<()> { self.current_tx_id = 0; Ok(()) }
    fn rollback_transaction(&mut self) -> SqlResult<()> { self.current_tx_id = 0; Ok(()) }
    fn in_transaction(&self) -> bool { self.current_tx_id != 0 }
    fn set_current_tx_id(&mut self, tx_id: u64) { self.current_tx_id = tx_id; }
    fn create_index(&mut self, _table: &str, _column: &str, _column_index: usize) -> SqlResult<()> { Ok(()) }
    fn drop_index(&mut self, _table: &str, _column: &str) -> SqlResult<()> { Ok(()) }
    fn add_column(&mut self, _table: &str, _column: crate::engine::ColumnDefinition) -> SqlResult<()> { Ok(()) }
    fn rename_table(&mut self, _table: &str, _new_name: &str) -> SqlResult<()> { Ok(()) }
    fn create_trigger(&mut self, _trigger: crate::engine::TriggerInfo) -> SqlResult<()> { Ok(()) }
    fn drop_trigger(&mut self, _name: &str) -> SqlResult<()> { Ok(()) }
    fn get_trigger(&self, _name: &str) -> Option<crate::engine::TriggerInfo> { None }
    fn list_triggers(&self, _table: &str) -> Vec<crate::engine::TriggerInfo> { vec![] }
    fn list_indexes(&self, _table: &str) -> Vec<(String, String)> { vec![] }
    fn has_view(&self, _name: &str) -> bool { false }
}
