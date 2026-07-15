## ADDED Requirements

### Requirement: Password Age Enforcement

Password age is tracked from the time of last password change. After the configured lifetime (default 90 days), the password is considered expired.

#### Scenario: Password within lifetime
- **WHEN** a user authenticates with a password changed less than `lifetime_days` ago
- **THEN** authentication succeeds normally

#### Scenario: Password exceeds lifetime
- **WHEN** a user authenticates with a password changed `>= lifetime_days` ago
- **THEN** authentication fails with `AuthError::PasswordExpired`

#### Scenario: Unknown user
- **WHEN** `is_expired()` is called for a user with no recorded password change
- **THEN** returns `false` (no expiration policy applies)

### Requirement: Manual Password Expiration (ALTER USER PASSWORD EXPIRE)

Administrators can manually expire a user's password via `ALTER USER ... PASSWORD EXPIRE`, requiring the user to change it on next login.

#### Scenario: ALTER USER PASSWORD EXPIRE
- **WHEN** `ALTER USER 'alice'@'%' PASSWORD EXPIRE` is executed
- **THEN** subsequent connections for 'alice' fail with `AuthError::PasswordExpired` until password is changed
- **THEN** `is_expired('alice')` returns `true`

#### Scenario: Manual expiration does not block writes immediately
- **WHEN** a user's password is manually expired via `ALTER USER PASSWORD EXPIRE`
- **THEN** the user's existing sessions remain active until they reconnect
- **THEN** new connection attempts are rejected with `AuthError::PasswordExpired`

### Requirement: Password History (No Reuse)

The system maintains a history of recent password hashes per user and prevents reuse within the history window.

#### Scenario: Password reuse within history window
- **WHEN** a user attempts to change their password to a hash in their history
- **THEN** the change is rejected with an error indicating password was recently used

#### Scenario: Password reuse outside history window
- **WHEN** a user changes to a password whose hash is not in their history
- **THEN** the change succeeds and the old hash is added to history

#### Scenario: History size limit
- **WHEN** a user changes their password more than `history_size` times
- **THEN** the oldest entry is evicted from history (FIFO)
- **AND** a password matching the evicted hash can be reused

### Requirement: Configurable Policy

The password rotation policy is configurable with a global default and runtime updates.

#### Scenario: Default policy values
- **WHEN** a `PasswordRotationManager` is created with `PasswordRotationManager::new()`
- **THEN** `lifetime_days = 90`, `history_size = 5`, `enforce_on_write = true`

#### Scenario: Policy update at runtime
- **WHEN** `set_policy()` is called with a new `PasswordPolicy`
- **THEN** subsequent checks use the new policy values

#### Scenario: Policy with lifetime=0 (disabled)
- **WHEN** `lifetime_days = 0`
- **THEN** no password ever expires due to age
- **AND** `is_expired()` always returns `false`

### Requirement: Write Blocking for Expired Passwords

When a user's password is expired, write operations are blocked until the password is changed.

#### Scenario: Write blocked for expired user
- **WHEN** a user's password is expired and `enforce_on_write = true`
- **THEN** write operations (INSERT/UPDATE/DELETE) by that user are rejected
- **THEN** `is_write_blocked(user)` returns `true`

#### Scenario: Write allowed when enforcement disabled
- **WHEN** a user's password is expired but `enforce_on_write = false`
- **THEN** write operations are allowed
- **AND** `is_write_blocked(user)` returns `false`
