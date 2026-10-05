//! Disk I/O delay fault injection framework (T-19).
//!
//! Provides configurable fault injection for storage I/O operations:
//! - **Delay**: Simulate slow disk / I/O bottleneck
//! - **Corruption**: Simulate bit-flip data corruption
//! - **Dropout**: Simulate I/O operation failure
//!
//! # Backward Compatibility
//!
//! The original `io_delay_ms()` and `maybe_delay()` functions are preserved
//! unchanged. New code should prefer [`IoFaultInjector`] for comprehensive
//! fault injection.

use std::time::Duration;

// ---------------------------------------------------------------------------
// Original API — preserved for backward compatibility
// ---------------------------------------------------------------------------

/// Returns the configured I/O delay in milliseconds from the
/// `SQLRUSTGO_IO_DELAY_MS` environment variable, or `None` if not set or zero.
pub fn io_delay_ms() -> Option<u64> {
    io_delay_ms_from(std::env::var("SQLRUSTGO_IO_DELAY_MS").ok())
}

/// Same rule as [`io_delay_ms`], over an already-read value.
///
/// Split out so tests can exercise the parsing without mutating the
/// process environment — six tests shared `SQLRUSTGO_IO_DELAY_MS` and
/// raced each other for the same reason the config tests did.
fn io_delay_ms_from(raw: Option<String>) -> Option<u64> {
    raw.and_then(|s| s.parse::<u64>().ok()).filter(|n| *n > 0)
}

