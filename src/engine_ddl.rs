//! DDL/DML execution methods: GRANT, REVOKE, CREATE/DROP ROLE,
//! SHOW, DESCRIBE, ALTER TABLE, PREPARE/EXECUTE/DEALLOCATE,
//! and related catalog operations.
//!
//! Extracted from execution_engine.rs for C-ARCH-05 compliance
//! (line count limit: 1500). All methods are `impl ExecutionEngine`.

use crate::execution_engine::ExecutionEngine;
use crate::{SqlError, SqlResult, Value};
use sqlrustgo_catalog::auth::{Privilege, UserIdentity};
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_parser::parser::{
    AlterColumnOperation, AlterTableOperation, AlterTableStatement, CreateRoleStatement,
    DescribeStatement, DropRoleStatement, GrantRoleStatement, GrantStatement,
    ObjectType as ParserObjectType, Privilege as ParserPrivilege, RevokeRoleStatement,
    RevokeStatement, SetRoleStatement, ShowStatement,
};
use sqlrustgo_storage::{ColumnDefinition, StorageEngine};

impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    pub(crate) fn execute_grant(&mut self, grant: &GrantStatement) -> SqlResult<ExecutorResult> {
        let catalog_guard = self.catalog.as_ref().ok_or_else(|| {
            SqlError::ExecutionError("Catalog not available for GRANT".to_string())
        })?;
        let mut catalog = catalog_guard.write();

        for privilege in &grant.privileges {
            let priv_str = match privilege {
                ParserPrivilege::Select => "SELECT",
                ParserPrivilege::Insert => "INSERT",
                ParserPrivilege::Update => "UPDATE",
                ParserPrivilege::Delete => "DELETE",
                ParserPrivilege::Read => "READ",
                ParserPrivilege::Write => "WRITE",
                ParserPrivilege::Execute => "EXECUTE",
                ParserPrivilege::Usage => "USAGE",
                ParserPrivilege::All => "ALL",
            };
            let priv_obj = Privilege::from_str(priv_str).ok_or_else(|| {
                SqlError::ExecutionError(format!("Unknown privilege: {}", priv_str))
            })?;

            let obj_type = match &grant.object_type {
                ParserObjectType::Table => sqlrustgo_catalog::auth::ObjectType::Table,
                ParserObjectType::Database => sqlrustgo_catalog::auth::ObjectType::Database,
                ParserObjectType::Column => sqlrustgo_catalog::auth::ObjectType::Column,
                ParserObjectType::Procedure => sqlrustgo_catalog::auth::ObjectType::Table,
                ParserObjectType::Function => sqlrustgo_catalog::auth::ObjectType::Table,
            };

            for recipient in &grant.recipients {
                let identity = UserIdentity::new(recipient, "%");
                if grant.object_type == ParserObjectType::Column {
                    for column in &grant.columns {
                        catalog
                            .grant_column_privilege(&identity, priv_obj, &grant.object_name, column)
                            .map_err(|e| {
                                SqlError::ExecutionError(format!("GRANT failed: {}", e))
                            })?;
                    }
                } else {
                    catalog
                        .grant_privilege(
                            &identity,
                            priv_obj,
                            obj_type,
                            &grant.object_name,
                            grant.with_grant_option,
                        )
                        .map_err(|e| SqlError::ExecutionError(format!("GRANT failed: {}", e)))?;
                }
            }
        }

        Ok(ExecutorResult::new(
            vec![vec![Value::Integer(grant.recipients.len() as i64)]],
            1,
        ))
    }

    pub(crate) fn execute_revoke(&mut self, revoke: &RevokeStatement) -> SqlResult<ExecutorResult> {
        let catalog_guard = self.catalog.as_ref().ok_or_else(|| {
            SqlError::ExecutionError("Catalog not available for REVOKE".to_string())
        })?;
        let mut catalog = catalog_guard.write();

        for privilege in &revoke.privileges {
            let priv_str = match privilege {
                ParserPrivilege::Select => "SELECT",
                ParserPrivilege::Insert => "INSERT",
                ParserPrivilege::Update => "UPDATE",
                ParserPrivilege::Delete => "DELETE",
                ParserPrivilege::Read => "READ",
                ParserPrivilege::Write => "WRITE",
                ParserPrivilege::Execute => "EXECUTE",
                ParserPrivilege::Usage => "USAGE",
                ParserPrivilege::All => "ALL",
            };
            let priv_obj = Privilege::from_str(priv_str).ok_or_else(|| {
                SqlError::ExecutionError(format!("Unknown privilege: {}", priv_str))
            })?;

            let obj_type = match &revoke.object_type {
                ParserObjectType::Table => sqlrustgo_catalog::auth::ObjectType::Table,
                ParserObjectType::Database => sqlrustgo_catalog::auth::ObjectType::Database,
                ParserObjectType::Column => sqlrustgo_catalog::auth::ObjectType::Column,
                ParserObjectType::Procedure => sqlrustgo_catalog::auth::ObjectType::Table,
                ParserObjectType::Function => sqlrustgo_catalog::auth::ObjectType::Table,
            };

            for user in &revoke.from_users {
                let identity = UserIdentity::new(user, "%");
                catalog
                    .revoke_privilege(&identity, priv_obj, obj_type, &revoke.object_name)
                    .map_err(|e| SqlError::ExecutionError(format!("REVOKE failed: {}", e)))?;
            }
        }

        Ok(ExecutorResult::new(
            vec![vec![Value::Integer(revoke.from_users.len() as i64)]],
            1,
        ))
    }

    pub(crate) fn execute_create_role(
        &mut self,
        stmt: &CreateRoleStatement,
    ) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let mut catalog_guard = catalog.write();

        let parent_role_id = if let Some(ref parent_name) = stmt.parent_role {
            let parent_role = catalog_guard
                .auth_manager()
                .find_role_by_name(parent_name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Parent role '{}' not found", parent_name))
                })?;
            Some(parent_role.id)
        } else {
            None
        };

        catalog_guard
            .create_role(&stmt.name, parent_role_id)
            .map_err(|e| SqlError::ExecutionError(format!("CREATE ROLE failed: {}", e)))?;

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!("Role {} created", stmt.name))]],
            1,
        ))
    }

    pub(crate) fn execute_drop_role(
        &mut self,
        stmt: &DropRoleStatement,
    ) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let mut catalog_guard = catalog.write();

        let role_id = {
            let role = catalog_guard
                .auth_manager()
                .find_role_by_name(&stmt.name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Role '{}' not found", stmt.name))
                })?;
            role.id
        };

        catalog_guard
            .drop_role(role_id)
            .map_err(|e| SqlError::ExecutionError(format!("DROP ROLE failed: {}", e)))?;

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!("Role {} dropped", stmt.name))]],
            1,
        ))
    }

    pub(crate) fn execute_grant_role(
        &mut self,
        stmt: &GrantRoleStatement,
    ) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let mut catalog_guard = catalog.write();

        let role_id = {
            let role = catalog_guard
                .auth_manager()
                .find_role_by_name(&stmt.role_name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Role '{}' not found", stmt.role_name))
                })?;
            role.id
        };

        let user_identity = UserIdentity::new(&stmt.user_name, stmt.host.as_deref().unwrap_or("%"));

        let user_id = {
            catalog_guard
                .auth_manager()
                .get_user_id_by_identity(&user_identity)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("User '{}' not found", stmt.user_name))
                })?
        };

        catalog_guard
            .grant_role_to_user(user_id, role_id, 0)
            .map_err(|e| SqlError::ExecutionError(format!("GRANT ROLE failed: {}", e)))?;

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!(
                "Grant {} to {}",
                stmt.role_name, stmt.user_name
            ))]],
            1,
        ))
    }

    pub(crate) fn execute_revoke_role(
        &mut self,
        stmt: &RevokeRoleStatement,
    ) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let mut catalog_guard = catalog.write();

        let role_id = {
            let role = catalog_guard
                .auth_manager()
                .find_role_by_name(&stmt.role_name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Role '{}' not found", stmt.role_name))
                })?;
            role.id
        };

        let user_identity = UserIdentity::new(&stmt.user_name, stmt.host.as_deref().unwrap_or("%"));

        let user_id = {
            catalog_guard
                .auth_manager()
                .get_user_id_by_identity(&user_identity)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("User '{}' not found", stmt.user_name))
                })?
        };

        catalog_guard
            .revoke_role_from_user(user_id, role_id)
            .map_err(|e| SqlError::ExecutionError(format!("REVOKE ROLE failed: {}", e)))?;

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!(
                "Revoke {} from {}",
                stmt.role_name, stmt.user_name
            ))]],
            1,
        ))
    }

    pub(crate) fn execute_set_role(
        &mut self,
        stmt: &SetRoleStatement,
    ) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;

        let role_name = {
            let catalog_guard = catalog.read();
            let role = catalog_guard
                .auth_manager()
                .find_role_by_name(&stmt.role_name)
                .ok_or_else(|| {
                    SqlError::ExecutionError(format!("Role '{}' not found", stmt.role_name))
                })?;
            role.name.clone()
        };

        self.current_role = Some(stmt.role_name.clone());

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!("SET ROLE to {}", role_name))]],
            1,
        ))
    }

    pub(crate) fn execute_show_roles(&self) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let catalog_guard = catalog.read();

        let roles = catalog_guard.auth_manager().list_roles();
        let rows: Vec<Vec<Value>> = roles
            .iter()
            .map(|r| {
                vec![
                    Value::Integer(r.id as i64),
                    Value::Text(r.name.clone()),
                    r.parent_role_id
                        .map(|id| Value::Integer(id as i64))
                        .unwrap_or(Value::Null),
                ]
            })
            .collect();

        Ok(ExecutorResult::new(rows, 3))
    }

    /// Dispatch `Statement::Show` to a concrete sub-handler.
    /// PR-SHOW-TABLES: P1 backlog fix for v3.7.0.
    /// G13-OLTP-1: `pub(crate)` so the mysql-server dispatch site can
    /// call this on a read-lock guard (the COM_QUERY / COM_STMT_EXECUTE
    /// path uses `&self` to allow concurrent SELECTs).
    pub fn execute_show(&self, show: &ShowStatement) -> SqlResult<ExecutorResult> {
        match show {
            ShowStatement::Tables => self.execute_show_tables(),
            ShowStatement::Databases => self.execute_show_databases(),
            ShowStatement::CreateTable { table } => self.execute_show_create_table(table),
            ShowStatement::Index { table } => self.execute_show_index(table),
            ShowStatement::Grants { user } => self.execute_show_grants(user.as_deref()),
            ShowStatement::Columns { table, pattern } => {
                self.execute_show_columns(table, pattern.as_deref())
            }
            ShowStatement::Sequences => self.execute_show_sequences(),
            // V312-35 #4218: SHOW PROCESSLIST — delegates to
            // ExecutionEngine::execute_show_processlist_impl for full
            // process info via StorageEngine::list_processes.
            ShowStatement::Processlist { full } => self.execute_show_processlist_impl(*full),
            // V312-55A / Issue #4238: SHOW PROCEDURE STATUS [LIKE 'pat']
            ShowStatement::ProcedureStatus { pattern } => {
                self.execute_show_procedure_status(pattern.as_deref())
            }
        }
    }

    /// Round-21 / Issue #4218: SHOW PROCESSLIST / SHOW FULL PROCESSLIST.
    pub(crate) fn execute_show_tables(&self) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();
        let names = storage.list_tables();
        let rows: Vec<Vec<Value>> = names.into_iter().map(|n| vec![Value::Text(n)]).collect();
        Ok(ExecutorResult::new(rows, 1))
    }

    pub(crate) fn execute_show_databases(&self) -> SqlResult<ExecutorResult> {
        Ok(ExecutorResult::new(
            vec![vec![Value::Text("default".to_string())]],
            1,
        ))
    }
    pub(crate) fn execute_show_sequences(&self) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();
        let names = storage.list_sequences();
        let rows: Vec<Vec<Value>> = names.into_iter().map(|n| vec![Value::Text(n)]).collect();
        Ok(ExecutorResult::new(rows, 1))
    }

    /// V312-55A / Issue #4238: `SHOW PROCEDURE STATUS [LIKE 'pat']`.
    ///
    /// Lists procedures from the in-memory catalog. Each row is
    /// (Name, ParamCount, BodyStatementCount). The catalog must be
    /// attached (`ExecutionEngine::with_memory_and_catalog`) for this
    /// to return anything other than an empty set — consistent with
    /// other Procedure DDL endpoints.
    pub(crate) fn execute_show_procedure_status(
        &self,
        pattern: Option<&str>,
    ) -> SqlResult<ExecutorResult> {
        let catalog_guard = self.catalog.as_ref().ok_or_else(|| {
            SqlError::ExecutionError(
                "SHOW PROCEDURE STATUS requires stored procedure catalog".to_string(),
            )
        })?;
        let catalog = catalog_guard.read();

        let pattern_lower = pattern.map(|p| p.to_lowercase());
        let mut rows: Vec<Vec<Value>> = Vec::new();
        for proc in catalog.stored_procedures() {
            // MySQL SHOW PROCEDURE STATUS LIKE uses SQL LIKE glob
            // semantics. We approximate: leading/trailing `%` are
            // wildcards; the rest must match case-insensitively.
            // Anything else (e.g. `_`) is treated as a literal — good
            // enough for the v3.12.0 controlled-subset wire surface.
            if let Some(ref pat) = pattern_lower {
                if !like_match(&pat, &proc.name.to_lowercase()) {
                    continue;
                }
            }
            rows.push(vec![
                Value::Text(proc.name.clone()),
                Value::Integer(proc.params.len() as i64),
                Value::Integer(proc.body.len() as i64),
            ]);
        }
        Ok(ExecutorResult::new(rows, 3))
    }

    /// SHOW CREATE TABLE — reconstruct CREATE TABLE from the live schema.
    pub(crate) fn execute_show_create_table(&self, table: &str) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();
        if !storage.list_tables().iter().any(|n| n == table) {
            return Err(SqlError::ExecutionError(format!(
                "Table '{}' does not exist",
                table
            )));
        }
        let info = storage
            .get_table_info(table)
            .map_err(|e| SqlError::ExecutionError(format!("cannot introspect {table}: {e}")))?;
        let cols: Vec<String> = info
            .columns
            .iter()
            .map(|c| {
                let nullable = if c.nullable { "" } else { " NOT NULL" };
                format!("{} {}{}", c.name, c.data_type, nullable)
            })
            .collect();
        let ddl = format!("CREATE TABLE {} ({})", table, cols.join(", "));
        Ok(ExecutorResult::new(vec![vec![Value::Text(ddl)]], 1))
    }

    /// SHOW INDEX — returns index information for a table.
    /// MySQL format: Table, Non_unique, Key_name, Seq_in_index, Column_name, Index_type
    pub(crate) fn execute_show_index(&self, table: &str) -> SqlResult<ExecutorResult> {
        let catalog = match &self.catalog {
            Some(c) => c,
            None => {
                // Fall back to storage if no catalog (e.g., memory storage without catalog)
                let storage = self.storage.read();
                if !storage.list_tables().iter().any(|n| n == table) {
                    return Err(SqlError::ExecutionError(format!(
                        "Table '{}' does not exist",
                        table
                    )));
                }
                // Storage doesn't have index metadata, return empty
                return Ok(ExecutorResult::new(vec![], 0));
            }
        };
        let catalog_guard = catalog.read();

        // Find the table in the catalog
        let table_ref = catalog_guard
            .all_schemas()
            .iter()
            .find_map(|(_, schema)| schema.get_table(table))
            .ok_or_else(|| SqlError::ExecutionError(format!("Table '{}' not found", table)))?;

        let mut rows: Vec<Vec<Value>> = Vec::new();
        for index in &table_ref.indices {
            for (seq, column_name) in index.columns.iter().enumerate() {
                let non_unique = if index.is_unique { 0 } else { 1 };
                let index_type = match index.index_type {
                    sqlrustgo_catalog::index::IndexType::BTree => "BTREE",
                    sqlrustgo_catalog::index::IndexType::Hash => "HASH",
                    sqlrustgo_catalog::index::IndexType::FullText => "FULLTEXT",
                };
                rows.push(vec![
                    Value::Text(table.to_string()),      // Table
                    Value::Integer(non_unique as i64),   // Non_unique
                    Value::Text(index.name.clone()),     // Key_name
                    Value::Integer((seq + 1) as i64),    // Seq_in_index
                    Value::Text(column_name.clone()),    // Column_name
                    Value::Text(index_type.to_string()), // Index_type
                ]);
            }
        }

        // If no indexes found, return empty result
        if rows.is_empty() {
            return Ok(ExecutorResult::new(vec![], 0));
        }

        Ok(ExecutorResult::new(rows, 6))
    }

    /// DESCRIBE table — return one row per column with Field/Type/Null/Key/Default/Extra.
    /// G13-OLTP-1: `pub(crate)` so the mysql-server dispatch site can
    /// call this on a read-lock guard.
    pub fn execute_describe(&self, desc: &DescribeStatement) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();
        if !storage.list_tables().iter().any(|n| n == &desc.table) {
            return Err(SqlError::ExecutionError(format!(
                "Table '{}' does not exist",
                desc.table
            )));
        }
        let info = storage.get_table_info(&desc.table).map_err(|e| {
            SqlError::ExecutionError(format!("cannot introspect {}: {e}", desc.table))
        })?;
        let rows: Vec<Vec<Value>> = info
            .columns
            .iter()
            .map(|c| {
                let null_str = if c.nullable { "YES" } else { "NO" };
                let key_str = if c.primary_key { "PRI" } else { "" };
                // Append (N) to data_type when char_max_length is set
                // (mirrors MySQL's "VARCHAR(10)" / "CHAR(50)" output).
                let type_str = match c.char_max_length {
                    Some(n) => format!("{}({})", c.data_type, n),
                    None => c.data_type.clone(),
                };
                vec![
                    Value::Text(c.name.clone()),
                    Value::Text(type_str),
                    Value::Text(null_str.to_string()),
                    Value::Text(key_str.to_string()),
                    Value::Text("NULL".to_string()),
                    Value::Text(String::new()),
                ]
            })
            .collect();
        Ok(ExecutorResult::new(rows, info.columns.len()))
    }

    /// SHOW GRANTS — placeholder (v3.7.0 grant tracking is limited to roles).
    pub(crate) fn execute_show_grants(&self, _user: Option<&str>) -> SqlResult<ExecutorResult> {
        Ok(ExecutorResult::new(vec![], 0))
    }

    /// SHOW COLUMNS — returns column information for a table.
    /// MySQL format: Field, Type, Null, Key, Default, Extra
    pub(crate) fn execute_show_columns(
        &self,
        table: &str,
        pattern: Option<&str>,
    ) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();
        if !storage.list_tables().iter().any(|n| n == table) {
            return Err(SqlError::ExecutionError(format!(
                "Table '{}' does not exist",
                table
            )));
        }
        let info = storage
            .get_table_info(table)
            .map_err(|e| SqlError::ExecutionError(format!("cannot introspect {table}: {e}")))?;

        let mut rows: Vec<Vec<Value>> = info
            .columns
            .iter()
            .filter(|c| {
                if let Some(p) = pattern {
                    // Simple glob pattern match
                    wildcard_match(&c.name, p)
                } else {
                    true
                }
            })
            .map(|c| {
                let null_str = if c.nullable { "YES" } else { "NO" };
                let key_str = if c.primary_key { "PRI" } else { "" };
                let type_str = match c.char_max_length {
                    Some(n) => format!("{}({})", c.data_type, n),
                    None => c.data_type.clone(),
                };
                vec![
                    Value::Text(c.name.clone()),
                    Value::Text(type_str),
                    Value::Text(null_str.to_string()),
                    Value::Text(key_str.to_string()),
                    Value::Text("NULL".to_string()),
                    Value::Text(String::new()),
                ]
            })
            .collect();

        if rows.is_empty() {
            return Ok(ExecutorResult::new(vec![], 0));
        }

        Ok(ExecutorResult::new(rows, 6))
    }

    pub(crate) fn execute_show_grants_for(&self, user_spec: &str) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let catalog_guard = catalog.read();

        let parts: Vec<&str> = user_spec.split('@').collect();
        let username = parts[0];
        let host = parts.get(1).unwrap_or(&"%");

        let identity = UserIdentity::new(username, host);
        let grants = catalog_guard
            .auth_manager()
            .get_all_grants_for_user(&identity);

        let rows: Vec<Vec<Value>> = grants
            .iter()
            .map(|g| {
                vec![
                    Value::Text(format!("{}@{}", g.user.username, g.user.host)),
                    Value::Text(g.privilege.to_string()),
                    Value::Text(format!("{:?}", g.object.object_type)),
                    Value::Text(g.object.object_name.clone()),
                ]
            })
            .collect();

        Ok(ExecutorResult::new(rows, 4))
    }

    pub(crate) fn execute_alter_table(
        &self,
        alter: &AlterTableStatement,
    ) -> SqlResult<ExecutorResult> {
        let mut storage = self.storage.write();

        match &alter.operation {
            AlterTableOperation::AddColumn {
                name,
                data_type,
                nullable,
                default_value: _,
            } => {
                let column = ColumnDefinition {
                    name: name.clone(),
                    data_type: data_type.clone(),
                    nullable: *nullable,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                };
                storage.add_column(&alter.table_name, column)?;
            }
            AlterTableOperation::DropColumn { name } => {
                storage.drop_column(&alter.table_name, name)?;
            }
            AlterTableOperation::ModifyColumn {
                name,
                data_type,
                nullable,
                char_max_length,
            } => {
                let column = ColumnDefinition {
                    name: name.clone(),
                    data_type: data_type.clone(),
                    nullable: *nullable,
                    primary_key: false,
                    char_max_length: *char_max_length,
                    collation: None,
                    default_value: None,
                };
                storage.modify_column(&alter.table_name, name, column)?;
            }
            AlterTableOperation::RenameTo { new_name } => {
                storage.rename_table(&alter.table_name, new_name)?;
            }
            AlterTableOperation::RenameColumn { name, new_name } => {
                storage.rename_column(&alter.table_name, name, new_name)?;
            }
            AlterTableOperation::AlterColumn { name, op } => match op {
                AlterColumnOperation::SetDataType { data_type } => {
                    // V312-19 / #4039: dispatch SET DATA TYPE to storage.modify_column
                    // preserving the existing column's nullable/char_max_length so that
                    // the case_insensitive_alter.test fixture (V313-11 simplified)
                    // can succeed without forcing an explicit CAST.
                    let info = storage.get_table_info(&alter.table_name)?;
                    let existing = info
                        .columns
                        .iter()
                        .find(|c| c.name.to_lowercase() == name.to_lowercase())
                        .ok_or_else(|| {
                            SqlError::ExecutionError(format!("Column not found: {}", name))
                        })?;
                    let new_def = ColumnDefinition {
                        name: existing.name.clone(),
                        data_type: data_type.to_string(),
                        nullable: existing.nullable,
                        primary_key: existing.primary_key,
                        char_max_length: existing.char_max_length,
                        collation: existing.collation.clone(),
                        default_value: None,
                    };
                    storage.modify_column(&alter.table_name, name, new_def)?;
                }
                AlterColumnOperation::SetDefault { default_value } => {
                    // V313-followup-1 / Issue #4154: persist the literal default.
                    storage.set_column_default(&alter.table_name, name, default_value.clone())?;
                }
                AlterColumnOperation::DropDefault => {
                    // V313-followup-1 / Issue #4154: clear the persisted default.
                    storage.set_column_default(&alter.table_name, name, None)?;
                }
                AlterColumnOperation::DropNotNull => {
                    return Err(SqlError::ParseError(format!(
                        "ALTER COLUMN '{}' DROP NOT NULL not supported",
                        name
                    )));
                }
            },
            AlterTableOperation::SetPartitionedBy => {
                return Err(SqlError::ParseError("not supported".to_string()));
            }
            AlterTableOperation::ResetPartitionedBy => {
                return Err(SqlError::ParseError("not supported".to_string()));
            }
        }

        Ok(ExecutorResult::empty())
    }

    pub(crate) fn execute_prepare(&mut self, name: &str, sql: &str) -> SqlResult<ExecutorResult> {
        let parsed = sqlrustgo_parser::parse(sql)
            .map_err(|e| SqlError::ParseError(format!("PREPARE failed to parse SQL: {}", e)))?;
        self.stmt_cache.prepare(name, sql, parsed);
        Ok(ExecutorResult::empty())
    }

    pub(crate) fn execute_execute(
        &mut self,
        name: &str,
        params: &[sqlrustgo_parser::Expression],
    ) -> SqlResult<ExecutorResult> {
        let sql = self.stmt_cache.execute_with_sql(name).ok_or_else(|| {
            SqlError::ExecutionError(format!(
                "prepared statement '{}' not found (call PREPARE first)",
                name
            ))
        })?;
        if !params.is_empty() {
            return Err(SqlError::ExecutionError(
                "EXECUTE ... USING with bind parameters is not yet supported in v3.9.0; \
                 use direct parameter substitution in the SQL body for now"
                    .to_string(),
            ));
        }
        self.execute(&sql)
    }

    pub(crate) fn execute_deallocate(&mut self, name: &str) -> SqlResult<ExecutorResult> {
        self.stmt_cache.deallocate(name);
        Ok(ExecutorResult::empty())
    }

    pub fn stmt_cache_stats(&self) -> sqlrustgo_cache::CacheStats {
        self.stmt_cache.stats()
    }
}

