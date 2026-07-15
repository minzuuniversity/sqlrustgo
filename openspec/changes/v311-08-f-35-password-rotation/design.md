## Context

**Current state**: `PasswordRotationManager` lives in `tests/integration/sql/password_rotation_test.rs` (ISOLATED). It is a self-contained module that tracks password age, history, and policy — verified by 8 passing unit tests.

**Target**: Promote into `crates/catalog/src/auth.rs` as part of `AuthManager`, wire into `mysql-server` for MySQL wire protocol support.

**Background**: F-35 was added in v3.9.0 as ISOLATED per the Phase 1 plan (see `ISOLATED_MODULES.md §1`). v3.11.0 ALPHA is the window for main-path integration.

## Goals / Non-Goals

**Goals:**
- Promote `PasswordRotationManager` to `crates/catalog/src/auth.rs`
- Add `password_expired: bool` field to `UserAuthInfo` for manual `PASSWORD EXPIRE`
- Add `password_changed_at: u64` timestamp to `UserAuthInfo` for age tracking
- Integrate into `AuthManager`: check on connection, block writes for expired passwords
- Handle `ALTER USER ... PASSWORD EXPIRE` in mysql-server packet handler
- Pass all existing `password_rotation_test.rs` tests

**Non-Goals:**
- Changing the password hash algorithm (keep existing SCRAM-SHA-256)
- Per-user password policy (global policy only for ALPHA)
- Feature flag — always-on for ALPHA
- `ALTER USER ... IDENTIFIED BY ... REPLACE ...` (history reuse enforcement deferred)

## Decisions

### Decision 1: Where does `PasswordRotationManager` live?

**Choice**: Inline in `crates/catalog/src/auth.rs` (not a separate crate)

**Rationale**: Aligns with existing pattern — `AuthManager` already owns user lifecycle. Separate crate adds unnecessary complexity for v3.11.0 ALPHA scope. Can extract to `crates/password-rotation/` later if it grows.

### Decision 2: How to handle the policy store?

**Choice**: Policy lives in `AuthManager` (global, not per-user)

**Rationale**: Per-user policy adds schema migration complexity. ALPHA uses global `PasswordPolicy` with `lifetime_days=90`. Can extend to per-user in a follow-up.

### Decision 3: How does password age interact with the serialization format?

**Choice**: `password_changed_at: u64` (Unix timestamp) added to `UserAuthInfo`

**Rationale**: Existing `UserAuthInfo` is serde-serializable. Adding a timestamp field is backward-compatible with default=0 (treat as "never changed" = not expired). The `PasswordRotationManager` computes age on demand from this timestamp.

### Decision 4: How does `ALTER USER ... PASSWORD EXPIRE` work?

**Choice**: Sets `password_changed_at` to `now - 365 days` (making it immediately expired)

**Rationale**: Matches MySQL semantics — `PASSWORD EXPIRE` doesn't revoke access immediately, it marks the password as expired so the user must change it on next login. Write operations should be blocked for expired users (enforce_on_write=true).

## Implementation Plan

### Step 1: Add fields to `UserAuthInfo`

```rust
// crates/catalog/src/auth.rs
pub struct UserAuthInfo {
    pub identity: UserIdentity,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: u64,
    pub updated_at: u64,
    pub password_changed_at: u64,  // NEW: Unix timestamp, 0 = unknown
    pub password_expired: bool,    // NEW: manually expired via ALTER USER
}
```

### Step 2: Add `PasswordRotationManager` to `AuthManager`

```rust
pub struct AuthManager {
    // ... existing fields ...
    password_rotation: PasswordRotationManager,
}
```

`PasswordRotationManager` is initialized with default policy (90-day lifetime, history size 5).

### Step 3: Wire into auth flow

- `create_user()` → calls `password_rotation.record_password_change()`
- `authenticate()` → checks `is_expired()` and returns error if expired
- `check_write_permission()` → checks `is_write_blocked()` for expired users

### Step 4: MySQL wire protocol — `ALTER USER ... PASSWORD EXPIRE`

In `mysql-server`, parse `COM_ALTER_USER` or `ALTER USER ... PASSWORD EXPIRE` and call `auth_manager.expire_password(user_identity)`.

### Step 5: Remove inline module from test file

After promotion, the test file imports from `crate::auth::PasswordRotationManager` (or re-exports via `crate::auth`).

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| serde backwards compatibility | `password_changed_at: 0` default means "unknown" = not expired; existing serialized data degrades gracefully |
| Thread safety | `PasswordRotationManager` uses `RwLock` (already in the code); `AuthManager` already uses internal locking |
| Performance on authenticate | `is_expired()` is O(1) hashmap lookup; acceptable for connection-time check |
| Test file coupling | Test file imports from `crates::auth` after promotion; tests verify the real integration |
