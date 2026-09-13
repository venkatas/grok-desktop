use std::path::{Path, PathBuf};

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = path.metadata() {
            return meta.permissions().mode() & 0o111 != 0;
        }
        false
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn resolve_grok_bin() -> Option<PathBuf> {
    let env_bin = std::env::var_os("GROK_BIN").map(PathBuf::from);
    let path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    resolve_grok_bin_in(env_bin, &crate::paths::grok_home(), &path_dirs)
}

pub fn resolve_grok_bin_in(
    env_bin: Option<PathBuf>,
    grok_home: &Path,
    path_dirs: &[PathBuf],
) -> Option<PathBuf> {
    if let Some(p) = env_bin {
        if is_executable(&p) {
            return Some(p);
        }
    }
    let home_bin = grok_home.join("bin").join("grok");
    if is_executable(&home_bin) {
        return Some(home_bin);
    }
    for dir in path_dirs {
        let p = dir.join("grok");
        if is_executable(&p) {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!("gb-bin-{}", std::process::id()));
        let _ = fs::create_dir_all(&p);
        p
    }

    fn touch_exec(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    fn touch_plain(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"not exec").unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(path, perms).unwrap();
    }

    #[test]
    fn env_bin_wins_when_executable() {
        let root = tmp().join("env-win");
        let env_bin = root.join("custom-grok");
        let home = root.join("home");
        let path_bin = root.join("path").join("grok");
        touch_exec(&env_bin);
        touch_exec(&home.join("bin").join("grok"));
        touch_exec(&path_bin);
        let got = resolve_grok_bin_in(Some(env_bin.clone()), &home, &[root.join("path")]);
        assert_eq!(got, Some(env_bin));
    }

    #[test]
    fn skips_env_bin_when_not_executable() {
        let root = tmp().join("env-skip");
        let env_bin = root.join("custom-grok");
        let home = root.join("home");
        let home_bin = home.join("bin").join("grok");
        touch_plain(&env_bin);
        touch_exec(&home_bin);
        let got = resolve_grok_bin_in(Some(env_bin), &home, &[]);
        assert_eq!(got, Some(home_bin));
    }

    #[test]
    fn uses_grok_home_bin_before_path() {
        let root = tmp().join("home-first");
        let home = root.join("home");
        let home_bin = home.join("bin").join("grok");
        let path_bin = root.join("path").join("grok");
        touch_exec(&home_bin);
        touch_exec(&path_bin);
        let got = resolve_grok_bin_in(None, &home, &[root.join("path")]);
        assert_eq!(got, Some(home_bin));
    }

    #[test]
    fn falls_back_to_path() {
        let root = tmp().join("path-only");
        let home = root.join("home");
        let path_bin = root.join("path").join("grok");
        touch_exec(&path_bin);
        let got = resolve_grok_bin_in(None, &home, &[root.join("path")]);
        assert_eq!(got, Some(path_bin));
    }

    #[test]
    fn missing_returns_none() {
        let root = tmp().join("none");
        assert_eq!(resolve_grok_bin_in(None, &root.join("home"), &[]), None);
    }
}
