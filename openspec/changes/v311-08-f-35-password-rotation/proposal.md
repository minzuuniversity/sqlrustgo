## Why

Password rotation is a MySQL security compliance requirement (CIS Benchmark, SOX, GDPR). F-35 Password Rotation was implemented as ISOLATED in v3.9.0 but never integrated into the main path. Issue #3495 tracks bringing it to main path integration during v3.11.0 ALPHA.

Current state: `PasswordRotationManager` lives in `tests/integration/sql/password_rotation_test.rs` as an inline module — verified working but not accessible to production code.

## What Changes

Integrate `PasswordRotationManager` into the main `AuthManager` in `crates/catalog/src/auth.rs`, wire it into the `mysql-server` component, and add MySQL wire protocol support for `ALTER USER ... PASSWORD EXPIRE`.

### Scope

1. **Promote `PasswordRotationManager`** from test file → `crates/catalog/src/auth.rs`
2. **Add `password_expired` field** to `UserAuthInfo` struct
3. **Wire into auth flow**: check password age on connection, block writes for expired passwords
4. **MySQL wire protocol**: handle `ALTER USER ... PASSWORD EXPIRE` in `mysql-server`
5. **Integration test**: `tests/integration/sql/password_rotation_test.rs` must pass

## Open Questions

- Should `PasswordRotationManager` live in `crates/catalog/` or a new `crates/password-rotation/` crate?
- Do we need a feature flag (`password-rotation`) for gradual rollout?
- Password lifetime policy — where is it stored? Per-user or global?
