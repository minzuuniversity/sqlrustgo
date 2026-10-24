//! Backup and Restore Module
//!
//! Logical backup and restore (mysqldump style) for SQLRustGo.
//!
//! # #5525 — what used to be here was a skeleton that reported success
//!
//! while doing nothing:
//!
//! * [`run_backup`] handed the backup an **empty** table map, so the `.sql`
//!   it produced contained **zero** tables — and it still printed
//!   `Backup completed successfully!`.
//! * The DDL was derived from the **first row's** column names
//!   (`first_row.keys()`), so it carried no declared type, no `NOT NULL` and
//!   no primary key — and a table with **no rows vanished from the backup
//!   entirely**.
//! * [`BackupManager::restore`] counted `INSERT INTO` lines and returned an
//!   empty row list per table: it restored **nothing** and reported a table
//!   count as if it had.
//! * `schema_only` / `compress` / `drop_first` / `backup_type` were
//!   **accepted and then silently dropped**.
//!
//! What the module does now:
//!
//! * **Enumerates tables for real** — [`collect_all_tables`] walks a
//!   [`StorageEngine`], and a database with no tables is an **error**, not a
//!   zero-byte "success".
//! * Writes **typed DDL** — declared type, `NOT NULL`, `PRIMARY KEY`,
//!   `UNIQUE`, `FOREIGN KEY` — and keeps **empty tables** in the dump.
//! * Writes rows with **correct escaping** — `'` doubled, backslash escaped,
//!   `NULL` unquoted, numbers unquoted, blobs as `X'..'`.
//! * **Restores for real** — [`BackupManager::restore_into`] writes the
//!   tables and the rows into a [`StorageEngine`] and then **verifies the
//!   landed row count** of every table.
//! * Turns every previously-ignored switch into an **explicit** behaviour:
//!   `schema_only` works, `compress` and a non-`full` `backup_type` are
//!   **rejected** with an error, and `drop_first` works.
//!
//! ## Artifacts
//!
//! A backup is three files that live next to each other:
//!
//! | file | role |
//! |------|------|
//! | `<id>.sql` | the portable dump — `mysql < <id>.sql` loads it |
//! | `<id>.data.json` | the **payload** — the machine-readable source `restore` consumes |
//! | `<id>.meta.json` | the metadata (id, type, size, checksum, table list) |
//!
//! The `.sql` and the `.data.json` are both rendered from the *same*
//! [`BackupContent`], so they cannot drift: the `.sql` is a **rendering** of
//! the payload, not a second source of truth. `restore` reads the payload,
//! because a round trip through SQL literals cannot represent every value
//! faithfully (JSON documents, `NaN`, `-0.0`).

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlrustgo_storage::{
    ColumnDefinition, ForeignKeyAction, ForeignKeyConstraint, Record, StorageEngine, TableInfo,
    UniqueConstraint, Value,
};

/// Backup type
#[derive(Debug, Clone)]
pub enum BackupType {
    Full,
    Incremental,
    Differential,
}

/// Backup status
#[derive(Debug, Clone)]
pub enum BackupStatus {
    InProgress,
    Completed,
    #[allow(dead_code)]
    Failed(String),
}

/// Backup metadata
#[derive(Debug, Clone)]
pub struct BackupMetadata {
    pub id: String,
    pub backup_type: BackupType,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub size_bytes: u64,
    pub database: String,
    pub tables: Vec<String>,
    pub status: BackupStatus,
    /// SHA-256 of the `.sql` artifact, hex encoded.
    ///
    /// #5525: this used to be `md5_simple`, a 32-bit rolling hash that
    /// cannot detect corruption. A checksum that a restore trusts has to
    /// be a real hash.
    pub checksum: Option<String>,
    /// SHA-256 of the `.data.json` payload, hex encoded.
    pub payload_checksum: Option<String>,
}

impl BackupMetadata {
    pub fn new(id: String, backup_type: BackupType, database: String) -> Self {
        Self {
            id,
            backup_type,
            started_at: chrono_lite_now(),
            completed_at: None,
            size_bytes: 0,
            database,
            tables: Vec::new(),
            status: BackupStatus::InProgress,
            checksum: None,
            payload_checksum: None,
        }
    }

    pub fn complete(&mut self, size: u64, checksum: String) {
        self.completed_at = Some(chrono_lite_now());
        self.size_bytes = size;
        self.status = BackupStatus::Completed;
        self.checksum = Some(checksum);
    }

    #[allow(dead_code)]
    pub fn fail(&mut self, error: String) {
        self.completed_at = Some(chrono_lite_now());
        self.status = BackupStatus::Failed(error);
    }
}

/// Simple datetime string (no external dependency)
pub fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}", now)
}

// ============================================================================
// Typed backup content — the single source of truth
// ============================================================================

/// One column of a logical backup.
///
/// #5525: the old backup guessed the column list from the first row's keys,
/// so it had no type information at all. The declared type now travels with
/// the backup instead of being re-derived (and lost) on the way out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupColumn {
    /// Column name, exactly as the catalog stores it.
    pub name: String,
    /// Declared SQL type, exactly as the catalog stores it.
    pub data_type: String,
    /// `CHAR(n)` / `VARCHAR(n)` width, when the catalog recorded one.
    #[serde(default)]
    pub char_max_length: Option<usize>,
    /// The column is part of the primary key.
    #[serde(default)]
    pub primary_key: bool,
    /// The column rejects `NULL`.
    #[serde(default)]
    pub not_null: bool,
}

impl BackupColumn {
    /// A column with the given name and declared type.
    pub fn new(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            char_max_length: None,
            primary_key: false,
            not_null: false,
        }
    }

    /// Mark the column as part of the primary key.
    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }

    /// Mark the column as `NOT NULL`.
    pub fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    /// Record the declared width of a `CHAR(n)` / `VARCHAR(n)` column.
    pub fn with_max_length(mut self, len: usize) -> Self {
        self.char_max_length = Some(len);
        self
    }

    /// The type as it appears in a `CREATE TABLE` statement.
    pub fn type_sql(&self) -> String {
        match self.char_max_length {
            Some(len) if !self.data_type.contains('(') => format!("{}({})", self.data_type, len),
            _ => self.data_type.clone(),
        }
    }
}

