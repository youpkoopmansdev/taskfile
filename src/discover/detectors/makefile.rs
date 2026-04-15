use std::fs;
use std::path::Path;

use super::sanitize_task_name;
use crate::discover::detector::DiscoveredTask;

pub fn detect(dir: &Path) -> Vec<DiscoveredTask> {
    let path = if dir.join("Makefile").exists() {
        dir.join("Makefile")
    } else if dir.join("makefile").exists() {
        dir.join("makefile")
    } else {
        return Vec::new();
    };

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut tasks = Vec::new();

    for line in content.lines() {
        if let Some(colon_pos) = line.find(':') {
            let target = line[..colon_pos].trim();
            if target.is_empty()
                || target.starts_with('.')
                || target.starts_with('#')
                || target.starts_with('\t')
                || target.starts_with(' ')
                || target.contains('=')
                || target.contains('$')
                || target.contains('%')
            {
                continue;
            }

            let mut body_lines = Vec::new();
            let target_line_idx = content.lines().position(|l| std::ptr::eq(l, line));
            if let Some(idx) = target_line_idx {
                for recipe_line in content.lines().skip(idx + 1) {
                    if recipe_line.starts_with('\t') {
                        body_lines.push(recipe_line.trim_start_matches('\t'));
                    } else if recipe_line.trim().is_empty() {
                        continue;
                    } else {
                        break;
                    }
                }
            }

            let body = if body_lines.is_empty() {
                format!("make {target}")
            } else {
                body_lines.join("\n")
            };

            let name = sanitize_task_name(target);
            tasks.push(DiscoveredTask {
                name,
                description: format!("Makefile target: {target}"),
                body,
                source: "Makefile".into(),
            });
        }
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detect_targets() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Makefile"),
            "build:\n\tgo build ./...\n\ntest:\n\tgo test ./...\n\n.PHONY: build test\n",
        )
        .unwrap();

        let tasks = detect(tmp.path());
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"build"));
        assert!(names.contains(&"test"));
        assert!(!names.iter().any(|n| n.contains("PHONY")));
    }
}
