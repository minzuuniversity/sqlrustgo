//! PostgreSQL Authentication Handler
//!
//! This module provides simple cleartext password authentication
//! for PostgreSQL wire protocol compatibility.
//!
//! ## Authentication Flow (P0)
//!
//! 1. Client sends StartupMessage (user, database)
//! 2. Server responds with AuthenticationCleartextPassword
//! 3. Client sends PasswordMessage (password)
//! 4. Server validates and responds with AuthenticationOk or ErrorResponse
//!
//! ## Security Note
//!
//! Cleartext password authentication is provided for compatibility only.
//! In production, SCRAM authentication should be used.

use thiserror::Error;

/// Authentication error types
#[derive(Error, Debug, Clone)]
pub enum AuthError {
    #[error("Invalid password")]
    InvalidPassword,
    #[error("User not found: {0}")]
    UserNotFound(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
}

/// Result type for authentication operations
pub type AuthResult<T> = Result<T, AuthError>;

/// PostgreSQL authentication message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMessage {
    /// Authentication request - cleartext password
    AuthenticationCleartextPassword,
    /// Authentication request - MD5 password (not implemented in P0)
    AuthenticationMd5Password,
    /// Authentication request - SCRAM (not implemented in P0)
    AuthenticationScram,
    /// Authentication successful
    AuthenticationOk,
}

/// Authentication handler for PostgreSQL protocol
#[derive(Debug, Clone)]
pub struct AuthHandler {
    /// Supported authentication methods
    supported_methods: Vec<AuthMessage>,
}

impl AuthHandler {
    /// Create a new authentication handler with default methods
    pub fn new() -> Self {
        Self {
            supported_methods: vec![AuthMessage::AuthenticationCleartextPassword],
        }
    }

    /// Get the preferred authentication method
    pub fn preferred_method(&self) -> AuthMessage {
        // P0 only supports cleartext password
        AuthMessage::AuthenticationCleartextPassword
    }

    /// Validate cleartext password
    ///
    /// In P0, this performs simple comparison.
    /// In production, this should validate against a user store.
    pub fn validate_password(
        &self,
        user: &str,
        password: &str,
        _stored_hash: Option<&str>,
    ) -> AuthResult<()> {
        // P0: Simple validation - in production, this would check against user store
        if password.is_empty() {
            return Err(AuthError::InvalidPassword);
        }
        if user.is_empty() {
            return Err(AuthError::UserNotFound(user.to_string()));
        }
        Ok(())
    }

    /// Handle authentication request from client password message
    pub fn handle_password(&self, password: &str, _salt: Option<&[u8]>) -> AuthResult<()> {
        // P0: Cleartext password - no salt used
        if password.is_empty() {
            return Err(AuthError::InvalidPassword);
        }
        Ok(())
    }

    /// Check if a method is supported
    pub fn supports_method(&self, method: AuthMessage) -> bool {
        self.supported_methods.contains(&method)
    }
}

impl Default for AuthHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Build an AuthenticationCleartextPassword message
pub fn auth_cleartext_password() -> Vec<u8> {
    let mut msg = Vec::new();
    msg.extend_from_slice(b"R\0"); // Authentication message type
    msg.extend_from_slice(&5i32.to_be_bytes()); // Message length (includes itself)
    msg.extend_from_slice(&3i32.to_be_bytes()); // Auth type: CleartextPassword = 3
    msg
}

/// Build an AuthenticationOk message
pub fn auth_ok() -> Vec<u8> {
    let mut msg = Vec::new();
    msg.extend_from_slice(b"R\0"); // Authentication message type
    msg.extend_from_slice(&8i32.to_be_bytes()); // Message length (includes itself)
    msg.extend_from_slice(&0i32.to_be_bytes()); // Auth type: OK = 0
    msg
}

/// Build an ErrorResponse message
pub fn error_response(message: &str) -> Vec<u8> {
    let mut msg = Vec::new();
    msg.extend_from_slice(b"E\0"); // Error message type

    // Calculate length manually since we need to include all fields
    let mut body = Vec::new();
    body.push(b'C'); // Severity: ERROR
    body.extend_from_slice(b"ERROR\0");
    body.push(b'M'); // Message
    body.extend_from_slice(message.as_bytes());
    body.push(b'\0'); // Terminator
    body.push(b'\0'); // End of message

    let len = body.len() + 4; // Body length + int32 length field
    msg.extend_from_slice(&(len as i32).to_be_bytes());
    msg.extend_from_slice(&body);

    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_handler_new() {
        let handler = AuthHandler::new();
        assert!(handler.supports_method(AuthMessage::AuthenticationCleartextPassword));
        assert_eq!(
            handler.preferred_method(),
            AuthMessage::AuthenticationCleartextPassword
        );
    }

    #[test]
    fn test_auth_handler_validate_password() {
        let handler = AuthHandler::new();
        assert!(handler.validate_password("user", "password", None).is_ok());
        assert!(handler.validate_password("", "password", None).is_err());
        assert!(handler.validate_password("user", "", None).is_err());
    }

    #[test]
    fn test_auth_handler_handle_password() {
        let handler = AuthHandler::new();
        assert!(handler.handle_password("password", None).is_ok());
        assert!(handler.handle_password("", None).is_err());
    }

    #[test]
    fn test_auth_ok_message() {
        let msg = auth_ok();
        assert_eq!(&msg[0..2], b"R\0");
        // msg[2..6] = length (8), msg[6..10] = auth type (0)
        assert_eq!(i32::from_be_bytes([msg[6], msg[7], msg[8], msg[9]]), 0);
    }

    #[test]
    fn test_error_response_message() {
        let msg = error_response("password authentication failed");
        assert_eq!(&msg[0..2], b"E\0");
        assert!(msg.windows(30).any(|w| w == b"password authentication failed"));
    }
}