/// A `UNIQUE` constraint of a logical backup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupUniqueConstraint {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

/// A `FOREIGN KEY` of a logical backup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupForeignKey {
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<ForeignKeyAction>,
    pub on_update: Option<ForeignKeyAction>,
}

/// One table of a logical backup.
///
/// An **empty** `rows` is a legitimate table: the dump still carries its
/// `CREATE TABLE`, so restoring it recreates an empty table rather than
/// dropping it from the schema. #5525: the old dump only emitted a table
/// when it had at least one row, so empty tables disappeared entirely.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupTable {
    pub name: String,
    pub columns: Vec<BackupColumn>,
    /// Row values, positionally aligned with `columns`.
    #[serde(default)]
    pub rows: Vec<Vec<Value>>,
    #[serde(default)]
    pub unique_constraints: Vec<BackupUniqueConstraint>,
    #[serde(default)]
    pub foreign_keys: Vec<BackupForeignKey>,
}

impl BackupTable {
    /// A table with the given name and column list (no rows yet).
    pub fn new(name: impl Into<String>, columns: Vec<BackupColumn>) -> Self {
        Self {
            name: name.into(),
            columns,
            rows: Vec::new(),
            unique_constraints: Vec::new(),
            foreign_keys: Vec::new(),
        }
    }

    /// Number of rows in the table.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// The catalog description of this table, so a restore can recreate it.
    pub fn to_table_info(&self) -> TableInfo {
        TableInfo {
            name: self.name.clone(),
            columns: self
                .columns
                .iter()
                .map(|c| ColumnDefinition {
                    name: c.name.clone(),
                    data_type: c.data_type.clone(),
                    nullable: !c.not_null,
                    primary_key: c.primary_key,
                    char_max_length: c.char_max_length,
                    ..Default::default()
                })
                .collect(),
            unique_constraints: self
                .unique_constraints
                .iter()
                .map(|u| UniqueConstraint {
                    name: u.name.clone(),
                    columns: u.columns.clone(),
                })
                .collect(),
            foreign_keys: self
                .foreign_keys
                .iter()
                .map(|f| ForeignKeyConstraint {
                    name: f.name.clone(),
                    columns: f.columns.clone(),
                    referenced_table: f.referenced_table.clone(),
                    referenced_columns: f.referenced_columns.clone(),
                    on_delete: f.on_delete.clone(),
                    on_update: f.on_update.clone(),
                })
                .collect(),
            ..Default::default()
        }
    }
}

/// The complete content of a logical backup.
///
/// This is the payload: the `.sql` dump and the `.data.json` file are both
/// rendered from it, and `restore` reads it back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupContent {
    pub database: String,
    pub tables: Vec<BackupTable>,
}

/// The on-disk envelope of a backup payload.
///
/// The body carries its own SHA-256 so that a corrupted payload is detected
/// **before** it is restored, instead of being restored into a database as if
/// it were intact.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupPayload {
    checksum: String,
    content: BackupContent,
}

/// Backup manager
pub struct BackupManager {
    backup_dir: PathBuf,
    metadata: RwLock<HashMap<String, BackupMetadata>>,
}

impl BackupManager {
    pub fn new(backup_dir: PathBuf) -> Self {
        fs::create_dir_all(&backup_dir).ok();
        Self {
            backup_dir,
            metadata: RwLock::new(HashMap::new()),
        }
    }

    /// Create a full logical backup from an explicit, typed table list.
    ///
    /// The table list is **not** optional and **not** inferred: a caller
    /// that has no tables has nothing to back up, and
    /// [`collect_all_tables`](collect_all_tables) refuses to produce an
    /// empty list in the first place.
    pub fn create_backup(
        &self,
        database: &str,
        tables: &[BackupTable],
    ) -> Result<BackupMetadata, String> {
        self.create_backup_with(database, tables, &ExportOptions::default())
    }

    /// Create a full logical backup with explicit export options.
    pub fn create_backup_with(
        &self,
        database: &str,
        tables: &[BackupTable],
        options: &ExportOptions,
    ) -> Result<BackupMetadata, String> {
        // A switch that is accepted and then ignored is the bug #5525 is
        // about, so an unimplemented one is an error, not a no-op.
        if options.lock_tables {
            return Err(
                "LOCK TABLES is not implemented: refusing to write a dump that promises \
                 table locking it does not take"
                    .to_string(),
            );
        }
        if tables.is_empty() {
            return Err(format!(
                "no tables to back up in database `{database}`: refusing to write an empty \
                 backup that would be reported as a success"
            ));
        }
        for table in tables {
            if table.columns.is_empty() {
                return Err(format!(
                    "table `{}` has no columns: refusing to write a dump without a schema",
                    table.name
                ));
            }
            for row in &table.rows {
                if row.len() != table.columns.len() {
                    return Err(format!(
                        "table `{}` has a row with {} values but {} columns: refusing to \
                         write a dump whose rows do not match its schema",
                        table.name,
                        row.len(),
                        table.columns.len()
                    ));
                }
            }
        }

        let backup_id = format!("backup_{}", chrono_lite_now());
        let mut metadata =
            BackupMetadata::new(backup_id.clone(), BackupType::Full, database.to_string());
        metadata.tables = tables.iter().map(|t| t.name.clone()).collect();

        let content = BackupContent {
            database: database.to_string(),
            tables: tables.to_vec(),
        };

        let sql = render_sql(&content, options)?;
        // The envelope's checksum covers the **payload**, not the dump: the
        // payload is what `restore` trusts, so that is what has to be
        // verified.
        let body = serde_json::to_string(&content)
            .map_err(|e| format!("cannot serialize backup payload: {e}"))?;
        let payload_checksum = sha256_hex(body.as_bytes());
        let payload = BackupPayload {
            checksum: payload_checksum.clone(),
            content,
        };
        let payload_json = serde_json::to_string(&payload)
            .map_err(|e| format!("cannot serialize backup payload: {e}"))?;

        // The `.sql` is the portable artifact, the `.data.json` is the
        // payload `restore` consumes. Both come from the same `BackupContent`
        // above, so they cannot disagree about the backup.
        let sql_file = self.backup_dir.join(format!("{}.sql", backup_id));
        write_artifact(&sql_file, sql.as_bytes())?;
        let payload_file = self.backup_dir.join(format!("{}.data.json", backup_id));
        write_artifact(&payload_file, payload_json.as_bytes())?;

        metadata.payload_checksum = Some(payload_checksum);
        metadata.complete(
            (sql.len() + payload_json.len()) as u64,
            sha256_hex(sql.as_bytes()),
        );

        let meta_file = self.backup_dir.join(format!("{}.meta.json", backup_id));
        let meta_content = serde_json_simple(&metadata);
        fs::write(&meta_file, meta_content).ok();

        self.metadata
            .write()
            .unwrap()
            .insert(backup_id.clone(), metadata.clone());

        Ok(metadata)
    }