// === V312-55A / Issue #4238: minimal SQL LIKE matcher for SHOW PROCEDURE STATUS LIKE ===
/// V312-55A / Issue #4238: minimal SQL LIKE matcher used by
/// `SHOW PROCEDURE STATUS LIKE 'pat'`. Supports `%` as zero-or-more
/// wildcard; everything else is matched literally. The pattern is
/// pre-lowercased by the caller; the haystack must also be
/// pre-lowercased.
///
/// This is intentionally tiny — the v3.12.0 controlled subset for
/// SHOW PROCEDURE STATUS only needs to filter by procedure name, and
/// the existing wire protocol already case-folds procedure names.
fn like_match(pattern: &str, haystack: &str) -> bool {
    if pattern == "%" {
        return true;
    }
    if !pattern.contains('%') {
        return pattern == haystack;
    }
    // Split on '%', iterate segments in order.
    let segments: Vec<&str> = pattern.split('%').collect();
    let mut pos = 0usize;
    let bytes = haystack.as_bytes();
    // Leading literal
    let first = segments.first().copied().unwrap_or("");
    if !first.is_empty() {
        if !haystack.starts_with(first) {
            return false;
        }
        pos = first.len();
    }
    // Trailing literal
    let last = segments.last().copied().unwrap_or("");
    if !last.is_empty() && !haystack.ends_with(last) {
        return false;
    }
    // Middle segments must appear in order
    for seg in &segments[1..segments.len().saturating_sub(1)] {
        if seg.is_empty() {
            continue;
        }
        if pos > bytes.len() {
            return false;
        }
        match haystack[pos..].find(seg) {
            Some(idx) => pos = pos + idx + seg.len(),
            None => return false,
        }
    }
    true
}

