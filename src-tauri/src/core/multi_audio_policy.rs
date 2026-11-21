//! Multi-audio retries are a user-visible, bounded choice, not a transport default.
pub fn should_retry(retries_used: u32, cancelled: bool) -> bool {
    !cancelled && retries_used == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exactly_one_automatic_retry_and_never_on_cancel() {
        assert!(should_retry(0, false));
        for attempts in [1, 2, 3, 100, u32::MAX] {
            assert!(!should_retry(attempts, false));
        }
        assert!(!should_retry(0, true));
    }
}
