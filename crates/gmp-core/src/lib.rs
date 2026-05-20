//! GMP Core library - Common types and error handling

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;
