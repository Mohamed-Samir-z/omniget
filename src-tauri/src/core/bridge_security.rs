use std::sync::atomic::{AtomicI64, Ordering};

/// Origins are structured identities, never string prefixes. Browser extension
/// hosts vary per installation, but ordinary web hosts must match exactly.
pub fn is_allowed_pair_origin(origin: &str) -> bool {
    if origin.is_empty() || origin.bytes().any(|b| b.is_ascii_whitespace()) {
        return false;
    }
    let Ok(url) = url::Url::parse(origin) else {
        return false;
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    match url.scheme() {
        "chrome-extension" | "moz-extension" | "safari-web-extension" => {
            url.port().is_none()
                && !host.is_empty()
                && host.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        }
        "tauri" => host == "localhost" && url.port().is_none(),
        "http" => matches!(host, "localhost" | "127.0.0.1"),
        "https" => host == "localhost",
        _ => false,
    }
}

/// Expiry check and consumption form one atomic operation, so concurrent
/// successful requests cannot each obtain the installation's bearer token.
pub fn consume_pairing_window(deadline: &AtomicI64, now: i64) -> bool {
    deadline
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |until| {
            (until > now).then_some(0)
        })
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_expected_origins_pass() {
        for origin in [
            "http://localhost",
            "http://localhost:5173",
            "http://127.0.0.1:47720",
            "https://localhost",
            "tauri://localhost",
            "chrome-extension://abcdefghijklmnop",
            "moz-extension://ad5f1e2e-1111-4444-9999-123456789abc",
            "safari-web-extension://extension-id",
        ] {
            assert!(is_allowed_pair_origin(origin), "{origin}");
        }
    }
    #[test]
    fn prefix_lookalikes_and_malformed_origins_fail() {
        for origin in [
            "https://localhost.attacker.invalid",
            "http://localhost.attacker.invalid",
            "http://127.0.0.1.attacker.invalid",
            "https://localhost@attacker.invalid",
            "http://attacker.invalid@localhost",
            "chrome-extension://good@attacker.invalid",
            "tauri://attacker.invalid",
            "https://example.invalid",
            "null",
            "",
            " https://localhost",
            "https://localhost ",
            "https://localhost/path",
            "https://localhost?x=1",
            "https://localhost#fragment",
            "chrome-extension://id/path",
            "chrome-extension://id:80",
            "http://localhost:bad",
            "http://localhost\n",
        ] {
            assert!(!is_allowed_pair_origin(origin), "{origin}");
        }
    }
    #[test]
    fn closed_and_expired_windows_fail() {
        for until in [0, 99, 100] {
            assert!(!consume_pairing_window(&AtomicI64::new(until), 100));
        }
        let until = AtomicI64::new(101);
        assert!(consume_pairing_window(&until, 100));
        assert!(!consume_pairing_window(&until, 100));
    }
    #[test]
    fn concurrent_requests_have_exactly_one_winner() {
        let until = std::sync::Arc::new(AtomicI64::new(200));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(32));
        let threads: Vec<_> = (0..32)
            .map(|_| {
                let until = until.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    consume_pairing_window(&until, 100)
                })
            })
            .collect();
        assert_eq!(
            threads
                .into_iter()
                .filter_map(|t| t.join().ok())
                .filter(|winner| *winner)
                .count(),
            1
        );
    }
}
