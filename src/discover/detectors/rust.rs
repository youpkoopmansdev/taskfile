use std::fs;
use std::path::Path;

use crate::discover::detector::DiscoveredTask;

pub fn detect(dir: &Path) -> Vec<DiscoveredTask> {
    let path = dir.join("Cargo.toml");
    if !path.exists() {
        return Vec::new();
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut tasks = vec![
        DiscoveredTask {
            name: "build".into(),
            description: "Build the project".into(),
            body: "cargo build".into(),
            source: "Cargo.toml".into(),
        },
        DiscoveredTask {
            name: "test".into(),
            description: "Run tests".into(),
            body: "cargo test".into(),
            source: "Cargo.toml".into(),
        },
        DiscoveredTask {
            name: "check".into(),
            description: "Check for compilation errors".into(),
            body: "cargo clippy -- -D warnings".into(),
            source: "Cargo.toml".into(),
        },
        DiscoveredTask {
            name: "release".into(),
            description: "Build for release".into(),
            body: "cargo build --release".into(),
            source: "Cargo.toml".into(),
        },
    ];

    if content.contains("[workspace]") {
        tasks.push(DiscoveredTask {
            name: "test-all".into(),
            description: "Run tests for all workspace members".into(),
            body: "cargo test --workspace".into(),
            source: "Cargo.toml (workspace)".into(),
        });
    }

    if dir.join("benches").exists() {
        tasks.push(DiscoveredTask {
            name: "bench".into(),
            description: "Run benchmarks".into(),
            body: "cargo bench".into(),
            source: "Cargo.toml (benches/ detected)".into(),
        });
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detect_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*"]

[package]
name = "myapp"
"#,
        )
        .unwrap();

        let tasks = detect(tmp.path());
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"build"));
        assert!(names.contains(&"test"));
        assert!(names.contains(&"test-all"));
    }
}
