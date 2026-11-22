//! Redact sensitive diagnostic classes before logs reach UI, memory or exports.
use regex::Regex;
use std::sync::OnceLock;

pub fn redact_diagnostics(line: &str) -> String {
    static URL: OnceLock<Regex> = OnceLock::new();
    static HEADER: OnceLock<Regex> = OnceLock::new();
    static FIELD: OnceLock<Regex> = OnceLock::new();
    let urls = URL
        .get_or_init(|| Regex::new(r#"(?i)\b(?:https?|socks(?:4a?|5h?)?)://[^\s\"'<>]+"#).unwrap());
    let headers = HEADER.get_or_init(|| {
        Regex::new(r"(?i)\b(?:authorization|proxy-authorization|cookie|set-cookie|x-api-key)\s*:")
            .unwrap()
    });
    let fields=FIELD.get_or_init(|| Regex::new(r"(?i)\b(password|passwd|secret|api[_-]?key|access[_-]?token|refresh[_-]?token)\s*[:=]\s*\S+").unwrap());
    line.lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            // Debug argv and header dumps have too many serialization forms to
            // safely preserve. Ordinary progress/warnings remain useful below.
            if lower.contains("[debug]")
                || lower.contains("[libreget] $")
                || [
                    "--password",
                    "--video-password",
                    "--proxy",
                    "--add-header",
                    "--cookies",
                    "--netrc",
                    "--username",
                ]
                .iter()
                .any(|flag| lower.contains(flag))
            {
                return "[sensitive diagnostic command omitted]".to_string();
            }
            let line = if let Some(header) = headers.find(line) {
                format!("{} [REDACTED]", &line[..header.end()])
            } else {
                line.to_string()
            };
            let line = urls.replace_all(&line, |caps: &regex::Captures<'_>| {
                url::Url::parse(&caps[0])
                    .ok()
                    .and_then(|u| {
                        u.host_str()
                            .map(|host| format!("{}://{host}/[REDACTED]", u.scheme()))
                    })
                    .unwrap_or_else(|| "[URL REDACTED]".to_string())
            });
            fields.replace_all(&line, "$1: [REDACTED]").into_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_secrets_are_removed_from_all_requested_classes() {
        for (input, secret) in [
            (
                "ERROR: https://cdn.example.invalid/video?Expires=55&Policy=SYNTHETIC_SIGNED_URL",
                "SYNTHETIC_SIGNED_URL",
            ),
            (
                "fetch http://synthetic-user:SYNTHETIC_PASSWORD@proxy.invalid:8080/video",
                "SYNTHETIC_PASSWORD",
            ),
            (
                "Cookie: session=SYNTHETIC_COOKIE; other=also-secret",
                "SYNTHETIC_COOKIE",
            ),
            ("Authorization: Basic SYNTHETIC_BASIC", "SYNTHETIC_BASIC"),
            (
                "Proxy-Authorization: Bearer SYNTHETIC_BEARER",
                "SYNTHETIC_BEARER",
            ),
            (
                "[debug] Command-line config: ['--password', 'SYNTHETIC_ARGV']",
                "SYNTHETIC_ARGV",
            ),
            (
                "[libreget] $ yt-dlp --add-header 'X-Auth: SYNTHETIC_HEADER'",
                "SYNTHETIC_HEADER",
            ),
            (
                "proxy socks5://user:SYNTHETIC_SOCKS@localhost:1080",
                "SYNTHETIC_SOCKS",
            ),
            ("password: SYNTHETIC_FIELD", "SYNTHETIC_FIELD"),
        ] {
            let safe = redact_diagnostics(input);
            assert!(!safe.contains(secret), "{safe}");
        }
    }
    #[test]
    fn ordinary_progress_survives_and_redaction_is_idempotent() {
        let progress = "[download] 42.0% of 10 MiB at 1 MiB/s";
        assert_eq!(redact_diagnostics(progress), progress);
        let safe = redact_diagnostics("Arabic عنوان https://cdn.example.invalid/path?token=secret");
        assert_eq!(redact_diagnostics(&safe), safe);
    }
}
