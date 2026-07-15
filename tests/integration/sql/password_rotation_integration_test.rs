//! F-35 Password Rotation Integration Tests
//!
//! Tests PasswordRotationManager integrated into AuthManager:
//! - Authentication flow with password expiration
//! - ALTER USER PASSWORD EXPIRE simulation
//! - Password history (no reuse)
//! - Policy enforcement
//!
//! Issue: #3495 (V311-08)

use sqlrustgo_catalog::{AuthManager, PasswordPolicy, UserIdentity};

fn hash_password(password: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

// === Authentication with password expiration ===

#[test]
fn test_authenticate_new_user_not_expired() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("password123"))
        .unwrap();

    assert!(!auth.is_password_expired(&alice));
    assert!(auth.authenticate(&alice, "password123").is_ok());
}

#[test]
fn test_authenticate_wrong_password_rejected() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("correct")).unwrap();

    let result = auth.authenticate(&alice, "wrong_password");
    assert!(result.is_err());
}

#[test]
fn test_password_expire_blocks_authentication() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("password123"))
        .unwrap();

    auth.expire_password(&alice);

    let result = auth.authenticate(&alice, "password123");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("expired"),
        "Expected 'expired' in error message, got: {}",
        err.message
    );
}

#[test]
fn test_unknown_user_not_expired() {
    let auth = AuthManager::new();
    let unknown = UserIdentity::new("nobody", "localhost");
    assert!(!auth.is_password_expired(&unknown));
}

#[test]
fn test_expire_password_sets_manual_flag() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("password123"))
        .unwrap();

    assert!(!auth.is_password_expired(&alice));
    auth.expire_password(&alice);
    assert!(auth.is_password_expired(&alice));
}

// === Password history (no reuse) ===

#[test]
fn test_password_history_prevents_reuse() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("old_password"))
        .unwrap();

    auth.set_password_hash(&alice, &hash_password("new_password"))
        .unwrap();

    assert!(
        !auth.can_reuse_password(&alice, &hash_password("old_password")),
        "Should NOT be able to reuse old password"
    );
    assert!(
        auth.can_reuse_password(&alice, &hash_password("never_used")),
        "Should be able to use a never-used password"
    );
}

#[test]
fn test_password_history_size_limit() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("pwd0")).unwrap();

    // Change 10 times (history size is 5)
    for i in 1..=10 {
        auth.set_password_hash(&alice, &hash_password(&format!("pwd{}", i)))
            .unwrap();
    }
    // After 11 total (pwd0 + pwd1..pwd10), history contains pwd6..pwd10 (newest 5)
    // pwd0..pwd5 should be reusable (outside history)
    for i in 0..=5 {
        let hash = hash_password(&format!("pwd{}", i));
        assert!(
            auth.can_reuse_password(&alice, &hash),
            "pwd{} should be reusable (outside history)",
            i
        );
    }
    // pwd6..pwd10 should NOT be reusable (in history)
    for i in 6..=10 {
        let hash = hash_password(&format!("pwd{}", i));
        assert!(
            !auth.can_reuse_password(&alice, &hash),
            "pwd{} should be in history (cannot reuse)",
            i
        );
    }
}

// === Policy enforcement ===

#[test]
fn test_default_policy_90_days() {
    let auth = AuthManager::new();
    let policy = auth.password_policy();
    assert_eq!(policy.lifetime_days, 90);
    assert_eq!(policy.history_size, 5);
    assert!(policy.enforce_on_write);
}

#[test]
fn test_set_policy_at_runtime() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("password123"))
        .unwrap();

    auth.set_password_policy(PasswordPolicy {
        lifetime_days: 30,
        history_size: 3,
        enforce_on_write: true,
    });

    let policy = auth.password_policy();
    assert_eq!(policy.lifetime_days, 30);
    assert_eq!(policy.history_size, 3);
}

// === Write blocking ===

#[test]
fn test_is_password_write_blocked_when_expired() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("password123"))
        .unwrap();

    assert!(!auth.is_password_write_blocked(&alice));
    auth.expire_password(&alice);
    assert!(auth.is_password_write_blocked(&alice));
}

#[test]
fn test_is_password_write_blocked_when_policy_disabled() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("password123"))
        .unwrap();

    auth.set_password_policy(PasswordPolicy::disabled());
    auth.expire_password(&alice);
    assert!(!auth.is_password_write_blocked(&alice));
}

// === Wildcard user (separate UserIdentity) ===

#[test]
fn test_wildcard_and_exact_are_independent() {
    let mut auth = AuthManager::new();
    // Create both wildcard and exact for same username
    let wildcard = UserIdentity::new("alice", "%");
    let exact = UserIdentity::new("alice", "localhost");
    auth.create_user(&wildcard, &hash_password("wildcard_pwd"))
        .unwrap();
    auth.create_user(&exact, &hash_password("exact_pwd"))
        .unwrap();

    // Expire wildcard only
    auth.expire_password(&wildcard);

    // Exact match should NOT be affected (separate UserIdentity entries)
    assert!(
        !auth.is_password_expired(&exact),
        "Exact match should be independent of wildcard expiration"
    );
    assert!(auth.authenticate(&exact, "exact_pwd").is_ok());

    // Wildcard should be expired
    assert!(auth.is_password_expired(&wildcard));
}

// === Multiple users ===

#[test]
fn test_multiple_users_independent() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    let bob = UserIdentity::new("bob", "localhost");
    let carol = UserIdentity::new("carol", "localhost");

    auth.create_user(&alice, &hash_password("alice_pwd"))
        .unwrap();
    auth.create_user(&bob, &hash_password("bob_pwd")).unwrap();
    auth.create_user(&carol, &hash_password("carol_pwd"))
        .unwrap();

    auth.expire_password(&bob);

    assert!(auth.authenticate(&alice, "alice_pwd").is_ok());
    assert!(auth.authenticate(&carol, "carol_pwd").is_ok());

    let result = auth.authenticate(&bob, "bob_pwd");
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("expired"));
}

// === Password change clears expired flag ===

#[test]
fn test_reauthenticate_after_expire_then_change() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("old_password"))
        .unwrap();

    auth.expire_password(&alice);
    assert!(auth.authenticate(&alice, "old_password").is_err());

    auth.set_password_hash(&alice, &hash_password("new_password"))
        .unwrap();
    assert!(auth.authenticate(&alice, "new_password").is_ok());
    assert!(!auth.is_password_expired(&alice));
}

// === Clone preserves state ===

#[test]
fn test_auth_manager_clone_preserves_password_rotation() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    auth.create_user(&alice, &hash_password("password123"))
        .unwrap();

    auth.expire_password(&alice);

    let auth2 = auth.clone();
    assert!(auth2.is_password_expired(&alice));
    assert!(auth2.authenticate(&alice, "password123").is_err());
}

// === Edge cases ===

#[test]
fn test_set_password_hash_user_not_found() {
    let mut auth = AuthManager::new();
    let alice = UserIdentity::new("alice", "localhost");
    let result = auth.set_password_hash(&alice, &hash_password("pwd"));
    assert!(result.is_err());
}

#[test]
fn test_special_characters_in_username() {
    let mut auth = AuthManager::new();
    let special = UserIdentity::new("alice@#$,", "localhost");
    let result = auth.create_user(&special, &hash_password("password123"));
    assert!(result.is_ok());
}
