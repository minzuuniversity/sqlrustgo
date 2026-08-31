use sqlrustgo_types::Value;

/// An entry in the per-transaction undo log.
///
/// `Insert` records the table + primary-key columns of a freshly inserted
/// row so a rollback can delete it. `Delete` / `Update` carry the full
/// pre-image row so a rollback can re-insert the old tuple verbatim.
///
/// #4519 (清华 MySQL 课程第 9 章核心): prior to this commit the
/// `key` and `old_value` fields were opaque `Vec<u8>` — the byte-level
/// representation could not be reverse-mapped back into storage
/// (`storage.delete(table, &[Value])` takes typed `Value`s). The physical
/// rollback closure in `SavepointManager::rollback_to` therefore never
/// ran (the orchestrator wired `|_| Ok(())`). This struct now carries
/// enough typed information for the executor's on-undo closure to drive
/// `storage.delete` / `storage.insert` directly.
///
/// # v312-60: empty-key fallback (`row` / `new_value`)
/// `key` holds the primary-key column values for the affected row and is
/// empty when the table has no declared primary key. To let replayers
/// (see `src/execution_engine.rs::execute_savepoint` and
/// `TransactionManager::rollback_with_undo`) safely handle that case
/// without wiping pre-transaction rows, `Insert` carries `row` (the full
/// inserted row) and `Update` carries `new_value` (the full post-update
/// row). Replayers MUST fall back to full-row matching
/// (`storage.delete(table, &row)` / `storage.delete(table, &new_value)`)
/// when `key` is empty — an empty filter to `storage.delete` means
/// "clear the whole table" in every storage engine
/// (`MemoryStorage::delete` / `FileStorage::delete`), which would wipe
/// pre-transaction rows and corrupt the database.
///
/// `Delete` does not need a separate fallback field because it re-inserts
/// `old_value` (which is already the full row); the empty-key case is
/// handled by the same fall-back path used by `Update` when needed.
#[derive(Debug, Clone)]
pub enum UndoRecord {
    Insert {
        table: String,
        key: Vec<Value>,
        /// Full inserted row; used by replayers to delete by full-row
        /// match when the table has no primary key.
        row: Vec<Value>,
    },
    Delete {
        table: String,
        key: Vec<Value>,
        old_value: Vec<Value>,
    },
    Update {
        table: String,
        key: Vec<Value>,
        old_value: Vec<Value>,
        /// Full post-update row; used by replayers to delete by
        /// full-row match when the table has no primary key (so the
        /// pre-image can be re-inserted via `old_value`).
        new_value: Vec<Value>,
    },
}

#[derive(Debug, Clone)]
pub struct Savepoint {
    pub name: String,
    pub undo_log_index: usize,
}

impl Savepoint {
    pub fn new(name: String, undo_log_index: usize) -> Self {
        Self {
            name,
            undo_log_index,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SavepointError {
    NotFound,
    InvalidOperation,
}

impl std::fmt::Display for SavepointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SavepointError::NotFound => write!(f, "savepoint not found"),
            SavepointError::InvalidOperation => write!(f, "invalid savepoint operation"),
        }
    }
}

impl std::error::Error for SavepointError {}

pub struct SavepointManager {
    savepoints: Vec<Savepoint>,
    undo_log: Vec<UndoRecord>,
}

// Manual Debug impl (auto-derive would work too, but the inner types
// already impl Debug via the additions above).
impl std::fmt::Debug for SavepointManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SavepointManager")
            .field("savepoints", &self.savepoints)
            .field("undo_log_len", &self.undo_log.len())
            .finish()
    }
}

impl SavepointManager {
    pub fn new() -> Self {
        Self {
            savepoints: Vec::new(),
            undo_log: Vec::new(),
        }
    }

    pub fn savepoint(&mut self, name: String) -> Result<(), SavepointError> {
        if let Some(idx) = self.savepoints.iter().rposition(|s| s.name == name) {
            self.savepoints[idx].undo_log_index = self.undo_log.len();
        } else {
            self.savepoints
                .push(Savepoint::new(name, self.undo_log.len()));
        }
        Ok(())
    }

