use std::path::Path;

pub fn is_signed_in_at(auth_json: &Path) -> bool {
    match std::fs::File::open(auth_json) {
        Ok(_) => auth_json.is_file(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("gb-auth-{}-{}", std::process::id(), name));
        let _ = fs::create_dir_all(&p);
        p
    }

    #[test]
    fn signed_in_when_file_readable() {
        let dir = tmp("ok");
        let path = dir.join("auth.json");
        fs::write(&path, b"{}").unwrap();
        assert!(is_signed_in_at(&path));
    }

    #[test]
    fn not_signed_in_when_missing() {
        let path = tmp("missing").join("auth.json");
        assert!(!is_signed_in_at(&path));
    }
}
