use thiserror::Error;

#[derive(Debug, Error)]
pub enum UnifiedQueryError {
    #[error("SQL error: {0}")]
    Sql(String),
    #[error("Vector error: {0}")]
    Vector(String),
    #[error("Graph error: {0}")]
    Graph(String),
    #[error("Routing error: {0}")]
    Routing(String),
    #[error("Fusion error: {0}")]
    Fusion(String),
}

pub enum QueryResult<T> {
    Ok(T),
    Partial(Vec<String>),
    Err(String),
}

impl<T: std::fmt::Debug> std::fmt::Debug for QueryResult<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryResult::Ok(t) => f.debug_tuple("Ok").field(t).finish(),
            QueryResult::Partial(warnings) => f.debug_tuple("Partial").field(warnings).finish(),
            QueryResult::Err(msg) => f.debug_tuple("Err").field(msg).finish(),
        }
    }
}

impl<T> QueryResult<T> {
    pub fn is_ok(&self) -> bool {
        matches!(self, QueryResult::Ok(_))
    }

    pub fn is_partial(&self) -> bool {
        matches!(self, QueryResult::Partial(_))
    }

    pub fn unwrap_or_default(&self) -> Option<T>
    where
        T: Clone,
    {
        match self {
            QueryResult::Ok(t) => Some((*t).clone()),
            _ => None,
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            QueryResult::Ok(t) => t,
            QueryResult::Partial(warnings) => panic!("QueryResult::Partial({:?})", warnings),
            QueryResult::Err(msg) => panic!("QueryResult::Err({})", msg),
        }
    }

    pub fn expect(self, msg: &str) -> T {
        match self {
            QueryResult::Ok(t) => t,
            QueryResult::Partial(warnings) => panic!("{}: Partial({:?})", msg, warnings),
            QueryResult::Err(err_msg) => panic!("{}: Err({})", msg, err_msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_result_ok() {
        let result: QueryResult<i32> = QueryResult::Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or_default(), Some(42));
    }

    #[test]
    fn test_query_result_partial() {
        let result: QueryResult<i32> = QueryResult::Partial(vec!["warning1".to_string()]);
        assert!(result.is_partial());
        assert_eq!(result.unwrap_or_default(), None);
    }

    #[test]
    fn test_query_result_err() {
        let result: QueryResult<i32> = QueryResult::Err("error".to_string());
        assert!(!result.is_ok());
        assert!(!result.is_partial());
    }
}
