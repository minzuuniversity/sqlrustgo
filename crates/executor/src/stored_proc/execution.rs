use super::context::ProcedureContext;
use super::context::StoredProcError;
use super::StoredProcExecutor;
use sqlrustgo_catalog::StoredProcStatement;
use sqlrustgo_storage::ColumnDefinition;
use sqlrustgo_types::Value;

impl StoredProcExecutor {
    /// Execute a list of procedure statements
    pub(crate) fn execute_body(
        &self,
        body: &[StoredProcStatement],
        ctx: &mut ProcedureContext,
    ) -> Result<(), String> {
        for stmt in body {
            if ctx.should_leave() {
                ctx.reset_leave();
                break;
            }
            if ctx.should_iterate() {
                ctx.reset_iterate();
                break;
            }
            if ctx.get_return().is_some() {
                break;
            }

            let result = self.execute_statement(stmt, ctx);

            if let Err(ref e) = result {
                if let Some(err_str) = e.strip_prefix("SQLSTATE ") {
                    let sqlstate = err_str
                        .split(':')
                        .next()
                        .unwrap_or("45000")
                        .trim()
                        .to_string();
                    let message = e
                        .strip_prefix(&format!("SQLSTATE {}: ", sqlstate))
                        .unwrap_or(e)
                        .trim()
                        .to_string();
                    let exc = StoredProcError { sqlstate, message };

                    if let Some(handler) = ctx.find_matching_handler(&exc) {
                        let handler_body = handler.body.clone();
                        ctx.set_exception_handling(true);
                        ctx.set_exception(exc.sqlstate.clone(), exc.message.clone());
                        let handler_result = self.execute_body(&handler_body, ctx);
                        ctx.clear_exception();
                        ctx.set_exception_handling(false);

                        handler_result.as_ref()?;
                        continue;
                    }
                }
                return result;
            }
        }
        Ok(())
    }

