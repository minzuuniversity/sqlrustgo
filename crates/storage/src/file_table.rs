//! FileTable - Per-table file storage implementation
//!
//! V311-09: Table-level lock architecture

use crate::engine::{Record, SqlResult, TableInfo, Value};
use crate::table_engine::TableEngine;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
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
        if data_path.exists() {
            let mut file = File::open(&data_path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            let mut pos = 0;
            while pos + 4 <= buf.len() {
                let mut len_bytes = [0u8; 4];
                len_bytes.copy_from_slice(&buf[pos..pos+4]);
                let len = u32::from_le_bytes(len_bytes) as usize;
                pos += 4;
                if pos + len > buf.len() { break; }
                match self.deserialize_record(&buf[pos..pos+len]) {
                    Ok(r) => self.records.push(r),
                    Err(_) => break,
                }
                pos += len;
            }
        }
        Ok(())
    }
    
    fn deserialize_record(&self, buf: &[u8]) -> Result<Record, String> {
        let mut record = Vec::new();
        let mut pos = 0;
        for _ in 0..self.info.columns.len() {
            if pos >= buf.len() { break; }
            let type_byte = buf[pos];
            pos += 1;
            match type_byte {
                0 => record.push(Value::Null),
                1 => {
                    if pos + 8 > buf.len() { return Err("Unexpected end".to_string()); }
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&buf[pos..pos+8]);
                    record.push(Value::Integer(i64::from_le_bytes(bytes)));
                    pos += 8;
                }
                2 => {
                    if pos + 8 > buf.len() { return Err("Unexpected end".to_string()); }
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(&buf[pos..pos+8]);
                    record.push(Value::Float(f64::from_le_bytes(bytes)));
                    pos += 8;
                }
                3 => {
                    if pos + 4 > buf.len() { return Err("Unexpected end".to_string()); }
                    let mut len_bytes = [0u8; 4];
                    len_bytes.copy_from_slice(&buf[pos..pos+4]);
                    let len = u32::from_le_bytes(len_bytes) as usize;
                    pos += 4;
                    if pos + len > buf.len() { return Err("Unexpected end".to_string()); }
                    let s = String::from_utf8(buf[pos..pos+len].to_vec())
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
                Value::Integer(i) => { buf.push(1); buf.extend(&i.to_le_bytes()); }
                Value::Float(f) => { buf.push(2); buf.extend(&f.to_le_bytes()); }
                Value::Text(s) => { buf.push(3); let bytes = s.as_bytes(); buf.extend(&(bytes.len() as u32).to_le_bytes()); buf.extend(bytes); }
                Value::Null => buf.push(0),
                _ => { buf.push(3); let s = value.to_string(); let bytes = s.as_bytes(); buf.extend(&(bytes.len() as u32).to_le_bytes()); buf.extend(bytes); }
            }
        }
        buf
    }
}

impl TableEngine for FileTable {
    fn get_table_info(&self) -> &TableInfo { &self.info }
    fn insert(&mut self, records: Vec<Record>) -> SqlResult<()> {
        self.records.extend(records);
        self.dirty = true;
        self.dirty_tables.insert(self.table_name.clone());
        Ok(())
    }
    fn scan(&self) -> SqlResult<Vec<Record>> { Ok(self.records.clone()) }
    fn delete(&mut self, _filters: &[Value]) -> SqlResult<usize> {
        let deleted = self.records.len();
        self.records.clear();
        if deleted > 0 { self.dirty = true; self.dirty_tables.insert(self.table_name.clone()); }
        Ok(deleted)
    }
    fn update(&mut self, _filters: &[Value], _updates: &[(usize, Value)]) -> SqlResult<usize> { Ok(0) }
    fn flush(&mut self) -> SqlResult<()> {
        if !self.dirty { return Ok(()); }
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
    fn dirty_tables(&self) -> &HashSet<String> { &self.dirty_tables }
    fn mark_dirty(&mut self, table: &str) { self.dirty_tables.insert(table.to_string()); self.dirty = true; }
    fn table_name(&self) -> &str { &self.table_name }
}
