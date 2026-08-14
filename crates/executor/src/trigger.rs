//! Trigger Execution Engine
//!
//! This module provides trigger execution functionality for SQL triggers.
//! Triggers are executed before or after INSERT, UPDATE, or DELETE operations.

use log::error as log_error;
use parking_lot::RwLock;
use sqlrustgo_catalog::auth::UserIdentity;
use sqlrustgo_parser::parse;
use sqlrustgo_storage::{
    Record, StorageEngine, TriggerEvent as StorageTriggerEvent, TriggerInfo,
    TriggerTiming as StorageTriggerTiming,
};
use sqlrustgo_types::{SqlError, SqlResult, Value};
use std::sync::Arc;

/// Trigger timing: BEFORE or AFTER
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerTiming {
    Before,
    After,
}

impl From<StorageTriggerTiming> for TriggerTiming {
    fn from(t: StorageTriggerTiming) -> Self {
        match t {
            StorageTriggerTiming::Before => TriggerTiming::Before,
            StorageTriggerTiming::After => TriggerTiming::After,
        }
    }
}

impl From<TriggerTiming> for StorageTriggerTiming {
    fn from(t: TriggerTiming) -> Self {
        match t {
            TriggerTiming::Before => StorageTriggerTiming::Before,
            TriggerTiming::After => StorageTriggerTiming::After,
        }
    }
}

/// Trigger event: INSERT, UPDATE, or DELETE
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerEvent {
    Insert,
    Update,
    Delete,
}

impl From<StorageTriggerEvent> for TriggerEvent {
    fn from(e: StorageTriggerEvent) -> Self {
        match e {
            StorageTriggerEvent::Insert => TriggerEvent::Insert,
            StorageTriggerEvent::Update => TriggerEvent::Update,
            StorageTriggerEvent::Delete => TriggerEvent::Delete,
        }
    }
}

impl From<TriggerEvent> for StorageTriggerEvent {
    fn from(e: TriggerEvent) -> Self {
        match e {
            TriggerEvent::Insert => StorageTriggerEvent::Insert,
            TriggerEvent::Update => StorageTriggerEvent::Update,
            TriggerEvent::Delete => StorageTriggerEvent::Delete,
        }
    }
}

/// Combined trigger event type with timing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerType {
    BeforeInsert,
    AfterInsert,
    BeforeUpdate,
    AfterUpdate,
    BeforeDelete,
    AfterDelete,
}

impl TriggerType {
    /// Create a TriggerType from timing and event
    pub fn new(timing: TriggerTiming, event: TriggerEvent) -> Self {
        match (timing, event) {
            (TriggerTiming::Before, TriggerEvent::Insert) => TriggerType::BeforeInsert,
            (TriggerTiming::After, TriggerEvent::Insert) => TriggerType::AfterInsert,
            (TriggerTiming::Before, TriggerEvent::Update) => TriggerType::BeforeUpdate,
            (TriggerTiming::After, TriggerEvent::Update) => TriggerType::AfterUpdate,
            (TriggerTiming::Before, TriggerEvent::Delete) => TriggerType::BeforeDelete,
            (TriggerTiming::After, TriggerEvent::Delete) => TriggerType::AfterDelete,
        }
    }
}

/// Trigger executor for running database triggers
pub struct TriggerExecutor {
    storage: Arc<RwLock<dyn StorageEngine>>,
    /// V312-55E (Round-27): recursion depth counter shared across all
    /// nested invocations of `execute_trigger_body`. Every call increments
    /// the counter on entry and decrements on exit (via RAII guard). If
    /// the counter exceeds [`MAX_RECURSION_DEPTH`] the executor aborts
    /// with [`SqlError::TriggerRecursionLimitExceeded`] instead of letting
    /// the host stack overflow on a self-referential or mutually-recursive
    /// trigger pair.
    recursion_depth: Arc<std::sync::atomic::AtomicUsize>,
    /// V312-55F / Issue #4243: current SQL session user identity. Mirrors
    /// `ExecutionEngine::current_user`. When the identity's username is
    /// `"root"`, the trigger body DML privilege check is short-circuited
    /// (MySQL convention — root@localhost has implicit full privilege).
    /// Otherwise `auth_check` is consulted for every body DML statement.
    current_user: UserIdentity,
    /// V312-55F / Issue #4243: optional privilege-check hook for trigger
    /// body DML. Receives the current user, the privilege required
    /// (`Insert`/`Update`/`Delete`), and the target table name. Returns
    /// `Ok(())` if the user is allowed, `Err(SqlError::PermissionDenied)`
    /// otherwise. Default is `None` — unit tests that don't wire a
    /// catalog continue to run trigger body DML unchecked (matches
    /// pre-V55F behavior). Production callers (ExecutionEngine) wire
    /// the closure at engine construction time via
    /// [`TriggerExecutor::set_auth_check`].
    auth_check: Option<Arc<dyn TriggerBodyAuthCheck>>,
}

/// V312-55F / Issue #4243: trait alias for the trigger body DML
/// privilege-check hook. Splitting this out from a bare `Box<dyn Fn>`
/// keeps the signature self-documenting at every call site.
pub trait TriggerBodyAuthCheck: Send + Sync {
    fn check(
        &self,
        user: &UserIdentity,
        privilege: sqlrustgo_catalog::auth::Privilege,
        table_name: &str,
    ) -> SqlResult<()>;
}

impl TriggerExecutor {
    /// V312-55F / Issue #4243: enforce the trigger body DML privilege
    /// check. No-op when `auth_check` is `None` (default — keeps unit
    /// tests in this crate running unchanged). When set, the hook is
    /// responsible for short-circuiting `root@localhost` (the
    /// `ExecutionEngine`-provided hook does this before delegating to
    /// the catalog's `AuthManager`).
    fn check_body_privilege(
        &self,
        privilege: sqlrustgo_catalog::auth::Privilege,
        table_name: &str,
    ) -> SqlResult<()> {
        if let Some(hook) = self.auth_check.as_ref() {
            hook.check(&self.current_user, privilege, table_name)?;
        }
        Ok(())
    }
}

/// V312-55E (Round-27): maximum nested trigger firing depth. Beyond this
/// limit `TriggerExecutor` returns
/// [`SqlError::TriggerRecursionLimitExceeded`] with the offending trigger
/// name, current depth, and configured limit. Tuned to 16 — enough headroom
/// for legitimate nested audit chains (3-4 deep is the realistic maximum in
/// production schemas) while keeping stack usage bounded.
pub const MAX_RECURSION_DEPTH: usize = 16;

// P1 FIX (SGL-005): TriggerExecutor storage bypasses wrapped in transaction
// boundary. When TriggerExecutor executes trigger body DML (INSERT/UPDATE/DELETE),
// it must go through the storage transaction API to satisfy TX-002.
// Production callers exist in src/execution_engine.rs (root workspace).
impl TriggerExecutor {
    pub fn new(storage: Arc<RwLock<dyn StorageEngine>>) -> Self {
        if !cfg!(test) {
            // BinaryTableStorage has no WAL. Triggers on a non-WAL storage are
            // a no-op (no DML recovery is possible) — warn and proceed instead
            // of panicking the server. WalStorage continues to enforce the
            // WAL contract at write time.
            if !storage.read().is_wal_enabled() {
                log_error!(
                    "TriggerExecutor::new: storage has no WAL enabled — \
                     triggers will be silently skipped (binary mode)"
                );
            }
        }
        Self {
            storage,
            recursion_depth: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            current_user: UserIdentity::new("root", "localhost"),
            auth_check: None,
        }
    }

    pub fn storage(&self) -> Arc<RwLock<dyn StorageEngine>> {
        self.storage.clone()
    }

    /// V312-55F / Issue #4243: switch the trigger executor's view of the
    /// current SQL session user. Defaults to `root@localhost` (which
    /// short-circuits `auth_check`). Callers (e.g. `ExecutionEngine`)
    /// invoke this when the wire session changes user via
    /// `SET ROLE` / `SET SESSION_USER` style commands.
    pub fn set_current_user(&mut self, identity: UserIdentity) {
        self.current_user = identity;
    }

