use colored::Colorize;
use dialoguer::MultiSelect;

use super::detector::DiscoveredTask;

/// Shows an interactive checkbox selection UI and returns indices of chosen tasks.
/// Returns `None` if the user cancels (Esc) or selects nothing.
pub fn select_tasks(tasks: &[&DiscoveredTask], categories: &[&str]) -> Option<Vec<usize>> {
    let items: Vec<String> = tasks
        .iter()
        .zip(categories.iter())
        .map(|(t, cat)| format!("[{}] {} — {}", cat, t.name, t.description))
        .collect();

    let defaults = vec![true; tasks.len()];

    eprintln!();
    let indices = MultiSelect::new()
        .with_prompt("Select tasks to add (space to toggle, enter to confirm)")
        .items(&items)
        .defaults(&defaults)
        .interact_opt()
        .ok()
        .flatten()?;

    if indices.is_empty() {
        eprintln!("{}", "No tasks selected.".dimmed());
        return None;
    }

    Some(indices)
}
