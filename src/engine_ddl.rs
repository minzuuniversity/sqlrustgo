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
        }
    }

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

    /// SHOW INDEX — placeholder (v3.7.0 indexes are not cataloged).
    pub(crate) fn execute_show_index(&self, _table: &str) -> SqlResult<ExecutorResult> {
        Ok(ExecutorResult::new(vec![], 0))
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

    /// SHOW COLUMNS — placeholder (v3.7.0 column metadata not exposed).
    pub(crate) fn execute_show_columns(
        &self,
        _table: &str,
        _pattern: Option<&str>,
    ) -> SqlResult<ExecutorResult> {
        Ok(ExecutorResult::new(vec![], 0))
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
                    // V312-19 #4039: defer to storage.modify_column (same path as
                    // ALTER TABLE ... MODIFY). Cast mismatch will surface as a
                    // query error on read of old rows; we accept the type change
                    // rather than blocking it.
                    let info = storage
                        .get_table_info(&alter.table_name)
                        .map_err(|e| SqlError::ExecutionError(e.to_string()))?;
                    let col = info
                        .columns
                        .iter()
                        .find(|c| c.name.to_uppercase() == name.to_uppercase())
                        .ok_or_else(|| {
                            SqlError::ExecutionError(format!(
                                "Column '{}' not found in table '{}'",
                                name, alter.table_name
                            ))
                        })?;
                    let new_def = ColumnDefinition {
                        name: col.name.clone(),
                        data_type: data_type.clone(),
                        nullable: col.nullable,
                        primary_key: col.primary_key,
                        char_max_length: col.char_max_length,
                    };
                    storage.modify_column(&alter.table_name, name, new_def)?;
                }
                AlterColumnOperation::SetDefault { .. } => {
                    return Err(SqlError::ParseError(format!(
                        "ALTER COLUMN '{}' SET DEFAULT not supported",
                        name
                    )));
                }
                AlterColumnOperation::DropDefault => {
                    return Err(SqlError::ParseError(format!(
                        "ALTER COLUMN '{}' DROP DEFAULT not supported",
                        name
                    )));
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

    /// Get prepared statement cache statistics.
    pub fn stmt_cache_stats(&self) -> sqlrustgo_cache::CacheStats {
        self.stmt_cache.stats()
    }
}
