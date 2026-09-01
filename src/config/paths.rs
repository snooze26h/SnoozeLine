use std::path::{Path, PathBuf};

const HOME_ENV: &str = "SNOOZELINE_HOME";

pub(crate) fn root_dir() -> PathBuf {
    resolve_root(
        dirs::home_dir().as_deref(),
        std::env::var_os(HOME_ENV).as_deref().map(Path::new),
    )
}

pub(crate) fn config_file() -> PathBuf {
    root_dir().join("config.toml")
}

pub(crate) fn models_file() -> PathBuf {
    root_dir().join("models.toml")
}

pub(crate) fn themes_dir() -> PathBuf {
    root_dir().join("themes")
}

pub(crate) fn api_usage_cache_file() -> PathBuf {
    root_dir().join(".api_usage_cache.json")
}

fn resolve_root(home: Option<&Path>, override_path: Option<&Path>) -> PathBuf {
    if let Some(path) = override_path.filter(|path| path.is_absolute()) {
        return path.to_path_buf();
    }

    home.map(|path| path.join(".claude").join("snoozeline"))
        .unwrap_or_else(|| PathBuf::from(".claude").join("snoozeline"))
}

#[cfg(test)]
mod tests {
    use super::resolve_root;
    use std::path::Path;

    #[test]
    fn absolute_override_wins() {
        assert_eq!(
            resolve_root(
                Some(Path::new("/home/test")),
                Some(Path::new("/tmp/custom"))
            ),
            Path::new("/tmp/custom")
        );
    }

    #[test]
    fn relative_override_is_ignored() {
        assert_eq!(
            resolve_root(Some(Path::new("/home/test")), Some(Path::new("relative"))),
            Path::new("/home/test/.claude/snoozeline")
        );
    }
}
