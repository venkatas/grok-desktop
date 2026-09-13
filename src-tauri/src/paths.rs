use std::path::{Path, PathBuf};

pub fn grok_home() -> PathBuf {
    grok_home_in(std::env::var_os("GROK_HOME").map(PathBuf::from), dirs::home_dir())
}

pub fn grok_home_in(grok_home: Option<PathBuf>, home: Option<PathBuf>) -> PathBuf {
    if let Some(p) = grok_home {
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    home.unwrap_or_else(|| PathBuf::from("/tmp")).join(".grok")
}

pub fn recents_path() -> PathBuf {
    recents_path_in(dirs::data_dir())
}

pub fn recents_path_in(data_dir: Option<PathBuf>) -> PathBuf {
    data_dir
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("Grok Build")
        .join("state.json")
}

pub fn auth_json_path(home: &Path) -> PathBuf {
    home.join("auth.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grok_home_prefers_env() {
        assert_eq!(
            grok_home_in(Some(PathBuf::from("/tmp/ghome")), Some(PathBuf::from("/Users/a"))),
            PathBuf::from("/tmp/ghome")
        );
    }

    #[test]
    fn grok_home_defaults_to_dot_grok() {
        assert_eq!(
            grok_home_in(None, Some(PathBuf::from("/Users/a"))),
            PathBuf::from("/Users/a/.grok")
        );
    }

    #[test]
    fn recents_live_under_app_support() {
        assert_eq!(
            recents_path_in(Some(PathBuf::from("/Users/a/Library/Application Support"))),
            PathBuf::from("/Users/a/Library/Application Support/Grok Build/state.json")
        );
    }
}
