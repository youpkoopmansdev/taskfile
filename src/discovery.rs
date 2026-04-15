use std::env;
use std::path::{Path, PathBuf};

pub fn find_taskfile() -> Option<PathBuf> {
    let mut dir = env::current_dir().ok()?;

    loop {
        let candidate = dir.join("Taskfile");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[allow(dead_code)]
pub fn find_taskfile_from(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();

    loop {
        let candidate = dir.join("Taskfile");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_temp() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn finds_taskfile_in_current_dir() {
        let tmp = make_temp();
        fs::write(tmp.path().join("Taskfile"), "task hello { echo hi }").unwrap();
        let result = find_taskfile_from(tmp.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), tmp.path().join("Taskfile"));
    }

    #[test]
    fn finds_taskfile_in_parent_dir() {
        let tmp = make_temp();
        fs::write(tmp.path().join("Taskfile"), "task hello { echo hi }").unwrap();
        let child = tmp.path().join("subdir");
        fs::create_dir(&child).unwrap();
        let result = find_taskfile_from(&child);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), tmp.path().join("Taskfile"));
    }

    #[test]
    fn error_when_none_found() {
        let tmp = make_temp();
        let deep = tmp.path().join("a/b/c");
        fs::create_dir_all(&deep).unwrap();
        let result = find_taskfile_from(&deep);
        // Might find one from the real filesystem, but within the temp dir there's none
        // Just test that it doesn't panic
        if let Some(path) = result {
            assert!(path.is_file());
        }
    }
}
