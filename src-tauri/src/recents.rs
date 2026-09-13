use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecentsState {
    #[serde(default)]
    pub folders: Vec<String>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
}

pub fn load_recents(path: &Path) -> RecentsState {
    let Ok(raw) = fs::read_to_string(path) else {
        return RecentsState::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_recents(path: &Path, state: &RecentsState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

pub fn push_recent(mut state: RecentsState, folder: String) -> RecentsState {
    state.folders.retain(|f| f != &folder);
    state.folders.insert(0, folder);
    state.folders.truncate(20);
    state
}

pub fn remove_recent(mut state: RecentsState, folder: &str) -> RecentsState {
    state.folders.retain(|f| f != folder);
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn missing_file_is_empty() {
        let path = PathBuf::from("/tmp/gb-recents-does-not-exist.json");
        assert_eq!(load_recents(&path), RecentsState::default());
    }

    #[test]
    fn push_unique_to_front_and_cap_20() {
        let mut state = RecentsState::default();
        for i in 0..25 {
            state = push_recent(state, format!("/p/{i}"));
        }
        state = push_recent(state, "/p/3".into());
        assert_eq!(state.folders.len(), 20);
        assert_eq!(state.folders[0], "/p/3");
        assert!(!state.folders.contains(&"/p/0".to_string()));
    }

    #[test]
    fn round_trip_file() {
        let path = std::env::temp_dir().join(format!("gb-recents-{}.json", std::process::id()));
        let state = push_recent(RecentsState::default(), "/tmp/proj".into());
        save_recents(&path, &state).unwrap();
        assert_eq!(load_recents(&path).folders, vec!["/tmp/proj".to_string()]);
    }
}
