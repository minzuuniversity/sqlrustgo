//! Stored Procedure definitions for the catalog

use serde::{Deserialize, Serialize};

/// Stored procedure definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredProcedure {
    pub name: String,
    pub params: Vec<StoredProcParam>,
    pub body: Vec<StoredProcStatement>,
}

/// Stored procedure parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredProcParam {
    pub name: String,
    pub mode: ParamMode,
    pub data_type: String,
}

/// Parameter mode for stored procedure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamMode {
    In,
    Out,
    InOut,
}

/// Stored procedure statement types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StoredProcStatement {
    /// Raw SQL statement
    RawSql(String),
    /// SELECT ... INTO var1, var2 FROM ...
    SelectInto {
        columns: Vec<String>,
        into_vars: Vec<String>,
        table: String,
        where_clause: Option<String>,
    },
    /// SET variable = value
    Set { variable: String, value: String },
    /// DECLARE variable statement
    Declare {
        name: String,
        data_type: String,
        default_value: Option<String>,
    },
    /// IF condition THEN statements [ELSEIF ...] [ELSE ...] END IF
    If {
        condition: String,
        then_body: Vec<StoredProcStatement>,
        elseif_body: Vec<(String, Vec<StoredProcStatement>)>,
        else_body: Vec<StoredProcStatement>,
    },
    /// WHILE condition DO statements END WHILE
    While {
        condition: String,
        body: Vec<StoredProcStatement>,
    },
    /// LOOP statements END LOOP (with optional LEAVE to exit)
    Loop { body: Vec<StoredProcStatement> },
    /// CASE case_value WHEN value1 THEN result1 ... ELSE result END
    Case {
        case_value: Option<String>,
        when_clauses: Vec<(String, String)>,
        else_result: Option<String>,
    },
    /// CASE WHEN condition1 THEN result1 ... ELSE result END
    CaseWhen {
        when_clauses: Vec<(String, String)>,
        else_result: Option<String>,
    },
    /// REPEAT statements UNTIL condition END REPEAT
    Repeat {
        body: Vec<StoredProcStatement>,
        condition: String,
    },
    /// RETURN expression
    Return { value: String },
    /// LEAVE label - exit a loop
    Leave { label: String },
    /// ITERATE label - continue to next iteration
    Iterate { label: String },
    /// CALL another stored procedure
    Call {
        procedure_name: String,
        args: Vec<String>,
        into_var: Option<String>,
    },
    /// SIGNAL - raise an exception
    Signal {
        sqlstate: Option<String>,
        message: Option<String>,
    },
    /// RESIGNAL - re-raise an exception
    Resignal {
        sqlstate: Option<String>,
        message: Option<String>,
    },
    /// BEGIN...END block (with optional label)
    Block {
        label: Option<String>,
        body: Vec<StoredProcStatement>,
    },
    /// DECLARE HANDLER for exception conditions
    DeclareHandler {
        condition_type: HandlerCondition,
        body: Vec<StoredProcStatement>,
    },
    /// DECLARE cursor
    DeclareCursor { name: String, query: String },
    /// OPEN cursor
    OpenCursor { name: String },
    /// FETCH from cursor
    Fetch {
        name: String,
        into_vars: Vec<String>,
    },
    /// CLOSE cursor
    CloseCursor { name: String },
}

/// Handler condition types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HandlerCondition {
    /// SQLEXCEPTION - any error
    SqlException,
    /// SQLWARNING - any warning
    SqlWarning,
    /// NOT FOUND - no rows found
    NotFound,
    /// Specific SQLSTATE
    SqlState(String),
    /// Custom error name
    Custom(String),
}

/// SQL error codes for common conditions
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum SqlCondition {
    SqlException,
    SqlWarning,
    NotFound,
    Custom(String),
}

