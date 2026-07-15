## 1. Core Data Structures

- [x] 1.1 Add `password_changed_at: u64` and `password_expired: bool` fields to `UserAuthInfo` in `crates/catalog/src/auth.rs`
- [x] 1.2 Add `DEFAULT_PASSWORD_LIFETIME_DAYS = 90` and `DEFAULT_PASSWORD_HISTORY_SIZE = 5` constants
- [x] 1.3 Add `PasswordAge`, `PasswordHistory`, `PasswordPolicy`, `PasswordRotationManager` structs to `crates/catalog/src/auth.rs`
- [x] 1.4 Add `password_rotation: PasswordRotationManager` field to `AuthManager`
- [x] 1.5 Add `#[derive(Serialize, Deserialize)]` to `PasswordAge`, `PasswordHistory`, `PasswordPolicy` — ensure serde compatibility with existing `AuthManager` serialization

## 2. AuthManager Integration

- [x] 2.1 Wire `record_password_change()` into `AuthManager::create_user()`
- [x] 2.2 Add `is_password_expired(&self, user: &UserIdentity) -> bool` to `AuthManager`
- [x] 2.3 Add `expire_password(&self, user: &UserIdentity)` to `AuthManager` (for `ALTER USER PASSWORD EXPIRE`)
- [x] 2.4 Add `can_reuse_password(&self, user: &UserIdentity, password_hash: &str) -> bool` to `AuthManager`
- [x] 2.5 Add `password_policy(&self) -> PasswordPolicy` and `set_password_policy(&self, policy: PasswordPolicy)` to `AuthManager`
- [x] 2.6 Add `is_password_write_blocked(&self, user: &UserIdentity) -> bool` to `AuthManager`
- [x] 2.7 Add `set_password_hash(&self, user: &UserIdentity, password_hash: &str)` to `AuthManager` (for password change flow)

## 3. Wire into authenticate() and permission checks

- [x] 3.1 In `AuthManager::authenticate()`, check `is_password_expired()` after successful password verify — return `AuthError::PasswordExpired` if expired
- [ ] 3.2 Ensure `check_privilege()` or write operations check `is_password_write_blocked()` for expired users

## 4. MySQL Wire Protocol — ALTER USER PASSWORD EXPIRE

- [ ] 4.1 Find or create `ALTER USER` parser in `crates/mysql-server/` or `crates/parser/`
- [ ] 4.2 Add `PASSWORD EXPIRE` clause handling to the ALTER USER statement
- [ ] 4.3 Wire `expire_password()` into the mysql-server execution path for `ALTER USER ... PASSWORD EXPIRE`
- [ ] 4.4 Verify MySQL protocol compatibility (what packet does the server return for expired password?)

## 5. Test Updates

- [x] 5.1 Update `tests/integration/sql/password_rotation_test.rs` to import from `crate::auth::PasswordRotationManager` instead of inline module
- [x] 5.2 Add integration test `password_rotation_integration_test.rs` (17 tests covering auth, history, policy, write blocking, clone, edge cases)
- [x] 5.3 Run full test suite: `cargo test --all-features -- password_rotation` — must pass

## 6. Clippy & Fmt

- [x] 6.1 Run `cargo clippy --all-features --workspace -- -D warnings` — 0 errors
- [x] 6.2 Run `cargo fmt --check --all` — 0 diffs

## 7. Documentation

- [ ] 7.1 Update `docs/releases/v3.11.0/CHANGELOG.md` with F-35 entry
- [ ] 7.2 Update `V311_ISSUES_PLAN.md` with V311-08 progress

## Deferred to follow-up (V311-08 follow-up)

- Task 3.2: write operation blocking (requires `is_password_write_blocked` integration into executor permission checks)
- Task 4.x: MySQL wire protocol ALTER USER PASSWORD EXPIRE (requires parser + mysql-server changes)
- Task 7.x: documentation updates

## Verification Results (this session)

```
cargo test --test password_rotation_integration_test   # 17/17 PASS ✅
cargo test --test password_rotation_test               # 8/8 PASS ✅
cargo clippy --all-features --workspace -- -D warnings # 0 errors ✅
cargo fmt --check --all                                # 0 diffs ✅
```
