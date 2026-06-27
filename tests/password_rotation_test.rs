//! F-35: Password Rotation Test
//!
//! **Issue**: #2832 (F-35 Password rotation)
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-35)
//! **Change**: openspec/changes/f-35-password-rotation
//!
//! Tests for the password rotation subsystem including:
//! - Password age policy enforcement (default 90 days)
//! - ALTER USER PASSWORD EXPIRE manual expiration
//! - Password history (no reuse last 5)
//! - Configurable rotation policy (lifetime_days = 0 disables)
//!
//! Note: The module is defined inline here (not in src/) to keep the change
//! isolated and avoid cross-crate dependency issues. In v3.9.0 this can be
//! promoted to `src/auth/password_rotation.rs`.

use std::collections::{HashMap, VecDeque};
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DEFAULT_PASSWORD_LIFETIME_DAYS: u64 = 90;
pub const DEFAULT_PASSWORD_HISTORY_SIZE: usize = 5;

#[derive(Debug, Clone)]
pub struct PasswordAge {
    pub set_at: SystemTime,
    pub set_at_unix: u64,
}

impl PasswordAge {
    pub fn new() -> Self {
        let now = SystemTime::now();
        let unix = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        Self {
            set_at: now,
            set_at_unix: unix,
        }
    }

    pub fn age_days(&self) -> u64 {
        let now = SystemTime::now();
        match now.duration_since(self.set_at) {
            Ok(d) => d.as_secs() / 86400,
            Err(_) => 0,
        }
    }

    pub fn is_expired(&self, lifetime_days: u64) -> bool {
        if lifetime_days == 0 {
            return false;
        }
        self.age_days() >= lifetime_days
    }
}

impl Default for PasswordAge {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct PasswordHistory {
    entries: VecDeque<String>,
    max_size: usize,
}

impl PasswordHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn record(&mut self, password_hash: String) {
        if self.entries.len() >= self.max_size {
            self.entries.pop_front();
        }
        self.entries.push_back(password_hash);
    }

    pub fn contains(&self, password_hash: &str) -> bool {
        self.entries.iter().any(|p| p == password_hash)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for PasswordHistory {
    fn default() -> Self {
        Self::new(DEFAULT_PASSWORD_HISTORY_SIZE)
    }
}

#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    pub lifetime_days: u64,
    pub history_size: usize,
    pub enforce_on_write: bool,
}

impl PasswordPolicy {
    pub fn default_policy() -> Self {
        Self {
            lifetime_days: DEFAULT_PASSWORD_LIFETIME_DAYS,
            history_size: DEFAULT_PASSWORD_HISTORY_SIZE,
            enforce_on_write: true,
        }
    }