    /// 物理回滚 (Phase 3): 从最新到保存点反向应用 UndoRecord
    ///
    /// 对 undo_log[sp.undo_log_index..] 中的记录反向遍历：
    /// - `Insert` → 回调 `on_delete(key)` 删除该行
    /// - `Delete` → 回调 `on_insert(key, old_value)` 恢复旧行
    /// - `Update` → 回调 `on_update(key, old_value)` 恢复旧值
    ///
    /// 清理 undo_log 和 savepoint 栈。
    ///
    /// # 参数
    /// - `name`: 目标 savepoint 名称
    /// - `on_undo`: 回调函数，接收 (UndoRecord) → Result<(), String>
    ///   由调用者（通常是 ExecutionEngine）提供实际的存储操作
    pub fn rollback_to<F>(&mut self, name: &str, on_undo: F) -> Result<(), SavepointError>
    where
        F: FnMut(&UndoRecord) -> Result<(), String>,
    {
        let idx = self
            .savepoints
            .iter()
            .rposition(|s| s.name == name)
            .ok_or(SavepointError::NotFound)?;

        let sp = &self.savepoints[idx];
        let mut undo = on_undo;

        // 反向遍历 undo_log 并应用
        for record in self.undo_log[sp.undo_log_index..].iter().rev() {
            if let Err(e) = undo(record) {
                // 回滚过程中出错，记录但继续（尽力而为）
                eprintln!("Savepoint undo failed (continuing): {}", e);
            }
        }

        while self.undo_log.len() > sp.undo_log_index {
            self.undo_log.pop();
        }

        self.savepoints.truncate(idx + 1);

        Ok(())
    }

    /// 旧 API 兼容: 仅清除 undo_log 不还原物理数据
    /// 新代码应使用 rollback_to(name, on_undo)
    #[deprecated(
        since = "3.9.0",
        note = "Use rollback_to(name, on_undo) for physical rollback"
    )]
    pub fn rollback_to_noop(&mut self, name: &str) -> Result<(), SavepointError> {
        let idx = self
            .savepoints
            .iter()
            .rposition(|s| s.name == name)
            .ok_or(SavepointError::NotFound)?;

        let sp = &self.savepoints[idx];

        while self.undo_log.len() > sp.undo_log_index {
            self.undo_log.pop();
        }

        self.savepoints.truncate(idx + 1);

        Ok(())
    }

    pub fn release_savepoint(&mut self, name: &str) -> Result<(), SavepointError> {
        self.savepoints.retain(|s| s.name != name);
        Ok(())
    }

    pub fn add_undo(&mut self, record: UndoRecord) {
        self.undo_log.push(record);
    }

    /// Issue #4581: drain the undo log so the caller (typically
    /// `TransactionManager::rollback_with_undo`) can replay the
    /// entries in reverse via its closure. The SavepointManager keeps
    /// an empty undo log after this call; subsequent DML operations
    /// inside the same transaction continue to append fresh entries.
    pub fn take_undo_log(&mut self) -> Vec<UndoRecord> {
        std::mem::take(&mut self.undo_log)
    }

    pub fn get_savepoint_count(&self) -> usize {
        self.savepoints.len()
    }

    /// Number of entries currently in the undo log. Added in #3110 so
    /// external tests can verify rollback behaviour without exposing the
    /// private `undo_log` field.
    pub fn undo_log_len(&self) -> usize {
        self.undo_log.len()
    }
}

