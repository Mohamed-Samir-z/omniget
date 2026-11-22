//! Public title/thumbnail only. Never substitutes for full download/format metadata.
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct QuickPreview {
    pub url: String,
    pub title: String,
    pub author: String,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<f64>,
}

pub fn is_youtube_url(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && matches!(
            url.host_str(),
            Some(
                "youtube.com"
                    | "www.youtube.com"
                    | "m.youtube.com"
                    | "music.youtube.com"
                    | "youtu.be"
            )
        )
}

pub fn parse(value: serde_json::Value, source_url: &str) -> Option<QuickPreview> {
    let title = value["title"].as_str()?.trim();
    if title.is_empty() {
        return None;
    }
    let thumbnail_url = value["thumbnail_url"].as_str().and_then(|value| {
        let parsed = url::Url::parse(value).ok()?;
        let host = parsed.host_str()?;
        (parsed.scheme() == "https"
            && (host == "ytimg.com" || host.ends_with(".ytimg.com") || host == "img.youtube.com"))
            .then(|| value.to_string())
    });
    Some(QuickPreview {
        url: source_url.to_string(),
        title: title.chars().take(500).collect(),
        author: value["author_name"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(250)
            .collect(),
        thumbnail_url,
        duration_seconds: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_only_youtube_not_lookalikes_or_other_schemes() {
        assert!(is_youtube_url("https://youtu.be/video"));
        assert!(!is_youtube_url(
            "https://youtube.com.attacker.invalid/video"
        ));
        assert!(!is_youtube_url("file://youtube.com/video"));
    }
    #[test]
    fn ignores_embed_html_and_untrusted_thumbnails_without_inventing_duration() {
        let preview=parse(serde_json::json!({"title":"Current video", "thumbnail_url":"https://attacker.invalid/image", "html":"<script>ignored</script>"}),"https://youtu.be/current").unwrap();
        assert!(preview.thumbnail_url.is_none());
        assert!(preview.duration_seconds.is_none());
        assert_eq!(preview.url, "https://youtu.be/current");
        assert!(parse(serde_json::json!({"title":""}), "url").is_none());
    }
}
