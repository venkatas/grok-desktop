use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub fn encode_cwd(cwd: &Path) -> String {
    cwd.to_string_lossy()
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SessionRow {
    pub id: String,
    pub title: String,
    pub updated_at: String,
    pub model: Option<String>,
}

pub fn session_group_dir(home: &Path, cwd: &Path) -> PathBuf {
    home.join("sessions").join(encode_cwd(cwd))
}

fn title_from_summary(v: &serde_json::Value) -> String {
    v.get("generated_title")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("session_summary").and_then(|x| x.as_str()))
        .unwrap_or("Untitled")
        .to_string()
}

fn row_from_summary(path: &Path) -> Option<SessionRow> {
    let raw = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let id = v
        .pointer("/info/id")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("id").and_then(|x| x.as_str()))?
        .to_string();
    let updated_at = v
        .get("last_active_at")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("updated_at").and_then(|x| x.as_str()))
        .unwrap_or("")
        .to_string();
    let model = v
        .get("current_model_id")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    Some(SessionRow {
        id,
        title: title_from_summary(&v),
        updated_at,
        model,
    })
}

fn collect_group(group: &Path, out: &mut Vec<SessionRow>) {
    let Ok(rd) = fs::read_dir(group) else {
        return;
    };
    for ent in rd.flatten() {
        let summary = ent.path().join("summary.json");
        if let Some(row) = row_from_summary(&summary) {
            out.push(row);
        }
    }
}

pub fn list_sessions(home: &Path, cwd: &Path) -> Vec<SessionRow> {
    let sessions_root = home.join("sessions");
    let mut rows = Vec::new();
    collect_group(&session_group_dir(home, cwd), &mut rows);

    if let Ok(rd) = fs::read_dir(&sessions_root) {
        for ent in rd.flatten() {
            let cwd_file = ent.path().join(".cwd");
            if !cwd_file.is_file() {
                continue;
            }
            let Ok(stored) = fs::read_to_string(&cwd_file) else {
                continue;
            };
            if Path::new(stored.trim()) == cwd {
                collect_group(&ent.path(), &mut rows);
            }
        }
    }

    rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    rows.dedup_by(|a, b| a.id == b.id);
    rows
}

pub fn load_updates_jsonl(home: &Path, cwd: &Path, session_id: &str) -> Vec<serde_json::Value> {
    let path = session_group_dir(home, cwd)
        .join(session_id)
        .join("updates.jsonl");
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_full_path_like_grok() {
        assert_eq!(
            encode_cwd(Path::new("/Users/a/b")),
            "%2FUsers%2Fa%2Fb"
        );
    }

    #[test]
    fn lists_summary_json_for_encoded_cwd() {
        let root = std::env::temp_dir().join(format!("gb-sess-{}", std::process::id()));
        let cwd = PathBuf::from("/Users/a/proj");
        let id = "01aaaaaaaaaaaaaaaaaaaaaaaaaa";
        let dir = session_group_dir(&root, &cwd).join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("summary.json"),
            r#"{
              "info": { "id": "01aaaaaaaaaaaaaaaaaaaaaaaaaa", "cwd": "/Users/a/proj" },
              "generated_title": "Fix login",
              "updated_at": "2026-09-13T08:00:00Z",
              "current_model_id": "grok-4.6"
            }"#,
        )
        .unwrap();
        let rows = list_sessions(&root, &cwd);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].title, "Fix login");
        assert_eq!(rows[0].model.as_deref(), Some("grok-4.6"));
    }

    #[test]
    fn includes_cwd_fallback_groups() {
        let root = std::env::temp_dir().join(format!("gb-sess-cwd-{}", std::process::id()));
        let cwd = PathBuf::from("/very/long/path/that/would/be/hashed");
        let group = root.join("sessions").join("slug-hash");
        let dir = group.join("01bbbbbbbbbbbbbbbbbbbbbbbbbb");
        fs::create_dir_all(&dir).unwrap();
        fs::write(group.join(".cwd"), cwd.to_string_lossy().as_bytes()).unwrap();
        fs::write(
            dir.join("summary.json"),
            r#"{
              "info": { "id": "01bbbbbbbbbbbbbbbbbbbbbbbbbb" },
              "session_summary": "Hashed group",
              "last_active_at": "2026-09-13T09:00:00Z"
            }"#,
        )
        .unwrap();
        let rows = list_sessions(&root, &cwd);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Hashed group");
    }
}
