mod detector;
mod detectors;
mod json;
mod prompt;
mod writer;

use std::collections::BTreeMap;
use std::path::Path;

use colored::Colorize;

use detector::DiscoveredTask;

/// A group of tasks from a single detector category.
struct TaskGroup {
    category: String,
    tasks: Vec<DiscoveredTask>,
}

/// Scan a project directory for discoverable tasks and interactively add them.
pub fn run_discover(project_dir: &Path) {
    eprintln!(
        "{} Scanning {}...\n",
        "discover:".cyan().bold(),
        project_dir.display()
    );

    let mut groups: Vec<TaskGroup> = Vec::new();

    for det in detectors::ALL {
        let tasks = (det.detect)(project_dir);
        if !tasks.is_empty() {
            eprintln!("  {} {} ({} tasks)", "✓".green(), det.name, tasks.len());
            groups.push(TaskGroup {
                category: det.category.to_string(),
                tasks,
            });
        }
    }

    if groups.is_empty() {
        eprintln!(
            "\n{} No project files detected. Nothing to discover.",
            "info:".dimmed()
        );
        return;
    }

    let existing = writer::load_existing_task_names(project_dir);

    // Build flat list with category tracking for selection
    let indexed: Vec<(usize, usize)> = groups
        .iter()
        .enumerate()
        .flat_map(|(gi, g)| {
            g.tasks
                .iter()
                .enumerate()
                .filter(|(_, t)| !existing.contains(&t.name))
                .map(move |(ti, _)| (gi, ti))
        })
        .collect();

    if indexed.is_empty() {
        eprintln!(
            "\n{} All discovered tasks already exist in your Taskfile.",
            "info:".dimmed()
        );
        return;
    }

    let display_tasks: Vec<&DiscoveredTask> = indexed
        .iter()
        .map(|&(gi, ti)| &groups[gi].tasks[ti])
        .collect();

    let categories: Vec<&str> = indexed
        .iter()
        .map(|&(gi, _)| groups[gi].category.as_str())
        .collect();

    let selected = match prompt::select_tasks(&display_tasks, &categories) {
        Some(idx) => idx,
        None => return,
    };

    // Group selected tasks by category
    let mut categorized: BTreeMap<String, Vec<&DiscoveredTask>> = BTreeMap::new();
    for &i in &selected {
        let (gi, ti) = indexed[i];
        categorized
            .entry(groups[gi].category.clone())
            .or_default()
            .push(&groups[gi].tasks[ti]);
    }

    writer::write_categorized(project_dir, &categorized);
}
