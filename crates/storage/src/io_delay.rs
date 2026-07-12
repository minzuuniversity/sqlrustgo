use std::time::Duration;

pub fn io_delay_ms() -> Option<u64> {
    std::env::var("SQLRUSTGO_IO_DELAY_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
}

pub fn maybe_delay() {
    if let Some(delay_ms) = io_delay_ms() {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_delay_parsing() {
        std::env::set_var("SQLRUSTGO_IO_DELAY_MS", "50");
        assert_eq!(io_delay_ms(), Some(50));
        std::env::remove_var("SQLRUSTGO_IO_DELAY_MS");
    }

    #[test]
    fn test_io_delay_zero() {
        std::env::set_var("SQLRUSTGO_IO_DELAY_MS", "0");
        assert_eq!(io_delay_ms(), None);
        std::env::remove_var("SQLRUSTGO_IO_DELAY_MS");
    }

    #[test]
    fn test_io_delay_not_set() {
        std::env::remove_var("SQLRUSTGO_IO_DELAY_MS");
        assert_eq!(io_delay_ms(), None);
    }
}