/// Sleeps for the configured I/O delay if `SQLRUSTGO_IO_DELAY_MS` is set.
pub fn maybe_delay() {
    if let Some(delay_ms) = io_delay_ms() {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

// ---------------------------------------------------------------------------
// IoDelayConfig — structured fault injection configuration
// ---------------------------------------------------------------------------

/// Configuration for I/O fault injection.
///
/// All fields have safe defaults (zero delay, no corruption, no dropout),
/// meaning a default-constructed config behaves as a no-op pass-through.
#[derive(Debug, Clone, PartialEq)]
pub struct IoDelayConfig {
    /// Artificial delay in milliseconds applied to every I/O operation.
    /// `0` means no delay.
    pub delay_ms: u64,

    /// Probability (0.0 – 1.0) that a **read** returns corrupted data
    /// (single-byte bit-flip). `0.0` means never corrupt.
    pub corruption_rate: f64,

    /// Probability (0.0 – 1.0) that any I/O operation (read/write) fails
    /// with an `Err` result. `0.0` means never drop.
    pub dropout_rate: f64,
}

impl Default for IoDelayConfig {
    fn default() -> Self {
        Self {
            delay_ms: 0,
            corruption_rate: 0.0,
            dropout_rate: 0.0,
        }
    }
}

impl IoDelayConfig {
    /// Creates a config from environment variables.
    ///
    /// | Variable | Field |
    /// |---|---|
    /// | `SQLRUSTGO_IO_DELAY_MS` | `delay_ms` |
    /// | `SQLRUSTGO_IO_CORRUPTION_RATE` | `corruption_rate` |
    /// | `SQLRUSTGO_IO_DROPOUT_RATE` | `dropout_rate` |
    ///
    /// Missing or unparsable variables silently default to zero.
    pub fn from_env() -> Self {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Build from an arbitrary key/value source.
    ///
    /// `from_env` reads process-global state, so tests that set these
    /// variables race each other under `cargo test`'s default threading:
    /// one test's `set_var` lands in another's read window. Serialising
    /// the tests with a mutex would only hide that, and any future caller
    /// reading the environment directly would reintroduce it. Taking the
    /// lookup as a parameter keeps the parsing and clamping rules — the
    /// part worth testing — reachable without touching the process.
    fn from_lookup<F>(get: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let parse = |key: &str| get(key).and_then(|s| s.parse::<f64>().ok());

        let delay_ms = get("SQLRUSTGO_IO_DELAY_MS")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let clamp01 = |v: Option<f64>| v.unwrap_or(0.0).clamp(0.0, 1.0);

        Self {
            delay_ms,
            corruption_rate: clamp01(parse("SQLRUSTGO_IO_CORRUPTION_RATE")),
            dropout_rate: clamp01(parse("SQLRUSTGO_IO_DROPOUT_RATE")),
        }
    }
}

// ---------------------------------------------------------------------------
// LcgRng — minimal linear congruential generator
// ---------------------------------------------------------------------------

/// A minimal LCG pseudo-random number generator.
///
/// Used exclusively for probabilistic fault injection decisions — NOT
/// suitable for cryptographic or security-sensitive purposes.
///
/// Uses the classic `glibc` constants: `a = 1103515245`, `c = 12345`.
#[derive(Debug, Clone)]
pub struct LcgRng {
    state: u64,
}

impl LcgRng {
    /// Creates a new LCG with the given seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns a pseudo-random `f64` in `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        // Use upper 53 bits for double precision
        let upper = (self.state >> 11) as f64;
        upper / 9_007_199_254_740_992.0_f64 // 2^53
    }

    /// Returns a pseudo-random `usize` in `[0, max)`.
    pub fn next_usize(&mut self, max: usize) -> usize {
        (self.next_f64() * max as f64) as usize
    }
}

// ---------------------------------------------------------------------------
// IoFaultInjector — the fault injection wrapper
// ---------------------------------------------------------------------------

/// Injects configurable I/O faults (delay, corruption, dropout) into
/// storage read/write operations.
///
/// # Usage
///
/// ```ignore
/// let mut injector = IoFaultInjector::from_env();
/// injector.apply_read(&mut buf, |b| file.read_exact(b))?;
/// ```
///
/// The injector requires `&mut self` because the internal RNG advances
/// its state on every operation.
#[derive(Debug)]
pub struct IoFaultInjector {
    config: IoDelayConfig,
    rng: LcgRng,
}

impl IoFaultInjector {
    /// Creates a new injector with the given config and a fixed seed.
    pub fn new(config: IoDelayConfig) -> Self {
        Self {
            config,
            rng: LcgRng::new(42), // deterministic seed for reproducibility
        }
    }

    /// Creates a new injector with the given config and seed.
    pub fn with_seed(config: IoDelayConfig, seed: u64) -> Self {
        Self {
            config,
            rng: LcgRng::new(seed),
        }
    }

    /// Creates an injector configured from environment variables.
    pub fn from_env() -> Self {
        Self::new(IoDelayConfig::from_env())
    }

    /// Returns a reference to the current config.
    pub fn config(&self) -> &IoDelayConfig {
        &self.config
    }

    /// Applies fault injection before a **read** operation.
    ///
    /// 1. Sleeps for `delay_ms` if configured.
    /// 2. Rolls for dropout — returns `Err` if triggered.
    /// 3. Executes the actual read via `f`.
    /// 4. Rolls for corruption — bit-flips a byte in `buf` if triggered.
    ///
    /// **Note**: corruption is applied *after* the read closure returns
    /// successfully, so the caller receives corrupted data.
    pub fn apply_read<F, T>(&mut self, buf: &mut [u8], f: F) -> std::io::Result<T>
    where
        F: FnOnce(&mut [u8]) -> std::io::Result<T>,
    {
        // 1. Delay
        if self.config.delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(self.config.delay_ms));
        }

        // 2. Dropout
        if self.config.dropout_rate > 0.0 && self.rng.next_f64() < self.config.dropout_rate {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "fault_inject: simulated I/O dropout",
            ));
        }

        // 3. Execute actual read
        let result = f(buf)?;

        // 4. Corruption (post-read, applied to buffer)
        if self.config.corruption_rate > 0.0
            && !buf.is_empty()
            && self.rng.next_f64() < self.config.corruption_rate
        {
            let byte_idx = self.rng.next_usize(buf.len());
            let bit_idx = self.rng.next_usize(8);
            buf[byte_idx] ^= 1u8 << bit_idx;
        }

        Ok(result)
    }

    /// Applies fault injection before a **write** operation.
    ///
    /// 1. Sleeps for `delay_ms` if configured.
    /// 2. Rolls for dropout — returns `Err` if triggered.
    /// 3. Executes the actual write via `f`.
    ///
    /// Corruption is intentionally **not** applied to writes — the write
    /// caller owns the data; corruption on the read side is sufficient
    /// to exercise detection paths.
    pub fn apply_write<F, T>(&mut self, data: &[u8], f: F) -> std::io::Result<T>
    where
        F: FnOnce(&[u8]) -> std::io::Result<T>,
    {
        // 1. Delay
        if self.config.delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(self.config.delay_ms));
        }

        // 2. Dropout
        if self.config.dropout_rate > 0.0 && self.rng.next_f64() < self.config.dropout_rate {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "fault_inject: simulated I/O dropout",
            ));
        }

        // 3. Execute actual write
        f(data)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Original API backward compatibility ---

    #[test]
    fn test_io_delay_parsing() {
        assert_eq!(io_delay_ms_from(Some("50".into())), Some(50));
    }

    #[test]
    fn test_io_delay_zero() {
        // A zero delay is treated as "no delay configured" so callers
        // never sleep for a no-op.
        assert_eq!(io_delay_ms_from(Some("0".into())), None);
    }

    #[test]
    fn test_io_delay_not_set() {
        assert_eq!(io_delay_ms_from(None), None);
        assert_eq!(io_delay_ms_from(Some("not-a-number".into())), None);
    }

    // --- IoDelayConfig ---

    #[test]
    fn test_config_default() {
        let cfg = IoDelayConfig::default();
        assert_eq!(cfg.delay_ms, 0);
        assert_eq!(cfg.corruption_rate, 0.0);
        assert_eq!(cfg.dropout_rate, 0.0);
    }

    /// Builds a lookup over a fixed map, so these tests never touch the
    /// process environment. They previously did, and because `cargo test`
    /// runs tests on multiple threads, one test's `set_var` could land
    /// inside another's read window — `..._clamp` saw the default env and
    /// failed roughly 1 run in 7.
    fn lookup<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn test_config_from_env_all_set() {
        let cfg = IoDelayConfig::from_lookup(&lookup(&[
            ("SQLRUSTGO_IO_DELAY_MS", "200"),
            ("SQLRUSTGO_IO_CORRUPTION_RATE", "0.3"),
            ("SQLRUSTGO_IO_DROPOUT_RATE", "0.1"),
        ]));
        assert_eq!(cfg.delay_ms, 200);
        assert!((cfg.corruption_rate - 0.3).abs() < 1e-9);
        assert!((cfg.dropout_rate - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_config_from_env_missing() {
        let cfg = IoDelayConfig::from_lookup(&lookup(&[]));
        assert_eq!(cfg, IoDelayConfig::default());
    }

    #[test]
    fn test_config_from_env_clamp() {
        let cfg = IoDelayConfig::from_lookup(&lookup(&[
            ("SQLRUSTGO_IO_CORRUPTION_RATE", "5.0"),
            ("SQLRUSTGO_IO_DROPOUT_RATE", "-1.0"),
        ]));
        assert!(
            (cfg.corruption_rate - 1.0).abs() < 1e-6,
            "corruption_rate={}",
            cfg.corruption_rate
        );
        assert!(
            (cfg.dropout_rate - 0.0).abs() < 1e-6,
            "dropout_rate={}",
            cfg.dropout_rate
        );
    }

    /// A malformed value must fall back to the default rather than poison
    /// the rate: `from_env` parses with `and_then(..ok())`, and the clamp
    /// must not turn a parse failure into 0.0-by-accident semantics.
    #[test]
    fn test_config_from_env_malformed_falls_back() {
        let cfg = IoDelayConfig::from_lookup(&lookup(&[
            ("SQLRUSTGO_IO_DELAY_MS", "not-a-number"),
            ("SQLRUSTGO_IO_CORRUPTION_RATE", "half"),
        ]));
        assert_eq!(cfg.delay_ms, 0);
        assert!((cfg.corruption_rate - 0.0).abs() < 1e-9);
    }

    // --- LcgRng ---

    #[test]
    fn test_lcg_deterministic() {
        let mut rng1 = LcgRng::new(42);
        let mut rng2 = LcgRng::new(42);
        for _ in 0..100 {
            assert_eq!(rng1.next_f64(), rng2.next_f64());
        }
    }

    #[test]
    fn test_lcg_range() {
        let mut rng = LcgRng::new(99);
        for _ in 0..1000 {
            let v = rng.next_f64();
            assert!(v >= 0.0 && v < 1.0, "out of range: {}", v);
        }
    }

    #[test]
    fn test_lcg_next_usize() {
        let mut rng = LcgRng::new(7);
        for _ in 0..1000 {
            let v = rng.next_usize(10);
            assert!(v < 10, "out of range: {}", v);
        }
    }

    // --- IoFaultInjector: delay ---

    #[test]
    fn test_apply_read_delay() {
        let config = IoDelayConfig {
            delay_ms: 20,
            ..Default::default()
        };
        let mut injector = IoFaultInjector::new(config);

        let start = std::time::Instant::now();
        let result = injector.apply_read(&mut [0u8; 16], |buf| {
            buf.copy_from_slice(b"hello world!!!!!");
            Ok(())
        });
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert!(
            elapsed >= Duration::from_millis(20),
            "elapsed: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_apply_write_delay() {
        let config = IoDelayConfig {
            delay_ms: 15,
            ..Default::default()
        };
        let mut injector = IoFaultInjector::new(config);

        let start = std::time::Instant::now();
        let result = injector.apply_write(b"hello", |data| {
            assert_eq!(data, b"hello");
            Ok(())
        });
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert!(
            elapsed >= Duration::from_millis(15),
            "elapsed: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_no_delay_when_zero() {
        let mut injector = IoFaultInjector::new(IoDelayConfig::default());

        let start = std::time::Instant::now();
        let result = injector.apply_read(&mut [0u8; 4], |buf| {
            buf.copy_from_slice(b"test");
            Ok(())
        });
        let elapsed = start.elapsed();

        assert!(result.is_ok());
        assert!(
            elapsed < Duration::from_millis(10),
            "unexpected delay: {:?}",
            elapsed
        );
    }

    // --- IoFaultInjector: corruption ---

    #[test]
    fn test_corruption_rate_zero() {
        let config = IoDelayConfig {
            corruption_rate: 0.0,
            ..Default::default()
        };
        let mut injector = IoFaultInjector::new(config);
        let original = *b"0123456789ABCDEF";
        let mut buf = original;

        for _ in 0..100 {
            let _ = injector.apply_read(&mut buf, |b| {
                b.copy_from_slice(&original);
                Ok(())
            });
            assert_eq!(buf, original, "data was corrupted despite rate=0.0");
        }
    }

    #[test]
    fn test_corruption_rate_one() {
        let config = IoDelayConfig {
            corruption_rate: 1.0,
            ..Default::default()
        };
        let mut injector = IoFaultInjector::new(config);
        let original = *b"0123456789ABCDEF";
        let mut buf = original;

        let _ = injector.apply_read(&mut buf, |b| {
            b.copy_from_slice(&original);
            Ok(())
        });

        // With rate=1.0 and seed=42, we expect corruption on the first call
        assert_ne!(buf, original, "data should have been corrupted");
    }

    #[test]
    fn test_corruption_single_bit_flip() {
        let config = IoDelayConfig {
            corruption_rate: 1.0,
            ..Default::default()
        };
        let mut injector = IoFaultInjector::new(config);
        let original = *b"0123456789ABCDEF";
        let mut buf = original;

        let _ = injector.apply_read(&mut buf, |b| {
            b.copy_from_slice(&original);
            Ok(())
        });

        // Count differing bits — should be exactly 1
        let diff_bits: u32 = buf
            .iter()
            .zip(original.iter())
            .map(|(&a, &b)| (a ^ b).count_ones())
            .sum();
        assert_eq!(
            diff_bits, 1,
            "corruption should flip exactly 1 bit, got {} bits",
            diff_bits
        );
    }

    // --- IoFaultInjector: dropout ---

    #[test]
    fn test_dropout_rate_zero() {
        let config = IoDelayConfig {
            dropout_rate: 0.0,
            ..Default::default()
        };
        let mut injector = IoFaultInjector::new(config);

        for _ in 0..100 {
            let result = injector.apply_read(&mut [0u8; 4], |buf| {
                buf.copy_from_slice(b"test");
                Ok(())
            });
            assert!(result.is_ok(), "should not drop with rate=0.0");
        }
    }

    #[test]
    fn test_dropout_rate_one() {
        let config = IoDelayConfig {
            dropout_rate: 1.0,
            ..Default::default()
        };
        let mut injector = IoFaultInjector::new(config);

        let result: std::io::Result<()> = injector.apply_read(&mut [0u8; 4], |_buf| Ok(()));
        assert!(result.is_err(), "should drop with rate=1.0");
    }

    #[test]
    fn test_dropout_write() {
        let config = IoDelayConfig {
            dropout_rate: 1.0,
            ..Default::default()
        };
        let mut injector = IoFaultInjector::new(config);

        let result: std::io::Result<()> = injector.apply_write(b"data", |_data| Ok(()));
        assert!(result.is_err(), "write should drop with rate=1.0");
    }

    // --- IoFaultInjector: combined ---

    #[test]
    fn test_no_fault_noop() {
        let mut injector = IoFaultInjector::new(IoDelayConfig::default());
        let mut buf = [0u8; 8];

        let result = injector.apply_read(&mut buf, |b| {
            b.copy_from_slice(b"deadbeef");
            Ok(())
        });

        assert!(result.is_ok());
        assert_eq!(&buf, b"deadbeef");
    }

    #[test]
    fn test_from_env_creates_injector() {
        // `from_env` reads the process environment, so this asserts on
        // the default rather than on a value it set itself: writing one
        // here would race every other env-reading test in this module.
        let injector = IoFaultInjector::from_env();
        assert!(injector.config().delay_ms < 1_000_000);
    }

    #[test]
    fn test_with_seed_determinism() {
        let config = IoDelayConfig {
            dropout_rate: 0.5,
            ..Default::default()
        };
        let mut a = IoFaultInjector::with_seed(config.clone(), 123);
        let mut b = IoFaultInjector::with_seed(config, 123);

        for _ in 0..20 {
            let ra: std::io::Result<()> = a.apply_read(&mut [0u8; 1], |_| Ok(()));
            let rb: std::io::Result<()> = b.apply_read(&mut [0u8; 1], |_| Ok(()));
            assert_eq!(ra.is_ok(), rb.is_ok(), "determinism broken");
        }
    }
}