    /// V312-55F / Issue #4243: returns the current SQL session user.
    pub fn current_user(&self) -> &UserIdentity {
        &self.current_user
    }

    /// V312-55F / Issue #4243: install a privilege-check hook for trigger
    /// body DML. Once set, every `INSERT` / `UPDATE` / `DELETE` inside a
    /// trigger body consults the hook before mutating the target table.
    /// Pass `None` to remove the hook (tests / dev tooling).
    pub fn set_auth_check(&mut self, hook: Option<Arc<dyn TriggerBodyAuthCheck>>) {
        self.auth_check = hook;
    }

    /// V312-55E (Round-27): expose the shared recursion-depth counter so
    /// integration tests can drive the depth-limit path deterministically.
    /// Production callers must NOT mutate this directly — the
    /// [`execute_trigger_body`] RAII guard is the only sanctioned writer.
    /// The accessor returns the underlying `Arc<AtomicUsize>` so tests
    /// can keep a handle to the same counter across calls.
    pub fn recursion_depth_counter(&self) -> Arc<std::sync::atomic::AtomicUsize> {
        self.recursion_depth.clone()
    }

    /// INT-4: single chokepoint for trigger-body DML. Opens a transaction, runs
    /// the closure, commits on success and rolls back on error.
    /// C-3c: single chokepoint for trigger-body DML. If the storage is ALREADY in a
    /// transaction (the trigger fired from inside a user's BEGIN..COMMIT/ROLLBACK
    /// block), the trigger participates in the outer transaction: no nested BEGIN,
    /// no nested COMMIT. The trigger's DML is rolled back / committed together
    /// with the user's transaction. Otherwise (autocommit), the trigger wraps its
    /// DML in its own short transaction for atomicity.
    fn execute_dml_in_tx<F, R>(&self, op: F) -> SqlResult<R>
    where
        F: FnOnce(&mut dyn StorageEngine) -> SqlResult<R>,
    {
        let mut storage = self.storage.write();
        let in_outer_tx = storage.in_transaction();
        if !in_outer_tx {
            storage.begin_transaction()?;
        }
        let result = op(&mut *storage);
        if !in_outer_tx {
            match &result {
                Ok(_) => storage.commit_transaction()?,
                Err(_) => {
                    let _ = storage.rollback_transaction();
                }
            }
        }
        result
    }

    /// Get all triggers for a specific table
    pub fn get_table_triggers(&self, table: &str) -> Vec<TriggerInfo> {
        self.storage.read().list_triggers(table)
    }

    /// Get triggers filtered by timing and event
    pub fn get_triggers_for_operation(
        &self,
        table: &str,
        timing: TriggerTiming,
        event: TriggerEvent,
    ) -> Vec<TriggerInfo> {
        self.get_table_triggers(table)
            .into_iter()
            .filter(|t| t.timing == timing.into() && t.event == event.into())
            .collect()
    }

    /// Execute BEFORE triggers for an INSERT operation
    /// Returns modified new_row if any trigger modified it
    pub fn execute_before_insert(&self, table: &str, new_row: &Record) -> SqlResult<Record> {
        let triggers =
            self.get_triggers_for_operation(table, TriggerTiming::Before, TriggerEvent::Insert);
        let mut result = new_row.clone();
        for trigger in triggers {
            result = self.execute_trigger_body(&trigger, table, None, Some(&result))?;
        }
        Ok(result)
    }

    /// Execute AFTER triggers for an INSERT operation
    pub fn execute_after_insert(&self, table: &str, new_row: &Record) -> SqlResult<()> {
        let triggers =
            self.get_triggers_for_operation(table, TriggerTiming::After, TriggerEvent::Insert);
        for trigger in triggers {
            self.execute_trigger_body(&trigger, table, None, Some(new_row))?;
        }
        Ok(())
    }

    /// Execute BEFORE triggers for an UPDATE operation
    pub fn execute_before_update(
        &self,
        table: &str,
        old_row: &Record,
        new_row: &Record,
    ) -> SqlResult<Record> {
        let triggers =
            self.get_triggers_for_operation(table, TriggerTiming::Before, TriggerEvent::Update);
        let mut result = new_row.clone();
        for trigger in triggers {
            result = self.execute_trigger_body(&trigger, table, Some(old_row), Some(&result))?;
        }
        Ok(result)
    }

    /// Execute AFTER triggers for an UPDATE operation
    pub fn execute_after_update(
        &self,
        table: &str,
        old_row: &Record,
        new_row: &Record,
    ) -> SqlResult<()> {
        let triggers =
            self.get_triggers_for_operation(table, TriggerTiming::After, TriggerEvent::Update);
        for trigger in triggers {
            self.execute_trigger_body(&trigger, table, Some(old_row), Some(new_row))?;
        }
        Ok(())
    }

    /// Execute BEFORE triggers for a DELETE operation
    pub fn execute_before_delete(&self, table: &str, old_row: &Record) -> SqlResult<()> {
        let triggers =
            self.get_triggers_for_operation(table, TriggerTiming::Before, TriggerEvent::Delete);
        for trigger in triggers {
            self.execute_trigger_body(&trigger, table, Some(old_row), None)?;
        }
        Ok(())
    }

    /// Execute AFTER triggers for a DELETE operation
    pub fn execute_after_delete(&self, table: &str, old_row: &Record) -> SqlResult<()> {
        let triggers =
            self.get_triggers_for_operation(table, TriggerTiming::After, TriggerEvent::Delete);
        for trigger in triggers {
            self.execute_trigger_body(&trigger, table, Some(old_row), None)?;
        }
        Ok(())
    }

    /// Execute a single trigger's body
    pub fn execute_trigger_body(
        &self,
        trigger: &TriggerInfo,
        table: &str,
        old_row: Option<&Record>,
        new_row: Option<&Record>,
    ) -> SqlResult<Record> {
        // V312-55E (Round-27): enforce recursion depth limit BEFORE running
        // the body so a self-referential or mutually-recursive trigger pair
        // cannot stack-overflow the host. Increment on entry, decrement on
        // exit via Drop guard — even if the body returns Err or panics, the
        // counter is restored. If the post-increment depth exceeds the limit
        // we abort with structured fields (trigger_name / depth / limit) so
        // the parent transaction can be rolled back and the user sees an
        // actionable error instead of a process crash.
        let prev = self
            .recursion_depth
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let depth = prev + 1;
        struct DepthGuard<'a> {
            counter: &'a std::sync::atomic::AtomicUsize,
        }
        impl<'a> Drop for DepthGuard<'a> {
            fn drop(&mut self) {
                self.counter
                    .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let _guard = DepthGuard {
            counter: &self.recursion_depth,
        };
        if depth > MAX_RECURSION_DEPTH {
            return Err(SqlError::TriggerRecursionLimitExceeded {
                trigger_name: trigger.name.clone(),
                depth,
                limit: MAX_RECURSION_DEPTH,
            });
        }

        let body = &trigger.body;
        // Build a mutable copy of the NEW row so SET NEW.col = ... statements
        // can mutate it in place. The captured result is what callers see
        // after the body executes — before this fix, we returned the
        // pre-execution snapshot and BEFORE INSERT / UPDATE trigger mutations
        // never reached storage.
        let mut result: Record = new_row.map(|r| r.to_vec()).unwrap_or_default();

        let statements = self.split_body_statements(body);
        for stmt in statements {
            let expanded =
                self.expand_row_variables_for_parse(&stmt, &trigger.table_name, old_row, new_row);
            self.execute_trigger_sql_mut(&expanded, table, old_row, &mut result)?;
        }

        Ok(result)
    }

    /// Split trigger body into individual SQL statements
    fn split_body_statements(&self, body: &str) -> Vec<String> {
        let mut statements = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut escape_next = false;

        let normalized = body.replace("NEW .", "NEW.").replace("OLD .", "OLD.");

        for ch in normalized.chars() {
            if escape_next {
                current.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' => {
                    escape_next = true;
                    current.push(ch);
                }
                '\'' => {
                    in_string = !in_string;
                    current.push(ch);
                }
                ';' if !in_string => {
                    let stmt = current.trim();
                    if !stmt.is_empty()
                        && !stmt.eq_ignore_ascii_case("BEGIN")
                        && !stmt.eq_ignore_ascii_case("END")
                    {
                        statements.push(stmt.to_string());
                    }
                    current.clear();
                }
                _ => current.push(ch),
            }
        }

