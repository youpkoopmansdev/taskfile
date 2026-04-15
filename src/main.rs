mod cli;
mod discovery;
mod display;
mod executor;
mod parser;
mod resolver;
mod runner;
mod script;
mod suggest;

use std::process;

use clap::Parser;
use colored::Colorize;

fn main() {
    let cli = cli::Cli::parse();

    let taskfile_path = match discovery::find_taskfile() {
        Some(path) => path,
        None => {
            eprintln!(
                "{} No Taskfile found in current or parent directories.",
                "error:".red().bold()
            );
            process::exit(1);
        }
    };

    let registry = match resolver::resolve(&taskfile_path) {
        Ok(reg) => reg,
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            process::exit(1);
        }
    };

    if cli.list || cli.task_name.is_none() {
        display::print_help_with_tasks(&registry);
        return;
    }

    let task_name = cli.task_name.as_deref().unwrap();

    if !registry.contains_key(task_name) {
        eprintln!(
            "{} unknown task '{}'",
            "error:".red().bold(),
            task_name.yellow()
        );
        let names: Vec<&str> = registry.keys().map(|k| k.as_str()).collect();
        suggest::suggest_similar(task_name, &names);
        process::exit(1);
    }

    let runner = runner::BashRunner;
    match executor::execute_task(task_name, &cli.task_args, &registry, &runner) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            match e {
                executor::ExecError::TaskFailed { code, .. } => process::exit(code),
                _ => process::exit(1),
            }
        }
    }
}
