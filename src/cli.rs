use colored::Colorize;

pub struct Cli {
    pub task_name: Option<String>,
    pub list: bool,
    pub init: bool,
    pub dry_run: bool,
    pub file: Option<String>,
    pub completions: Option<String>,
    pub update: Option<String>,
    pub discover: bool,
    pub task_args: Vec<String>,
    pub help: bool,
    pub version: bool,
}

impl Cli {
    pub fn parse_args() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        Self::parse_from(args)
    }

    fn parse_from(args: Vec<String>) -> Self {
        let mut cli = Cli {
            task_name: None,
            list: false,
            init: false,
            dry_run: false,
            file: None,
            completions: None,
            update: None,
            discover: false,
            task_args: Vec::new(),
            help: false,
            version: false,
        };

        let mut i = 0;
        let mut task_found = false;

        while i < args.len() {
            let arg = &args[i];

            // After task name: runner flags still recognized, rest are task args
            if task_found {
                match arg.as_str() {
                    "--" => {
                        // Skip legacy separator, keep collecting
                        i += 1;
                        continue;
                    }
                    "--dry-run" => cli.dry_run = true,
                    "--list" | "-l" => cli.list = true,
                    _ => cli.task_args.push(arg.clone()),
                }
                i += 1;
                continue;
            }

            // Before task name: parse known flags
            match arg.as_str() {
                "--" => {
                    // Explicit separator: remaining args are task args
                    i += 1;
                    while i < args.len() {
                        cli.task_args.push(args[i].clone());
                        i += 1;
                    }
                    break;
                }
                "--list" | "-l" => cli.list = true,
                "--init" => cli.init = true,
                "--dry-run" => cli.dry_run = true,
                "--discover" => cli.discover = true,
                "--help" | "-h" => cli.help = true,
                "--version" | "-v" => cli.version = true,
                _ if arg.starts_with("--file=") => {
                    cli.file = arg.strip_prefix("--file=").map(String::from);
                }
                "--file" | "-f" => {
                    i += 1;
                    if i < args.len() {
                        cli.file = Some(args[i].clone());
                    }
                }
                _ if arg.starts_with("--completions=") => {
                    cli.completions = arg.strip_prefix("--completions=").map(String::from);
                }
                "--completions" => {
                    i += 1;
                    if i < args.len() {
                        cli.completions = Some(args[i].clone());
                    }
                }
                _ if arg.starts_with("--update=") => {
                    cli.update = arg.strip_prefix("--update=").map(String::from);
                }
                "--update" => {
                    cli.update = Some(String::new());
                }
                _ if arg.starts_with('-') => {
                    eprintln!(
                        "{} unknown option '{}'\n\n  Usage: task [OPTIONS] <TASK> [ARGS...]\n\n  For more information, try '--help'.",
                        "error:".red().bold(),
                        arg
                    );
                    std::process::exit(2);
                }
                _ => {
                    cli.task_name = Some(arg.clone());
                    task_found = true;
                }
            }
            i += 1;
        }

        cli
    }

    /// Build a clap Command for shell completions generation only
    pub fn command() -> clap::Command {
        clap::Command::new("task")
            .about("A modern task runner that reads Taskfile files")
            .version(env!("CARGO_PKG_VERSION"))
            .arg(clap::Arg::new("task_name").help("Task name to run"))
            .arg(
                clap::Arg::new("list")
                    .short('l')
                    .long("list")
                    .action(clap::ArgAction::SetTrue)
                    .help("List all available tasks"),
            )
            .arg(
                clap::Arg::new("init")
                    .long("init")
                    .action(clap::ArgAction::SetTrue)
                    .help("Create a new Taskfile"),
            )
            .arg(
                clap::Arg::new("dry_run")
                    .long("dry-run")
                    .action(clap::ArgAction::SetTrue)
                    .help("Print the generated script without executing"),
            )
            .arg(
                clap::Arg::new("file")
                    .short('f')
                    .long("file")
                    .help("Path to a specific Taskfile"),
            )
            .arg(
                clap::Arg::new("completions")
                    .long("completions")
                    .help("Generate shell completions (bash, zsh, fish)"),
            )
            .arg(
                clap::Arg::new("update")
                    .long("update")
                    .help("Update to the latest version"),
            )
            .arg(
                clap::Arg::new("discover")
                    .long("discover")
                    .action(clap::ArgAction::SetTrue)
                    .help("Discover project tasks from project files"),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::parse_from(args.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn task_args_without_separator() {
        let cli = parse(&["test", "--file=file.ts", "--verbose"]);
        assert_eq!(cli.task_name.as_deref(), Some("test"));
        assert_eq!(cli.task_args, vec!["--file=file.ts", "--verbose"]);
        assert!(cli.file.is_none());
    }

    #[test]
    fn task_args_with_legacy_separator() {
        let cli = parse(&["test", "--", "--file=file.ts"]);
        assert_eq!(cli.task_name.as_deref(), Some("test"));
        assert_eq!(cli.task_args, vec!["--file=file.ts"]);
    }

    #[test]
    fn runner_flags_before_task() {
        let cli = parse(&["--dry-run", "-f", "myfile", "build", "--env=prod"]);
        assert!(cli.dry_run);
        assert_eq!(cli.file.as_deref(), Some("myfile"));
        assert_eq!(cli.task_name.as_deref(), Some("build"));
        assert_eq!(cli.task_args, vec!["--env=prod"]);
    }

    #[test]
    fn list_flag() {
        let cli = parse(&["--list"]);
        assert!(cli.list);
        assert!(cli.task_name.is_none());
    }

    #[test]
    fn version_flag() {
        let cli = parse(&["-v"]);
        assert!(cli.version);
    }

    #[test]
    fn update_with_version() {
        let cli = parse(&["--update=v1.0.0"]);
        assert_eq!(cli.update.as_deref(), Some("v1.0.0"));
    }

    #[test]
    fn update_bare() {
        let cli = parse(&["--update"]);
        assert_eq!(cli.update.as_deref(), Some(""));
    }

    #[test]
    fn file_equals_syntax() {
        let cli = parse(&["--file=custom.Taskfile", "build"]);
        assert_eq!(cli.file.as_deref(), Some("custom.Taskfile"));
        assert_eq!(cli.task_name.as_deref(), Some("build"));
    }

    #[test]
    fn no_args() {
        let cli = parse(&[]);
        assert!(cli.task_name.is_none());
        assert!(!cli.list);
    }

    #[test]
    fn separator_before_task_name() {
        let cli = parse(&["--", "test", "--file=x"]);
        assert!(cli.task_name.is_none());
        assert_eq!(cli.task_args, vec!["test", "--file=x"]);
    }

    #[test]
    fn mixed_task_args() {
        let cli = parse(&["deploy", "--env=production", "--target=v2.1.0"]);
        assert_eq!(cli.task_name.as_deref(), Some("deploy"));
        assert_eq!(
            cli.task_args,
            vec!["--env=production", "--target=v2.1.0"]
        );
    }
}
