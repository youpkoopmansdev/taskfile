use std::collections::HashMap;

use colored::Colorize;

use crate::resolver::ResolvedTask;

pub fn print_task_list(registry: &HashMap<String, ResolvedTask>) {
    let mut tasks: Vec<&ResolvedTask> = registry.values().collect();
    tasks.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));

    // Group by namespace
    let mut root_tasks: Vec<&ResolvedTask> = Vec::new();
    let mut namespaced: HashMap<String, Vec<&ResolvedTask>> = HashMap::new();

    for task in &tasks {
        if let Some((ns, _)) = task.qualified_name.split_once(':') {
            let top_ns = ns.split(':').next().unwrap_or(ns);
            namespaced.entry(top_ns.to_string()).or_default().push(task);
        } else {
            root_tasks.push(task);
        }
    }

    // Find max name width for alignment
    let max_name_width = tasks
        .iter()
        .map(|t| format_task_name(t).len())
        .max()
        .unwrap_or(0);

    // Print root tasks
    for task in &root_tasks {
        print_task_entry(task, max_name_width);
    }

    // Print namespaced groups
    let mut ns_keys: Vec<&String> = namespaced.keys().collect();
    ns_keys.sort();

    for ns in ns_keys {
        println!();
        println!(" {}:", ns.yellow().bold());
        let group = &namespaced[ns];
        for task in group {
            print_task_entry(task, max_name_width);
        }
    }
}

pub fn print_help_with_tasks(registry: &HashMap<String, ResolvedTask>) {
    println!(
        "{} {}",
        "Task".green().bold(),
        env!("CARGO_PKG_VERSION").dimmed()
    );
    println!();
    println!("{}", "Usage:".yellow().bold());
    println!("  task {} {}", "<name>".green(), "[-- args...]".dimmed());
    println!();
    println!("{}", "Options:".yellow().bold());
    println!(
        "  {}       {}",
        "--list, -l".green(),
        "List all available tasks".dimmed()
    );
    println!(
        "  {}         {}",
        "--init".green(),
        "Create a new Taskfile".dimmed()
    );
    println!(
        "  {}      {}",
        "--dry-run".green(),
        "Show the script without running it".dimmed()
    );
    println!(
        "  {}   {}",
        "--file, -f".green(),
        "Use a specific Taskfile path".dimmed()
    );
    println!(
        "  {}  {}",
        "--completions".green(),
        "Generate shell completions (bash, zsh, fish)".dimmed()
    );
    println!("  {}       {}", "--help, -h".green(), "Show help".dimmed());
    println!(
        "  {}    {}",
        "--version, -v".green(),
        "Show version".dimmed()
    );
    println!();
    println!("{}", "Available tasks:".yellow().bold());

    print_task_list(registry);
}

fn format_task_name(task: &ResolvedTask) -> String {
    let mut name = task.qualified_name.clone();
    if !task.task.params.is_empty() {
        let params: Vec<String> = task
            .task
            .params
            .iter()
            .map(|p| match &p.default {
                Some(def) => format!("{}={}", p.name, def),
                None => p.name.clone(),
            })
            .collect();
        name = format!("{} [{}]", name, params.join(", "));
    }
    name
}

fn print_task_entry(task: &ResolvedTask, max_width: usize) {
    let name_str = format_task_name(task);
    let padding = max_width.saturating_sub(name_str.len()) + 2;
    let desc = task.task.description.as_deref().unwrap_or("");

    if desc.is_empty() {
        println!("  {}", name_str.green());
    } else {
        println!("  {}{}{}", name_str.green(), " ".repeat(padding), desc);
    }
}
