use std::io::{self, Write};

use colored::Colorize;

use super::detector::DiscoveredTask;

/// Shows the interactive selection UI and returns indices of chosen tasks.
/// Returns `None` if the user cancels.
pub fn select_tasks(tasks: &[DiscoveredTask]) -> Option<Vec<usize>> {
    eprintln!("\n{}", "Select tasks to add:".yellow().bold());

    for (i, task) in tasks.iter().enumerate() {
        eprintln!(
            "  {} {} — {} {}",
            format!("[{}]", i + 1).dimmed(),
            task.name.green(),
            task.description,
            format!("(from {})", task.source).dimmed()
        );
    }

    eprint!(
        "\n{} (enter numbers to toggle, {} to confirm, {} to cancel): ",
        "Selection".cyan().bold(),
        "enter".green(),
        "q".red()
    );
    io::stderr().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return None;
    }
    let input = input.trim();

    if input.eq_ignore_ascii_case("q") {
        eprintln!("{}", "Cancelled.".dimmed());
        return None;
    }

    let mut selected = vec![true; tasks.len()];

    if !input.is_empty() {
        selected = vec![false; tasks.len()];
        for part in input.split([',', ' ']) {
            let part = part.trim();
            if let Ok(n) = part.parse::<usize>() {
                if n >= 1 && n <= tasks.len() {
                    selected[n - 1] = true;
                }
            } else if let Some((start, end)) = part.split_once('-')
                && let (Ok(s), Ok(e)) =
                    (start.trim().parse::<usize>(), end.trim().parse::<usize>())
            {
                for n in s..=e {
                    if n >= 1 && n <= tasks.len() {
                        selected[n - 1] = true;
                    }
                }
            }
        }
    }

    let indices: Vec<usize> = selected
        .iter()
        .enumerate()
        .filter(|(_, s)| **s)
        .map(|(i, _)| i)
        .collect();

    if indices.is_empty() {
        eprintln!("{}", "No tasks selected.".dimmed());
        return None;
    }

    Some(indices)
}
