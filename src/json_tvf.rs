//! V312-88 / Issue #4755: SQLite-compatible `JSON_EACH` and `JSON_TREE`
//! table-valued functions.
//!
//! Usage:
//!   SELECT * FROM JSON_EACH('[1, 2, 3]');
//!   SELECT * FROM JSON_TREE('{"a": 1, "b": 2}');
//!
//! Both functions take a JSON value as the first argument and an
//! optional `path` (default `'$'`) as the second. They expand the JSON
//! document into rows with the columns SQLite documents at
//! <https://www.sqlite.org/json1.html#json_each>:
//!
//!   key     TEXT  — array index (as text) or object key; NULL for
//!                   non-array/non-object top-level values
//!   value   TEXT  — JSON representation of this element
//!   type    TEXT  — 'null' | 'true' | 'false' | 'integer' | 'real'
//!                  | 'text' | 'array' | 'object'
//!   atom    —     — primitive value for non-array/non-object rows;
//!                   NULL for container rows
//!   id      INTEGER — unique row id
//!   parent  INTEGER — parent row id, or 0 for the root
//!   fullkey TEXT  — full JSON path like '$[0]' or '$.a.b'
//!   path    TEXT  — JSON path of the element's container
//!
//! `JSON_EACH` only emits the immediate children of the matched element
//! (i.e. one level deep). `JSON_TREE` recursively expands all
//! descendants.

use serde_json::Value as Json;
use sqlrustgo_parser::Expression;

use crate::system_tables::try_evaluate_const;
use crate::{ExecutorResult, SqlError, SqlResult, Value};

/// SQLite-compatible column list for `json_each` and `json_tree`.
pub const JSON_TVF_SCHEMA: &[&str] = &[
    "key", "value", "type", "atom", "id", "parent", "fullkey", "path",
];

/// Build the rows for `SELECT * FROM JSON_EACH(...)` or `JSON_TREE(...)`.
/// Returns `None` when `table_name` is not one of the JSON TVFs so the
/// caller can fall through to other dispatchers.
pub fn try_json_tvf_select(
    table_name: &str,
    args: Option<&Vec<Expression>>,
) -> Option<SqlResult<ExecutorResult>> {
    let upper = table_name.to_ascii_uppercase();
    let recursive = match upper.as_str() {
        "JSON_EACH" => false,
        "JSON_TREE" => true,
        _ => return None,
    };

    let args = match args {
        Some(a) if !a.is_empty() => a,
        _ => {
            return Some(Err(SqlError::ExecutionError(format!(
                "{} requires at least 1 argument (the JSON document)",
                upper
            ))))
        }
    };

    let json_text = match try_evaluate_const(&args[0]) {
        Some(Value::Text(s)) => s,
        Some(Value::Json(j)) => j.to_string(),
        Some(Value::Null) | None => {
            return Some(Ok(ExecutorResult::new(Vec::new(), JSON_TVF_SCHEMA.len())))
        }
        Some(other) => {
            return Some(Err(SqlError::ExecutionError(format!(
                "{} first argument must be JSON text, got {:?}",
                upper, other
            ))))
        }
    };

    let doc: Json = match serde_json::from_str(&json_text) {
        Ok(v) => v,
        Err(_) => {
            return Some(Err(SqlError::ExecutionError(format!(
                "{} first argument is not valid JSON: {}",
                upper, json_text
            ))))
        }
    };

    let path = if args.len() >= 2 {
        match try_evaluate_const(&args[1]) {
            Some(Value::Text(s)) => s,
            _ => "$".to_string(),
        }
    } else {
        "$".to_string()
    };

    let root: Json = if path == "$" {
        doc.clone()
    } else {
        let pointer = jsonpath_to_pointer(&path);
        match doc.pointer(&pointer) {
            Some(v) => v.clone(),
            None => return Some(Ok(ExecutorResult::new(Vec::new(), JSON_TVF_SCHEMA.len()))),
        }
    };

    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut next_id: i64 = 1;
    if recursive {
        walk_recursive(&root, "$", 0, &mut next_id, &mut rows);
    } else {
        walk_one_level(&root, "$", 0, &mut next_id, &mut rows);
    }
    Some(Ok(ExecutorResult::new(rows, JSON_TVF_SCHEMA.len())))
}

fn json_type_name(v: &Json) -> &'static str {
    match v {
        Json::Null => "null",
        Json::Bool(true) => "true",
        Json::Bool(false) => "false",
        Json::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "real"
            }
        }
        Json::String(_) => "text",
        Json::Array(_) => "array",
        Json::Object(_) => "object",
    }
}