impl Default for SavepointManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_record_insert() {
        let record = UndoRecord::Insert {
            table: "t".to_string(),
            key: vec![Value::Integer(1)],
            row: vec![Value::Integer(1)],
        };
        assert!(matches!(record, UndoRecord::Insert { .. }));
    }

    #[test]
    fn test_savepoint_new() {
        let sp = Savepoint::new("test".to_string(), 10);
        assert_eq!(sp.name, "test");
        assert_eq!(sp.undo_log_index, 10);
    }

    #[test]
    fn test_savepoint_create() {
        let mut manager = SavepointManager::new();
        manager.savepoint("sp1".to_string()).unwrap();
        assert_eq!(manager.savepoints.len(), 1);
    }

    #[test]
    fn test_nested_savepoints() {
        let mut manager = SavepointManager::new();
        manager.savepoint("sp1".to_string()).unwrap();
        manager.savepoint("sp2".to_string()).unwrap();
        assert_eq!(manager.savepoints.len(), 2);

        manager.rollback_to("sp1", |_| Ok(())).unwrap();
        assert_eq!(manager.savepoints.len(), 1);
    }

    #[test]
    fn test_savepoint_not_found() {
        let mut manager = SavepointManager::new();
        let result = manager.rollback_to("nonexistent", |_| Ok(()));
        assert!(matches!(result, Err(SavepointError::NotFound)));
    }

    #[test]
    fn test_savepoint_error_display() {
        let err_not_found = SavepointError::NotFound;
        let err_invalid = SavepointError::InvalidOperation;
        assert_eq!(err_not_found.to_string(), "savepoint not found");
        assert_eq!(err_invalid.to_string(), "invalid savepoint operation");
    }

    #[test]
    fn test_add_undo() {
        let mut manager = SavepointManager::new();
        manager.add_undo(UndoRecord::Insert {
            table: "t".to_string(),
            key: vec![Value::Integer(1)],
            row: vec![Value::Integer(1)],
        });
        assert_eq!(manager.undo_log.len(), 1);
    }

    #[test]
    fn test_add_undo_update() {
        let mut manager = SavepointManager::new();
        manager.add_undo(UndoRecord::Update {
            table: "t".to_string(),
            key: vec![Value::Integer(1)],
            old_value: vec![Value::Integer(2)],
            new_value: vec![Value::Integer(3)],
        });
        manager.add_undo(UndoRecord::Delete {
            table: "t".to_string(),
            key: vec![Value::Integer(3)],
            old_value: vec![Value::Integer(4)],
        });
        assert_eq!(manager.undo_log.len(), 2);
    }

    #[test]
    fn test_get_savepoint_count() {
        let mut manager = SavepointManager::new();
        assert_eq!(manager.get_savepoint_count(), 0);

        manager.savepoint("sp1".to_string()).unwrap();
        assert_eq!(manager.get_savepoint_count(), 1);

        manager.savepoint("sp2".to_string()).unwrap();
        assert_eq!(manager.get_savepoint_count(), 2);
    }

    #[test]
    fn test_release_savepoint() {
        let mut manager = SavepointManager::new();
        manager.savepoint("sp1".to_string()).unwrap();
        manager.savepoint("sp2".to_string()).unwrap();

        manager.release_savepoint("sp1").unwrap();
        assert_eq!(manager.get_savepoint_count(), 1);

        manager.release_savepoint("sp2").unwrap();
        assert_eq!(manager.get_savepoint_count(), 0);
    }

    #[test]
    fn test_release_nonexistent_savepoint() {
        let mut manager = SavepointManager::new();
        let result = manager.release_savepoint("nonexistent");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rollback_preserves_earlier_savepoints() {
        let mut manager = SavepointManager::new();
        manager.savepoint("sp1".to_string()).unwrap();
        manager.savepoint("sp2".to_string()).unwrap();

        manager.rollback_to("sp1", |_| Ok(())).unwrap();

        assert!(manager.savepoints.iter().any(|s| s.name == "sp1"));
    }

    #[test]
    fn test_savepoint_override() {
        let mut manager = SavepointManager::new();
        manager.add_undo(UndoRecord::Insert {
            table: "t".to_string(),
            key: vec![Value::Integer(1)],
            row: vec![Value::Integer(1)],
        });
        manager.savepoint("sp1".to_string()).unwrap();

        manager.add_undo(UndoRecord::Insert {
            table: "t".to_string(),
            key: vec![Value::Integer(2)],
            row: vec![Value::Integer(2)],
        });
        manager.add_undo(UndoRecord::Insert {
            table: "t".to_string(),
            key: vec![Value::Integer(3)],
            row: vec![Value::Integer(3)],
        });

        let idx = manager
            .savepoints
            .iter()
            .rposition(|s| s.name == "sp1")
            .unwrap();
        assert_eq!(manager.savepoints[idx].undo_log_index, 1);

        manager.savepoint("sp1".to_string()).unwrap();

        let idx2 = manager
            .savepoints
            .iter()
            .rposition(|s| s.name == "sp1")
            .unwrap();
        assert_eq!(manager.savepoints[idx2].undo_log_index, 3);
    }
}