// === V312-56A / Issue #4251: simple glob pattern matcher for SHOW COLUMNS LIKE ===
/// Simple glob pattern matching for SHOW COLUMNS LIKE pattern.
/// Supports: * matches any characters, ? matches single character.
fn wildcard_match(s: &str, pattern: &str) -> bool {
    let s_bytes = s.as_bytes();
    let p_bytes = pattern.as_bytes();
    let mut si = 0usize; // current byte position in s
    let mut pi = 0usize; // current byte position in pattern
    // When we see a `*`, remember the pattern position and where we were in s.
    // On mismatch later, backtrack to that `*` and consume one more char of s.
    let mut star_pi: Option<usize> = None;
    let mut star_si: usize = 0;

    while si < s_bytes.len() {
        if pi < p_bytes.len() {
            match p_bytes[pi] {
                b'*' => {
                    star_pi = Some(pi);
                    star_si = si;
                    pi += 1;
                }
                b'?' => {
                    si += 1;
                    pi += 1;
                }
                c => {
                    if s_bytes[si] == c {
                        si += 1;
                        pi += 1;
                    } else if let Some(saved_pi) = star_pi {
                        // Backtrack: let the previous `*` consume one more byte of s.
                        pi = saved_pi + 1;
                        star_si += 1;
                        si = star_si;
                    } else {
                        return false;
                    }
                }
            }
        } else if let Some(saved_pi) = star_pi {
            // Pattern exhausted but s still has bytes — backtrack through last `*`.
            pi = saved_pi + 1;
            star_si += 1;
            si = star_si;
        } else {
            return false;
        }
    }

    // Allow trailing `*`s in the pattern (they match the empty tail of s).
    while pi < p_bytes.len() && p_bytes[pi] == b'*' {
        pi += 1;
    }
    pi == p_bytes.len()
}