    /// Read back the payload of a backup.
    ///
    /// The payload's own SHA-256 is verified before the content is handed
    /// out: a corrupted backup is an error, never a partial restore.
    pub fn read_backup(&self, backup_id: &str) -> Result<BackupContent, String> {
        let payload_file = self.backup_dir.join(format!("{}.data.json", backup_id));
        if !payload_file.exists() {
            return Err(format!(
                "backup payload not found: {backup_id} (expected {})",
                payload_file.display()
            ));
        }
        let raw = fs::read_to_string(&payload_file)
            .map_err(|e| format!("cannot read backup {backup_id}: {e}"))?;
        let payload: BackupPayload = serde_json::from_str(&raw)
            .map_err(|e| format!("cannot parse backup payload {backup_id}: {e}"))?;

        // `serde_json` round-trips a struct deterministically, so re-encoding
        // the parsed content yields the bytes the checksum was taken over.
        let body = serde_json::to_string(&payload.content)
            .map_err(|e| format!("cannot re-encode backup payload {backup_id}: {e}"))?;
        let actual = sha256_hex(body.as_bytes());
        if actual != payload.checksum {
            return Err(format!(
                "backup payload {backup_id} is corrupt: checksum mismatch \
                 (recorded {}, computed {})",
                payload.checksum, actual
            ));
        }
        if payload.content.tables.is_empty() {
            return Err(format!(
                "backup {backup_id} contains no tables: refusing to restore an empty backup"
            ));
        }
        Ok(payload.content)
    }

    /// List all backups
    #[allow(dead_code)]
    pub fn list_backups(&self) -> Vec<BackupMetadata> {
        self.metadata.read().unwrap().values().cloned().collect()
    }

    /// Get backup metadata
    #[allow(dead_code)]
    pub fn get_backup(&self, id: &str) -> Option<BackupMetadata> {
        self.metadata.read().unwrap().get(id).cloned()
    }

    /// Read back the payload of a backup.
    ///
    /// #5525: this used to *count* `INSERT INTO` lines and return an empty
    /// row list per table, so it restored nothing while reporting a table
    /// count. It now returns the payload; use
    /// [`restore_into`](Self::restore_into) to actually write the data back.
    pub fn restore(&self, backup_id: &str) -> Result<BackupContent, String> {
        self.read_backup(backup_id)
    }

