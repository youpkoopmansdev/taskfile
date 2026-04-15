use std::fs;
use std::io::Write;
use std::path::Path;

use colored::Colorize;

use super::detector::DiscoveredTask;

/// Format discovered tasks into Taskfile syntax.
pub fn format_tasks(tasks: &[&DiscoveredTask]) -> String {
    let mut output = String::new();
    output.push('\n');
    for task in tasks {
        output.push_str(&format!("@description {}\n", task.description));
        output.push_str(&format!("task {} {{\n", task.name));
        for line in task.body.lines() {
            output.push_str(&format!("  {}\n", line));
        }
        output.push_str("}\n\n");
    }
    output
}

/// Write chosen tasks to the project's Taskfile (creates or appends).
pub fn write_tasks(project_dir: &Path, tasks: &[&DiscoveredTask]) {
    let content = format_tasks(tasks);
    let taskfile_path = project_dir.join("Taskfile");

    if taskfile_path.exists() {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&taskfile_path)
            .unwrap_or_else(|e| {
                eprintln!("{} Cannot open Taskfile: {e}", "error:".red().bold());
                std::process::exit(1);
            });
        file.write_all(content.as_bytes()).unwrap_or_else(|e| {
            eprintln!("{} Cannot write to Taskfile: {e}", "error:".red().bold());
            std::process::exit(1);
        });
        eprintln!(
            "\n{} Appended {} tasks to {}",
            "✓".green().bold(),
            tasks.len(),
            taskfile_path.display()
        );
    } else {
        fs::write(&taskfile_path, content).unwrap_or_else(|e| {
            eprintln!("{} Cannot create Taskfile: {e}", "error:".red().bold());
            std::process::exit(1);
        });
        eprintln!(
            "\n{} Created {} with {} tasks",
            "✓".green().bold(),
            taskfile_path.display(),
            tasks.len()
        );
    }

    for task in tasks {
        eprintln!("  {} {}", "+".green(), task.name.green());
    }
}

/// Load task names already defined in the project's Taskfile.
pub fn load_existing_task_names(project_dir: &Path) -> Vec<String> {
    let taskfile = project_dir.join("Taskfile");
    if !taskfile.exists() {
        return Vec::new();
    }

    let content = match fs::read_to_string(&taskfile) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut names = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("task ") {
            let rest = trimmed.strip_prefix("task ").unwrap();
            if let Some(name) = rest.split_whitespace().next() {
                let name = name.trim_end_matches('{');
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn load_existing_tasks() {
        let tmp: TempDir = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Taskfile"),
            "task build {\n  cargo build\n}\n\ntask test {\n  cargo test\n}\n",
        )
        .unwrap();

        let names = load_existing_task_names(tmp.path());
        assert!(names.contains(&"build".to_string()));
        assert!(names.contains(&"test".to_string()));
    }
}