#[cfg(test)]
mod like_match_tests {
    use super::like_match;

    #[test]
    fn procedure_like_match_basic() {
        // % is the universal wildcard
        assert!(like_match("%", "anything"));
        // No wildcard → exact match
        assert!(like_match("foo", "foo"));
        assert!(!like_match("foo", "bar"));
        assert!(!like_match("foo", "foobar"));
        // Leading wildcard
        assert!(like_match("%bar", "foobar"));
        assert!(like_match("%bar", "bar"));
        assert!(!like_match("%bar", "foobaz"));
        // Trailing wildcard
        assert!(like_match("foo%", "foobar"));
        assert!(like_match("foo%", "foo"));
        // Both sides
        assert!(like_match("%o%", "foobar"));
        assert!(!like_match("%z%", "foobar"));
        // Multiple segments
        assert!(like_match("a%c", "abc"));
        assert!(like_match("a%c", "abbc"));
        assert!(!like_match("a%c", "abx"));
    }
}

#[cfg(test)]
mod wildcard_match_tests {
    use super::wildcard_match;

    #[test]
    fn show_columns_wildcard_match_basic() {
        // * matches any characters
        assert!(wildcard_match("anything", "*"));
        assert!(wildcard_match("foobar", "foo*"));
        assert!(wildcard_match("foo", "foo*"));
        assert!(!wildcard_match("foobar", "bar*"));
        // ? matches single character
        assert!(wildcard_match("foo", "f?o"));
        assert!(!wildcard_match("foo", "f??o"));
        // Exact
        assert!(wildcard_match("foo", "foo"));
        assert!(!wildcard_match("foo", "bar"));
    }
}