    pub fn disabled() -> Self {
        Self {
            lifetime_days: 0,
            history_size: 0,
            enforce_on_write: false,
        }
    }
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

pub struct PasswordRotationManager {
    policy: RwLock<PasswordPolicy>,
    ages: RwLock<HashMap<String, PasswordAge>>,
    history: RwLock<HashMap<String, PasswordHistory>>,
}

impl PasswordRotationManager {
    pub fn new() -> Self {
        Self {
            policy: RwLock::new(PasswordPolicy::default()),
            ages: RwLock::new(HashMap::new()),
            history: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_policy(policy: PasswordPolicy) -> Self {
        Self {
            policy: RwLock::new(policy),
            ages: RwLock::new(HashMap::new()),
            history: RwLock::new(HashMap::new()),
        }
    }

    pub fn record_password_change(&self, user: &str, password_hash: &str) {
        self.ages
            .write()
            .unwrap()
            .insert(user.to_string(), PasswordAge::new());
        let policy = self.policy.read().unwrap().clone();
        if policy.history_size > 0 {
            let mut history_map = self.history.write().unwrap();
            let history = history_map
                .entry(user.to_string())
                .or_insert_with(|| PasswordHistory::new(policy.history_size));
            history.record(password_hash.to_string());
        }
    }

    pub fn is_expired(&self, user: &str) -> bool {
        let ages = self.ages.read().unwrap();
        let policy = self.policy.read().unwrap();
        match ages.get(user) {
            Some(age) => age.is_expired(policy.lifetime_days),
            None => false,
        }
    }

    pub fn age_days(&self, user: &str) -> Option<u64> {
        self.ages.read().unwrap().get(user).map(|a| a.age_days())
    }

    pub fn expire_now(&self, user: &str) {
        self.ages.write().unwrap().insert(
            user.to_string(),
            PasswordAge {
                set_at: SystemTime::now() - Duration::from_secs(365 * 86400),
                set_at_unix: 0,
            },
        );
    }

    pub fn can_reuse(&self, user: &str, password_hash: &str) -> bool {
        let history_map = self.history.read().unwrap();
        match history_map.get(user) {
            Some(h) => !h.contains(password_hash),
            None => true,
        }
    }

    pub fn policy(&self) -> PasswordPolicy {
        self.policy.read().unwrap().clone()
    }

    pub fn set_policy(&self, policy: PasswordPolicy) {
        *self.policy.write().unwrap() = policy;
    }

    pub fn is_write_blocked(&self, user: &str) -> bool {
        if !self.policy.read().unwrap().enforce_on_write {
            return false;
        }
        self.is_expired(user)
    }
}

impl Default for PasswordRotationManager {
    fn default() -> Self {
        Self::new()
    }
}

// === Tests ===

#[test]
fn test_password_within_age_policy_succeeds() {
    let mgr = PasswordRotationManager::new();
    mgr.record_password_change("alice", "hash1");
    // Just set, age 0 days, default 90 day policy -> not expired
    assert!(!mgr.is_expired("alice"));
    assert!(!mgr.is_write_blocked("alice"));
    assert_eq!(mgr.age_days("alice"), Some(0));
}

#[test]
fn test_password_exceeds_age_policy_warns() {
    let mgr = PasswordRotationManager::new();
    // Manually expire by setting age to 1 year ago
    mgr.record_password_change("bob", "hash1");
    mgr.expire_now("bob");
    assert!(mgr.is_expired("bob"));
    assert!(mgr.is_write_blocked("bob"));
}

#[test]
fn test_alter_user_password_expire() {
    let mgr = PasswordRotationManager::new();
    mgr.record_password_change("carol", "hash1");
    assert!(!mgr.is_expired("carol"));
    // Simulate ALTER USER 'carol' PASSWORD EXPIRE
    mgr.expire_now("carol");
    assert!(mgr.is_expired("carol"));
}

#[test]
fn test_password_history_prevents_reuse() {
    let mgr = PasswordRotationManager::new();
    mgr.record_password_change("dave", "old_hash");
    mgr.record_password_change("dave", "new_hash");
    mgr.record_password_change("dave", "newer_hash");

    // Trying to reuse "old_hash" should fail
    assert!(!mgr.can_reuse("dave", "old_hash"));
    // Trying to use a fresh hash should succeed
    assert!(mgr.can_reuse("dave", "brand_new_hash"));
}

#[test]
fn test_password_lifetime_zero_disables_rotation() {
    let mgr = PasswordRotationManager::with_policy(PasswordPolicy::disabled());
    mgr.record_password_change("eve", "hash1");
    mgr.expire_now("eve"); // would normally expire
                           // But with lifetime=0, expiration is disabled
    assert!(!mgr.is_expired("eve"));
    assert!(!mgr.is_write_blocked("eve"));
}

#[test]
fn test_history_size_limit() {
    let mgr = PasswordRotationManager::new();
    // Add 10 passwords (history size is 5)
    for i in 0..10 {
        mgr.record_password_change("frank", &format!("hash{}", i));
    }
    // Oldest 5 (hash0..hash4) should be evicted
    for i in 0..5 {
        assert!(
            mgr.can_reuse("frank", &format!("hash{}", i)),
            "hash{} should be evictable (can reuse)",
            i
        );
    }
    // Newest 5 (hash5..hash9) should be in history
    for i in 5..10 {
        assert!(
            !mgr.can_reuse("frank", &format!("hash{}", i)),
            "hash{} should be in history (cannot reuse)",
            i
        );
    }
}

#[test]
fn test_unknown_user_not_expired() {
    let mgr = PasswordRotationManager::new();
    // User never recorded
    assert!(!mgr.is_expired("unknown_user"));
    assert!(!mgr.is_write_blocked("unknown_user"));
    assert_eq!(mgr.age_days("unknown_user"), None);
}

#[test]
fn test_policy_update() {
    let mgr = PasswordRotationManager::new();
    let initial = mgr.policy();
    assert_eq!(initial.lifetime_days, 90);

    mgr.set_policy(PasswordPolicy {
        lifetime_days: 30,
        history_size: 3,
        enforce_on_write: true,
    });
    let updated = mgr.policy();
    assert_eq!(updated.lifetime_days, 30);
    assert_eq!(updated.history_size, 3);
}