    /// Restore a backup into a storage — **for real**.
    ///
    /// Every table of the payload is created and its rows are inserted, and
    /// the landed row count of each table is verified against the payload.
    /// A mismatch is an error: the restore does not report a table count it
    /// did not actually land.
    ///
    /// `drop_first` removes an already-existing table before recreating it.
    /// Without it, restoring onto an existing table is an **error** — the
    /// storage's `create_table_in_db` overwrites silently, so a restore that
    /// did not check would destroy the target's data without saying so.
    pub fn restore_into(
        &self,
        backup_id: &str,
        storage: &mut dyn StorageEngine,
        db: &str,
        drop_first: bool,
    ) -> Result<RestoreResult, String> {
        let content = self.read_backup(backup_id)?;
        let started = Instant::now();

        let mut tables_restored = 0usize;
        let mut rows_restored = 0u64;

        for table in &content.tables {
            if storage.has_table_in(db, &table.name) {
                if !drop_first {
                    return Err(format!(
                        "table `{}` already exists in database `{db}`: refusing to overwrite it. \
                         Pass --drop-first to replace it.",
                        table.name
                    ));
                }
                storage
                    .drop_table_in_db(db, &table.name)
                    .map_err(|e| format!("cannot drop table `{}`: {e}", table.name))?;
            }

            storage
                .create_table_in_db(db, &table.to_table_info())
                .map_err(|e| format!("cannot create table `{}`: {e}", table.name))?;

            if !table.rows.is_empty() {
                let records: Vec<Record> = table.rows.clone();
                storage
                    .insert_in_db(db, &table.name, records)
                    .map_err(|e| format!("cannot insert into table `{}`: {e}", table.name))?;
            }

            // Verify: the rows that actually landed must be the rows the
            // backup promised. This is the check that turns "restore
            // reported success" into "restore did what it said".
            let landed = storage
                .scan_in_db(db, &table.name)
                .map_err(|e| format!("cannot verify table `{}`: {e}", table.name))?
                .len();
            if landed != table.row_count() {
                return Err(format!(
                    "restore verification failed for table `{}`: backup holds {} row(s), \
                     {} landed in database `{db}`",
                    table.name,
                    table.row_count(),
                    landed
                ));
            }

            tables_restored += 1;
            rows_restored += landed as u64;
        }

        Ok(RestoreResult {
            backup_id: backup_id.to_string(),
            tables_restored,
            rows_restored,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }

    /// Delete a backup
    #[allow(dead_code)]
    pub fn delete_backup(&self, backup_id: &str) -> Result<(), String> {
        let sql_file = self.backup_dir.join(format!("{}.sql", backup_id));
        let meta_file = self.backup_dir.join(format!("{}.meta.json", backup_id));
        let payload_file = self.backup_dir.join(format!("{}.data.json", backup_id));

        fs::remove_file(&sql_file).map_err(|e| e.to_string())?;
        fs::remove_file(&meta_file).ok();
        fs::remove_file(&payload_file).ok();

        self.metadata.write().unwrap().remove(backup_id);

        Ok(())
    }

    /// Get backup directory
    #[allow(dead_code)]
    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }
}

// ============================================================================
// Collecting tables from a storage
// ============================================================================

/// Collect one table from a storage, with its schema and its rows.
///
/// #5525: the backup used to receive an empty table map, so this is the
/// function that makes the enumeration real.
pub fn collect_table(
    storage: &dyn StorageEngine,
    db: &str,
    table: &str,
) -> Result<BackupTable, String> {
    let info = storage
        .get_table_info_in(db, table)
        .map_err(|e| format!("cannot read the schema of `{db}`.`{table}`: {e}"))?;
    let rows = storage
        .scan_in_db(db, table)
        .map_err(|e| format!("cannot read the rows of `{db}`.`{table}`: {e}"))?;

    let columns: Vec<BackupColumn> = info
        .columns
        .iter()
        .map(|c| BackupColumn {
            name: c.name.clone(),
            data_type: c.data_type.clone(),
            char_max_length: c.char_max_length,
            primary_key: c.primary_key,
            not_null: !c.nullable,
        })
        .collect();

    for (idx, row) in rows.iter().enumerate() {
        if row.len() != columns.len() {
            return Err(format!(
                "`{db}`.`{table}`: row {idx} has {} value(s) but the table has {} column(s): \
                 refusing to back up a table whose rows do not match its schema",
                row.len(),
                columns.len()
            ));
        }
    }

    let unique_constraints = info
        .unique_constraints
        .iter()
        .map(|u| BackupUniqueConstraint {
            name: u.name.clone(),
            columns: u.columns.clone(),
        })
        .collect();
    let foreign_keys = info
        .foreign_keys
        .iter()
        .map(|f| BackupForeignKey {
            name: f.name.clone(),
            columns: f.columns.clone(),
            referenced_table: f.referenced_table.clone(),
            referenced_columns: f.referenced_columns.clone(),
            on_delete: f.on_delete.clone(),
            on_update: f.on_update.clone(),
        })
        .collect();

    Ok(BackupTable {
        name: info.name.clone(),
        columns,
        rows,
        unique_constraints,
        foreign_keys,
    })
}

/// Collect every table of a database from a storage.
///
/// A database with **no** tables is an error: the caller asked for a
/// backup, and an empty one would be indistinguishable from the broken
/// backup #5525 is about — a "success" that captured nothing.
pub fn collect_all_tables(
    storage: &dyn StorageEngine,
    db: &str,
) -> Result<Vec<BackupTable>, String> {
    let names = storage.list_tables_in_db(db);
    let mut tables = Vec::with_capacity(names.len());
    for name in &names {
        tables.push(collect_table(storage, db, name)?);
    }
    if tables.is_empty() {
        return Err(format!(
            "database `{db}` contains no tables: refusing to report an empty backup as a success"
        ));
    }
    Ok(tables)
}

// ============================================================================
// Rendering
// ============================================================================

/// Quote an identifier for SQL output.
pub fn quote_ident(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

/// Escape a string for a single-quoted SQL literal.
///
/// MySQL-compatible: `'` is doubled, `\` is escaped, and the control
/// characters that would otherwise terminate or corrupt the literal are
/// escaped.
pub fn escape_sql_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for ch in text.chars() {
        match ch {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            '\0' => out.push_str("\\0"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{1a}' => out.push_str("\\Z"),
            other => out.push(other),
        }
    }
    out
}

/// Render a value as a SQL literal.
///
/// `JSON` and `POINT` values have no portable SQL literal, so they are
/// rendered as text. The `.data.json` payload keeps them exactly — see the
/// module docs.
pub fn render_value(value: &Value) -> Result<String, String> {
    Ok(match value {
        Value::Null => "NULL".to_string(),
        Value::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => {
            if !f.is_finite() {
                return Err(format!(
                    "cannot represent the non-finite float {f} in a SQL dump"
                ));
            }
            // `{:?}` is the shortest representation that round-trips back to
            // the same f64.
            let mut text = format!("{f:?}");
            if !text.contains('.') && !text.contains('e') && !text.contains('E') {
                text.push_str(".0");
            }
            text
        }
        Value::Text(t) => format!("'{}'", escape_sql_text(t)),
        Value::Blob(bytes) => format!(
            "X'{}'",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ),
        Value::Json(json) => format!("'{}'", escape_sql_text(&json.to_string())),
        Value::Point(x, y) => format!("'{}'", escape_sql_text(&format!("POINT({x}, {y})"))),
    })
}

/// Render the `CREATE TABLE` statement of a table.
pub fn render_create_table(table: &BackupTable) -> String {
    let mut defs: Vec<String> = Vec::new();
    for column in &table.columns {
        let mut def = format!("  {} {}", quote_ident(&column.name), column.type_sql());
        // A primary key column rejects NULL whether or not the original
        // declaration said so.
        if column.not_null || column.primary_key {
            def.push_str(" NOT NULL");
        }
        defs.push(def);
    }

    let pk_columns: Vec<String> = table
        .columns
        .iter()
        .filter(|c| c.primary_key)
        .map(|c| quote_ident(&c.name))
        .collect();
    if !pk_columns.is_empty() {
        defs.push(format!("  PRIMARY KEY ({})", pk_columns.join(", ")));
    }

    for unique in &table.unique_constraints {
        let columns = unique
            .columns
            .iter()
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");
        match &unique.name {
            Some(name) => defs.push(format!(
                "  CONSTRAINT {} UNIQUE ({})",
                quote_ident(name),
                columns
            )),
            None => defs.push(format!("  UNIQUE ({})", columns)),
        }
    }

    for fk in &table.foreign_keys {
        let mut def = String::new();
        if let Some(name) = &fk.name {
            def.push_str(&format!("CONSTRAINT {} ", quote_ident(name)));
        }
        def.push_str(&format!(
            "FOREIGN KEY ({}) REFERENCES {} ({})",
            fk.columns
                .iter()
                .map(|c| quote_ident(c))
                .collect::<Vec<_>>()
                .join(", "),
            quote_ident(&fk.referenced_table),
            fk.referenced_columns
                .iter()
                .map(|c| quote_ident(c))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        if let Some(action) = &fk.on_delete {
            def.push_str(&format!(" ON DELETE {}", render_fk_action(action)));
        }
        if let Some(action) = &fk.on_update {
            def.push_str(&format!(" ON UPDATE {}", render_fk_action(action)));
        }
        defs.push(format!("  {def}"));
    }

    format!(
        "CREATE TABLE {} (\n{}\n);\n",
        quote_ident(&table.name),
        defs.join(",\n")
    )
}

fn render_fk_action(action: &ForeignKeyAction) -> &'static str {
    match action {
        ForeignKeyAction::Cascade => "CASCADE",
        ForeignKeyAction::SetNull => "SET NULL",
        ForeignKeyAction::Restrict => "RESTRICT",
        ForeignKeyAction::NoAction => "NO ACTION",
    }
}

/// Render the portable SQL dump of a backup.
pub fn render_sql(content: &BackupContent, options: &ExportOptions) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("-- SQLRustGo logical backup\n");
    out.push_str(&format!("-- Database: {}\n", content.database));
    out.push_str(&format!("-- Tables: {}\n", content.tables.len()));
    out.push_str(&format!("-- Generated: {}\n", chrono_lite_now()));
    out.push_str("--\n");
    out.push_str("-- The companion <id>.data.json file is the machine-readable payload that\n");
    out.push_str("-- `sqlrustgo restore` consumes; this file is the portable dump.\n");

    let exotic = count_exotic_values(content);
    if exotic > 0 {
        out.push_str(&format!(
            "-- NOTE: {exotic} JSON/POINT value(s) are rendered as text here; the companion\n\
             --       <id>.data.json payload preserves them exactly.\n"
        ));
    }
    out.push('\n');

    if options.single_transaction {
        out.push_str("START TRANSACTION;\n\n");
    }

    for table in &content.tables {
        out.push_str(&format!(
            "-- Table: {} ({} row(s))\n",
            table.name,
            table.row_count()
        ));
        if options.add_drop {
            out.push_str(&format!(
                "DROP TABLE IF EXISTS {};\n",
                quote_ident(&table.name)
            ));
        }
        out.push_str(&render_create_table(table));
        out.push('\n');
        if !options.schema_only {
            for row in &table.rows {
                out.push_str(&render_insert(table, row)?);
            }
        }
        out.push('\n');
    }

    if options.single_transaction {
        out.push_str("COMMIT;\n");
    }
    Ok(out)
}

fn render_insert(table: &BackupTable, row: &[Value]) -> Result<String, String> {
    let columns = table
        .columns
        .iter()
        .map(|c| quote_ident(&c.name))
        .collect::<Vec<_>>()
        .join(", ");
    let values = row
        .iter()
        .map(render_value)
        .collect::<Result<Vec<_>, String>>()?
        .join(", ");
    Ok(format!(
        "INSERT INTO {} ({}) VALUES ({});\n",
        quote_ident(&table.name),
        columns,
        values
    ))
}

fn count_exotic_values(content: &BackupContent) -> usize {
    content
        .tables
        .iter()
        .flat_map(|t| t.rows.iter())
        .flatten()
        .filter(|v| matches!(v, Value::Json(_) | Value::Point(_, _)))
        .count()
}

// ============================================================================
// Checksums
// ============================================================================

/// SHA-256 of `data`, hex encoded.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Legacy helper: a **non-cryptographic** 32-bit checksum.
///
/// #5525: this used to be the integrity check recorded in
/// [`BackupMetadata::checksum`]. It cannot detect corruption, so the backup
/// path now uses SHA-256 ([`sha256_hex`]). The function is kept for API
/// compatibility and for callers that only need a cheap change detector.
pub fn md5_simple(data: &str) -> u32 {
    let mut hash: u32 = 0;
    for (i, byte) in data.bytes().enumerate() {
        hash = hash.wrapping_add((byte as u32).wrapping_mul(i as u32 + 1));
        hash = hash.rotate_left(5);
    }
    hash
}

/// Simple JSON serialization
pub fn serde_json_simple(metadata: &BackupMetadata) -> String {
    let status_str = match &metadata.status {
        BackupStatus::InProgress => "in_progress".to_string(),
        BackupStatus::Completed => "completed".to_string(),
        BackupStatus::Failed(e) => format!("failed:{}", e),
    };

    format!(
        r#"{{"id":"{}","type":"{:?}","started":"{}","completed":"{:?}","size":{},"database":"{}","tables":{:?},"status":"{}","checksum":"{:?}","payload_checksum":"{:?}"}}"#,
        metadata.id,
        metadata.backup_type,
        metadata.started_at,
        metadata.completed_at,
        metadata.size_bytes,
        metadata.database,
        metadata.tables,
        status_str,
        metadata.checksum,
        metadata.payload_checksum
    )
}

fn write_artifact(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file =
        File::create(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    file.write_all(bytes)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    file.flush()
        .map_err(|e| format!("cannot flush {}: {e}", path.display()))
}

/// Restore result
#[derive(Debug, Clone)]
pub struct RestoreResult {
    pub backup_id: String,
    /// Number of tables that were created and verified.
    pub tables_restored: usize,
    /// Number of rows that were inserted and verified.
    pub rows_restored: u64,
    pub duration_ms: u64,
}

/// Backup export options
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Include schema only (no data)
    pub schema_only: bool,
    /// Include drop statements
    pub add_drop: bool,
    /// Single transaction (for InnoDB)
    pub single_transaction: bool,
    /// Lock tables
    ///
    /// **Not implemented.** Setting it makes
    /// [`BackupManager::create_backup_with`] fail instead of silently
    /// ignoring the request.
    pub lock_tables: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            schema_only: false,
            add_drop: true,
            single_transaction: true,
            lock_tables: false,
        }
    }
}