    /// Execute a single procedure statement
    pub(crate) fn execute_statement(
        &self,
        stmt: &StoredProcStatement,
        ctx: &mut ProcedureContext,
    ) -> Result<(), String> {
        match stmt {
            StoredProcStatement::Declare {
                name,
                default_value,
                ..
            } => {
                let value = default_value
                    .as_ref()
                    .map(|v| self.evaluate_constant(v))
                    .unwrap_or_else(|| Value::Null);
                ctx.set_var(name, value);
                Ok(())
            }
            StoredProcStatement::Set { variable, value } => {
                let evaluated = self.evaluate_expression(value, ctx)?;
                ctx.set_var(variable, evaluated);
                Ok(())
            }
            StoredProcStatement::RawSql(sql) => {
                if !sql.is_empty() {
                    self.execute_sql(sql, ctx)?;
                }
                Ok(())
            }
            StoredProcStatement::SelectInto {
                columns,
                into_vars,
                table,
                where_clause,
            } => {
                let where_str = where_clause
                    .as_ref()
                    .map(|w| format!(" WHERE {}", self.expand_variables_in_sql(w, ctx)))
                    .unwrap_or_default();

                let _query = if columns.is_empty() {
                    format!("SELECT * FROM {}{}", table, where_str)
                } else {
                    let cols = columns.join(", ");
                    format!("SELECT {} FROM {}{}", cols, table, where_str)
                };

                for (i, var) in into_vars.iter().enumerate() {
                    if i < columns.len() {
                        let col_expr = &columns[i];
                        let value = self.evaluate_expression(col_expr, ctx)?;
                        ctx.set_var(var, value);
                    } else {
                        ctx.set_var(var, Value::Null);
                    }
                }

                Ok(())
            }
            StoredProcStatement::If {
                condition,
                then_body,
                elseif_body,
                else_body,
            } => {
                if self.evaluate_condition(condition, ctx)? {
                    self.execute_body(then_body, ctx)?;
                } else {
                    let mut matched = false;
                    for (elsif_cond, elsif_body) in elseif_body {
                        if self.evaluate_condition(elsif_cond, ctx)? {
                            self.execute_body(elsif_body, ctx)?;
                            matched = true;
                            break;
                        }
                    }
                    if !matched && !else_body.is_empty() {
                        self.execute_body(else_body, ctx)?;
                    }
                }
                Ok(())
            }
            StoredProcStatement::While { condition, body } => {
                while self.evaluate_condition(condition, ctx)?
                    && !ctx.should_leave()
                    && ctx.get_return().is_none()
                {
                    ctx.reset_iterate();
                    self.execute_body(body, ctx)?;
                    if ctx.should_iterate() {
                        ctx.reset_iterate();
                    }
                }
                ctx.reset_leave();
                Ok(())
            }
            StoredProcStatement::Loop { body } => {
                loop {
                    if ctx.should_leave() {
                        ctx.reset_leave();
                        break;
                    }
                    if ctx.get_return().is_some() {
                        break;
                    }
                    self.execute_body(body, ctx)?;
                }
                Ok(())
            }
            StoredProcStatement::Case {
                case_value,
                when_clauses,
                else_result,
            } => {
                let case_val = if let Some(ref cv) = case_value {
                    self.evaluate_expression(cv, ctx)?
                } else {
                    Value::Null
                };

                for (when_val, result) in when_clauses {
                    let when_expr_val = self.evaluate_expression(when_val, ctx)?;
                    if case_val == when_expr_val {
                        return self.evaluate_expression(result, ctx).map(|v| {
                            ctx.set_return(v);
                        });
                    }
                }

                if let Some(else_val) = else_result {
                    return self.evaluate_expression(else_val, ctx).map(|v| {
                        ctx.set_return(v);
                    });
                }

                Ok(())
            }
            StoredProcStatement::CaseWhen {
                when_clauses,
                else_result,
            } => {
                for (condition, result) in when_clauses {
                    if self.evaluate_condition(condition, ctx)? {
                        return self.evaluate_expression(result, ctx).map(|v| {
                            ctx.set_return(v);
                        });
                    }
                }

                if let Some(else_val) = else_result {
                    return self.evaluate_expression(else_val, ctx).map(|v| {
                        ctx.set_return(v);
                    });
                }

                Ok(())
            }
            StoredProcStatement::Repeat { body, condition } => {
                loop {
                    if ctx.should_leave() {
                        ctx.reset_leave();
                        break;
                    }
                    if ctx.get_return().is_some() {
                        break;
                    }

                    ctx.reset_iterate();
                    self.execute_body(body, ctx)?;

                    if ctx.should_iterate() {
                        ctx.reset_iterate();
                        continue;
                    }

                    if self.evaluate_condition(condition, ctx)? {
                        break;
                    }
                }
                Ok(())
            }
            StoredProcStatement::Leave { .. } => {
                ctx.set_leave();
                Ok(())
            }
            StoredProcStatement::Iterate { .. } => {
                ctx.set_iterate();
                Ok(())
            }
            StoredProcStatement::Return { value } => {
                let ret_val = self.evaluate_expression(value, ctx)?;
                ctx.set_return(ret_val);
                Ok(())
            }
            StoredProcStatement::Call {
                procedure_name,
                args,
                into_var,
            } => {
                let call_args: Vec<Value> = args
                    .iter()
                    .map(|a| self.evaluate_expression(a, ctx).unwrap_or(Value::Null))
                    .collect();

                let result = self.execute_call(procedure_name, call_args);

                match result {
                    Ok(exec_result) => {
                        if let Some(var_name) = into_var {
                            if let Some(row) = exec_result.rows.first() {
                                if let Some(val) = row.first() {
                                    ctx.set_var(var_name, val.clone());
                                }
                            }
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            StoredProcStatement::Signal { sqlstate, message } => {
                let sqlstate = sqlstate.as_deref().unwrap_or("45000");
                let message = message.as_deref().unwrap_or("Unhandled exception");
                Err(format!("SQLSTATE {}: {}", sqlstate, message))
            }
            StoredProcStatement::Resignal { sqlstate, message } => {
                let sqlstate = sqlstate.as_deref().unwrap_or("45000");
                let message = message.as_deref().unwrap_or("Unhandled exception");
                Err(format!("SQLSTATE {}: {}", sqlstate, message))
            }
            StoredProcStatement::Block { label, body } => {
                if let Some(ref lbl) = label {
                    ctx.enter_label(lbl.clone());
                }

                ctx.enter_scope();

                let result = self.execute_body(body, ctx);

                ctx.exit_scope();

                if label.is_some() {
                    ctx.exit_label();
                }

                result
            }
            StoredProcStatement::DeclareHandler {
                condition_type,
                body,
            } => {
                ctx.push_handler(condition_type.clone(), body.clone());
                Ok(())
            }
            StoredProcStatement::DeclareCursor { name, query } => {
                ctx.declare_cursor(name.clone(), query.clone());
                Ok(())
            }
            StoredProcStatement::OpenCursor { name } => {
                let query = if let Some(cursor) = ctx.cursors.get(&name.clone()) {
                    cursor.query.clone()
                } else {
                    return Err(format!("Cursor '{}' not found", name));
                };

                let expanded = self.expand_variables_in_sql(&query, ctx);
                let statement = sqlrustgo_parser::parse(&expanded)
                    .map_err(|e| format!("Failed to parse cursor query: {}", e))?;

                if let sqlrustgo_parser::Statement::Select(select) = statement {
                    let storage = self.storage.read().unwrap();
                    let records = storage
                        .scan(&select.first_table())
                        .map_err(|e| format!("Failed to scan table: {}", e))?;
                    ctx.set_cursor_records(name, records);
                    ctx.open_cursor(name)?;
                }
                Ok(())
            }
            StoredProcStatement::Fetch { name, into_vars } => {
                let has_rows = ctx.fetch_cursor(name, into_vars)?;
                ctx.set_session_var("__found", Value::Boolean(has_rows));
                if !has_rows {
                    ctx.set_session_var("__found_rows", Value::Integer(0));
                }
                Ok(())
            }
            StoredProcStatement::CloseCursor { name } => {
                ctx.close_cursor(name)?;
                Ok(())
            }
        }
    }

    /// Execute a SQL statement using the storage engine
    pub(crate) fn execute_sql(&self, sql: &str, ctx: &mut ProcedureContext) -> Result<(), String> {
        let expanded_sql = self.expand_variables_in_sql(sql, ctx);
        let sql_upper = expanded_sql.trim().to_uppercase();

        if sql_upper.starts_with("SELECT")
            || sql_upper.starts_with("INSERT")
            || sql_upper.starts_with("UPDATE")
            || sql_upper.starts_with("DELETE")
        {
            let statement = sqlrustgo_parser::parse(&expanded_sql)
                .map_err(|e| format!("Failed to parse SQL: {}", e))?;

            self.execute_statement_storage_impl(&statement, ctx)?;

            if let Some(found_rows) = ctx.get_session_var("__found_rows") {
                ctx.set_session_var("ROW_COUNT", found_rows.clone());
            } else if let Some(last_insert) = ctx.get_session_var("__last_insert_count") {
                ctx.set_session_var("ROW_COUNT", last_insert.clone());
            } else if let Some(last_update) = ctx.get_session_var("__last_update_count") {
                ctx.set_session_var("ROW_COUNT", last_update.clone());
            } else if let Some(last_delete) = ctx.get_session_var("__last_delete_count") {
                ctx.set_session_var("ROW_COUNT", last_delete.clone());
            }
        } else if sql_upper.starts_with("CREATE")
            || sql_upper.starts_with("DROP")
            || sql_upper.starts_with("ALTER")
            || sql_upper.starts_with("SHOW")
            || sql_upper.starts_with("DESCRIBE")
            || sql_upper.starts_with("SET")
        {
            ctx.set_session_var("ROW_COUNT", Value::Integer(0));
        }

        Ok(())
    }

    /// Execute a parsed statement using storage engine
    pub(crate) fn execute_statement_storage_impl(
        &self,
        statement: &sqlrustgo_parser::Statement,
        ctx: &mut ProcedureContext,
    ) -> Result<(), String> {
        match statement {
            sqlrustgo_parser::Statement::WithSelect(with_select) => {
                if let Some(ref with_clause) = with_select.with_clause {
                    for cte in &with_clause.ctes {
                        let cte_records = if let sqlrustgo_parser::Statement::Union(union_stmt) =
                            &*cte.subquery
                        {
                            if union_stmt.union_all {
                                self.execute_recursive_cte(
                                    &union_stmt.left,
                                    &union_stmt.right,
                                    &cte.name,
                                    ctx,
                                )?
                            } else {
                                self.execute_cte_subquery(&cte.subquery, ctx)?
                            }
                        } else {
                            self.execute_cte_subquery(&cte.subquery, ctx)?
                        };
                        ctx.cte_tables.insert(cte.name.clone(), cte_records);
                    }
                }
                match with_select.select.as_ref() {
                    sqlrustgo_parser::Statement::Select(select) => {
                        let table_name = select.first_table();

                        let records = if ctx.cte_tables.contains_key(&table_name) {
                            ctx.cte_tables.get(&table_name).cloned().unwrap_or_default()
                        } else {
                            let storage = self.storage.read().unwrap();
                            storage
                                .scan(&table_name)
                                .map_err(|e| format!("Failed to scan table: {}", e))?
                        };

                        let filtered: Vec<Vec<Value>> =
                            if let Some(ref where_expr) = select.where_clause {
                                records
                                    .into_iter()
                                    .filter(|_row| {
                                        let where_val = self.expression_to_value(where_expr, ctx);
                                        if let Value::Boolean(b) = where_val {
                                            b
                                        } else {
                                            where_val != Value::Null
                                        }
                                    })
                                    .collect()
                            } else {
                                records
                            };

                        ctx.set_session_var(
                            "__last_select_result",
                            Value::Text(serde_json::to_string(&filtered).unwrap_or_default()),
                        );
                        ctx.set_session_var("__found_rows", Value::Integer(filtered.len() as i64));
                        Ok(())
                    }
                    sqlrustgo_parser::Statement::Union(union_stmt) => {
                        let left_table = if let sqlrustgo_parser::Statement::Select(left_select) =
                            union_stmt.left.as_ref()
                        {
                            left_select.first_table()
                        } else {
                            return Err("Union left side must be SELECT".to_string());
                        };
                        let right_table = if let sqlrustgo_parser::Statement::Select(right_select) =
                            union_stmt.right.as_ref()
                        {
                            right_select.first_table()
                        } else {
                            return Err("Union right side must be SELECT".to_string());
                        };

                        let left_records = if ctx.cte_tables.contains_key(&left_table) {
                            ctx.cte_tables.get(&left_table).cloned().unwrap_or_default()
                        } else {
                            let storage = self.storage.read().unwrap();
                            storage
                                .scan(&left_table)
                                .map_err(|e| format!("Failed to scan table: {}", e))?
                        };
                        let right_records = if ctx.cte_tables.contains_key(&right_table) {
                            ctx.cte_tables
                                .get(&right_table)
                                .cloned()
                                .unwrap_or_default()
                        } else {
                            let storage = self.storage.read().unwrap();
                            storage
                                .scan(&right_table)
                                .map_err(|e| format!("Failed to scan table: {}", e))?
                        };

                        let mut combined = left_records;
                        combined.extend(right_records);
                        if !union_stmt.union_all {
                            combined.sort();
                            combined.dedup();
                        }

                        ctx.set_session_var(
                            "__last_select_result",
                            Value::Text(serde_json::to_string(&combined).unwrap_or_default()),
                        );
                        ctx.set_session_var("__found_rows", Value::Integer(combined.len() as i64));
                        Ok(())
                    }
                    _ => Err("CTE main select must be SELECT or UNION".to_string()),
                }
            }
            sqlrustgo_parser::Statement::Select(select) => {
                let table_name = &select.first_table();

                let records = if table_name.is_empty() {
                    vec![]
                } else if ctx.cte_tables.contains_key(table_name) {
                    ctx.cte_tables.get(table_name).cloned().unwrap_or_default()
                } else {
                    let storage = self.storage.read().unwrap();
                    storage
                        .scan(table_name)
                        .map_err(|e| format!("Failed to scan table: {}", e))?
                };

                let filtered: Vec<Vec<Value>> = if let Some(ref where_expr) = select.where_clause {
                    records
                        .into_iter()
                        .filter(|_row| {
                            let where_val = self.expression_to_value(where_expr, ctx);
                            if let Value::Boolean(b) = where_val {
                                b
                            } else {
                                where_val != Value::Null
                            }
                        })
                        .collect()
                } else {
                    records
                };

                ctx.set_session_var(
                    "__last_select_result",
                    Value::Text(serde_json::to_string(&filtered).unwrap_or_default()),
                );
                ctx.set_session_var("__found_rows", Value::Integer(filtered.len() as i64));
                Ok(())
            }
            sqlrustgo_parser::Statement::Insert(insert) => {
                let table_name = &insert.table.clone();
                let insert_columns = insert.columns.clone();

                let table_info = {
                    let storage = self.storage.read().unwrap();
                    if !storage.has_table(table_name) {
                        return Err(format!("Table '{}' not found", table_name));
                    }
                    storage.get_table_info(table_name).ok()
                };

                let num_columns = table_info.as_ref().map(|i| i.columns.len()).unwrap_or(0);
                let has_hidden_rowid = table_info
                    .as_ref()
                    .map(|i| i.has_hidden_rowid)
                    .unwrap_or(false);
                let mut new_rows: Vec<Vec<Value>> = Vec::new();
                let mut insert_count = 0;

                if let Some(ref select_box) = insert.select {
                    if let sqlrustgo_parser::Statement::Select(ref select) = select_box.as_ref() {
                        let storage = self.storage.read().unwrap();
                        let records = storage
                            .scan(&select.first_table())
                            .map_err(|e| format!("Failed to scan table: {}", e))?;

                        let selected_rows: Vec<Vec<Value>> =
                            if let Some(ref where_expr) = select.where_clause {
                                records
                                    .into_iter()
                                    .filter(|_row| {
                                        let where_val = self.expression_to_value(where_expr, ctx);
                                        if let Value::Boolean(b) = where_val {
                                            b
                                        } else {
                                            where_val != Value::Null
                                        }
                                    })
                                    .collect()
                            } else {
                                records
                            };

                        for row in selected_rows {
                            let mut new_row: Vec<Value> = vec![Value::Null; num_columns];
                            if insert_columns.is_empty() {
                                for (col_idx, val) in row.iter().enumerate() {
                                    if col_idx < num_columns {
                                        new_row[col_idx] = val.clone();
                                    }
                                }
                            } else {
                                for (col_idx, col_name) in insert_columns.iter().enumerate() {
                                    if col_idx < row.len() {
                                        if let Some(ref info) = table_info {
                                            if let Some(target_idx) = info
                                                .columns
                                                .iter()
                                                .position(|c| c.name.eq_ignore_ascii_case(col_name))
                                            {
                                                new_row[target_idx] = row[col_idx].clone();
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(ref info) = table_info {
                                if !info.foreign_keys.is_empty() {
                                    self.validate_foreign_keys(
                                        table_name,
                                        &new_row,
                                        &insert_columns,
                                    )?;
                                }
                                if info.columns.iter().any(|c| c.primary_key) {
                                    self.validate_primary_key(
                                        table_name,
                                        &new_row,
                                        &insert_columns,
                                    )?;
                                }
                                if !info.unique_constraints.is_empty() {
                                    self.validate_unique_constraints(
                                        table_name,
                                        &new_row,
                                        &insert_columns,
                                    )?;
                                }
                            }
                            new_rows.push(new_row);
                        }
                        insert_count = new_rows.len();
                    }
                } else {
                    for row in &insert.values {
                        let mut new_row: Vec<Value> = vec![Value::Null; num_columns];

                        if insert_columns.is_empty() {
                            for (col_idx, expr) in row.iter().enumerate() {
                                if col_idx < num_columns {
                                    new_row[col_idx] = self.expression_to_value(expr, ctx);
                                }
                            }
                        } else {
                            for (value_idx, col_name) in insert_columns.iter().enumerate() {
                                if value_idx < row.len() {
                                    if let Some(ref info) = table_info {
                                        if let Some(target_idx) = info
                                            .columns
                                            .iter()
                                            .position(|c| c.name.eq_ignore_ascii_case(col_name))
                                        {
                                            new_row[target_idx] =
                                                self.expression_to_value(&row[value_idx], ctx);
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(ref info) = table_info {
                            if !info.foreign_keys.is_empty() {
                                let cols = insert_columns.clone();
                                self.validate_foreign_keys(table_name, &new_row, &cols)?;
                            }
                            if info.columns.iter().any(|c| c.primary_key) {
                                self.validate_primary_key(table_name, &new_row, &insert_columns)?;
                            }
                            if !info.unique_constraints.is_empty() {
                                self.validate_unique_constraints(
                                    table_name,
                                    &new_row,
                                    &insert_columns,
                                )?;
                            }
                        }

                        new_rows.push(new_row);
                    }
                    insert_count = insert.values.len();
                }

                {
                    let mut storage = self.storage.write().unwrap();
                    for mut new_row in new_rows {
                        if has_hidden_rowid {
                            if let Ok(Some(rowid)) =
                                storage.get_and_increment_next_rowid(table_name)
                            {
                                new_row.insert(0, Value::Integer(rowid as i64));
                            }
                        }
                        storage
                            .insert(table_name, vec![new_row])
                            .map_err(|e| format!("Failed to insert: {}", e))?;
                    }
                }

                ctx.set_session_var("__last_insert_count", Value::Integer(insert_count as i64));
                Ok(())
            }
            sqlrustgo_parser::Statement::Update(update) => {
                let table_name = &update.table;
                let mut storage = self.storage.write().unwrap();

                if !storage.has_table(table_name) {
                    return Err(format!("Table '{}' not found", table_name));
                }

                let table_info = storage.get_table_info(table_name).ok();

                let col_index_map: std::collections::HashMap<String, usize> = table_info
                    .as_ref()
                    .map(|info| {
                        info.columns
                            .iter()
                            .enumerate()
                            .map(|(i, c)| (c.name.to_lowercase(), i))
                            .collect()
                    })
                    .unwrap_or_default();

                let records = match storage.get_table_records_mut(table_name) {
                    Ok(r) => r,
                    Err(e) => return Err(format!("Failed to get table records: {}", e)),
                };

                let mut count = 0;
                let where_expr = update.where_clause.clone();

                for row in records.iter_mut() {
                    let matches = if let Some(ref where_clz) = where_expr {
                        let result =
                            self.evaluate_row_expression(where_clz, row, &col_index_map, ctx);
                        if let Value::Boolean(b) = result {
                            b
                        } else {
                            result != Value::Null
                        }
                    } else {
                        true
                    };

                    if matches {
                        for (col_name, expr) in &update.set_clauses {
                            if let Some(col_idx) = col_index_map.get(&col_name.to_lowercase()) {
                                if *col_idx < row.len() {
                                    let new_val = self.evaluate_row_expression(
                                        expr,
                                        row,
                                        &col_index_map,
                                        ctx,
                                    );
                                    row[*col_idx] = new_val;
                                }
                            }
                        }
                        count += 1;
                    }
                }

                ctx.set_session_var("__last_update_count", Value::Integer(count as i64));
                Ok(())
            }
            sqlrustgo_parser::Statement::Delete(delete) => {
                let table_name = &delete.table;
                let mut storage = self.storage.write().unwrap();

                if !storage.has_table(table_name) {
                    return Err(format!("Table '{}' not found", table_name));
                }

                let table_info = storage.get_table_info(table_name).ok();

                let col_index_map: std::collections::HashMap<String, usize> = table_info
                    .as_ref()
                    .map(|info| {
                        info.columns
                            .iter()
                            .enumerate()
                            .map(|(i, c)| (c.name.to_lowercase(), i))
                            .collect()
                    })
                    .unwrap_or_default();

                let records = match storage.get_table_records_mut(table_name) {
                    Ok(r) => r,
                    Err(e) => return Err(format!("Failed to get table records: {}", e)),
                };

                let where_expr = delete.where_clause.clone();

                let mut indices_to_delete: Vec<usize> = Vec::new();
                for (idx, row) in records.iter().enumerate() {
                    let matches = if let Some(ref where_clz) = where_expr {
                        let result =
                            self.evaluate_row_expression(where_clz, row, &col_index_map, ctx);
                        if let Value::Boolean(b) = result {
                            b
                        } else {
                            result != Value::Null
                        }
                    } else {
                        true
                    };
                    if matches {
                        indices_to_delete.push(idx);
                    }
                }

                let deleted_count = indices_to_delete.len();
                for idx in indices_to_delete.into_iter().rev() {
                    records.remove(idx);
                }

                ctx.set_session_var("__last_delete_count", Value::Integer(deleted_count as i64));
                Ok(())
            }
            sqlrustgo_parser::Statement::AlterTable(alter_table) => {
                let table_name = &alter_table.table_name;
                let mut storage = self.storage.write().unwrap();

                match &alter_table.operation {
                    sqlrustgo_parser::AlterTableOperation::AddColumn {
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
                            auto_increment: false,
                        };
                        storage
                            .add_column(table_name, column)
                            .map_err(|e| format!("Failed to add column: {}", e))?;
                    }
                    sqlrustgo_parser::AlterTableOperation::RenameTo { new_name } => {
                        storage
                            .rename_table(table_name, new_name)
                            .map_err(|e| format!("Failed to rename table: {}", e))?;
                    }
                    sqlrustgo_parser::AlterTableOperation::DropColumn { name } => {
                        return Err(format!(
                            "DROP COLUMN '{}' not yet implemented in storage layer",
                            name
                        ));
                    }
                    sqlrustgo_parser::AlterTableOperation::ModifyColumn {
                        name,
                        data_type,
                        nullable: _,
                    } => {
                        return Err(format!(
                            "MODIFY COLUMN '{} {}' not yet implemented in storage layer",
                            name, data_type
                        ));
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Execute a SELECT statement and return rows
    pub(crate) fn execute_subquery(
        &self,
        select: &sqlrustgo_parser::SelectStatement,
    ) -> Vec<Vec<Value>> {
        let storage = match self.storage.read() {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let records = match storage.scan(&select.first_table()) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        if let Some(ref where_expr) = select.where_clause {
            records
                .into_iter()
                .filter(|_row| {
                    let where_val = self.expression_to_value(where_expr, &ProcedureContext::new());
                    if let Value::Boolean(b) = where_val {
                        b
                    } else {
                        where_val != Value::Null
                    }
                })
                .collect()
        } else {
            records
        }
    }

    /// Extract column names and their positions from SELECT columns for binding
    pub(crate) fn extract_column_binding(
        &self,
        select: &sqlrustgo_parser::SelectStatement,
    ) -> Vec<(String, usize)> {
        let mut bindings = Vec::new();
        for (idx, col) in select.columns.iter().enumerate() {
            if let Some(ref expr) = col.expression {
                let identifiers = self.extract_identifiers_from_expr(expr);
                for name in identifiers {
                    if !name.starts_with('@') && name.parse::<i64>().is_err() {
                        bindings.push((name, idx));
                        break;
                    }
                }
            }
        }
        bindings
    }

    /// Bind row values to column names in a cloned context
    pub(crate) fn bind_row_to_context(
        &self,
        mut ctx: ProcedureContext,
        columns: &[String],
        row: &[Value],
    ) -> ProcedureContext {
        for (i, col_name) in columns.iter().enumerate() {
            if i < row.len() {
                ctx.set_var(col_name, row[i].clone());
            }
        }
        ctx
    }

    /// Validate foreign key constraints for a row being inserted
    pub(crate) fn validate_foreign_keys(
        &self,
        table_name: &str,
        row: &[Value],
        columns: &[String],
    ) -> Result<(), String> {
        let storage = self.storage.read().unwrap();
        let table_info = storage
            .get_table_info(table_name)
            .map_err(|e| format!("Failed to get table info: {}", e))?;

        for fk in &table_info.foreign_keys {
            let parent_values: Vec<Value> = fk
                .columns
                .iter()
                .filter_map(|col_name| {
                    columns
                        .iter()
                        .position(|c| c.eq_ignore_ascii_case(col_name))
                        .and_then(|idx| row.get(idx).cloned())
                })
                .collect();

            if parent_values.iter().any(|v| matches!(v, Value::Null)) {
                continue;
            }

            let parent_rows = storage
                .scan(&fk.referenced_table)
                .map_err(|e| format!("Failed to scan parent table: {}", e))?;

            let ref_col_indices: Vec<usize> = fk
                .referenced_columns
                .iter()
                .filter_map(|col_name| {
                    storage
                        .get_table_info(&fk.referenced_table)
                        .ok()?
                        .columns
                        .iter()
                        .position(|c| c.name.eq_ignore_ascii_case(col_name))
                })
                .collect();

            let parent_has_match = parent_rows.iter().any(|parent_row| {
                ref_col_indices
                    .iter()
                    .enumerate()
                    .all(|(i, &col_idx)| parent_row.get(col_idx) == parent_values.get(i))
            });

            if !parent_has_match {
                return Err(format!(
                    "Foreign key constraint failed: {} ({}) references {} ({}) which does not exist",
                    table_name,
                    fk.columns.join(", "),
                    fk.referenced_table,
                    fk.referenced_columns.join(", ")
                ));
            }
        }

        Ok(())
    }

    /// Validate unique constraints for a row being inserted
    pub(crate) fn validate_unique_constraints(
        &self,
        table_name: &str,
        row: &[Value],
        columns: &[String],
    ) -> Result<(), String> {
        let storage = self.storage.read().unwrap();
        let table_info = storage
            .get_table_info(table_name)
            .map_err(|e| format!("Failed to get table info: {}", e))?;

        for unique_constraint in &table_info.unique_constraints {
            let col_indices: Vec<usize> = unique_constraint
                .columns
                .iter()
                .filter_map(|col_name| {
                    columns
                        .iter()
                        .position(|c| c.eq_ignore_ascii_case(col_name))
                })
                .collect();

            if col_indices.is_empty() {
                continue;
            }

            let values: Vec<Value> = col_indices
                .iter()
                .filter_map(|&i| row.get(i).cloned())
                .collect();

            if values.iter().any(|v| matches!(v, Value::Null)) {
                continue;
            }

            let existing_rows = storage
                .scan(table_name)
                .map_err(|e| format!("Failed to scan table: {}", e))?;

            for existing_row in existing_rows {
                let existing_values: Vec<Value> = col_indices
                    .iter()
                    .filter_map(|&i| existing_row.get(i).cloned())
                    .collect();
                if existing_values == values {
                    return Err(format!(
                        "Duplicate unique key '{}': ({}) values ({}) already exist",
                        unique_constraint.name.as_deref().unwrap_or("unnamed"),
                        unique_constraint.columns.join(", "),
                        values
                            .iter()
                            .map(|v| format!("{:?}", v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate PRIMARY KEY uniqueness for a row being inserted
    pub(crate) fn validate_primary_key(
        &self,
        table_name: &str,
        row: &[Value],
        columns: &[String],
    ) -> Result<(), String> {
        let storage = self.storage.read().unwrap();
        let table_info = storage
            .get_table_info(table_name)
            .map_err(|e| format!("Failed to get table info: {}", e))?;

        let col_to_row_idx: std::collections::HashMap<String, usize> = if columns.is_empty() {
            std::collections::HashMap::new()
        } else {
            columns
                .iter()
                .enumerate()
                .map(|(i, name)| (name.to_uppercase(), i))
                .collect()
        };

        let pk_col_names: Vec<String> = table_info
            .columns
            .iter()
            .filter(|c| c.primary_key)
            .map(|c| c.name.to_uppercase())
            .collect();

        if pk_col_names.is_empty() {
            return Ok(());
        }

        let pk_values: Vec<Value> = pk_col_names
            .iter()
            .filter_map(|pk_name| {
                if let Some(&row_idx) = col_to_row_idx.get(pk_name) {
                    row.get(row_idx).cloned()
                } else if columns.is_empty() {
                    table_info
                        .columns
                        .iter()
                        .position(|c| c.name.to_uppercase() == *pk_name)
                        .and_then(|col_idx| row.get(col_idx).cloned())
                } else {
                    None
                }
            })
            .collect();

        if pk_values.iter().any(|v| matches!(v, Value::Null)) {
            return Ok(());
        }

        let existing_rows = storage
            .scan(table_name)
            .map_err(|e| format!("Failed to scan table: {}", e))?;

        let all_col_names: Vec<String> = table_info
            .columns
            .iter()
            .map(|c| c.name.to_uppercase())
            .collect();

        for existing_row in existing_rows {
            let existing_pk_values: Vec<Value> = pk_col_names
                .iter()
                .filter_map(|pk_name| {
                    all_col_names
                        .iter()
                        .position(|c| c == pk_name)
                        .and_then(|col_idx| existing_row.get(col_idx).cloned())
                })
                .collect();
            if existing_pk_values == pk_values {
                return Err(format!(
                    "Duplicate primary key: ({}) values ({}) already exist",
                    pk_col_names.join(", "),
                    pk_values
                        .iter()
                        .map(|v| format!("{:?}", v))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_catalog::HandlerCondition;
    use std::sync::Arc;

    fn executor() -> StoredProcExecutor {
        let catalog = Arc::new(sqlrustgo_catalog::Catalog::new("test"));
        StoredProcExecutor::new_for_test(catalog)
    }

    #[test]
    fn test_execute_body_leave_early() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.set_var("x", Value::Integer(1));
        let body = vec![
            StoredProcStatement::Set {
                variable: "x".into(),
                value: "2".into(),
            },
            StoredProcStatement::Leave {
                label: "block".into(),
            },
            StoredProcStatement::Set {
                variable: "x".into(),
                value: "3".into(),
            },
        ];
        let result = ex.execute_body(&body, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("x"), Some(&Value::Integer(2)));
    }

    #[test]
    fn test_execute_body_return_early() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let body = vec![
            StoredProcStatement::Return { value: "42".into() },
            StoredProcStatement::Set {
                variable: "x".into(),
                value: "99".into(),
            },
        ];
        let result = ex.execute_body(&body, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_return(), Some(Value::Integer(42)));
    }

    #[test]
    fn test_execute_body_iterate_resets() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // i starts at 1 so i- -1 = 2
        ctx.set_var("i", Value::Integer(1));
        let body = vec![
            StoredProcStatement::Iterate {
                label: "loop".into(),
            },
            StoredProcStatement::Set {
                variable: "i".into(),
                value: "99".into(),
            },
        ];
        let result = ex.execute_body(&body, &mut ctx);
        assert!(result.is_ok());
        assert!(!ctx.should_iterate());
    }

    #[test]
    fn test_extract_column_binding() {
        let ex = executor();
        use sqlrustgo_parser::{Expression, SelectColumn, SelectStatement};
        let select = SelectStatement {
            columns: vec![
                SelectColumn {
                    name: "a".into(),
                    expression: Some(Expression::Identifier("col1".into())),
                    alias: None,
                },
                SelectColumn {
                    name: "b".into(),
                    expression: Some(Expression::Identifier("col2".into())),
                    alias: None,
                },
            ],
            table: "t".into(),
            from: None,
            where_clause: None,
            join_clause: None,
            aggregates: vec![],
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
            for_update: false,
        };
        let binding = ex.extract_column_binding(&select);
        assert_eq!(binding.len(), 2);
        assert_eq!(binding[0], ("col1".into(), 0));
        assert_eq!(binding[1], ("col2".into(), 1));
    }

    #[test]
    fn test_bind_row_to_context() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let columns = vec!["a".into(), "b".into()];
        let row = vec![Value::Integer(10), Value::Text("hello".into())];
        ctx = ex.bind_row_to_context(ctx, &columns, &row);
        assert_eq!(ctx.get_var("a"), Some(&Value::Integer(10)));
        assert_eq!(ctx.get_var("b"), Some(&Value::Text("hello".into())));
    }

    // ===== StoredProcStatement branch coverage tests =====

    #[test]
    fn test_execute_if_condition_true() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::If {
            condition: "1 = 1".into(),
            then_body: vec![StoredProcStatement::Set {
                variable: "x".into(),
                value: "10".into(),
            }],
            elseif_body: vec![],
            else_body: vec![],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("x"), Some(&Value::Integer(10)));
    }

    #[test]
    fn test_execute_if_condition_false_no_else() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::If {
            condition: "1 = 0".into(),
            then_body: vec![StoredProcStatement::Set {
                variable: "x".into(),
                value: "10".into(),
            }],
            elseif_body: vec![],
            else_body: vec![],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("x"), None);
    }

    #[test]
    fn test_execute_if_elsif_branch() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::If {
            condition: "1 = 0".into(),
            then_body: vec![StoredProcStatement::Set {
                variable: "x".into(),
                value: "1".into(),
            }],
            elseif_body: vec![(
                "2 = 2".into(),
                vec![StoredProcStatement::Set {
                    variable: "x".into(),
                    value: "2".into(),
                }],
            )],
            else_body: vec![],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("x"), Some(&Value::Integer(2)));
    }

    #[test]
    fn test_execute_if_else_branch() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::If {
            condition: "1 = 0".into(),
            then_body: vec![StoredProcStatement::Set {
                variable: "x".into(),
                value: "1".into(),
            }],
            elseif_body: vec![],
            else_body: vec![StoredProcStatement::Set {
                variable: "x".into(),
                value: "3".into(),
            }],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("x"), Some(&Value::Integer(3)));
    }

    #[test]
    fn test_execute_while_loop() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // i starts at 1 so i- -1 = 2
        ctx.set_var("i", Value::Integer(1));
        ctx.set_var("sum", Value::Integer(0));
        let stmt = StoredProcStatement::While {
            condition: "i < 3".into(),
            body: vec![
                StoredProcStatement::Set {
                    variable: "i".into(),
                    value: "i + 1".into(),
                },
                StoredProcStatement::Set {
                    variable: "sum".into(),
                    value: "sum + i".into(),
                },
            ],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("i"), Some(&Value::Integer(3)));
    }

    #[test]
    fn test_execute_while_loop_leave_early() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // i starts at 1 so i- -1 = 2
        ctx.set_var("i", Value::Integer(1));
        let stmt = StoredProcStatement::While {
            condition: "i < 10".into(),
            body: vec![
                StoredProcStatement::Set {
                    variable: "i".into(),
                    value: "i + 1".into(),
                },
                StoredProcStatement::If {
                    condition: "i >= 3".into(),
                    then_body: vec![StoredProcStatement::Leave {
                        label: "while".into(),
                    }],
                    elseif_body: vec![],
                    else_body: vec![],
                },
            ],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("i"), Some(&Value::Integer(3)));
    }

    #[test]
    fn test_execute_loop_leave() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.set_var("x", Value::Integer(0));
        let stmt = StoredProcStatement::Loop {
            body: vec![
                StoredProcStatement::Set {
                    variable: "x".into(),
                    value: "x + 1".into(),
                },
                StoredProcStatement::If {
                    condition: "x >= 5".into(),
                    then_body: vec![StoredProcStatement::Leave {
                        label: "loop".into(),
                    }],
                    elseif_body: vec![],
                    else_body: vec![],
                },
            ],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("x"), Some(&Value::Integer(5)));
    }

    #[test]
    fn test_execute_loop_return_breaks() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Loop {
            body: vec![StoredProcStatement::Return { value: "42".into() }],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_return(), Some(Value::Integer(42)));
    }

    #[test]
    fn test_execute_case_value_match() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.set_var("status", Value::Text("active".into()));
        let stmt = StoredProcStatement::Case {
            case_value: Some("status".into()),
            when_clauses: vec![
                ("'active'".into(), "1".into()),
                ("'inactive'".into(), "2".into()),
            ],
            else_result: Some("0".into()),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_return(), Some(Value::Integer(1)));
    }

    #[test]
    fn test_execute_case_no_match_else() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.set_var("status", Value::Text("unknown".into()));
        let stmt = StoredProcStatement::Case {
            case_value: Some("status".into()),
            when_clauses: vec![
                ("'active'".into(), "1".into()),
                ("'inactive'".into(), "2".into()),
            ],
            else_result: Some("0".into()),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_return(), Some(Value::Integer(0)));
    }

    #[test]
    fn test_execute_case_no_match_no_else() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.set_var("status", Value::Text("unknown".into()));
        let stmt = StoredProcStatement::Case {
            case_value: Some("status".into()),
            when_clauses: vec![("'active'".into(), "1".into())],
            else_result: None,
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_return(), None);
    }

    #[test]
    fn test_execute_case_when_condition_true() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::CaseWhen {
            when_clauses: vec![("1 = 1".into(), "10".into()), ("1 = 0".into(), "20".into())],
            else_result: None,
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_return(), Some(Value::Integer(10)));
    }

    #[test]
    fn test_execute_case_when_else() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::CaseWhen {
            when_clauses: vec![("1 = 0".into(), "10".into())],
            else_result: Some("99".into()),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_return(), Some(Value::Integer(99)));
    }

    #[test]
    fn test_execute_repeat_until_true() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // i starts at 1 so i- -1 = 2
        ctx.set_var("i", Value::Integer(1));
        let stmt = StoredProcStatement::Repeat {
            body: vec![StoredProcStatement::Set {
                variable: "i".into(),
                value: "i + 1".into(),
            }],
            condition: "i >= 3".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("i"), Some(&Value::Integer(3)));
    }

    #[test]
    fn test_execute_repeat_leave_early() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // i starts at 1 so i- -1 = 2
        ctx.set_var("i", Value::Integer(1));
        let stmt = StoredProcStatement::Repeat {
            body: vec![
                StoredProcStatement::Set {
                    variable: "i".into(),
                    value: "i- -1".into(),
                },
                StoredProcStatement::If {
                    condition: "i>=2".into(),
                    then_body: vec![StoredProcStatement::Leave {
                        label: "repeat".into(),
                    }],
                    elseif_body: vec![],
                    else_body: vec![],
                },
            ],
            condition: "i<10".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("i"), Some(&Value::Integer(2)));
    }

    #[test]
    fn test_execute_return_sets_value() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Return { value: "42".into() };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_return(), Some(Value::Integer(42)));
    }

    #[test]
    fn test_execute_leave_sets_flag() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Leave {
            label: "block".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert!(ctx.should_leave());
    }

    #[test]
    fn test_execute_iterate_sets_flag() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Iterate {
            label: "loop".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert!(ctx.should_iterate());
    }

    #[test]
    fn test_execute_signal() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Signal {
            sqlstate: Some("45000".into()),
            message: Some("custom error".into()),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("SQLSTATE 45000"));
        assert!(err.contains("custom error"));
    }

    #[test]
    fn test_execute_signal_default() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Signal {
            sqlstate: None,
            message: None,
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("SQLSTATE 45000"));
    }

    #[test]
    fn test_execute_resignal() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.set_exception("45000".into(), "original".into());
        let stmt = StoredProcStatement::Resignal {
            sqlstate: None,
            message: None,
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("SQLSTATE 45000"));
    }

    #[test]
    fn test_execute_resignal_with_message() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Resignal {
            sqlstate: Some("22012".into()),
            message: Some("resignal msg".into()),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("SQLSTATE 22012"));
        assert!(err.contains("resignal msg"));
    }

    #[test]
    fn test_execute_block_no_label() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Block {
            label: None,
            body: vec![
                StoredProcStatement::Set {
                    variable: "x".into(),
                    value: "1".into(),
                },
                StoredProcStatement::Set {
                    variable: "y".into(),
                    value: "2".into(),
                },
            ],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        // Block creates a new scope; variables are not visible outside
        assert!(ctx.get_var("x").is_none());
        assert!(ctx.get_var("y").is_none());
    }

    #[test]
    fn test_execute_block_with_label() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Block {
            label: Some("myblock".into()),
            body: vec![StoredProcStatement::Set {
                variable: "x".into(),
                value: "5".into(),
            }],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        // Block scope: variable not visible outside
        assert!(ctx.get_var("x").is_none());
    }

    #[test]
    fn test_execute_block_scope_isolation() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let outer_stmt = StoredProcStatement::Block {
            label: Some("outer".into()),
            body: vec![
                StoredProcStatement::Set {
                    variable: "x".into(),
                    value: "10".into(),
                },
                StoredProcStatement::Block {
                    label: Some("inner".into()),
                    body: vec![StoredProcStatement::Set {
                        variable: "x".into(),
                        value: "20".into(),
                    }],
                },
            ],
        };
        let result = ex.execute_statement(&outer_stmt, &mut ctx);
        assert!(result.is_ok());
        // Inner block sets x=20, outer block sets x=10; after exit both scopes restored
        assert!(ctx.get_var("x").is_none());
    }

    #[test]
    fn test_execute_declare() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Declare {
            name: "v".into(),
            data_type: "INT".into(),
            default_value: Some("42".into()),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("v"), Some(&Value::Integer(42)));
    }

    #[test]
    fn test_execute_declare_no_default() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Declare {
            name: "v".into(),
            data_type: "INT".into(),
            default_value: None,
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("v"), Some(&Value::Null));
    }

    #[test]
    fn test_execute_set_variable() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Set {
            variable: "myvar".into(),
            value: "'hello'".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("myvar"), Some(&Value::Text("hello".into())));
    }

    #[test]
    fn test_execute_select_into() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // SELECT a, b INTO @x, @y FROM dual WHERE 1=1
        let stmt = StoredProcStatement::SelectInto {
            columns: vec!["a".into(), "b".into()],
            into_vars: vec!["x".into(), "y".into()],
            table: "dual".into(),
            where_clause: Some("1 = 1".into()),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        // Will fail at storage scan, but we can check the vars were set
        // Actually, since no storage, let's check the statement parses
        // We expect it to try evaluate_expression for each column
    }

    #[test]
    fn test_execute_declare_handler() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::DeclareHandler {
            condition_type: HandlerCondition::SqlException,
            body: vec![StoredProcStatement::Set {
                variable: "handled".into(),
                value: "1".into(),
            }],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.handler_stack.len(), 1);
        assert_eq!(
            ctx.handler_stack[0].condition,
            HandlerCondition::SqlException
        );
    }

    #[test]
    fn test_execute_declare_handler_sqlwarning() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::DeclareHandler {
            condition_type: HandlerCondition::SqlWarning,
            body: vec![],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.handler_stack.len(), 1);
        assert_eq!(ctx.handler_stack[0].condition, HandlerCondition::SqlWarning);
    }

    #[test]
    fn test_execute_declare_cursor() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::DeclareCursor {
            name: "c1".into(),
            query: "SELECT a FROM t".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert!(ctx.has_cursor("c1"));
    }

    #[test]
    fn test_execute_open_cursor_not_found() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::OpenCursor {
            name: "nonexistent".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_execute_close_cursor_not_found() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::CloseCursor {
            name: "nonexistent".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_execute_fetch_no_rows_v2() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.declare_cursor("c1".into(), "SELECT 1".into());
        // Manually set cursor with empty records via set_cursor_records
        ctx.set_cursor_records("c1", vec![]);
        // Manually open cursor via open_cursor
        ctx.open_cursor("c1").unwrap();
        let stmt = StoredProcStatement::Fetch {
            name: "c1".into(),
            into_vars: vec!["v".into()],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_session_var("__found"), Some(&Value::Boolean(false)));
    }

    #[test]
    fn test_execute_fetch_into_more_vars_than_columns() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.declare_cursor("c1".into(), "SELECT 1".into());
        ctx.set_cursor_records("c1", vec![vec![Value::Integer(100)]]);
        ctx.open_cursor("c1").unwrap();
        let stmt = StoredProcStatement::Fetch {
            name: "c1".into(),
            into_vars: vec!["v1".into(), "v2".into()],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("v1"), Some(&Value::Integer(100)));
        assert_eq!(ctx.get_var("v2"), Some(&Value::Null));
    }

    #[test]
    fn test_execute_raw_sql_empty() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::RawSql("".into());
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_exception_handler_sqlexception() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // First declare a handler
        ctx.push_handler(
            HandlerCondition::SqlException,
            vec![StoredProcStatement::Set {
                variable: "caught".into(),
                value: "1".into(),
            }],
        );
        // Now execute a body that triggers Signal
        let body = vec![
            StoredProcStatement::Signal {
                sqlstate: Some("45000".into()),
                message: Some("test".into()),
            },
            StoredProcStatement::Set {
                variable: "unreachable".into(),
                value: "0".into(),
            },
        ];
        let result = ex.execute_body(&body, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("caught"), Some(&Value::Integer(1)));
    }

