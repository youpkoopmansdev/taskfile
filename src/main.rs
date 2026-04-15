mod cli;
mod discovery;
mod display;
mod executor;
mod parser;
mod resolver;
mod runner;
mod scaffold;
mod script;
mod suggest;
mod updater;

use std::path::PathBuf;
use std::process;

use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
use colored::Colorize;

fn main() {
    let cli = cli::Cli::parse();

    // Handle --completions before anything else
    if let Some(shell) = &cli.completions {
        let shell = match shell.to_lowercase().as_str() {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "powershell" => Shell::PowerShell,
            "elvish" => Shell::Elvish,
            _ => {
                eprintln!(
                    "{} unknown shell '{}' — use bash, zsh, fish, powershell, or elvish",
                    "error:".red().bold(),
                    shell
                );
                process::exit(1);
            }
        };
        let mut cmd = cli::Cli::command();
        generate(shell, &mut cmd, "task", &mut std::io::stdout());
        return;
    }

    // Handle --update before anything else (no Taskfile needed)
    if let Some(version) = &cli.update {
        let v = if version.is_empty() {
            None
        } else {
            Some(version.as_str())
        };
        updater::self_update(v);
        return;
    }

    // Handle --init (no Taskfile needed)
    if cli.init {
        if scaffold::create() {
            process::exit(0);
        } else {
            process::exit(1);
        }
    }

    // Background update check (non-blocking, once per day)
    updater::check_for_update_background();

    // Find Taskfile: --file flag overrides discovery
    let taskfile_path = if let Some(ref path) = cli.file {
        let p = PathBuf::from(path);
        if !p.is_file() {
            eprintln!("{} Taskfile not found: {}", "error:".red().bold(), path);
            process::exit(1);
        }
        p
    } else {
        match discovery::find_taskfile() {
            Some(path) => path,
            None => {
                if cli.task_name.is_some() || cli.list || cli.dry_run {
                    eprintln!(
                        "{} No Taskfile found in current or parent directories.",
                        "error:".red().bold()
                    );
                    process::exit(1);
                }
                match scaffold::prompt_and_create() {
                    Some(path) => path,
                    None => process::exit(0),
                }
            }
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
    match executor::execute_task(task_name, &cli.task_args, &registry, &runner, cli.dry_run) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            match e {
                executor::ExecError::TaskFailed { code, .. } => process::exit(code),
                executor::ExecError::Cancelled { .. } => process::exit(0),
                _ => process::exit(1),
            }
        }
    }
}