// ============================================================================
// CLI Commands and Run Functions (for sqlrustgo-tools binary)
// ============================================================================

use structopt::StructOpt;

/// Backup command options
#[derive(Debug, StructOpt)]
pub struct BackupCommand {
    /// Database name to backup
    #[structopt(long, default_value = "default")]
    pub database: String,

    /// Backup output directory
    #[structopt(long, default_value = "./backups")]
    pub output_dir: String,

    /// Backup type: full
    #[structopt(long, default_value = "full")]
    pub backup_type: String,

    /// Data directory of the storage to back up
    ///
    /// #5525: a logical backup has to read the tables from somewhere. The
    /// old implementation had no data source at all and backed up an empty
    /// table map while reporting success.
    #[structopt(long)]
    pub data_dir: String,

    /// Include schema only (no data)
    #[structopt(long)]
    pub schema_only: bool,

    /// Enable compression
    ///
    /// **Not implemented.** Passing it is an error rather than a silently
    /// ignored flag.
    #[structopt(long)]
    pub compress: bool,
}

/// Restore command options
#[derive(Debug, StructOpt)]
pub struct RestoreCommand {
    /// Database name to restore
    #[structopt(long)]
    pub database: String,

    /// Backup ID to restore
    #[structopt(long)]
    pub backup_id: String,

