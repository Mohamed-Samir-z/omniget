/// Fixed owner-approved profile destinations. No arbitrary URL is accepted.
pub fn profile_url(target: &str) -> Option<&'static str> {
    match target {
        "whatsapp" => Some("https://wa.me/201007569287"),
        "portfolio" => Some("https://mohamed-samir-portfolio.mo2868417.workers.dev"),
        "github" => Some("https://github.com/Mohamed-Samir-z"),
        "project" => Some("https://github.com/Mohamed-Samir-z/omniget"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_destinations_are_distinct_and_fixed() {
        assert_eq!(profile_url("whatsapp"), Some("https://wa.me/201007569287"));
        assert_eq!(
            profile_url("portfolio"),
            Some("https://mohamed-samir-portfolio.mo2868417.workers.dev")
        );
        assert_eq!(
            profile_url("github"),
            Some("https://github.com/Mohamed-Samir-z")
        );
        assert_eq!(
            profile_url("project"),
            Some("https://github.com/Mohamed-Samir-z/omniget")
        );
    }
    #[test]
    fn caller_cannot_supply_a_url_or_path() {
        for target in [
            "",
            "https://example.com",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "../portfolio",
            "PORTFOLIO",
        ] {
            assert_eq!(profile_url(target), None);
        }
    }
}
