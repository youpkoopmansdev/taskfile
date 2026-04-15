mod detector;
mod detectors;
mod json;
mod prompt;
mod writer;

use std::path::Path;

use colored::Colorize;

/// Scan a project directory for discoverable tasks and interactively add them.
pub fn run_discover(project_dir: &Path) {
    eprintln!(
        "{} Scanning {}...\n",
        "discover:".cyan().bold(),
        project_dir.display()
    );

    let mut all_tasks = Vec::new();

    for det in detectors::ALL {
        let tasks = (det.detect)(project_dir);
        if !tasks.is_empty() {
            eprintln!(
                "  {} {} ({} tasks)",
                "✓".green(),
                det.name,
                tasks.len()
            );
            all_tasks.extend(tasks);
        }
    }

    if all_tasks.is_empty() {
        eprintln!(
            "\n{} No project files detected. Nothing to discover.",
            "info:".dimmed()
        );
        return;
    }

    let existing = writer::load_existing_task_names(project_dir);
    let new_tasks: Vec<_> = all_tasks
        .into_iter()
        .filter(|t| !existing.contains(&t.name))
        .collect();

    if new_tasks.is_empty() {
        eprintln!(
            "\n{} All discovered tasks already exist in your Taskfile.",
            "info:".dimmed()
        );
        return;
    }

    let indices = match prompt::select_tasks(&new_tasks) {
        Some(idx) => idx,
        None => return,
    };

    let chosen: Vec<_> = indices.iter().map(|&i| &new_tasks[i]).collect();
    writer::write_tasks(project_dir, &chosen);
}