    /// Backup directory
    #[structopt(long, default_value = "./backups")]
    pub backup_dir: String,

    /// Data directory of the storage to restore into
    #[structopt(long)]
    pub data_dir: String,

    /// Drop existing tables before restore
    #[structopt(long)]
    pub drop_first: bool,
}

/// Run backup command
pub fn run_backup(cmd: BackupCommand) -> Result<(), anyhow::Error> {
    use sqlrustgo_storage::FileStorage;

    println!("Starting backup for database: {}", cmd.database);
    println!("Output directory: {}", cmd.output_dir);
    println!("Backup type: {}", cmd.backup_type);

    // #5525: a switch that is accepted and then ignored is exactly the bug
    // this issue is about, so the unsupported ones are rejected up front.
    match cmd.backup_type.to_lowercase().as_str() {
        "full" => {}
        other => {
            return Err(anyhow::anyhow!(
                "--backup-type `{other}` is not implemented; only `full` is supported. \
                 Refusing to report a backup that did not happen."
            ));
        }
    }
    if cmd.compress {
        return Err(anyhow::anyhow!(
            "--compress is not implemented; refusing to write an uncompressed dump under a \
             compressed name. Use `physical-backup backup --compress` for compressed backups."
        ));
    }

    let storage = FileStorage::new(PathBuf::from(&cmd.data_dir))
        .map_err(|e| anyhow::anyhow!("cannot open the storage at {}: {e}", cmd.data_dir))?;

    let tables = collect_all_tables(&storage, &cmd.database).map_err(anyhow::Error::msg)?;

    let options = ExportOptions {
        schema_only: cmd.schema_only,
        ..Default::default()
    };

    let backup_dir = PathBuf::from(&cmd.output_dir);
    let manager = BackupManager::new(backup_dir);
    let metadata = manager
        .create_backup_with(&cmd.database, &tables, &options)
        .map_err(anyhow::Error::msg)?;

    let total_rows: usize = tables.iter().map(|t| t.row_count()).sum();
    println!("Backup completed successfully!");
    println!("Backup ID: {}", metadata.id);
    println!("Tables backed up: {}", metadata.tables.len());
    println!("Rows backed up: {}", total_rows);
    println!("Size: {} bytes", metadata.size_bytes);
    if let Some(checksum) = metadata.checksum {
        println!("Checksum: {}", checksum);
    }
    Ok(())
}

