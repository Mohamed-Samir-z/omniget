use std::path::Path;

/// A saved artifact must exist and contain bytes before it can count as support-prompt success.
pub fn saved_artifact_exists(path: &Path) -> bool {
    let Ok(meta) = path.metadata() else {
        return false;
    };
    if meta.is_file() {
        return meta.len() > 0;
    }
    if !meta.is_dir() {
        return false;
    }
    std::fs::read_dir(path).ok().is_some_and(|items| {
        items
            .take(1000)
            .filter_map(Result::ok)
            .any(|item| item.metadata().is_ok_and(|m| m.is_file() && m.len() > 0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_empty_and_nonempty_artifacts_are_distinct() {
        let dir = std::env::temp_dir().join(format!(
            "omniget-support-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("video.mp4");
        assert!(!saved_artifact_exists(&file));
        assert!(!saved_artifact_exists(&dir));
        std::fs::write(&file, []).unwrap();
        assert!(!saved_artifact_exists(&file));
        assert!(!saved_artifact_exists(&dir));
        std::fs::write(&file, b"saved media fixture").unwrap();
        assert!(saved_artifact_exists(&file));
        assert!(saved_artifact_exists(&dir));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