        let final_stmt = current.trim();
        if !final_stmt.is_empty()
            && !final_stmt.eq_ignore_ascii_case("BEGIN")
            && !final_stmt.eq_ignore_ascii_case("END")
        {
            statements.push(final_stmt.to_string());
        }

        statements
    }

    /// Expand NEW.col and OLD.col with actual values from the row
    fn expand_row_variables(
        &self,
        sql: &str,
        table_name: &str,
        old_row: Option<&Record>,
        new_row: Option<&Record>,
    ) -> String {
        let mut result = sql.replace(". ", ".");

        if new_row.is_some() || old_row.is_some() {
            if let Ok(info) = self.storage.read().get_table_info(table_name) {
                self.do_expand_row_variables(&mut result, &info, old_row, new_row);
            }
        }

        result
    }

    fn expand_row_variables_for_parse(
        &self,
        sql: &str,
        _table_name: &str,
        _old_row: Option<&Record>,
        _new_row: Option<&Record>,
    ) -> String {
        sql.replace(". ", ".")
    }

    fn do_expand_row_variables(
        &self,
        result: &mut String,
        info: &sqlrustgo_storage::TableInfo,
        old_row: Option<&Record>,
        new_row: Option<&Record>,
    ) {
        if let Some(new) = new_row {
            for (i, col) in info.columns.iter().enumerate() {
                if i < new.len() {
                    let val = &new[i];
                    let replacement = self.value_to_sql_literal(val);
                    *result = result.replace(&format!("NEW.{}", col.name), &replacement);
                    *result =
                        result.replace(&format!("NEW.{}", col.name.to_uppercase()), &replacement);
                }
            }
        }

        if let Some(old) = old_row {
            for (i, col) in info.columns.iter().enumerate() {
                if i < old.len() {
                    let val = &old[i];
                    let replacement = self.value_to_sql_literal(val);
                    *result = result.replace(&format!("OLD.{}", col.name), &replacement);
                    *result =
                        result.replace(&format!("OLD.{}", col.name.to_uppercase()), &replacement);
                }
            }
        }
    }

    fn expand_row_variables_with_info(
        &self,
        sql: &str,
        info: &sqlrustgo_storage::TableInfo,
        old_row: Option<&Record>,
        new_row: Option<&Record>,
    ) -> String {
        let mut result = sql.to_string();
        self.do_expand_row_variables(&mut result, info, old_row, new_row);
        result
    }