/// Run restore command
pub fn run_restore(cmd: RestoreCommand) -> Result<(), anyhow::Error> {
    use sqlrustgo_storage::FileStorage;

    println!("Starting restore for database: {}", cmd.database);
    println!("Backup ID: {}", cmd.backup_id);
    println!("Backup directory: {}", cmd.backup_dir);
    println!("Data directory: {}", cmd.data_dir);

    let backup_dir = PathBuf::from(&cmd.backup_dir);
    let manager = BackupManager::new(backup_dir);

    let mut storage = FileStorage::new(PathBuf::from(&cmd.data_dir))
        .map_err(|e| anyhow::anyhow!("cannot open the storage at {}: {e}", cmd.data_dir))?;

    let result = manager
        .restore_into(&cmd.backup_id, &mut storage, &cmd.database, cmd.drop_first)
        .map_err(anyhow::Error::msg)?;

    // Make the restored rows durable: `insert_in_db` buffers, and a backup
    // that reports success must not leave the data in a buffer.
    storage
        .flush()
        .map_err(|e| anyhow::anyhow!("cannot flush the restored data: {e}"))?;

    println!("Restore completed successfully!");
    println!("Tables restored: {}", result.tables_restored);
    println!("Total rows restored: {}", result.rows_restored);
    println!("Duration: {} ms", result.duration_ms);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::MemoryStorage;

    fn sample_table() -> BackupTable {
        BackupTable {
            name: "users".to_string(),
            columns: vec![
                BackupColumn::new("id", "INTEGER").primary_key(),
                BackupColumn::new("name", "TEXT"),
            ],
            rows: vec![
                vec![Value::Integer(1), Value::Text("alice".to_string())],
                vec![Value::Integer(2), Value::Text("bob".to_string())],
            ],
            unique_constraints: Vec::new(),
            foreign_keys: Vec::new(),
        }
    }

    #[test]
    fn test_backup_manager() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_backup");
        let manager = BackupManager::new(temp_dir.clone());

        let result = manager.create_backup("testdb", &[sample_table()]);
        assert!(result.is_ok());

        let backups = manager.list_backups();
        assert!(!backups.is_empty());

        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_md5() {
        let hash = md5_simple("test");
        assert!(hash > 0);
    }

    /// #5525: a backup with no tables is an error, not a success.
    #[test]
    fn test_create_backup_rejects_empty_table_list() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_backup_empty");
        let manager = BackupManager::new(temp_dir.clone());
        let err = manager.create_backup("testdb", &[]).unwrap_err();
        assert!(
            err.contains("no tables to back up"),
            "unexpected error: {err}"
        );
        fs::remove_dir_all(temp_dir).ok();
    }

    /// #5525: an empty table must still appear in the dump.
    #[test]
    fn test_empty_table_is_kept_in_the_dump() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_backup_empty_table");
        let manager = BackupManager::new(temp_dir.clone());

        let table = BackupTable::new(
            "empty_table",
            vec![BackupColumn::new("id", "INTEGER").primary_key()],
        );
        let meta = manager.create_backup("testdb", &[table]).unwrap();

        let sql = fs::read_to_string(temp_dir.join(format!("{}.sql", meta.id))).unwrap();
        assert!(
            sql.contains("CREATE TABLE `empty_table`"),
            "the empty table must be dumped: {sql}"
        );

        let content = manager.read_backup(&meta.id).unwrap();
        assert_eq!(content.tables.len(), 1);
        assert_eq!(content.tables[0].row_count(), 0);

        fs::remove_dir_all(temp_dir).ok();
    }

    /// #5525: the DDL must carry the type, NOT NULL and the primary key.
    #[test]
    fn test_dump_carries_the_schema() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_backup_schema");
        let manager = BackupManager::new(temp_dir.clone());

        let table = BackupTable::new(
            "users",
            vec![
                BackupColumn::new("id", "INTEGER").primary_key(),
                BackupColumn::new("name", "TEXT").not_null(),
                BackupColumn::new("email", "TEXT").with_max_length(255),
            ],
        );
        let meta = manager.create_backup("testdb", &[table]).unwrap();

        let sql = fs::read_to_string(temp_dir.join(format!("{}.sql", meta.id))).unwrap();
        assert!(sql.contains("`id` INTEGER NOT NULL"), "{sql}");
        assert!(sql.contains("`name` TEXT NOT NULL"), "{sql}");
        assert!(sql.contains("`email` TEXT(255)"), "{sql}");
        assert!(sql.contains("PRIMARY KEY (`id`)"), "{sql}");

        fs::remove_dir_all(temp_dir).ok();
    }

    /// #5525: values must be escaped, and NULL must not be quoted.
    #[test]
    fn test_dump_escapes_values() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_backup_escape");
        let manager = BackupManager::new(temp_dir.clone());

        let table = BackupTable::new(
            "t",
            vec![
                BackupColumn::new("a", "TEXT"),
                BackupColumn::new("b", "INTEGER"),
                BackupColumn::new("c", "TEXT"),
            ],
        );
        let mut table = table;
        table.rows = vec![vec![
            Value::Text("O'Brien".to_string()),
            Value::Integer(42),
            Value::Null,
        ]];
        let meta = manager.create_backup("testdb", &[table]).unwrap();

        let sql = fs::read_to_string(temp_dir.join(format!("{}.sql", meta.id))).unwrap();
        assert!(sql.contains("'O''Brien'"), "{sql}");
        assert!(sql.contains("42"), "{sql}");
        assert!(sql.contains("NULL"), "{sql}");
        assert!(!sql.contains("'42'"), "a number must not be quoted: {sql}");

        fs::remove_dir_all(temp_dir).ok();
    }

    /// #5525: the restore must write the rows back and verify them.
    #[test]
    fn test_restore_into_lands_and_verifies_the_rows() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_restore_into");
        let manager = BackupManager::new(temp_dir.clone());
        let meta = manager.create_backup("testdb", &[sample_table()]).unwrap();

        let mut storage = MemoryStorage::new();
        let result = manager
            .restore_into(&meta.id, &mut storage, "default", false)
            .unwrap();

        assert_eq!(result.tables_restored, 1);
        assert_eq!(result.rows_restored, 2);

        let rows = storage.scan_in_db("default", "users").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0], Value::Integer(1));
        assert_eq!(rows[0][1], Value::Text("alice".to_string()));

        fs::remove_dir_all(temp_dir).ok();
    }

    /// #5525: restoring onto an existing table must not silently overwrite it.
    #[test]
    fn test_restore_into_refuses_to_overwrite_without_drop_first() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_restore_overwrite");
        let manager = BackupManager::new(temp_dir.clone());
        let meta = manager.create_backup("testdb", &[sample_table()]).unwrap();

        let mut storage = MemoryStorage::new();
        manager
            .restore_into(&meta.id, &mut storage, "default", false)
            .unwrap();

        let err = manager
            .restore_into(&meta.id, &mut storage, "default", false)
            .unwrap_err();
        assert!(err.contains("already exists"), "unexpected error: {err}");

        // With --drop-first the restore replaces the table.
        let result = manager
            .restore_into(&meta.id, &mut storage, "default", true)
            .unwrap();
        assert_eq!(result.rows_restored, 2);
        assert_eq!(storage.scan_in_db("default", "users").unwrap().len(), 2);

        fs::remove_dir_all(temp_dir).ok();
    }

    /// #5525: `lock_tables` was accepted and ignored — it must be rejected.
    #[test]
    fn test_lock_tables_is_rejected() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_lock_tables");
        let manager = BackupManager::new(temp_dir.clone());
        let options = ExportOptions {
            lock_tables: true,
            ..Default::default()
        };
        let err = manager
            .create_backup_with("testdb", &[sample_table()], &options)
            .unwrap_err();
        assert!(err.contains("LOCK TABLES"), "unexpected error: {err}");
        fs::remove_dir_all(temp_dir).ok();
    }

    /// #5525: `schema_only` was accepted and ignored — it must work.
    #[test]
    fn test_schema_only_drops_the_rows() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_schema_only");
        let manager = BackupManager::new(temp_dir.clone());
        let options = ExportOptions {
            schema_only: true,
            ..Default::default()
        };
        let meta = manager
            .create_backup_with("testdb", &[sample_table()], &options)
            .unwrap();

        let sql = fs::read_to_string(temp_dir.join(format!("{}.sql", meta.id))).unwrap();
        assert!(sql.contains("CREATE TABLE `users`"), "{sql}");
        assert!(!sql.contains("INSERT INTO"), "{sql}");

        fs::remove_dir_all(temp_dir).ok();
    }

    /// A corrupted payload must be refused, not restored.
    #[test]
    fn test_corrupted_payload_is_refused() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_corrupt");
        let manager = BackupManager::new(temp_dir.clone());
        let meta = manager.create_backup("testdb", &[sample_table()]).unwrap();

        let payload_file = temp_dir.join(format!("{}.data.json", meta.id));
        let raw = fs::read_to_string(&payload_file).unwrap();
        fs::write(&payload_file, raw.replace("alice", "mallory")).unwrap();

        let err = manager.read_backup(&meta.id).unwrap_err();
        assert!(err.contains("corrupt"), "unexpected error: {err}");

        fs::remove_dir_all(temp_dir).ok();
    }

    /// The `.sql` and the payload must describe the same backup.
    #[test]
    fn test_sql_and_payload_describe_the_same_backup() {
        let temp_dir = std::env::temp_dir().join("sqlrustgo_test_consistency");
        let manager = BackupManager::new(temp_dir.clone());
        let meta = manager.create_backup("testdb", &[sample_table()]).unwrap();

        let sql = fs::read_to_string(temp_dir.join(format!("{}.sql", meta.id))).unwrap();
        let content = manager.read_backup(&meta.id).unwrap();

        for table in &content.tables {
            assert!(
                sql.contains(&format!("CREATE TABLE `{}`", table.name)),
                "the dump is missing table {}",
                table.name
            );
            let inserts = sql
                .lines()
                .filter(|l| l.starts_with(&format!("INSERT INTO `{}`", table.name)))
                .count();
            assert_eq!(
                inserts,
                table.row_count(),
                "the dump and the payload disagree about the rows of {}",
                table.name
            );
        }

        fs::remove_dir_all(temp_dir).ok();
    }

    /// #5525: the enumeration must be real — the tables of the storage, with
    /// their schema and their rows.
    #[test]
    fn test_collect_all_tables_reads_the_storage() {
        let mut storage = MemoryStorage::new();
        let info = TableInfo {
            name: "widgets".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                nullable: false,
                primary_key: true,
                ..Default::default()
            }],
            ..Default::default()
        };
        storage.create_table_in_db("default", &info).unwrap();
        storage
            .insert_in_db("default", "widgets", vec![vec![Value::Integer(7)]])
            .unwrap();

        let tables = collect_all_tables(&storage, "default").unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "widgets");
        assert_eq!(tables[0].row_count(), 1);
        assert!(tables[0].columns[0].primary_key);
        assert!(tables[0].columns[0].not_null);
    }

    /// #5525: a database with no tables must be an error.
    #[test]
    fn test_collect_all_tables_rejects_an_empty_database() {
        let storage = MemoryStorage::new();
        let err = collect_all_tables(&storage, "default").unwrap_err();
        assert!(err.contains("no tables"), "unexpected error: {err}");
    }
}

