use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentSqlError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Plan error: {0}")]
    PlanError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Schema error: {0}")]
    SchemaError(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<String> for AgentSqlError {
    fn from(s: String) -> Self {
        AgentSqlError::InternalError(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_variants() {
        let e = AgentSqlError::ParseError("bad syntax".to_string());
        assert_eq!(e.to_string(), "Parse error: bad syntax");

        let e = AgentSqlError::PlanError("plan failed".to_string());
        assert_eq!(e.to_string(), "Plan error: plan failed");

        let e = AgentSqlError::ExecutionError("exec failed".to_string());
        assert_eq!(e.to_string(), "Execution error: exec failed");

        let e = AgentSqlError::SchemaError("schema invalid".to_string());
        assert_eq!(e.to_string(), "Schema error: schema invalid");

        let e = AgentSqlError::HttpError("http error".to_string());
        assert_eq!(e.to_string(), "HTTP error: http error");

        let e = AgentSqlError::InternalError("internal".to_string());
        assert_eq!(e.to_string(), "Internal error: internal");
    }

    #[test]
    fn test_from_string() {
        let e: AgentSqlError = "message".to_string().into();
        assert!(matches!(e, AgentSqlError::InternalError(msg) if msg == "message"));
    }

    #[test]
    fn test_error_debug() {
        let e = AgentSqlError::ParseError("test".to_string());
        let debug_str = format!("{:?}", e);
        assert!(debug_str.contains("ParseError"));
    }
}