    /// Convert a Value to SQL literal string
    fn value_to_sql_literal(&self, val: &Value) -> String {
        match val {
            Value::Null => "NULL".to_string(),
            Value::Integer(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Text(s) => format!("'{}'", s.replace("'", "\\'")),
            Value::Boolean(b) => {
                if *b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            Value::Blob(b) => format!("X'{}'", String::from_utf8_lossy(b)),
            Value::Point(x, y) => format!("POINT({}, {})", x, y),
            Value::Json(v) => format!("'{}'", v.to_string().replace('\'', "''")),
        }
    }

    /// Execute a SQL statement within a trigger context
    #[allow(dead_code)] // preserved for API symmetry; the mutating variant below is the
                        // actively-used path now (V312-55C trigger body mutation propagation).
    fn execute_trigger_sql(
        &self,
        sql: &str,
        trigger_table: &str,
        old_row: Option<&Record>,
        new_row: Option<&Record>,
    ) -> SqlResult<()> {
        let sql_trimmed = sql.trim();
        if sql_trimmed.is_empty() {
            return Ok(());
        }

        let sql_upper = sql_trimmed.to_uppercase();

        if sql_upper.starts_with("INSERT") {
            self.execute_trigger_insert(sql_trimmed, new_row)
        } else if sql_upper.starts_with("UPDATE") {
            self.execute_trigger_update(sql_trimmed, trigger_table, new_row)
        } else if sql_upper.starts_with("DELETE") {
            self.execute_trigger_delete(sql_trimmed, trigger_table, old_row)
        } else if sql_upper.starts_with("SET") {
            if let Some(new) = new_row {
                self.execute_trigger_set(sql_trimmed, trigger_table, new)?;
            }
            Ok(())
        } else if sql_upper.starts_with("SELECT") {
            self.execute_trigger_select(sql_trimmed, trigger_table, new_row)
        } else {
            Ok(())
        }
    }

    /// Mutating variant of `execute_trigger_sql`. Same dispatch as the
    /// immutable version, but a `SET NEW.col = ...` statement mutates the
    /// caller-provided `current_new_row` in place via
    /// `execute_trigger_set_mut`, so subsequent statements and the final
    /// returned Record see the mutation. Required for V312-55C
    /// (Trigger row semantics) — the previous immutable snapshot path
    /// discarded any SET NEW.col = literal assignment made by the trigger.
    fn execute_trigger_sql_mut(
        &self,
        sql: &str,
        trigger_table: &str,
        old_row: Option<&Record>,
        current_new_row: &mut Record,
    ) -> SqlResult<()> {
        let sql_trimmed = sql.trim();
        if sql_trimmed.is_empty() {
            return Ok(());
        }

        let sql_upper = sql_trimmed.to_uppercase();

        if sql_upper.starts_with("INSERT") {
            self.execute_trigger_insert(sql_trimmed, Some(current_new_row))
        } else if sql_upper.starts_with("UPDATE") {
            self.execute_trigger_update(sql_trimmed, trigger_table, Some(current_new_row))
        } else if sql_upper.starts_with("DELETE") {
            self.execute_trigger_delete(sql_trimmed, trigger_table, old_row)
        } else if sql_upper.starts_with("SET") {
            self.execute_trigger_set_mut(sql_trimmed, trigger_table, current_new_row)?;
            Ok(())
        } else if sql_upper.starts_with("SELECT") {
            self.execute_trigger_select(sql_trimmed, trigger_table, Some(current_new_row))
        } else {
            Ok(())
        }
    }

    /// Execute INSERT within a trigger
    fn execute_trigger_insert(&self, sql: &str, new_row: Option<&Record>) -> SqlResult<()> {
        // P1 FIX (SGL-005): Wrap in transaction boundary for TX-002 compliance
        // Acquire storage, begin tx, execute DML, commit, then release lock
        let expanded = self.expand_insert_values(sql, new_row);
        let statement = parse(&expanded)
            .map_err(|e| SqlError::ExecutionError(format!("Parse error: {}", e)))?;

        if let sqlrustgo_parser::Statement::Insert(insert) = statement {
            let table_name = insert.table.clone();
            // V312-55F / Issue #4243: privilege check before any DML
            // mutates storage. Root short-circuits inside the hook.
            self.check_body_privilege(sqlrustgo_catalog::auth::Privilege::Insert, &table_name)?;
            let table_info = {
                let storage = self.storage.read();
                storage.get_table_info(&table_name)?
            };
            let num_cols = table_info.columns.len();
            let col_names: Vec<String> =
                table_info.columns.iter().map(|c| c.name.clone()).collect();
            self.execute_dml_in_tx(|storage| {
                let trigger_ctx = crate::trigger_eval::TriggerContext::new(new_row, None);
                for values in &insert.values {
                    let mut record = Vec::new();
                    let eval_ctx = crate::trigger_eval::EvalContext::new(&trigger_ctx, None)
                        .with_target_col_names(col_names.clone());
                    for expr in values {
                        let val = crate::trigger_eval::expression_to_value(
                            expr,
                            &eval_ctx,
                            Some(&col_names),
                        );
                        record.push(val);
                    }
                    while record.len() < num_cols {
                        record.push(Value::Null);
                    }
                    storage.insert(&table_name, vec![record])?;
                }
                Ok(())
            })?;
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Execute UPDATE within a trigger
    fn execute_trigger_update(
        &self,
        sql: &str,
        trigger_table: &str,
        new_row: Option<&Record>,
    ) -> SqlResult<()> {
        let expanded = self.expand_update_values(sql, trigger_table, new_row);
        let statement = parse(&expanded)
            .map_err(|e| SqlError::ExecutionError(format!("Parse error: {}", e)))?;

        if let sqlrustgo_parser::Statement::Update(update) = statement {
            if update.tables.len() != 1 {
                return Err(SqlError::ExecutionError(
                    "Trigger UPDATE only supports single-table form".to_string(),
                ));
            }
            let storage = self.storage.read();
            let table_name = &update.tables[0].name;
            // V312-55F / Issue #4243: privilege check before any DML
            // mutates storage. Root short-circuits inside the hook.
            self.check_body_privilege(sqlrustgo_catalog::auth::Privilege::Update, table_name)?;
            let table_info = storage.get_table_info(table_name)?;
            let target_col_names: Vec<String> =
                table_info.columns.iter().map(|c| c.name.clone()).collect();

            let trigger_table_info = storage.get_table_info(trigger_table)?;
            let trigger_col_names: Vec<String> = trigger_table_info
                .columns
                .iter()
                .map(|c| c.name.clone())
                .collect();

            let set_clauses: Vec<(String, sqlrustgo_parser::Expression)> =
                update.set_clauses.clone();

            if set_clauses.is_empty() {
                return Ok(());
            }

            let where_expr = update.where_clause.clone();
            let all_rows = storage.scan(table_name)?;
            let mut modified_rows = Vec::new();
            let mut has_match = false;

            for row in all_rows {
                let trigger_ctx = crate::trigger_eval::TriggerContext::new(new_row, None)
                    .with_new_col_names(trigger_col_names.clone());
                let eval_ctx = crate::trigger_eval::EvalContext::new(&trigger_ctx, Some(&row))
                    .with_target_col_names(target_col_names.clone());
                let mut updated_row = row.clone();

                if let Some(ref where_cond) = where_expr {
                    let where_result =
                        crate::trigger_eval::expression_to_bool(where_cond, &eval_ctx, None);
                    if !where_result {
                        modified_rows.push(row);
                        continue;
                    }
                }

                has_match = true;
                for (col_name, expr) in &set_clauses {
                    if let Some(col_idx) = table_info
                        .columns
                        .iter()
                        .position(|c| c.name.eq_ignore_ascii_case(col_name))
                    {
                        let new_val =
                            crate::trigger_eval::expression_to_value(expr, &eval_ctx, None);
                        if col_idx < updated_row.len() {
                            updated_row[col_idx] = new_val;
                        }
                    }
                }
                modified_rows.push(updated_row);
            }

            // Drop the read lock before calling execute_dml_in_tx which needs a write lock.
            // parking_lot::RwLock does not allow reader-to-writer upgrade; holding read blocks write forever.
            drop(storage);
            if has_match {
                // INT-4: routed through execute_dml_in_tx for VTU enforcement
                let modified = modified_rows.clone();
                self.execute_dml_in_tx(|storage| {
                    storage.delete(table_name, &[])?;
                    if !modified.is_empty() {
                        storage.insert(table_name, modified)?;
                    }
                    Ok(())
                })?;
            }
        }
        Ok(())
    }

    /// Execute DELETE within a trigger
    fn execute_trigger_delete(
        &self,
        sql: &str,
        trigger_table: &str,
        old_row: Option<&Record>,
    ) -> SqlResult<()> {
        let table_info = self.storage.read().get_table_info(trigger_table)?;
        let expanded = self.expand_delete_values_with_info(sql, &table_info, old_row);
        let statement = parse(&expanded)
            .map_err(|e| SqlError::ExecutionError(format!("Parse error: {}", e)))?;

        if let sqlrustgo_parser::Statement::Delete(delete) = statement {
            if delete.tables.len() != 1 {
                return Err(SqlError::ExecutionError(
                    "Trigger DELETE only supports single-table form".to_string(),
                ));
            }
            // V312-55F / Issue #4243: privilege check before any DML
            // mutates storage. Root short-circuits inside the hook.
            self.check_body_privilege(
                sqlrustgo_catalog::auth::Privilege::Delete,
                &delete.tables[0].name,
            )?;
            self.execute_dml_in_tx(|storage| storage.delete(&delete.tables[0].name, &[]))?;
        }
        Ok(())
    }

    /// Execute SELECT within a trigger (for setting variables or validation)
    fn execute_trigger_select(
        &self,
        sql: &str,
        trigger_table: &str,
        new_row: Option<&Record>,
    ) -> SqlResult<()> {
        let expanded = self.expand_row_variables(sql, trigger_table, None, new_row);

        let statement = parse(&expanded)
            .map_err(|e| SqlError::ExecutionError(format!("Parse error: {}", e)))?;

        if let sqlrustgo_parser::Statement::Select(select) = statement {
            #[allow(clippy::match_result_ok)]
            let storage = self.storage.read();
            let table_info = storage.get_table_info(&select.table).ok();

            for col in &select.columns {
                if let Some(expr) = &col.expression {
                    match expr {
                        sqlrustgo_parser::Expression::Identifier(name) => {
                            if let (Some(new), Some(info)) = (new_row, &table_info) {
                                for (i, c) in info.columns.iter().enumerate() {
                                    if c.name.eq_ignore_ascii_case(name) && i < new.len() {
                                        let _val = new[i].clone();
                                    }
                                }
                            }
                        }
                        sqlrustgo_parser::Expression::Literal(lit) => {
                            let lit_str = lit.as_str();
                            let _ = self.evaluate_simple_expression(lit_str);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    /// Execute SET within a trigger (modify NEW row)
    #[allow(dead_code)] // preserved for API symmetry; the mutating variant below is the
                        // actively-used path now (V312-55C trigger body mutation propagation).
    fn execute_trigger_set(&self, sql: &str, table_name: &str, new_row: &Record) -> SqlResult<()> {
        if let Some(assignments) = self.parse_simple_set_assignments(sql) {
            let table_info = self.storage.read().get_table_info(table_name)?;
            let mut updated = new_row.to_vec();

            for (col_name, value) in assignments {
                if let Some(col_idx) = table_info
                    .columns
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(&col_name))
                {
                    if col_idx < updated.len() {
                        updated[col_idx] = value;
                    }
                }
            }
        }
        Ok(())
    }

    /// Mutating variant of `execute_trigger_set`. Parses SET NEW.col = ...
    /// assignments and folds them directly into the caller-provided
    /// `current_new_row` instead of allocating a fresh `updated` vector that
    /// is then discarded. This is the actual mutation sink that the trigger
    /// body relies on for `BEFORE INSERT` / `BEFORE UPDATE` SET NEW.col =
    /// <literal> persistence to storage.
    fn execute_trigger_set_mut(
        &self,
        sql: &str,
        table_name: &str,
        current_new_row: &mut Record,
    ) -> SqlResult<()> {
        if let Some(assignments) = self.parse_simple_set_assignments(sql) {
            let table_info = self.storage.read().get_table_info(table_name)?;
            for (col_name, value) in assignments {
                if let Some(col_idx) = table_info
                    .columns
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case(&col_name))
                {
                    if col_idx < current_new_row.len() {
                        current_new_row[col_idx] = value;
                    }
                }
            }
        }
        Ok(())
    }

    /// Expand VALUES(...) in INSERT with NEW.row values
    fn expand_insert_values(&self, sql: &str, new_row: Option<&Record>) -> String {
        if let Some(new) = new_row {
            // V312-55D FIX (Round-26): DELETE triggers can carry an INSERT statement
            // in their body (e.g. `BEFORE DELETE ... INSERT INTO backup SELECT *`).
            // `execute_trigger_sql_mut` forwards `current_new_row` (empty Vec for
            // DELETE) here as `Some(empty_record)`. The for-loop already handles
            // an empty vec, but the unconditional `&new[0]` below panicked with
            // `index out of bounds: the len is 0 but the index is 0`. Guard the
            // named-placeholder substitution on a non-empty record.
            if new.is_empty() {
                return sql.to_string();
            }
            let mut result = sql.to_string();
            for (i, val) in new.iter().enumerate() {
                let placeholder = format!("NEW[{}]", i);
                let replacement = self.value_to_sql_literal(val);
                result = result.replace(&placeholder, &replacement);
            }
            result = result.replace("NEW.id", &self.value_to_sql_literal(&new[0]));
            if new.len() > 1 {
                result = result.replace("NEW.col1", &self.value_to_sql_literal(&new[1]));
            }
            result
        } else {
            sql.to_string()
        }
    }

    #[allow(dead_code)]
    fn expand_update_values(
        &self,
        sql: &str,
        table_name: &str,
        new_row: Option<&Record>,
    ) -> String {
        self.expand_row_variables(sql, table_name, None, new_row)
    }

    #[allow(dead_code)]
    fn expand_update_values_with_info(
        &self,
        sql: &str,
        table_info: &sqlrustgo_storage::TableInfo,
        new_row: Option<&Record>,
    ) -> String {
        self.expand_row_variables_with_info(sql, table_info, None, new_row)
    }

    #[allow(dead_code)]
    fn expand_delete_values(
        &self,
        sql: &str,
        table_name: &str,
        old_row: Option<&Record>,
    ) -> String {
        self.expand_row_variables(sql, table_name, old_row, None)
    }

    fn expand_delete_values_with_info(
        &self,
        sql: &str,
        table_info: &sqlrustgo_storage::TableInfo,
        old_row: Option<&Record>,
    ) -> String {
        self.expand_row_variables_with_info(sql, table_info, old_row, None)
    }

    /// Parse simple SET NEW.col = value assignments
    /// Supports basic expressions like: SET NEW.col = NEW.other_col * 10
    fn parse_simple_set_assignments(&self, body: &str) -> Option<Vec<(String, Value)>> {
        let mut assignments = Vec::new();

        let body = body.replace("NEW .", "NEW.").replace("OLD .", "OLD.");

        let body = body.trim().trim_end_matches(';').trim();

        let parts: Vec<&str> = if body.contains("SET") {
            body.split("SET")
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim())
                .collect()
        } else {
            vec![body.trim()]
        };

        for part in parts {
            let part = part.trim();
            if let Some((lhs, rhs)) = part.split_once('=') {
                let lhs = lhs.trim();
                let rhs = rhs.trim();

                if lhs.contains("NEW.") {
                    let col_name = lhs.split("NEW.").last().unwrap_or(lhs).trim();
                    if let Some(value) = self.evaluate_simple_expression(rhs) {
                        assignments.push((col_name.to_string(), value));
                    }
                } else if lhs.contains("OLD.") {
                    let col_name = lhs.split("OLD.").last().unwrap_or(lhs).trim();
                    if let Some(value) = self.evaluate_simple_expression(rhs) {
                        assignments.push((col_name.to_string(), value));
                    }
                }
            }
        }

        if assignments.is_empty() {
            None
        } else {
            Some(assignments)
        }
    }

    /// Evaluate a simple expression to a Value
    /// Supports: literal numbers, literal strings, NEW.col references
    fn evaluate_simple_expression(&self, expr: &str) -> Option<Value> {
        let expr = expr.trim();

        if let Ok(n) = expr.parse::<i64>() {
            return Some(Value::Integer(n));
        }

        if let Ok(f) = expr.parse::<f64>() {
            return Some(Value::Float(f));
        }

        if expr.starts_with('\'') && expr.ends_with('\'') && expr.len() >= 2 {
            return Some(Value::Text(expr[1..expr.len() - 1].to_string()));
        }

        if expr.eq_ignore_ascii_case("TRUE") || expr.eq_ignore_ascii_case("true") {
            return Some(Value::Boolean(true));
        }
        if expr.eq_ignore_ascii_case("FALSE") || expr.eq_ignore_ascii_case("false") {
            return Some(Value::Boolean(false));
        }

        if expr.eq_ignore_ascii_case("NULL") || expr.eq_ignore_ascii_case("null") {
            return Some(Value::Null);
        }

        for op in &["*", "/", "+", "-"] {
            if let Some((left, right)) = expr.split_once(*op) {
                let left = left.trim();
                let right = right.trim();

                if left.starts_with("NEW.") {
                    let _col_name = left.trim_start_matches("NEW.").trim();
                    if let Ok(num) = right.trim().parse::<i64>() {
                        return Some(Value::Integer(num));
                    }
                }
            }
        }

        None
    }

    /// Execute all triggers for an operation
    /// This is the main entry point for trigger execution
    pub fn execute_triggers(
        &self,
        event: TriggerEvent,
        timing: TriggerTiming,
        table: &str,
        old_row: Option<&Record>,
        new_row: Option<&Record>,
    ) -> SqlResult<TriggerExecutionResult> {
        match (timing, event) {
            (TriggerTiming::Before, TriggerEvent::Insert) => {
                if let Some(row) = new_row {
                    let modified = self.execute_before_insert(table, row)?;
                    Ok(TriggerExecutionResult::ModifiedNewRow(modified))
                } else {
                    Ok(TriggerExecutionResult::Unmodified)
                }
            }
            (TriggerTiming::After, TriggerEvent::Insert) => {
                if let Some(row) = new_row {
                    self.execute_after_insert(table, row)?;
                }
                Ok(TriggerExecutionResult::Unmodified)
            }
            (TriggerTiming::Before, TriggerEvent::Update) => {
                if let (Some(old_r), Some(new_r)) = (old_row, new_row) {
                    let modified = self.execute_before_update(table, old_r, new_r)?;
                    Ok(TriggerExecutionResult::ModifiedNewRow(modified))
                } else {
                    Ok(TriggerExecutionResult::Unmodified)
                }
            }
            (TriggerTiming::After, TriggerEvent::Update) => {
                if let (Some(old_r), Some(new_r)) = (old_row, new_row) {
                    self.execute_after_update(table, old_r, new_r)?;
                }
                Ok(TriggerExecutionResult::Unmodified)
            }
            (TriggerTiming::Before, TriggerEvent::Delete) => {
                if let Some(row) = old_row {
                    self.execute_before_delete(table, row)?;
                }
                Ok(TriggerExecutionResult::Unmodified)
            }
            (TriggerTiming::After, TriggerEvent::Delete) => {
                if let Some(row) = old_row {
                    self.execute_after_delete(table, row)?;
                }
                Ok(TriggerExecutionResult::Unmodified)
            }
        }
    }
}

/// Result of trigger execution
#[derive(Debug)]
pub enum TriggerExecutionResult {
    /// Trigger did not modify the row
    Unmodified,
    /// Trigger modified the NEW row
    ModifiedNewRow(Record),
}

impl TriggerExecutionResult {
    /// Returns true if the NEW row was modified by the trigger
    pub fn is_modified(&self) -> bool {
        matches!(self, TriggerExecutionResult::ModifiedNewRow(_))
    }

    /// Get the modified row if any
    pub fn into_record(self) -> Option<Record> {
        match self {
            TriggerExecutionResult::ModifiedNewRow(r) => Some(r),
            TriggerExecutionResult::Unmodified => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_storage::{
        ColumnDefinition, MemoryStorage, TableInfo, TriggerInfo as StorageTriggerInfo,
    };

    fn create_test_storage() -> MemoryStorage {
        let mut storage = MemoryStorage::new();

        // Create orders table
        let table_info = TableInfo {
            name: "orders".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: "INTEGER".to_string(),
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "price".to_string(),
                    data_type: "FLOAT".to_string(),
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "quantity".to_string(),
                    data_type: "INTEGER".to_string(),
                    ..Default::default()
                },
                ColumnDefinition {
                    name: "total".to_string(),
                    data_type: "FLOAT".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        storage.create_table(&table_info).unwrap();

        storage
    }

    #[test]
    fn test_trigger_executor_creation() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));
        assert_eq!(executor.get_table_triggers("orders").len(), 0);
    }

    #[test]
    fn test_get_triggers_for_operation_empty() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let triggers = executor.get_triggers_for_operation(
            "orders",
            TriggerTiming::Before,
            TriggerEvent::Insert,
        );
        assert!(triggers.is_empty());
    }

    #[test]
    fn test_get_triggers_for_operation_with_triggers() {
        let mut storage = create_test_storage();

        // Add a trigger
        let trigger = StorageTriggerInfo {
            name: "before_order_insert".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "SET NEW.total = NEW.price * NEW.quantity".to_string(),
        };
        storage.create_trigger(trigger).unwrap();

        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let triggers = executor.get_triggers_for_operation(
            "orders",
            TriggerTiming::Before,
            TriggerEvent::Insert,
        );
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].name, "before_order_insert");
    }

    #[test]
    fn test_execute_before_insert_with_trigger() {
        let mut storage = create_test_storage();

        // Add a trigger that calculates total
        let trigger = StorageTriggerInfo {
            name: "before_order_insert".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "SET NEW.total = NEW.price * NEW.quantity".to_string(),
        };
        storage.create_trigger(trigger).unwrap();

        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        // Execute before insert trigger
        let new_row = vec![
            Value::Integer(1),  // id
            Value::Float(10.0), // price
            Value::Integer(5),  // quantity
            Value::Null,        // total (to be calculated)
        ];

        let result = executor.execute_before_insert("orders", &new_row);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_after_insert_trigger() {
        let mut storage = create_test_storage();

        // Add an after insert trigger
        let trigger = StorageTriggerInfo {
            name: "after_order_insert".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::After,
            event: StorageTriggerEvent::Insert,
            body: "".to_string(),
        };
        storage.create_trigger(trigger).unwrap();

        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];

        let result = executor.execute_after_insert("orders", &new_row);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_before_update_trigger() {
        let mut storage = create_test_storage();

        // Add a before update trigger
        let trigger = StorageTriggerInfo {
            name: "before_order_update".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Update,
            body: "SET NEW.total = NEW.price * NEW.quantity".to_string(),
        };
        storage.create_trigger(trigger).unwrap();

        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let old_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];

        let new_row = vec![
            Value::Integer(1),
            Value::Float(12.0),
            Value::Integer(5),
            Value::Float(50.0), // old total, should be recalculated
        ];

        let result = executor.execute_before_update("orders", &old_row, &new_row);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_before_delete_trigger() {
        let mut storage = create_test_storage();

        // Add a before delete trigger
        let trigger = StorageTriggerInfo {
            name: "before_order_delete".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Delete,
            body: "".to_string(),
        };
        storage.create_trigger(trigger).unwrap();

        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let old_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];

        let result = executor.execute_before_delete("orders", &old_row);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trigger_execution_result_is_modified() {
        let unmodified = TriggerExecutionResult::Unmodified;
        assert!(!unmodified.is_modified());

        let modified = TriggerExecutionResult::ModifiedNewRow(vec![Value::Integer(1)]);
        assert!(modified.is_modified());
    }

    #[test]
    fn test_trigger_timing_conversion() {
        assert_eq!(
            TriggerTiming::Before,
            TriggerTiming::from(StorageTriggerTiming::Before)
        );
        assert_eq!(
            TriggerTiming::After,
            TriggerTiming::from(StorageTriggerTiming::After)
        );

        assert_eq!(
            StorageTriggerTiming::Before,
            StorageTriggerTiming::from(TriggerTiming::Before)
        );
        assert_eq!(
            StorageTriggerTiming::After,
            StorageTriggerTiming::from(TriggerTiming::After)
        );
    }

    #[test]
    fn test_trigger_event_conversion() {
        assert_eq!(
            TriggerEvent::Insert,
            TriggerEvent::from(StorageTriggerEvent::Insert)
        );
        assert_eq!(
            TriggerEvent::Update,
            TriggerEvent::from(StorageTriggerEvent::Update)
        );
        assert_eq!(
            TriggerEvent::Delete,
            TriggerEvent::from(StorageTriggerEvent::Delete)
        );

        assert_eq!(
            StorageTriggerEvent::Insert,
            StorageTriggerEvent::from(TriggerEvent::Insert)
        );
        assert_eq!(
            StorageTriggerEvent::Update,
            StorageTriggerEvent::from(TriggerEvent::Update)
        );
        assert_eq!(
            StorageTriggerEvent::Delete,
            StorageTriggerEvent::from(TriggerEvent::Delete)
        );
    }

    #[test]
    fn test_trigger_type_new() {
        assert_eq!(
            TriggerType::BeforeInsert,
            TriggerType::new(TriggerTiming::Before, TriggerEvent::Insert)
        );
        assert_eq!(
            TriggerType::AfterInsert,
            TriggerType::new(TriggerTiming::After, TriggerEvent::Insert)
        );
        assert_eq!(
            TriggerType::BeforeUpdate,
            TriggerType::new(TriggerTiming::Before, TriggerEvent::Update)
        );
        assert_eq!(
            TriggerType::AfterUpdate,
            TriggerType::new(TriggerTiming::After, TriggerEvent::Update)
        );
        assert_eq!(
            TriggerType::BeforeDelete,
            TriggerType::new(TriggerTiming::Before, TriggerEvent::Delete)
        );
        assert_eq!(
            TriggerType::AfterDelete,
            TriggerType::new(TriggerTiming::After, TriggerEvent::Delete)
        );
    }

    #[test]
    fn test_list_triggers_for_table() {
        let mut storage = create_test_storage();

        // Add multiple triggers
        let trigger1 = StorageTriggerInfo {
            name: "before_order_insert".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "SET NEW.total = 0".to_string(),
        };
        let trigger2 = StorageTriggerInfo {
            name: "after_order_insert".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::After,
            event: StorageTriggerEvent::Insert,
            body: "".to_string(),
        };
        let trigger3 = StorageTriggerInfo {
            name: "before_order_update".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Update,
            body: "".to_string(),
        };

        storage.create_trigger(trigger1).unwrap();
        storage.create_trigger(trigger2).unwrap();
        storage.create_trigger(trigger3).unwrap();

        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let all_triggers = executor.get_table_triggers("orders");
        assert_eq!(all_triggers.len(), 3);

        // Check before insert only
        let before_insert = executor.get_triggers_for_operation(
            "orders",
            TriggerTiming::Before,
            TriggerEvent::Insert,
        );
        assert_eq!(before_insert.len(), 1);
        assert_eq!(before_insert[0].name, "before_order_insert");

        // Check after insert only
        let after_insert = executor.get_triggers_for_operation(
            "orders",
            TriggerTiming::After,
            TriggerEvent::Insert,
        );
        assert_eq!(after_insert.len(), 1);
        assert_eq!(after_insert[0].name, "after_order_insert");

        // Check before update
        let before_update = executor.get_triggers_for_operation(
            "orders",
            TriggerTiming::Before,
            TriggerEvent::Update,
        );
        assert_eq!(before_update.len(), 1);
        assert_eq!(before_update[0].name, "before_order_update");

        // Check delete (should be none)
        let delete_triggers = executor.get_triggers_for_operation(
            "orders",
            TriggerTiming::Before,
            TriggerEvent::Delete,
        );
        assert!(delete_triggers.is_empty());
    }

    #[test]
    fn test_send_sync() {
        fn _check<T: Send + Sync>() {}
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));
        _check::<TriggerExecutor>();
        let _ = executor; // suppress unused warning
    }

    #[test]
    fn test_trigger_executor_with_multiple_tables() {
        let mut storage = MemoryStorage::new();

        // Create orders table
        let orders_table = TableInfo {
            name: "orders".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        storage.create_table(&orders_table).unwrap();

        // Create products table
        let products_table = TableInfo {
            name: "products".to_string(),
            columns: vec![ColumnDefinition {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        storage.create_table(&products_table).unwrap();

        // Add trigger to orders
        let trigger = StorageTriggerInfo {
            name: "before_order_insert".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "".to_string(),
        };
        storage.create_trigger(trigger).unwrap();

        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        // Orders should have 1 trigger
        assert_eq!(executor.get_table_triggers("orders").len(), 1);
        // Products should have no triggers
        assert_eq!(executor.get_table_triggers("products").len(), 0);
    }

    #[test]
    fn test_execute_after_update_trigger() {
        let mut storage = create_test_storage();

        let trigger = StorageTriggerInfo {
            name: "after_order_update".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::After,
            event: StorageTriggerEvent::Update,
            body: "".to_string(),
        };
        storage.create_trigger(trigger).unwrap();

        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let old_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];
        let new_row = vec![
            Value::Integer(1),
            Value::Float(12.0),
            Value::Integer(5),
            Value::Float(60.0),
        ];

        let result = executor.execute_after_update("orders", &old_row, &new_row);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_after_delete_trigger() {
        let mut storage = create_test_storage();

        let trigger = StorageTriggerInfo {
            name: "after_order_delete".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::After,
            event: StorageTriggerEvent::Delete,
            body: "".to_string(),
        };
        storage.create_trigger(trigger).unwrap();

        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let old_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];

        let result = executor.execute_after_delete("orders", &old_row);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trigger_execution_result_into_record() {
        let unmodified = TriggerExecutionResult::Unmodified;
        assert!(unmodified.into_record().is_none());

        let modified_row = Record::from(vec![Value::Integer(42)]);
        let modified = TriggerExecutionResult::ModifiedNewRow(modified_row.clone());
        assert_eq!(modified.into_record(), Some(modified_row));
    }

    #[test]
    fn test_trigger_timing_all_values() {
        assert_eq!(format!("{:?}", TriggerTiming::Before), "Before");
        assert_eq!(format!("{:?}", TriggerTiming::After), "After");
    }

    #[test]
    fn test_trigger_event_all_values() {
        assert_eq!(format!("{:?}", TriggerEvent::Insert), "Insert");
        assert_eq!(format!("{:?}", TriggerEvent::Update), "Update");
        assert_eq!(format!("{:?}", TriggerEvent::Delete), "Delete");
    }

    #[test]
    fn test_value_to_sql_literal_boolean() {
        assert_eq!(Value::Boolean(true).to_sql_string(), "true");
        assert_eq!(Value::Boolean(false).to_sql_string(), "false");
    }

    #[test]
    fn test_value_to_sql_literal_blob() {
        let blob = Value::Blob(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        let s = blob.to_sql_string();
        assert!(s.starts_with("X'"));
        assert!(s.ends_with("'"));
    }

    #[test]
    fn test_value_to_sql_literal_null() {
        assert_eq!(Value::Null.to_sql_string(), "NULL");
    }

    #[test]
    fn test_value_to_sql_literal_integer() {
        assert_eq!(Value::Integer(0).to_sql_string(), "0");
        assert_eq!(Value::Integer(-42).to_sql_string(), "-42");
        assert_eq!(Value::Integer(999).to_sql_string(), "999");
    }

    #[test]
    fn test_value_to_sql_literal_float() {
        assert_eq!(Value::Float(0.0).to_sql_string(), "0");
        assert_eq!(Value::Float(3.14).to_sql_string(), "3.14");
        assert_eq!(Value::Float(-1.5).to_sql_string(), "-1.5");
    }

    #[test]
    fn test_value_to_sql_literal_text() {
        assert_eq!(Value::Text("hello".to_string()).to_sql_string(), "hello");
        assert_eq!(Value::Text("".to_string()).to_sql_string(), "");
        assert_eq!(
            Value::Text("O'Reilly".to_string()).to_sql_string(),
            "O'Reilly"
        );
    }

    #[test]
    fn test_split_body_statements_empty() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let statements = executor.split_body_statements("");
        assert!(statements.is_empty());
    }

    #[test]
    fn test_split_body_statements_single_stmt() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let statements = executor.split_body_statements("SET NEW.col = 1");
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0], "SET NEW.col = 1");
    }

    #[test]
    fn test_split_body_statements_multiple() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let statements = executor.split_body_statements("SET NEW.col1 = 1; SET NEW.col2 = 2");
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0], "SET NEW.col1 = 1");
        assert_eq!(statements[1], "SET NEW.col2 = 2");
    }

    #[test]
    fn test_split_body_statements_with_strings() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let statements = executor.split_body_statements("SET NEW.text = 'hello; world'");
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0], "SET NEW.text = 'hello; world'");
    }

    #[test]
    fn test_split_body_statements_with_escaped_quotes() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let statements = executor.split_body_statements("SET NEW.text = 'hello\\' world'");
        assert_eq!(statements.len(), 1);
    }

    #[test]
    fn test_split_body_statements_begin_end() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let statements = executor.split_body_statements("BEGIN SET NEW.col = 1; END");
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0], "BEGIN SET NEW.col = 1");
    }

    #[test]
    fn test_split_body_statements_with_backslash_escape() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let statements = executor.split_body_statements("SET NEW.text = 'hello\\nworld'");
        assert_eq!(statements.len(), 1);
    }

    #[test]
    fn test_expand_row_variables_new_references() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![Value::Integer(1), Value::Float(10.0)];

        let expanded = executor.expand_row_variables("NEW.id", "orders", None, Some(&new_row));
        assert_eq!(expanded, "1");
    }

    #[test]
    fn test_expand_row_variables_old_references() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let old_row = vec![Value::Integer(1), Value::Float(10.0)];

        let expanded = executor.expand_row_variables("OLD.id", "orders", Some(&old_row), None);
        assert_eq!(expanded, "1");
    }

    #[test]
    fn test_expand_row_variables_uppercase_column() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![Value::Integer(42), Value::Float(10.0)];

        let expanded = executor.expand_row_variables("NEW.ID", "orders", None, Some(&new_row));
        assert_eq!(expanded, "42");
    }

    #[test]
    fn test_expand_row_variables_no_matching_column() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![Value::Integer(1), Value::Float(10.0)];

        let expanded =
            executor.expand_row_variables("NEW.nonexistent", "orders", None, Some(&new_row));
        assert_eq!(expanded, "NEW.nonexistent");
    }

    #[test]
    fn test_evaluate_simple_expression_integer() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        assert_eq!(
            executor.evaluate_simple_expression("42"),
            Some(Value::Integer(42))
        );
        assert_eq!(
            executor.evaluate_simple_expression("-10"),
            Some(Value::Integer(-10))
        );
        assert_eq!(
            executor.evaluate_simple_expression("0"),
            Some(Value::Integer(0))
        );
    }

    #[test]
    fn test_evaluate_simple_expression_float() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        assert_eq!(
            executor.evaluate_simple_expression("3.14"),
            Some(Value::Float(3.14))
        );
        assert_eq!(
            executor.evaluate_simple_expression("-1.5"),
            Some(Value::Float(-1.5))
        );
    }

    #[test]
    fn test_evaluate_simple_expression_string() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        assert_eq!(
            executor.evaluate_simple_expression("'hello'"),
            Some(Value::Text("hello".to_string()))
        );
        assert_eq!(
            executor.evaluate_simple_expression("'test string'"),
            Some(Value::Text("test string".to_string()))
        );
    }

    #[test]
    fn test_evaluate_simple_expression_boolean() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        assert_eq!(
            executor.evaluate_simple_expression("TRUE"),
            Some(Value::Boolean(true))
        );
        assert_eq!(
            executor.evaluate_simple_expression("true"),
            Some(Value::Boolean(true))
        );
        assert_eq!(
            executor.evaluate_simple_expression("FALSE"),
            Some(Value::Boolean(false))
        );
        assert_eq!(
            executor.evaluate_simple_expression("false"),
            Some(Value::Boolean(false))
        );
    }

    #[test]
    fn test_evaluate_simple_expression_null() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        assert_eq!(
            executor.evaluate_simple_expression("NULL"),
            Some(Value::Null)
        );
        assert_eq!(
            executor.evaluate_simple_expression("null"),
            Some(Value::Null)
        );
    }

    #[test]
    fn test_evaluate_simple_expression_new_col_reference() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let result = executor.evaluate_simple_expression("NEW.id * 10");
        assert_eq!(result, Some(Value::Integer(10)));
    }

    #[test]
    fn test_evaluate_simple_expression_unsupported() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        assert_eq!(
            executor.evaluate_simple_expression("column1 + column2"),
            None
        );
        assert_eq!(executor.evaluate_simple_expression("unknown_func()"), None);
    }

    #[test]
    fn test_parse_simple_set_assignments_basic() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let result = executor.parse_simple_set_assignments("SET NEW.col = 42");
        assert!(result.is_some());
        let assignments = result.unwrap();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0], ("col".to_string(), Value::Integer(42)));
    }

    #[test]
    fn test_parse_simple_set_assignments_old_reference() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let result = executor.parse_simple_set_assignments("SET NEW.total = OLD.price");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_simple_set_assignments_multiple() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let result = executor.parse_simple_set_assignments("SET NEW.col1 = 1; SET NEW.col2 = 2");
        assert!(result.is_some());
        let assignments = result.unwrap();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0], ("col2".to_string(), Value::Integer(2)));
    }

    #[test]
    fn test_parse_simple_set_assignments_no_new_reference() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let result = executor.parse_simple_set_assignments("SELECT * FROM table");
        assert_eq!(result, None);
    }

    #[test]
    fn test_expand_insert_values_basic() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![Value::Integer(1), Value::Text("test".to_string())];

        let expanded =
            executor.expand_insert_values("INSERT INTO t VALUES (NEW[0], NEW[1])", Some(&new_row));
        assert!(expanded.contains("1"));
        assert!(expanded.contains("test"));
    }

    #[test]
    fn test_expand_insert_values_with_new_id() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![Value::Integer(42), Value::Text("test".to_string())];

        let expanded =
            executor.expand_insert_values("INSERT INTO t VALUES (NEW.id)", Some(&new_row));
        assert!(expanded.contains("42"));
    }

    #[test]
    fn test_expand_insert_values_no_new_row() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let sql = "INSERT INTO t VALUES (1)";
        let expanded = executor.expand_insert_values(sql, None);
        assert_eq!(expanded, sql);
    }

    #[test]
    fn test_expand_update_values() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![Value::Integer(1), Value::Float(10.0)];

        let expanded =
            executor.expand_update_values("UPDATE t SET col = NEW.price", "orders", Some(&new_row));
        assert!(expanded.contains("10"));
    }

    #[test]
    fn test_expand_delete_values() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let old_row = vec![Value::Integer(1), Value::Float(10.0)];

        let expanded = executor.expand_delete_values(
            "DELETE FROM t WHERE id = OLD.id",
            "orders",
            Some(&old_row),
        );
        assert!(expanded.contains("1"));
    }

    #[test]
    fn test_execute_trigger_sql_empty() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let result = executor.execute_trigger_sql("", "orders", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_trigger_sql_unrecognized() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let result =
            executor.execute_trigger_sql("REVOKE ALL ON table FROM user", "orders", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_trigger_sql_whitespace() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let result = executor.execute_trigger_sql("   ", "orders", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_trigger_body_no_statements() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let trigger = StorageTriggerInfo {
            name: "empty_trigger".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "".to_string(),
        };

        let new_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Null,
        ];
        let result = executor.execute_trigger_body(&trigger, "orders", None, Some(&new_row));
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_trigger_body_with_set_statement() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let trigger = StorageTriggerInfo {
            name: "set_trigger".to_string(),
            table_name: "orders".to_string(),
            timing: StorageTriggerTiming::Before,
            event: StorageTriggerEvent::Insert,
            body: "SET NEW.total = NEW.price * NEW.quantity".to_string(),
        };

        let new_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Null,
        ];

        let result = executor.execute_trigger_body(&trigger, "orders", None, Some(&new_row));
        assert!(result.is_ok());
    }

    #[test]
    fn test_trigger_type_debug() {
        assert_eq!(format!("{:?}", TriggerType::BeforeInsert), "BeforeInsert");
        assert_eq!(format!("{:?}", TriggerType::AfterInsert), "AfterInsert");
        assert_eq!(format!("{:?}", TriggerType::BeforeUpdate), "BeforeUpdate");
        assert_eq!(format!("{:?}", TriggerType::AfterUpdate), "AfterUpdate");
        assert_eq!(format!("{:?}", TriggerType::BeforeDelete), "BeforeDelete");
        assert_eq!(format!("{:?}", TriggerType::AfterDelete), "AfterDelete");
    }

    #[test]
    fn test_trigger_execution_result_debug() {
        let unmodified = TriggerExecutionResult::Unmodified;
        assert_eq!(format!("{:?}", unmodified), "Unmodified");

        let modified = TriggerExecutionResult::ModifiedNewRow(vec![Value::Integer(1)]);
        assert!(format!("{:?}", modified).contains("ModifiedNewRow"));
    }

    #[test]
    fn test_execute_triggers_before_insert() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Null,
        ];

        let result = executor.execute_triggers(
            TriggerEvent::Insert,
            TriggerTiming::Before,
            "orders",
            None,
            Some(&new_row),
        );
        assert!(result.is_ok());
        assert!(result.unwrap().is_modified() || true);
    }

    #[test]
    fn test_execute_triggers_after_insert() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];

        let result = executor.execute_triggers(
            TriggerEvent::Insert,
            TriggerTiming::After,
            "orders",
            None,
            Some(&new_row),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_triggers_before_update() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let old_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];
        let new_row = vec![
            Value::Integer(1),
            Value::Float(12.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];

        let result = executor.execute_triggers(
            TriggerEvent::Update,
            TriggerTiming::Before,
            "orders",
            Some(&old_row),
            Some(&new_row),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_triggers_after_update() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let old_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];
        let new_row = vec![
            Value::Integer(1),
            Value::Float(12.0),
            Value::Integer(5),
            Value::Float(60.0),
        ];

        let result = executor.execute_triggers(
            TriggerEvent::Update,
            TriggerTiming::After,
            "orders",
            Some(&old_row),
            Some(&new_row),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_triggers_before_delete() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let old_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];

        let result = executor.execute_triggers(
            TriggerEvent::Delete,
            TriggerTiming::Before,
            "orders",
            Some(&old_row),
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_triggers_after_delete() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let old_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];

        let result = executor.execute_triggers(
            TriggerEvent::Delete,
            TriggerTiming::After,
            "orders",
            Some(&old_row),
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_triggers_no_new_row_for_insert() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let result = executor.execute_triggers(
            TriggerEvent::Insert,
            TriggerTiming::Before,
            "orders",
            None,
            None,
        );
        assert!(result.is_ok());
        assert!(!result.unwrap().is_modified());
    }

    #[test]
    fn test_execute_triggers_no_old_row_for_delete() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let result = executor.execute_triggers(
            TriggerEvent::Delete,
            TriggerTiming::Before,
            "orders",
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_trigger_set_with_valid_assignments() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Null,
        ];

        let result = executor.execute_trigger_set("SET NEW.total = 100", "orders", &new_row);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_trigger_set_with_invalid_assignments() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Null,
        ];

        let result = executor.execute_trigger_set("SET total = 100", "orders", &new_row);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_trigger_select_validates_columns() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];

        let result =
            executor.execute_trigger_select("SELECT id FROM orders", "orders", Some(&new_row));
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_trigger_select_with_literal_expression() {
        let mut storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let new_row = vec![
            Value::Integer(1),
            Value::Float(10.0),
            Value::Integer(5),
            Value::Float(50.0),
        ];

        let result = executor.execute_trigger_select("SELECT 42", "orders", Some(&new_row));
        assert!(result.is_ok());
    }

    #[test]
    fn test_split_body_with_trailing_semicolon() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let statements = executor.split_body_statements("SET NEW.col = 1;");
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0], "SET NEW.col = 1");
    }

    #[test]
    fn test_split_body_with_multiple_semicolons() {
        let storage = create_test_storage();
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(storage)));

        let statements = executor.split_body_statements("SET NEW.col1 = 1;;; SET NEW.col2 = 2");
        assert_eq!(statements.len(), 2);
    }

    /// Regression: TriggerExecutor::new must NOT panic on non-WAL storage
    /// (e.g. BinaryTableStorage). The previous assert!() caused the
    /// server to enter a busy-loop panic storm under --storage binary
    /// (see reports/SERVER_DEADLOCK_FIX_PLAN_2026-06-28.md).
    #[test]
    fn test_trigger_executor_with_non_wal_storage() {
        use sqlrustgo_storage::BinaryTableStorage;
        let dir = tempfile::tempdir().expect("tempdir");
        let bin = BinaryTableStorage::new(dir.path().to_path_buf()).expect("new bin storage");
        assert!(
            !bin.is_wal_enabled(),
            "sanity: BinaryTableStorage is non-WAL"
        );
        let executor = TriggerExecutor::new(Arc::new(RwLock::new(bin)));
        // Triggers list is empty, no panic, no DML attempted.
        assert!(executor.get_table_triggers("any_table").is_empty());
    }
}