fn json_to_atom(v: &Json) -> Value {
    match v {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Boolean(*b),
        Json::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(u) = n.as_u64() {
                if u <= i64::MAX as u64 {
                    Value::Integer(u as i64)
                } else {
                    Value::Float(u as f64)
                }
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Text(n.to_string())
            }
        }
        Json::String(s) => Value::Text(s.clone()),
        Json::Array(_) | Json::Object(_) => Value::Null,
    }
}

fn push_row(
    key: Value,
    value: &Json,
    fullkey: &str,
    parent: i64,
    id: i64,
    rows: &mut Vec<Vec<Value>>,
) {
    let type_str = json_type_name(value);
    let value_text = value.to_string();
    let atom = json_to_atom(value);
    let path = parent_fullkey_to_path(fullkey);
    rows.push(vec![
        key,
        Value::Text(value_text),
        Value::Text(type_str.to_string()),
        atom,
        Value::Integer(id),
        Value::Integer(parent),
        Value::Text(fullkey.to_string()),
        Value::Text(path),
    ]);
}

fn walk_one_level(
    value: &Json,
    fullkey: &str,
    parent: i64,
    next_id: &mut i64,
    rows: &mut Vec<Vec<Value>>,
) {
    let root_id = *next_id;
    *next_id += 1;
    push_row(Value::Null, value, fullkey, parent, root_id, rows);

    match value {
        Json::Array(arr) => {
            for (idx, child) in arr.iter().enumerate() {
                let id = *next_id;
                *next_id += 1;
                let child_fullkey = format!("$[{}]", idx);
                push_row(
                    Value::Text(idx.to_string()),
                    child,
                    &child_fullkey,
                    root_id,
                    id,
                    rows,
                );
            }
        }
        Json::Object(obj) => {
            for (key, child) in obj.iter() {
                let id = *next_id;
                *next_id += 1;
                let child_fullkey = format!("$.{}", key);
                push_row(
                    Value::Text(key.clone()),
                    child,
                    &child_fullkey,
                    root_id,
                    id,
                    rows,
                );
            }
        }
        _ => {}
    }
}

fn walk_recursive(
    value: &Json,
    fullkey: &str,
    parent: i64,
    next_id: &mut i64,
    rows: &mut Vec<Vec<Value>>,
) {
    let root_id = *next_id;
    *next_id += 1;
    push_row(Value::Null, value, fullkey, parent, root_id, rows);

    recurse_into(value, fullkey, root_id, next_id, rows);
}

fn recurse_into(
    value: &Json,
    _fullkey: &str,
    parent: i64,
    next_id: &mut i64,
    rows: &mut Vec<Vec<Value>>,
) {
    match value {
        Json::Array(arr) => {
            for (idx, child) in arr.iter().enumerate() {
                let id = *next_id;
                *next_id += 1;
                let child_fullkey = format!("$[{}]", idx);
                push_row(
                    Value::Text(idx.to_string()),
                    child,
                    &child_fullkey,
                    parent,
                    id,
                    rows,
                );
                recurse_into(child, &child_fullkey, id, next_id, rows);
            }
        }
        Json::Object(obj) => {
            for (key, child) in obj.iter() {
                let id = *next_id;
                *next_id += 1;
                let child_fullkey = format!("$.{}", key);
                push_row(
                    Value::Text(key.clone()),
                    child,
                    &child_fullkey,
                    parent,
                    id,
                    rows,
                );
                recurse_into(child, &child_fullkey, id, next_id, rows);
            }
        }
        _ => {}
    }
}

fn jsonpath_to_pointer(path: &str) -> String {
    if path.is_empty() || path == "$" {
        return String::new();
    }
    if path.starts_with('/') {
        return path.to_string();
    }
    let mut rest = path;
    if let Some(stripped) = rest.strip_prefix('$') {
        rest = stripped;
    }
    if let Some(stripped) = rest.strip_prefix('.') {
        rest = stripped;
    }
    let mut p = String::new();
    for segment in rest.split('.') {
        if let Some(idx_start) = segment.find('[') {
            let name = &segment[..idx_start];
            if !name.is_empty() {
                p.push('/');
                p.push_str(name);
            }
            let cleaned: String = segment[idx_start..]
                .chars()
                .filter(|c| *c != '[' && *c != ']')
                .collect();
            p.push('/');
            p.push_str(&cleaned);
        } else if !segment.is_empty() {
            p.push('/');
            p.push_str(segment);
        }
    }
    p
}

fn parent_fullkey_to_path(fullkey: &str) -> String {
    if let Some(stripped) = fullkey.strip_prefix('$') {
        stripped.to_string()
    } else {
        fullkey.to_string()
    }
}