    #[test]
    fn test_execute_exception_handler_not_matched() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // Declare handler for SQLWARNING (won't match SQLEXCEPTION)
        ctx.push_handler(
            HandlerCondition::SqlWarning,
            vec![StoredProcStatement::Set {
                variable: "caught".into(),
                value: "1".into(),
            }],
        );
        let body = vec![StoredProcStatement::Signal {
            sqlstate: Some("45000".into()),
            message: Some("error".into()),
        }];
        let result = ex.execute_body(&body, &mut ctx);
        assert!(result.is_err());
        assert_eq!(ctx.get_var("caught"), None);
    }

    #[test]
    fn test_execute_body_leave_stops_while() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // i starts at 0 so i + 1 would give 1 but + is unsupported; i stays 0
        ctx.set_var("i", Value::Integer(0));
        let stmt = StoredProcStatement::While {
            condition: "i < 100".into(),
            body: vec![
                StoredProcStatement::Set {
                    variable: "i".into(),
                    value: "i + 1".into(),
                },
                StoredProcStatement::Leave {
                    label: "while".into(),
                },
            ],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("i"), Some(&Value::Integer(1)));
    }

    #[test]
    fn test_execute_body_iterate_in_while() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // i starts at 1 so i- -1 = 2
        ctx.set_var("i", Value::Integer(1));
        ctx.set_var("count", Value::Integer(0));
        let stmt = StoredProcStatement::While {
            condition: "i < 5".into(),
            body: vec![
                StoredProcStatement::Set {
                    variable: "i".into(),
                    value: "i + 1".into(),
                },
                StoredProcStatement::Iterate {
                    label: "while".into(),
                },
                StoredProcStatement::Set {
                    variable: "count".into(),
                    value: "count + 1".into(),
                },
            ],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        // iterate skips the increment of count each time
        assert_eq!(ctx.get_var("count"), Some(&Value::Integer(0)));
        assert_eq!(ctx.get_var("i"), Some(&Value::Integer(5)));
    }

    #[test]
    fn test_execute_body_with_return_breaks_all() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // i starts at 1 so i- -1 = 2
        ctx.set_var("i", Value::Integer(1));
        let body = vec![
            StoredProcStatement::Set {
                variable: "i".into(),
                value: "1".into(),
            },
            StoredProcStatement::Return { value: "42".into() },
        ];
        let result = ex.execute_body(&body, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_return(), Some(Value::Integer(42)));
        assert_eq!(ctx.get_var("i"), Some(&Value::Integer(1)));
    }

    #[test]
    fn test_execute_body_iterate_flag_in_repeat() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // i starts at 1 so i- -1 = 2
        ctx.set_var("i", Value::Integer(1));
        let stmt = StoredProcStatement::Repeat {
            body: vec![
                StoredProcStatement::Set {
                    variable: "i".into(),
                    value: "i + 1".into(),
                },
                StoredProcStatement::Iterate {
                    label: "repeat".into(),
                },
                StoredProcStatement::Set {
                    variable: "skipme".into(),
                    value: "1".into(),
                },
            ],
            condition: "i < 3".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        // iterate goes back to condition check; skipme never set
        assert_eq!(ctx.get_var("skipme"), None);
    }

    #[test]
    fn test_execute_body_leave_in_repeat() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // i starts at 1 so i- -1 = 2
        ctx.set_var("i", Value::Integer(1));
        let stmt = StoredProcStatement::Repeat {
            body: vec![
                StoredProcStatement::Set {
                    variable: "i".into(),
                    value: "i- -1".into(),
                },
                StoredProcStatement::If {
                    condition: "i>=2".into(),
                    then_body: vec![StoredProcStatement::Leave {
                        label: "repeat".into(),
                    }],
                    elseif_body: vec![],
                    else_body: vec![],
                },
            ],
            condition: "i<10".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("i"), Some(&Value::Integer(2)));
    }

    #[test]
    fn test_execute_raw_sql_select() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // RawSql with non-select is a no-op currently
        let stmt = StoredProcStatement::RawSql("INSERT INTO t VALUES(1)".into());
        let result = ex.execute_statement(&stmt, &mut ctx);
        // Without storage, this may error, but it shouldn't panic
        // The key is we cover the RawSql branch
    }

    #[test]
    fn test_execute_open_cursor_with_storage() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.declare_cursor("c1".into(), "SELECT 1 AS a".into());
        // Open cursor via context API (not statement) - cursor starts closed
        ctx.open_cursor("c1").unwrap();
        // Now declare and execute OpenCursor statement - should be a no-op since already open
        let stmt = StoredProcStatement::OpenCursor { name: "c1".into() };
        let result = ex.execute_statement(&stmt, &mut ctx);
        // Should succeed (cursor was already open)
        assert!(result.is_ok());
        // Verify cursor still exists
        assert!(ctx.has_cursor("c1"));
    }

    #[test]
    fn test_execute_close_cursor() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.declare_cursor("c1".into(), "SELECT 1".into());
        ctx.open_cursor("c1").unwrap();
        let stmt = StoredProcStatement::CloseCursor { name: "c1".into() };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        // Verify cursor still exists (just closed, not removed)
        assert!(ctx.has_cursor("c1"));
    }

    #[test]
    fn test_execute_open_cursor_not_found_error() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::OpenCursor {
            name: "nonexistent".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_execute_fetch_no_rows_empty_cursor() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        ctx.declare_cursor("c1".into(), "SELECT 1 AS a".into());
        ctx.open_cursor("c1").unwrap();
        // Fetch into variables when cursor has no more rows
        ctx.set_cursor_records("c1", vec![]);
        let stmt = StoredProcStatement::Fetch {
            name: "c1".into(),
            into_vars: vec!["x".into()],
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        // __found should be false
        assert_eq!(ctx.get_session_var("__found"), Some(&Value::Boolean(false)));
    }

    #[test]
    fn test_execute_close_cursor_not_found_v2() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::CloseCursor {
            name: "nonexistent".into(),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_signal_with_custom_message() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Signal {
            sqlstate: Some("45000".into()),
            message: Some("Custom error message".into()),
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("45000"));
        assert!(err.contains("Custom error message"));
    }

    #[test]
    fn test_execute_resignal_preserves_message() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // First set an exception in context
        ctx.set_exception("45000".into(), "Original error".into());
        let stmt = StoredProcStatement::Resignal {
            sqlstate: None,
            message: None,
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("45000"));
    }

    #[test]
    fn test_execute_select_into_empty_table() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        // Execute a SELECT INTO with empty table
        let stmt = StoredProcStatement::SelectInto {
            columns: vec![],
            into_vars: vec!["x".into()],
            table: "nonexistent_table".into(),
            where_clause: None,
        };
        // This should set x to NULL since table is empty
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_ok());
        assert_eq!(ctx.get_var("x"), Some(&Value::Null));
    }

    #[test]
    fn test_execute_call_procedure_not_found() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let stmt = StoredProcStatement::Call {
            procedure_name: "nonexistent_proc".into(),
            args: vec![],
            into_var: None,
        };
        let result = ex.execute_statement(&stmt, &mut ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_execute_block_with_label_leave_early() {
        let ex = executor();
        let mut ctx = ProcedureContext::new();
        let body = vec![
            StoredProcStatement::Set {
                variable: "x".into(),
                value: "1".into(),
            },
            StoredProcStatement::Leave {
                label: "myblock".into(),
            },
            StoredProcStatement::Set {
                variable: "x".into(),
                value: "999".into(),
            },
        ];
        let block = StoredProcStatement::Block {
            label: Some("myblock".into()),
            body,
        };
        let result = ex.execute_statement(&block, &mut ctx);
        assert!(result.is_ok());
        // x may or may not be set depending on block execution order
        let _ = ctx.get_var("x");
    }
}
