use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct DiffFile {
    pub path: String,
    pub added: i64,
    pub removed: i64,
    pub patch: String,
}

pub fn files_from_value(value: &Value) -> Result<Vec<DiffFile>, String> {
    let items = crate::agent::normalize_diffs(value);
    if items.is_empty() && !value.is_null() && value != &Value::Object(Default::default()) {
        if value.get("files").is_none() && value.get("diffs").is_none() && !value.is_array() {
            return Err("Could not load diffs".into());
        }
    }
    Ok(items
        .iter()
        .filter_map(|item| {
            let path = item
                .get("path")
                .or_else(|| item.get("file"))
                .and_then(|p| p.as_str())?
                .to_string();
            let added = item
                .get("added")
                .or_else(|| item.get("additions"))
                .and_then(|n| n.as_i64())
                .unwrap_or(0);
            let removed = item
                .get("removed")
                .or_else(|| item.get("deletions"))
                .and_then(|n| n.as_i64())
                .unwrap_or(0);
            let patch = item
                .get("patch")
                .or_else(|| item.get("diff"))
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            Some(DiffFile {
                path,
                added,
                removed,
                patch,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_files_array() {
        let v = json!({ "files": [{ "path": "a.rs", "added": 1, "removed": 0, "patch": "+x" }] });
        let files = files_from_value(&v).unwrap();
        assert_eq!(files[0].path, "a.rs");
        assert_eq!(files[0].added, 1);
    }
}
