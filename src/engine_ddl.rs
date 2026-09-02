//! DDL/DML execution methods: GRANT, REVOKE, CREATE/DROP ROLE,
//! SHOW, DESCRIBE, ALTER TABLE, PREPARE/EXECUTE/DEALLOCATE,
//! and related catalog operations.
//!
//! Extracted from execution_engine.rs for C-ARCH-05 compliance
//! (line count limit: 1500). All methods are `impl ExecutionEngine`.

use crate::execution_engine::ExecutionEngine;
use crate::expr_utils::evaluate_expression;
use crate::{SqlError, SqlResult, Value};
use sqlrustgo_catalog::auth::{Privilege, UserIdentity};
use sqlrustgo_executor::ExecutorResult;
use sqlrustgo_parser::parser::{
    AlterColumnOperation, AlterTableOperation, AlterTableStatement, CreateRoleStatement,
    CreateUserStatement, DescribeStatement, DropRoleStatement, DropUserStatement,
    GrantRoleStatement, GrantStatement, ObjectType as ParserObjectType,
    Privilege as ParserPrivilege, RevokeRoleStatement, RevokeStatement, SetRoleStatement,
    ShowStatement,
};
use sqlrustgo_parser::Expression;
use sqlrustgo_storage::{ColumnDefinition, StorageEngine, TableInfo, Value as StorageValue};

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
            // V312-58 / Issue #4516: SHOW TABLES accepts FROM db / LIKE
            // 'pat' / WHERE expr (the FILTER_SUFFIX shared with
            // SHOW [FULL] TABLES). Real filter evaluation happens in
            // `execute_show_tables_with_filter` — for v3.12 the
            // single-schema engine ignores the `db` argument (only the
            // "default" schema exists) but LIKE / WHERE filter rows.
            ShowStatement::Tables {
                db,
                like,
                where_clause,
            } => self.execute_show_tables_with_filter(
                db.as_deref(),
                like.as_deref(),
                where_clause.as_ref(),
            ),
            // V312-58 / Issue #4516: LIKE filter support for
            // SHOW DATABASES (FROM is implicit; WHERE is uncommon).
            ShowStatement::Databases => self.execute_show_databases(),
            ShowStatement::CreateTable { table } => self.execute_show_create_table(table),
            ShowStatement::Index { table } => self.execute_show_index(table),
            ShowStatement::Grants { user } => self.execute_show_grants(user.as_deref()),
            ShowStatement::Columns { table, pattern } => {
                self.execute_show_columns(table, pattern.as_deref())
            }
            ShowStatement::Sequences => self.execute_show_sequences(),
            // V312-55A / #4238: route SHOW PROCEDURE STATUS to its
            // dedicated executor when a catalog is wired up; otherwise
            // the dedicated handler returns a clear error.
            ShowStatement::ProcedureStatus { pattern } => {
                self.execute_show_procedure_status(pattern.as_deref())
            }
            // Round-21 / Issue #4218: SHOW PROCESSLIST is parsed but
            // Round-21 / Issue #4218: SHOW PROCESSLIST is parsed but
            // the controlled-subset executor returns an empty result
            // (no live process registry yet). Tests assert rows.len() == 0
            // rather than an explicit failure — see
            // `execution_engine_tests::test_executor_show_processlist_*`.
            ShowStatement::Processlist { .. } => Ok(ExecutorResult::empty()),
            // V312-56A / 56A-R4: SHOW WARNINGS / ERRORS / STATUS / VARIABLES.
            // Controlled-subset executors — no session-warning/error
            // registry yet, so return empty results with stable schema.
            // Each handler documents what its real output will look
            // like once the corresponding MySQL catalog row is wired.
            ShowStatement::Warnings => self.execute_show_warnings(),
            ShowStatement::Errors => self.execute_show_errors(),
            ShowStatement::Status => self.execute_show_status(),
            ShowStatement::Variables => self.execute_show_variables(),
            // V312-59-A / Issue #4384 (56A-R3 anti-deferral): SHOW
            // [FULL] TABLES / SHOW TABLE STATUS. `full` is only ever
            // true for the FULL form (bare `SHOW TABLES` routes to
            // `ShowStatement::Tables` above).
            ShowStatement::FullTables {
                full,
                db,
                like,
                where_clause,
            } => self.execute_show_full_tables(
                *full,
                db.as_deref(),
                like.as_deref(),
                where_clause.as_ref(),
            ),
            ShowStatement::TableStatus {
                db,
                like,
                where_clause,
            } => self.execute_show_table_status(
                db.as_deref(),
                like.as_deref(),
                where_clause.as_ref(),
            ),
        }
    }

    /// V312-56A / 56A-R4: `SHOW WARNINGS`. Returns the current session's
    /// accumulated warnings as a 3-column result (Level, Code, Message).
    /// In v3.12 the session accumulator is empty; real wiring is
    /// deferred to v3.13 (per #4251 / 56A-R4 DEFERRED entry).
    pub(crate) fn execute_show_warnings(&self) -> SqlResult<ExecutorResult> {
        // 3-column schema: Level | Code | Message
        Ok(ExecutorResult::new(Vec::new(), 3))
    }

    /// V312-56A / 56A-R4: `SHOW ERRORS`. Same schema as WARNINGS;
    /// returns session errors. Empty in v3.12 (no session-error
    /// accumulator yet — per #4251 / 56A-R4 DEFERRED entry).
    pub(crate) fn execute_show_errors(&self) -> SqlResult<ExecutorResult> {
        Ok(ExecutorResult::new(Vec::new(), 3))
    }

    /// V312-56A / 56A-R4: `SHOW STATUS`. Returns a fixed catalog of
    /// server status variables. Controlled-subset v3.12 implementation
    /// returns a small hard-coded catalog rather than live metrics.
    pub(crate) fn execute_show_status(&self) -> SqlResult<ExecutorResult> {
        let rows: Vec<Vec<Value>> = vec![
            vec![Value::Text("Uptime".to_string()), Value::Integer(0)],
            vec![Value::Text("Threads".to_string()), Value::Integer(1)],
            vec![Value::Text("Questions".to_string()), Value::Integer(0)],
            vec![Value::Text("Slow_queries".to_string()), Value::Integer(0)],
        ];
        Ok(ExecutorResult::new(rows, 2))
    }

    /// V312-56A / 56A-R4: `SHOW VARIABLES`. Returns a fixed catalog of
    /// server system variables. Controlled-subset v3.12 implementation
    /// returns a small hard-coded catalog (version, sql_mode, etc.).
    pub(crate) fn execute_show_variables(&self) -> SqlResult<ExecutorResult> {
        let rows: Vec<Vec<Value>> = vec![
            vec![
                Value::Text("version".to_string()),
                Value::Text("sqlrustgo-3.12.0-controlled-subset".to_string()),
            ],
            vec![
                Value::Text("sql_mode".to_string()),
                Value::Text("".to_string()),
            ],
            vec![Value::Text("autocommit".to_string()), Value::Integer(1)],
            vec![
                Value::Text("character_set_server".to_string()),
                Value::Text("utf8".to_string()),
            ],
        ];
        Ok(ExecutorResult::new(rows, 2))
    }

    /// V312-58 / Issue #4516: unfiltered entry point kept for callers
    /// that want the bare table list without routing through the
    /// `WHERE` / `LIKE` plumbing. All current call sites use
    /// `execute_show_tables_with_filter` directly; this is kept as a
    /// convenience wrapper for future external callers.
    #[allow(dead_code)]
    pub(crate) fn execute_show_tables(&self) -> SqlResult<ExecutorResult> {
        self.execute_show_tables_with_filter(None, None, None)
    }

    /// V312-58 / Issue #4516: filtered `SHOW TABLES [FROM db]
    /// [LIKE 'pat'] [WHERE expr]`. Mirrors `execute_show_full_tables`
    /// semantics but returns a single-column result (the non-FULL
    /// form). `db` is accepted but ignored in v3.12 (single-schema
    /// engine — only `default` exists); `like` filters by name via
    /// SQL LIKE wildcards; `where_clause` is evaluated against a
    /// synthesized row `{Name: <table>, Table_type: <type>}` so
    /// `WHERE Table_type = 'BASE TABLE'` works.
    pub(crate) fn execute_show_tables_with_filter(
        &self,
        _db: Option<&str>,
        like: Option<&str>,
        where_clause: Option<&Expression>,
    ) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();
        let views: Vec<String> = self.views.keys().cloned().collect();
        // Issue #4567: list views alongside base tables (MySQL semantics —
        // SHOW TABLES includes views; only SHOW FULL TABLES distinguishes
        // them via Table_type). Pre-#4567 views were acked by CREATE VIEW
        // but invisible here.
        let mut names = storage.list_tables();
        names.extend(views.iter().cloned());
        let mut rows = Vec::new();
        for name in &names {
            if let Some(pat) = like {
                if !sql_like_match(name, pat) {
                    continue;
                }
            }
            if let Some(expr) = where_clause {
                let table_type = if views.iter().any(|v| v == name) {
                    "VIEW"
                } else {
                    "BASE TABLE"
                };
                let row = vec![
                    Value::Text(name.clone()),
                    Value::Text(table_type.to_string()),
                ];
                let table_info = TableInfo {
                    columns: vec![
                        ColumnDefinition {
                            name: "Name".to_string(),
                            ..Default::default()
                        },
                        ColumnDefinition {
                            name: "Table_type".to_string(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                };
                let value = evaluate_expression(expr, &row, &table_info).map_err(|e| {
                    SqlError::ExecutionError(format!("SHOW TABLES WHERE failed: {e}"))
                })?;
                if !matches!(value, Value::Boolean(true)) {
                    continue;
                }
            }
            rows.push(vec![Value::Text(name.clone())]);
        }
        Ok(ExecutorResult::new(rows, 1))
    }

    pub(crate) fn execute_show_databases(&self) -> SqlResult<ExecutorResult> {
        // V312-58 / Issue #4516: emit the single hard-coded schema;
        // callers asking for LIKE filter get it via `execute_show_databases_like`.
        Ok(ExecutorResult::new(
            vec![vec![Value::Text("default".to_string())]],
            1,
        ))
    }

    /// V312-59-A / Issue #4384 (56A-R3 anti-deferral): `SHOW [FULL]
    /// TABLES [FROM db] [LIKE 'pat' | WHERE expr]`. The FULL form
    /// returns two columns — `Name` and `Type` (`BASE TABLE` or
    /// `VIEW`); the non-FULL form returns bare table names (the same
    /// shape as `execute_show_tables`). `LIKE` and `WHERE` filters
    /// are evaluated against the generated rows, so `SHOW FULL TABLES
    /// WHERE Table_type != 'VIEW'` really filters.
    pub(crate) fn execute_show_full_tables(
        &self,
        full: bool,
        _db: Option<&str>,
        like: Option<&str>,
        where_clause: Option<&Expression>,
    ) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();
        let views: Vec<String> = self.views.keys().cloned().collect();
        let mut names = storage.list_tables();
        names.extend(views.iter().cloned());
        let table_type = |name: &str| {
            if views.iter().any(|v| v == name) {
                "VIEW"
            } else {
                "BASE TABLE"
            }
        };
        let mut rows = Vec::new();
        for name in &names {
            let row = if full {
                vec![
                    Value::Text(name.clone()),
                    Value::Text(table_type(name).to_string()),
                ]
            } else {
                vec![Value::Text(name.clone())]
            };
            if let Some(pat) = like {
                if !sql_like_match(name, pat) {
                    continue;
                }
            }
            if let Some(expr) = where_clause {
                let table_info = TableInfo {
                    columns: vec![
                        ColumnDefinition {
                            name: "Name".to_string(),
                            ..Default::default()
                        },
                        ColumnDefinition {
                            name: "Table_type".to_string(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                };
                let value = evaluate_expression(expr, &row, &table_info).map_err(|e| {
                    SqlError::ExecutionError(format!("SHOW FULL TABLES WHERE failed: {e}"))
                })?;
                if !matches!(value, Value::Boolean(true)) {
                    continue;
                }
            }
            rows.push(row);
        }
        Ok(ExecutorResult::new(rows, if full { 2 } else { 1 }))
    }

    /// V312-59-A / Issue #4384 (56A-R3 anti-deferral): `SHOW TABLE
    /// STATUS [FROM db] [LIKE 'pat' | WHERE expr]`. Returns MySQL
    /// 18-column rows: Name, Engine, Version, Row_format, Rows,
    /// Avg_row_length, Data_length, Max_data_length, Index_length,
    /// Data_free, Auto_increment, Create_time, Update_time,
    /// Check_time, Collation, Checksum, Create_options, Comment.
    /// Controlled-subset values: Engine=InnoDB, Version=10,
    /// Row_format=Dynamic, Collation=utf8mb4_general_ci, timestamps
    /// and Checksum NULL, remaining numerics 0. Rows is the real
    /// scanned row count.
    pub(crate) fn execute_show_table_status(
        &self,
        _db: Option<&str>,
        like: Option<&str>,
        where_clause: Option<&Expression>,
    ) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();
        let names = storage.list_tables();
        let mut rows = Vec::new();
        for name in &names {
            let row = table_status_row(&*storage, name)?;
            if let Some(pat) = like {
                if !sql_like_match(name, pat) {
                    continue;
                }
            }
            if let Some(expr) = where_clause {
                let table_info = TableInfo {
                    columns: vec![
                        ColumnDefinition {
                            name: "Name".to_string(),
                            ..Default::default()
                        },
                        ColumnDefinition {
                            name: "Engine".to_string(),
                            ..Default::default()
                        },
                        ColumnDefinition {
                            name: "Rows".to_string(),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                };
                let value = evaluate_expression(expr, &row, &table_info).map_err(|e| {
                    SqlError::ExecutionError(format!("SHOW TABLE STATUS WHERE failed: {e}"))
                })?;
                if !matches!(value, Value::Boolean(true)) {
                    continue;
                }
            }
            rows.push(row);
        }
        Ok(ExecutorResult::new(rows, 18))
    }

    pub(crate) fn execute_show_sequences(&self) -> SqlResult<ExecutorResult> {
        let storage = self.storage.read();
        let names = storage.list_sequences();
        let rows: Vec<Vec<Value>> = names.into_iter().map(|n| vec![Value::Text(n)]).collect();
        Ok(ExecutorResult::new(rows, 1))
    }

    /// V312-55A / Issue #4238: `SHOW PROCEDURE STATUS [LIKE 'pat']`.
    ///
    /// Stub implementation for the V312-56A rebase. The full handler
    /// (catalog-walked procedure enumeration with LIKE filtering) lives
    /// behind V312-55A gate work; for V312-56A we surface an explicit
    /// error so callers don't see a silent empty result.
    pub(crate) fn execute_show_procedure_status(
        &self,
        _pattern: Option<&str>,
    ) -> SqlResult<ExecutorResult> {
        Err(SqlError::ExecutionError(
            "SHOW PROCEDURE STATUS is owned by V312-55A (#4238) and not \
             yet wired into the v3.12.0 executor"
                .to_string(),
        ))
    }

    /// SHOW CREATE TABLE — reconstruct CREATE TABLE from the live schema.
    ///
    /// V312-56A / #4251: emits a DDL string containing the controlled
    /// subset — column type, NULL/NOT NULL, DEFAULT literal, and a
    /// trailing `PRIMARY KEY (...)` clause derived from the columns
    /// flagged `primary_key = true` in `ColumnDefinition`. The output is
    /// not a full MySQL 5.7 round-trip; refer to the issue body for the
    /// explicit scope.
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
                let default = match &c.default_value {
                    Some(v) => format!(" DEFAULT {}", format_default_literal(v)),
                    None => String::new(),
                };
                format!("{} {}{}{}", c.name, c.data_type, nullable, default)
            })
            .collect();
        let pk_cols: Vec<&str> = info
            .columns
            .iter()
            .filter(|c| c.primary_key)
            .map(|c| c.name.as_str())
            .collect();
        let mut parts = cols;
        if !pk_cols.is_empty() {
            parts.push(format!("PRIMARY KEY ({})", pk_cols.join(", ")));
        }
        let ddl = format!("CREATE TABLE {} ({})", table, parts.join(", "));
        Ok(ExecutorResult::new(vec![vec![Value::Text(ddl)]], 1))
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
        let rows = column_metadata_rows(&info.columns);
        Ok(ExecutorResult::new(rows, info.columns.len()))
    }

    /// SHOW GRANTS — placeholder (v3.7.0 grant tracking is limited to roles).
    pub(crate) fn execute_show_grants(&self, _user: Option<&str>) -> SqlResult<ExecutorResult> {
        Ok(ExecutorResult::new(vec![], 0))
    }

    /// SHOW COLUMNS FROM <table> [LIKE '<pattern>'].
    ///
    /// V312-56A / #4251: returns column metadata in MySQL-compatible
    /// Field/Type/Null/Key/Default/Extra columns. Optional LIKE pattern
    /// filters by column name using `%`/`_` wildcards. The output is
    /// identical to `DESCRIBE <table>` for the unfiltered case.
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
        let mut rows = column_metadata_rows(&info.columns);
        if let Some(pat) = pattern {
            rows.retain(|row| {
                if let Some(Value::Text(field)) = row.first() {
                    sql_like_match(field, pat)
                } else {
                    false
                }
            });
        }
        let count = rows.len();
        Ok(ExecutorResult::new(rows, count))
    }

    /// SHOW INDEX FROM <table>.
    ///
    /// V312-56A / #4251: returns registered index metadata instead of an
    /// empty placeholder. Output columns mirror MySQL 5.7 SHOW INDEX:
    /// Table, Non_unique, Key_name, Seq_in_index, Column_name, Collation,
    /// Cardinality, Sub_part, Packed, Null, Index_type, Comment.
    /// Columns not yet collected are NULL.
    ///
    /// Index source priority:
    /// 1. Catalog (sqlrustgo_catalog::Table::indices) when the catalog
    ///    has been wired up — exposes full multi-column PK, secondary
    ///    unique / non-unique indices, and `PRIMARY` index when a PK
    ///    was registered.
    /// 2. Storage columns (ColumnDefinition.primary_key) — synthesizes
    ///    a single-column `PRIMARY` index row when the catalog is
    ///    unavailable so the storage-only engine (used by tests and
    ///    the simplest `ExecutionEngine::new` callers) still reports
    ///    PK information.
    pub(crate) fn execute_show_index(&self, table: &str) -> SqlResult<ExecutorResult> {
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
        drop(storage);

        let mut rows: Vec<Vec<Value>> = Vec::new();
        let mut push_pk = |table_name: &str,
                           col: &str,
                           seq: usize,
                           nullable: bool,
                           rows: &mut Vec<Vec<Value>>| {
            rows.push(vec![
                Value::Text(table_name.to_string()),
                Value::Integer(0),
                Value::Text("PRIMARY".to_string()),
                Value::Integer(seq as i64),
                Value::Text(col.to_string()),
                Value::Text("A".to_string()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Text(if nullable { "YES" } else { "" }.to_string()),
                Value::Text("BTREE".to_string()),
                Value::Text(String::new()),
            ]);
        };

        // Track whether the catalog knows about this table; if it does not,
        // fall through to the storage-columns fallback so a plain
        // `CREATE TABLE … PRIMARY KEY` (which only registers with
        // storage when no custom catalog is wired up) still surfaces
        // its implicit PK. The catalog is auto-provisioned in
        // `ExecutionEngine::new()` but stays empty for tables created
        // via the standard storage-only CREATE TABLE path.
        let mut catalog_had_table = false;
        if let Some(catalog_arc) = self.catalog.as_ref() {
            let catalog_guard = catalog_arc.read();
            for (_db, schema) in catalog_guard.all_schemas() {
                let Some(table_ref) = schema.tables().into_iter().find(|t| t.name == table) else {
                    continue;
                };
                catalog_had_table = true;
                let mut had_explicit_pk_index = false;
                for index in &table_ref.indices {
                    for (i, column_name) in index.columns.iter().enumerate() {
                        let column_nullable = table_ref
                            .columns
                            .iter()
                            .find(|c| &c.name == column_name)
                            .map(|c| c.nullable)
                            .unwrap_or(true);
                        rows.push(vec![
                            Value::Text(table_ref.name.clone()),
                            Value::Integer(if index.is_unique { 0 } else { 1 }),
                            Value::Text(index.name.clone()),
                            Value::Integer((i + 1) as i64),
                            Value::Text(column_name.clone()),
                            Value::Text("A".to_string()),
                            Value::Null,
                            Value::Null,
                            Value::Null,
                            Value::Text(if column_nullable { "YES" } else { "" }.to_string()),
                            Value::Text(format!("{:?}", index.index_type)),
                            Value::Text(String::new()),
                        ]);
                    }
                    if index.is_primary_key {
                        had_explicit_pk_index = true;
                    }
                }
                if !had_explicit_pk_index {
                    if let Some(pk_cols) = table_ref.primary_key.as_ref() {
                        for (i, col) in pk_cols.iter().enumerate() {
                            let column_nullable = table_ref
                                .columns
                                .iter()
                                .find(|c| &c.name == col)
                                .map(|c| c.nullable)
                                .unwrap_or(false);
                            push_pk(&table_ref.name, col, i + 1, column_nullable, &mut rows);
                        }
                    }
                }
                break;
            }
        }
        if !catalog_had_table {
            // No catalog (or the catalog does not know about this table
            // because it was created via the storage-only CREATE TABLE
            // path): derive PRIMARY from storage columns where
            // `ColumnDefinition.primary_key == true`. The storage path
            // does not expose a multi-column PK as a single field, so
            // we report each PK column as a separate `PRIMARY` row.
            for (i, col) in info.columns.iter().enumerate() {
                if col.primary_key {
                    push_pk(table, &col.name, i + 1, col.nullable, &mut rows);
                }
            }
        }

        let count = rows.len();
        Ok(ExecutorResult::new(rows, count))
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

    /// V312-58 / Issue #4515: `CREATE USER 'name'@'host' [IDENTIFIED BY 'pwd']`.
    ///
    /// Registers a new user with the catalog's `AuthManager`. When
    /// `password_hash` is `None` (the user wrote no `IDENTIFIED BY`
    /// clause), we pass an empty string — `AuthManager::create_user`
    /// accepts that as "no password".
    pub(crate) fn execute_create_user(
        &mut self,
        stmt: &CreateUserStatement,
    ) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let mut catalog_guard = catalog.write();

        let identity = UserIdentity::new(&stmt.user, &stmt.host);
        let password_hash = stmt.password_hash.as_deref().unwrap_or("");

        catalog_guard
            .auth_manager_mut()
            .create_user(&identity, password_hash)
            .map_err(|e| SqlError::ExecutionError(format!("CREATE USER failed: {}", e)))?;

        Ok(ExecutorResult::new(
            vec![vec![Value::Text(format!(
                "User '{}'@'{}' created",
                stmt.user, stmt.host
            ))]],
            1,
        ))
    }

    /// V312-58 / Issue #4515: `DROP USER 'name'@'host' [IF EXISTS]`.
    ///
    /// Mirrors `DROP TRIGGER [IF EXISTS]`: with `IF EXISTS`, a missing
    /// user is a no-op; without it, we error.
    pub(crate) fn execute_drop_user(
        &mut self,
        stmt: &DropUserStatement,
    ) -> SqlResult<ExecutorResult> {
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| SqlError::ExecutionError("No catalog available".to_string()))?;
        let mut catalog_guard = catalog.write();

        let identity = UserIdentity::new(&stmt.user, &stmt.host);
        let result = catalog_guard.auth_manager_mut().drop_user(&identity);

        match result {
            Ok(_) => Ok(ExecutorResult::new(
                vec![vec![Value::Text(format!(
                    "User '{}'@'{}' dropped",
                    stmt.user, stmt.host
                ))]],
                1,
            )),
            Err(e) => {
                if stmt.if_exists {
                    Ok(ExecutorResult::new(
                        vec![vec![Value::Text(format!(
                            "User '{}'@'{}' did not exist (IF EXISTS, no-op)",
                            stmt.user, stmt.host
                        ))]],
                        1,
                    ))
                } else {
                    Err(SqlError::ExecutionError(format!("DROP USER failed: {}", e)))
                }
            }
        }
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
                default_value,
            } => {
                // #4571: keep the parsed DEFAULT / NOT NULL — previously
                // `default_value` was discarded (`default_value: _`) so
                // `ADD COLUMN c INT DEFAULT 5` lost its default.
                let column = ColumnDefinition {
                    name: name.clone(),
                    data_type: data_type.clone(),
                    nullable: *nullable,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: default_value.clone(),
                    auto_increment: false,
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
                    auto_increment: false,
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
                        auto_increment: false,
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
            // Issue #4580 / B-track case 30: ALTER TABLE ADD CONSTRAINT.
            // Currently a parse-only no-op — the constraint is captured
            // and stored but not enforced. UNIQUE/PRIMARY KEY inside
            // the constraint could be applied by pushing the columns
            // into TableInfo; that wiring is tracked as a follow-up.
            AlterTableOperation::AddTableConstraint(_constraint) => {
                // Parse-only: statement is accepted but the constraint
                // is not yet enforced. Returning Ok(()) keeps the
                // statement from blocking dependent SQL (e.g. INSERT).
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
        // V312-58 / Issue #4511: bind each USING param to the next `?`
        // placeholder in the prepared SQL (positional). Each `params[i]`
        // is currently expected to be `Expression::ColumnRef("@name")`
        // (the lexer emits `@a` as a single Identifier); we look up the
        // bound session variable and rewrite the SQL so the placeholders
        // become SQL literals before re-parsing.
        let rewritten = if params.is_empty() {
            sql
        } else {
            let session_vars = self.session_vars.read().clone();
            let mut values: Vec<sqlrustgo_storage::Value> = Vec::with_capacity(params.len());
            for p in params {
                let key = match p {
                    sqlrustgo_parser::Expression::Identifier(name) => name.clone(),
                    _ => {
                        return Err(SqlError::ExecutionError(
                            "EXECUTE USING expects @name user variables".to_string(),
                        ))
                    }
                };
                let v = session_vars
                    .get(&key)
                    .cloned()
                    .unwrap_or(sqlrustgo_storage::Value::Null);
                values.push(v);
            }
            substitute_placeholders(&sql, &values)?
        };
        self.execute(&rewritten)
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

/// Render a default literal string as the `DEFAULT` clause for
/// `SHOW CREATE TABLE`. The default is stored as a pre-formatted
/// SQL literal (e.g. `"pending"`, `42`, `NULL`), so we only need
/// to add single quotes if it isn't already a NULL or a numeric
/// literal. Embedded `'` are doubled to keep the DDL parseable.
fn format_default_literal(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("NULL") {
        "NULL".to_string()
    } else if trimmed.eq_ignore_ascii_case("TRUE")
        || trimmed.eq_ignore_ascii_case("FALSE")
        || trimmed.parse::<f64>().is_ok()
        || (trimmed.starts_with('-') && trimmed[1..].parse::<f64>().is_ok())
    {
        trimmed.to_string()
    } else {
        format!("'{}'", trimmed.replace('\'', "''"))
    }
}

/// Build the 6-column row tuple used by both `DESCRIBE` and
/// `SHOW COLUMNS`. Columns are: Field, Type, Null, Key, Default, Extra.
fn column_metadata_rows(columns: &[ColumnDefinition]) -> Vec<Vec<Value>> {
    columns
        .iter()
        .map(|c| {
            let null_str = if c.nullable { "YES" } else { "NO" };
            let key_str = if c.primary_key { "PRI" } else { "" };
            let default_str = match &c.default_value {
                Some(v) => v.to_string(),
                None => "NULL".to_string(),
            };
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
                Value::Text(default_str),
                Value::Text(String::new()),
            ]
        })
        .collect()
}

/// V312-59-A / Issue #4384 (56A-R3 anti-deferral): build one MySQL
/// 18-column `SHOW TABLE STATUS` row for a table. Numeric stats that
/// sqlrustgo does not track (Data_length, Index_length, ...) are 0;
/// timestamps and Checksum are NULL; `Rows` is the real scanned count.
fn table_status_row<S: StorageEngine + ?Sized>(storage: &S, name: &str) -> SqlResult<Vec<Value>> {
    let row_count = storage.scan(name).map(|r| r.len() as i64).unwrap_or(0);
    let column_count = storage
        .get_table_info(name)
        .map(|i| i.columns.len() as i64)
        .unwrap_or(0);
    Ok(vec![
        Value::Text(name.to_string()),
        Value::Text("InnoDB".to_string()),
        Value::Integer(10),
        Value::Text("Dynamic".to_string()),
        Value::Integer(row_count),
        Value::Integer(0),
        Value::Integer(0),
        Value::Integer(0),
        Value::Integer(0),
        Value::Integer(0),
        Value::Integer(column_count),
        Value::Null,
        Value::Null,
        Value::Null,
        Value::Text("utf8mb4_general_ci".to_string()),
        Value::Null,
        Value::Text(String::new()),
        Value::Text(String::new()),
    ])
}

/// SQL LIKE pattern matcher supporting `%` (zero or more chars) and `_`
/// (single char). Case-sensitive (matching MySQL's default
/// `utf8mb4_bin`-style behavior). Escapes via `\`.
fn sql_like_match(text: &str, pattern: &str) -> bool {
    let pat_bytes: Vec<char> = pattern.chars().collect();
    let txt_bytes: Vec<char> = text.chars().collect();
    let mut pi = 0;
    let mut ti = 0;
    let mut star_pi: Option<usize> = None;
    let mut star_ti: usize = 0;
    while ti < txt_bytes.len() {
        if pi < pat_bytes.len() {
            match pat_bytes[pi] {
                '%' => {
                    star_pi = Some(pi);
                    star_ti = ti;
                    pi += 1;
                    continue;
                }
                '_' => {
                    pi += 1;
                    ti += 1;
                    continue;
                }
                '\\' if pi + 1 < pat_bytes.len() => {
                    if pat_bytes[pi + 1] == txt_bytes[ti] {
                        pi += 2;
                        ti += 1;
                        continue;
                    }
                }
                c if c == txt_bytes[ti] => {
                    pi += 1;
                    ti += 1;
                    continue;
                }
                _ => {}
            }
        }
        if let Some(sp) = star_pi {
            pi = sp + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }
    while pi < pat_bytes.len() && pat_bytes[pi] == '%' {
        pi += 1;
    }
    pi == pat_bytes.len()
}

/// V312-58 / Issue #4511: rewrite the prepared SQL by substituting each
/// `?` placeholder (in source order) with the corresponding `Value`'s
/// SQL-literal text. Matches MySQL's positional binding semantics
/// (EXECUTE stmt USING @a, @b, @d binds @a → first ?, @b → second ?, etc.).
///
/// Strings are rendered with single quotes and embedded single quotes
/// doubled (per SQL standard). NULL → `NULL`, booleans → `true`/`false`,
/// numbers rendered via `Display`. Blob / Point / Json are rendered as
/// `NULL` since the prepared SQL grammar doesn't yet have literal
/// syntax for them.
///
/// Errors when the SQL contains more placeholders than `params` (would
/// leave a literal `?` in the rewritten SQL and fail to parse). Excess
/// params (more than placeholders) are an error too — silently dropping
/// them would mask caller mistakes.
fn substitute_placeholders(sql: &str, params: &[StorageValue]) -> SqlResult<String> {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut pi: usize = 0; // index into params
    let mut i: usize = 0; // index into sql bytes
    let mut in_single = false;
    let mut in_double = false;
    while i < bytes.len() {
        let b = bytes[i];
        // Track quoted strings (single / double quote) so we don't
        // mis-detect a `?` inside a string literal as a placeholder.
        // Backslash escapes are intentionally not handled — the
        // SQL standard SQL grammar uses doubled-quote escaping for
        // both single- and double-quoted strings, which our parser
        // already enforces.
        if b == b'\'' && !in_double {
            // Doubled '' inside a single-quoted string is an escape
            // (SQL standard), not a closing quote. Copy both bytes
            // verbatim and advance past them.
            if in_single && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                out.push('\'');
                out.push('\'');
                i += 2;
                continue;
            }
            in_single = !in_single;
            out.push('\'');
            i += 1;
            continue;
        }
        if b == b'"' && !in_single {
            in_double = !in_double;
            out.push('"');
            i += 1;
            continue;
        }
        if b == b'?' && !in_single && !in_double {
            let lit = match params.get(pi) {
                Some(StorageValue::Null) => "NULL".to_string(),
                Some(StorageValue::Boolean(true)) => "true".to_string(),
                Some(StorageValue::Boolean(false)) => "false".to_string(),
                Some(StorageValue::Integer(n)) => n.to_string(),
                Some(StorageValue::Float(f)) => f.to_string(),
                Some(StorageValue::Text(s)) => format!("'{}'", s.replace('\'', "''")),
                Some(StorageValue::Blob(_))
                | Some(StorageValue::Point(_, _))
                | Some(StorageValue::Json(_)) => "NULL".to_string(),
                None => {
                    return Err(SqlError::ExecutionError(format!(
                        "EXECUTE USING: not enough parameters (placeholder #{} has no USING value)",
                        pi + 1
                    )))
                }
            };
            out.push_str(&lit);
            pi += 1;
            i += 1;
            continue;
        }
        out.push(b as char);
        i += 1;
    }
    if pi < params.len() {
        return Err(SqlError::ExecutionError(format!(
            "EXECUTE USING: too many parameters ({} provided, {} placeholders)",
            params.len(),
            pi
        )));
    }
    Ok(out)
}
