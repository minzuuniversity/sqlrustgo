use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum GateError {
    #[error("check failed: {0}")]
    CheckFailed(String),

    #[error("subprocess error: exit code {0}")]
    Subprocess(i32),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("evidence error: {0}")]
    Evidence(String),
}
