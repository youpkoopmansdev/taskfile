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

    /// Update to the latest version (or a specific version with --update=v0.2.0)
    #[arg(long, num_args = 0..=1, default_missing_value = "")]
    pub update: Option<String>,

    /// Arguments to pass to the task (after --)
    #[arg(last = true)]
    pub task_args: Vec<String>,
}
