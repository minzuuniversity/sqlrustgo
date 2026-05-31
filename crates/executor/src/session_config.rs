use std::env;

/// Memory budget for hash join spill-to-disk (64MB default)
const DEFAULT_MAX_MEMORY_PER_QUERY: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub benchmark_mode: bool,
    pub teaching_mode: bool,
    pub cache_enabled: bool,
    pub stats_enabled: bool,
    /// Max memory per query before spill-to-disk is triggered
    pub max_memory_per_query: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        let benchmark_mode = env::var("SQLRUSTGO_BENCHMARK_MODE")
            .map(|v| v == "1")
            .unwrap_or(false);

        let teaching_mode = env::var("SQLRUSTGO_TEACHING_MODE")
            .map(|v| v == "1")
            .unwrap_or(false);

        let max_memory_per_query = env::var("SQLRUSTGO_MAX_MEMORY_MB")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .map(|mb| mb * 1024 * 1024)
            .unwrap_or(DEFAULT_MAX_MEMORY_PER_QUERY);

        Self {
            benchmark_mode,
            teaching_mode,
            cache_enabled: !benchmark_mode && !teaching_mode,
            stats_enabled: !benchmark_mode && !teaching_mode,
            max_memory_per_query,
        }
    }
}

impl SessionConfig {
    pub fn new(benchmark_mode: bool) -> Self {
        Self {
            benchmark_mode,
            teaching_mode: false,
            cache_enabled: !benchmark_mode,
            stats_enabled: !benchmark_mode,
            max_memory_per_query: DEFAULT_MAX_MEMORY_PER_QUERY,
        }
    }

    pub fn with_teaching_mode(teaching_mode: bool) -> Self {
        Self {
            benchmark_mode: false,
            teaching_mode,
            cache_enabled: !teaching_mode,
            stats_enabled: !teaching_mode,
            max_memory_per_query: DEFAULT_MAX_MEMORY_PER_QUERY,
        }
    }

    pub fn with_memory_limit(mut self, max_memory_mb: usize) -> Self {
        self.max_memory_per_query = max_memory_mb * 1024 * 1024;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_not_benchmark_mode() {
        std::env::remove_var("SQLRUSTGO_BENCHMARK_MODE");
        std::env::remove_var("SQLRUSTGO_TEACHING_MODE");
        let config = SessionConfig::default();
        // Just verify config can be created
        let _ = config;
    }

    #[test]
    fn test_benchmark_mode_from_env() {
        std::env::remove_var("SQLRUSTGO_TEACHING_MODE");
        std::env::set_var("SQLRUSTGO_BENCHMARK_MODE", "1");
        let config = SessionConfig::default();
        assert!(config.benchmark_mode);
        assert!(!config.teaching_mode);
        assert!(!config.cache_enabled);
        assert!(!config.stats_enabled);
        std::env::remove_var("SQLRUSTGO_BENCHMARK_MODE");
    }

    #[test]
    fn test_teaching_mode_from_env() {
        std::env::remove_var("SQLRUSTGO_BENCHMARK_MODE");
        std::env::remove_var("SQLRUSTGO_TEACHING_MODE");
        std::env::set_var("SQLRUSTGO_TEACHING_MODE", "1");
        let config = SessionConfig::default();
        assert!(!config.benchmark_mode);
        assert!(config.teaching_mode);
        assert!(!config.cache_enabled);
        assert!(!config.stats_enabled);
        std::env::remove_var("SQLRUSTGO_TEACHING_MODE");
    }

    #[test]
    fn test_teaching_mode_with_constructor() {
        let config = SessionConfig::with_teaching_mode(true);
        assert!(!config.benchmark_mode);
        assert!(config.teaching_mode);
        assert!(!config.cache_enabled);
        assert!(!config.stats_enabled);
    }

    #[test]
    fn test_explicit_benchmark_mode() {
        let config = SessionConfig::new(true);
        assert!(config.benchmark_mode);
        assert!(!config.teaching_mode);
        assert!(!config.cache_enabled);
        assert!(!config.stats_enabled);
    }

    #[test]
    fn test_explicit_normal_mode() {
        let config = SessionConfig::new(false);
        assert!(!config.benchmark_mode);
        assert!(!config.teaching_mode);
        assert!(config.cache_enabled);
        assert!(config.stats_enabled);
    }

    #[test]
    fn test_default_memory_limit() {
        let config = SessionConfig::default();
        assert_eq!(config.max_memory_per_query, 64 * 1024 * 1024);
    }

    #[test]
    fn test_with_memory_limit() {
        let config = SessionConfig::new(false).with_memory_limit(128);
        assert_eq!(config.max_memory_per_query, 128 * 1024 * 1024);
    }

    #[test]
    fn test_memory_limit_from_env() {
        std::env::set_var("SQLRUSTGO_MAX_MEMORY_MB", "256");
        let config = SessionConfig::default();
        assert_eq!(config.max_memory_per_query, 256 * 1024 * 1024);
        std::env::remove_var("SQLRUSTGO_MAX_MEMORY_MB");
    }
}
