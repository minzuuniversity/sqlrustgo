## 1. Core Data Structures

- [ ] 1.1 Add `password_changed_at: u64` and `password_expired: bool` fields to `UserAuthInfo` in `crates/catalog/src/auth.rs`
- [ ] 1.2 Add `DEFAULT_PASSWORD_LIFETIME_DAYS = 90` and `DEFAULT_PASSWORD_HISTORY_SIZE = 5` constants
- [ ] 1.3 Add `PasswordAge`, `PasswordHistory`, `PasswordPolicy`, `PasswordRotationManager` structs to `crates/catalog/src/auth.rs`
- [ ] 1.4 Add `password_rotation: PasswordRotationManager` field to `AuthManager`
- [ ] 1.5 Add `#[derive(Serialize, Deserialize)]` to `PasswordAge`, `PasswordHistory`, `PasswordPolicy` — ensure serde compatibility with existing `AuthManager` serialization

## 2. AuthManager Integration

- [ ] 2.1 Wire `record_password_change()` into `AuthManager::create_user()`
- [ ] 2.2 Add `is_password_expired(&self, user: &UserIdentity) -> bool` to `AuthManager`
- [ ] 2.3 Add `expire_password(&self, user: &UserIdentity)` to `AuthManager` (for `ALTER USER PASSWORD EXPIRE`)
- [ ] 2.4 Add `can_reuse_password(&self, user: &UserIdentity, password_hash: &str) -> bool` to `AuthManager`
- [ ] 2.5 Add `password_policy(&self) -> PasswordPolicy` and `set_password_policy(&self, policy: PasswordPolicy)` to `AuthManager`
- [ ] 2.6 Add `is_write_blocked(&self, user: &UserIdentity) -> bool` to `AuthManager`

## 3. Wire into authenticate() and permission checks

- [ ] 3.1 In `AuthManager::authenticate()`, check `is_password_expired()` after successful password verify — return `AuthError::PasswordExpired` if expired
- [ ] 3.2 Ensure `check_privilege()` or write operations check `is_write_blocked()` for expired users

## 4. MySQL Wire Protocol — ALTER USER PASSWORD EXPIRE

- [ ] 4.1 Find or create `ALTER USER` parser in `crates/mysql-server/` or `crates/parser/`
- [ ] 4.2 Add `PASSWORD EXPIRE` clause handling to the ALTER USER statement
- [ ] 4.3 Wire `expire_password()` into the mysql-server execution path for `ALTER USER ... PASSWORD EXPIRE`
- [ ] 4.4 Verify MySQL protocol compatibility (what packet does the server return for expired password?)

## 5. Test Updates

- [ ] 5.1 Update `tests/integration/sql/password_rotation_test.rs` to import from `crate::auth::PasswordRotationManager` instead of inline module
- [ ] 5.2 Add integration test for `ALTER USER ... PASSWORD EXPIRE` flow (if not already covered)
- [ ] 5.3 Run full test suite: `cargo test --all-features -- password_rotation` — must pass

## 6. Clippy & Fmt

- [ ] 6.1 Run `cargo clippy --all-features --workspace -- -D warnings` — 0 errors
- [ ] 6.2 Run `cargo fmt --check --all` — 0 diffs

## 7. Documentation

- [ ] 7.1 Update `docs/releases/v3.11.0/CHANGELOG.md` with F-35 entry
- [ ] 7.2 Update `V311_ISSUES_PLAN.md` with V311-08 progress
