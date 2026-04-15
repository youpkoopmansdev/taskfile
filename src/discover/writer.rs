use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use colored::Colorize;

use super::detector::DiscoveredTask;

/// Format discovered tasks into Taskfile syntax with @description annotations.
fn format_tasks(tasks: &[&DiscoveredTask]) -> String {
    let mut output = String::new();
    for (i, task) in tasks.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        output.push_str(&format!("@description {}\n", task.description));
        output.push_str(&format!("task {} {{\n", task.name));
        for line in task.body.lines() {
            output.push_str(&format!("  {}\n", line));
        }
        output.push_str("}\n");
    }
    output
}

/// Write tasks organized by category.
///
/// - 1 category: writes directly to root Taskfile
/// - 2+ categories: creates `tasks/{category}.Taskfile` with includes in root
pub fn write_categorized(project_dir: &Path, groups: &BTreeMap<String, Vec<&DiscoveredTask>>) {
    let tasks_dir = project_dir.join("tasks");
    fs::create_dir_all(&tasks_dir).unwrap_or_else(|e| {
        eprintln!(
            "{} Cannot create tasks/ directory: {e}",
            "error:".red().bold()
        );
        std::process::exit(1);
    });

    for (category, tasks) in groups {
        let filename = format!("{category}.Taskfile");
        let path = tasks_dir.join(&filename);
        write_to_file(project_dir, &path, tasks);
    }

    write_includes(project_dir, groups.keys().collect());
}

fn write_to_file(project_dir: &Path, path: &Path, tasks: &[&DiscoveredTask]) {
    let content = format_tasks(tasks);
    let relative = path
        .strip_prefix(project_dir)
        .unwrap_or(path)
        .display()
        .to_string();

    if path.exists() {
        let mut existing = fs::read_to_string(path).unwrap_or_default();
        if !existing.ends_with('\n') && !existing.is_empty() {
            existing.push('\n');
        }
        existing.push('\n');
        existing.push_str(&content);
        fs::write(path, existing).unwrap_or_else(|e| {
            eprintln!("{} Cannot write to {relative}: {e}", "error:".red().bold());
            std::process::exit(1);
        });
        eprintln!(
            "\n{} Appended {} tasks to {relative}",
            "✓".green().bold(),
            tasks.len(),
        );
    } else {
        fs::write(path, &content).unwrap_or_else(|e| {
            eprintln!("{} Cannot create {relative}: {e}", "error:".red().bold());
            std::process::exit(1);
        });
        eprintln!(
            "\n{} Created {relative} with {} tasks",
            "✓".green().bold(),
            tasks.len(),
        );
    }

    for task in tasks {
        eprintln!("  {} {}", "+".green(), task.name.green());
    }
}

fn write_includes(project_dir: &Path, categories: Vec<&String>) {
    let taskfile_path = project_dir.join("Taskfile");
    let existing = if taskfile_path.exists() {
        fs::read_to_string(&taskfile_path).unwrap_or_default()
    } else {
        String::new()
    };

    let mut new_includes = Vec::new();
    for cat in &categories {
        let line = format!("include \"tasks/{cat}.Taskfile\"");
        if !existing.contains(&line) {
            new_includes.push(line);
        }
    }

    if new_includes.is_empty() {
        return;
    }

    let include_block = new_includes.join("\n") + "\n";
    let new_content = if existing.is_empty() {
        include_block
    } else {
        format!("{include_block}\n{existing}")
    };

    fs::write(&taskfile_path, new_content).unwrap_or_else(|e| {
        eprintln!("{} Cannot update Taskfile: {e}", "error:".red().bold());
        std::process::exit(1);
    });

    eprintln!("\n{} Updated Taskfile with includes:", "✓".green().bold(),);
    for cat in &categories {
        eprintln!("  {} include \"tasks/{cat}.Taskfile\"", "+".green());
    }
}

/// Load task names already defined in the project's Taskfile.
pub fn load_existing_task_names(project_dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    load_names_from_file(&project_dir.join("Taskfile"), &mut names);

    let tasks_dir = project_dir.join("tasks");
    if tasks_dir.is_dir()
        && let Ok(entries) = fs::read_dir(&tasks_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("Taskfile")
                || path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with(".Taskfile"))
            {
                load_names_from_file(&path, &mut names);
            }
        }
    }
    names
}

fn load_names_from_file(path: &Path, names: &mut Vec<String>) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

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

    #[test]
    fn load_existing_tasks_from_subdirs() {
        let tmp: TempDir = tempfile::tempdir().unwrap();
        let tasks_dir = tmp.path().join("tasks");
        fs::create_dir(&tasks_dir).unwrap();
        fs::write(
            tasks_dir.join("docker.Taskfile"),
            "task up {\n  docker compose up\n}\n",
        )
        .unwrap();

        let names = load_existing_task_names(tmp.path());
        assert!(names.contains(&"up".to_string()));
    }

    #[test]
    fn format_tasks_with_description() {
        let task = DiscoveredTask {
            name: "dev".into(),
            description: "Start dev server".into(),
            body: "npm run dev".into(),
            source: "package.json".into(),
        };
        let output = format_tasks(&[&task]);
        assert!(output.contains("@description Start dev server"));
        assert!(output.contains("task dev {"));
        assert!(output.contains("  npm run dev"));
    }
}