#[test]
fn test_md5_consistency() {
    let h1 = md5_simple("hello");
    let h2 = md5_simple("hello");
    assert_eq!(h1, h2);
}

#[test]
fn test_md5_different_inputs() {
    let h1 = md5_simple("hello");
    let h2 = md5_simple("world");
    assert_ne!(h1, h2);
}

#[test]
fn test_backup_manager_delete_backup() {
    let temp_dir = std::env::temp_dir().join("sqlrustgo_test_delete_backup");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let manager = BackupManager::new(temp_dir.clone());

    let table = BackupTable::new("t1", vec![BackupColumn::new("id", "INTEGER").primary_key()]);
    let result = manager.create_backup("testdb", &[table]);
    assert!(result.is_ok());
    let backup_id = result.unwrap().id.clone();

    let listed = manager.list_backups();
    assert!(!listed.is_empty());

    let deleted = manager.delete_backup(&backup_id);
    assert!(deleted.is_ok());

    let listed_after = manager.list_backups();
    assert!(listed_after.is_empty());

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_backup_manager_get_backup() {
    let temp_dir = std::env::temp_dir().join("sqlrustgo_test_get_backup");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let manager = BackupManager::new(temp_dir.clone());

    let table = BackupTable::new(
        "users",
        vec![BackupColumn::new("id", "INTEGER").primary_key()],
    );
    let created = manager.create_backup("testdb", &[table]).unwrap();
    let backup_id = created.id.clone();

    let retrieved = manager.get_backup(&backup_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, backup_id);

    // Non-existent backup
    let missing = manager.get_backup("nonexistent_backup_id");
    assert!(missing.is_none());

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_backup_manager_restore_not_found() {
    let temp_dir = std::env::temp_dir().join("sqlrustgo_test_restore_not_found");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let manager = BackupManager::new(temp_dir.clone());

    let result = manager.restore("this_backup_does_not_exist");
    assert!(result.is_err());

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_backup_manager_backup_dir() {
    let temp_dir = std::env::temp_dir().join("sqlrustgo_test_backup_dir");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let manager = BackupManager::new(temp_dir.clone());
    assert_eq!(manager.backup_dir(), std::path::Path::new(&temp_dir));
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_backup_metadata_fail() {
    let mut meta = BackupMetadata::new("bkp_1".into(), BackupType::Full, "db".into());
    meta.fail("disk full".into());
    assert!(matches!(meta.status, BackupStatus::Failed(_)));
}

#[test]
fn test_backup_metadata_complete() {
    let mut meta = BackupMetadata::new("bkp_2".into(), BackupType::Full, "db".into());
    meta.complete(1024, "abc123".into());
    assert!(matches!(meta.status, BackupStatus::Completed));
    assert_eq!(meta.size_bytes, 1024);
    assert_eq!(meta.checksum.as_ref().unwrap(), "abc123");
}

#[test]
fn test_backup_type_variants() {
    // Full backup
    let full = BackupMetadata::new("f".into(), BackupType::Full, "db".into());
    assert!(matches!(full.backup_type, BackupType::Full));

    // Incremental backup
    let incr = BackupMetadata::new("i".into(), BackupType::Incremental, "db".into());
    assert!(matches!(incr.backup_type, BackupType::Incremental));

    // Differential backup
    let diff = BackupMetadata::new("d".into(), BackupType::Differential, "db".into());
    assert!(matches!(diff.backup_type, BackupType::Differential));
}