impl StoredProcedure {
    /// Create a new stored procedure
    pub fn new(name: String, params: Vec<StoredProcParam>, body: Vec<StoredProcStatement>) -> Self {
        Self { name, params, body }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use crate::error::{CatalogError, CatalogResult};

    fn dummy_proc(name: &str) -> StoredProcedure {
        StoredProcedure::new(
            name.to_string(),
            vec![],
            vec![StoredProcStatement::RawSql("SELECT 1".to_string())],
        )
    }

    #[test]
    fn test_stored_procedure_creation() {
        let proc = StoredProcedure::new(
            "test_proc".to_string(),
            vec![StoredProcParam {
                name: "param1".to_string(),
                mode: ParamMode::In,
                data_type: "INTEGER".to_string(),
            }],
            vec![StoredProcStatement::RawSql("SELECT 1".to_string())],
        );

        assert_eq!(proc.name, "test_proc");
        assert_eq!(proc.params.len(), 1);
        assert_eq!(proc.body.len(), 1);
    }

    #[test]
    fn test_param_mode() {
        assert!(matches!(ParamMode::In, ParamMode::In));
        assert!(matches!(ParamMode::Out, ParamMode::Out));
        assert!(matches!(ParamMode::InOut, ParamMode::InOut));
    }

    /// V312-55A / Issue #4238: stored procedure DDL lifecycle.
    /// Covers: add, duplicate detection, case-insensitive lookup,
    /// remove, OR REPLACE semantics.
    ///
    /// This test is named with `procedure_` prefix so it matches the
    /// `cargo test -p sqlrustgo-catalog --lib procedure` filter used
    /// by `scripts/gate/check_v312_procedure_trigger_gate.sh`
    /// (`V55A-Procedure-DDL` check).
    #[test]
    fn procedure_ddl_lifecycle() -> CatalogResult<()> {
        let mut catalog = Catalog::with_default_database("test_db", "test_db");

        // 1) Add procedure — original casing preserved.
        catalog.add_stored_procedure(dummy_proc("MyProc"))?;
        assert!(catalog.has_stored_procedure("MyProc"));
        assert_eq!(catalog.get_stored_procedure("MyProc").unwrap().name, "MyProc");
        assert_eq!(catalog.stored_procedure_count(), 1);

        // 2) Duplicate detection uses case-insensitive name.
        let dup: CatalogResult<()> = catalog.add_stored_procedure(dummy_proc("myproc"));
        assert!(matches!(dup, Err(CatalogError::DuplicateProcedure(_))));
        // Plain all-lowercase dup also rejected.
        let dup2: CatalogResult<()> = catalog.add_stored_procedure(dummy_proc("MYPROC"));
        assert!(matches!(dup2, Err(CatalogError::DuplicateProcedure(_))));

        // 3) Case-insensitive lookup returns the original-cased record.
        assert!(catalog.has_stored_procedure("myproc"));
        assert!(catalog.has_stored_procedure("MYPROC"));
        assert!(catalog.has_stored_procedure("MyPrOc"));
        let looked_up = catalog.get_stored_procedure("MYPROC").unwrap();
        assert_eq!(looked_up.name, "MyProc", "casing preserved on lookup");

        // 4) OR REPLACE overwrites existing procedure with same key.
        let replaced = dummy_proc("MyProc");
        catalog.add_or_replace_stored_procedure(replaced)?;
        assert_eq!(catalog.stored_procedure_count(), 1);
        assert!(catalog.get_stored_procedure("MyProc").is_some());

        // 5) OR REPLACE on a fresh name acts like add.
        catalog.add_or_replace_stored_procedure(dummy_proc("Other_Proc"))?;
        assert_eq!(catalog.stored_procedure_count(), 2);

        // 6) Remove is case-insensitive.
        let removed = catalog.remove_stored_procedure("myproc");
        assert!(removed.is_some(), "case-insensitive remove should succeed");
        assert_eq!(catalog.stored_procedure_count(), 1);

        // 7) Remove on unknown procedure returns None.
        assert!(catalog.remove_stored_procedure("does_not_exist").is_none());

        Ok(())
    }
}
