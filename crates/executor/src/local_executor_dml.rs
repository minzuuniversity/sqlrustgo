//! Local Executor DML Module
//!
//! Placeholder for local executor DML operations.

/// Placeholder LocalExecutorDml
pub struct LocalExecutorDml;

/// Placeholder LocalExecutorDmlArc
pub struct LocalExecutorDmlArc;

impl LocalExecutorDml {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalExecutorDml {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_executor_dml_new() {
        let dml = LocalExecutorDml::new();
        let _ = dml;
    }

    #[test]
    fn test_local_executor_dml_default() {
        let dml = LocalExecutorDml::default();
        let _ = dml;
    }

    #[test]
    fn test_send_sync() {
        fn check<T: Send + Sync>() {}
        check::<LocalExecutorDml>();
        check::<LocalExecutorDmlArc>();
    }
}
