use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "task",
    about = "A modern task runner that reads Taskfile files",
    version
)]
pub struct Cli {
    /// Task name to run
    pub task_name: Option<String>,

    /// List all available tasks
    #[arg(short, long)]
    pub list: bool,

    /// Create a new Taskfile in the current directory
    #[arg(long)]
    pub init: bool,

    /// Print the generated script without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Path to a specific Taskfile
    #[arg(short, long)]
    pub file: Option<String>,

    /// Generate shell completions (bash, zsh, fish)
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<String>,

    /// Update to the latest version (or a specific version with --update=v0.2.0)
    #[arg(long, num_args = 0..=1, default_missing_value = "")]
    pub update: Option<String>,

    /// Discover project tasks from package.json, Cargo.toml, docker-compose, Makefile, etc.
    #[arg(long)]
    pub discover: bool,

    /// Arguments to pass to the task (after --)
    #[arg(last = true)]
    pub task_args: Vec<String>,
}
