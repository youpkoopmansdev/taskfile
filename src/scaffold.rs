use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use colored::Colorize;

const TEMPLATE: &str = r##"# ─────────────────────────────────────────────────
# Taskfile — your project's task runner
# ─────────────────────────────────────────────────
#
# Run tasks:   task <name>
# List tasks:  task --list  (or just: task)
# Dry run:     task <name> --dry-run
# Update CLI:  task --update
#
# Full docs:   https://github.com/youpkoopmansdev/taskfile

# ─── Includes ──────────────────────────────────────
# Split tasks into separate files for organization.
# The filename (without .Taskfile) becomes the namespace prefix.
#
#   include "tasks/docker.Taskfile"   → docker:up, docker:down, ...
#   include "tasks/deploy.Taskfile"   → deploy:staging, deploy:prod, ...
#
# Nested includes chain automatically:
#   docker.Taskfile includes compose.Taskfile → docker:compose:*
#
# Exports and aliases in included files are scoped to that namespace.

# ─── Dotenv ────────────────────────────────────────
# Load environment variables from a .env file.
# The file is sourced at the start of every task in this scope.
#
# dotenv ".env"
# dotenv ".env.local"

# ─── Exports ───────────────────────────────────────
# Environment variables available in all tasks defined in this file.

export PROJECT="myproject"

# ─── Aliases ───────────────────────────────────────
# Shorthand commands. These become shell functions inside tasks.

# alias dc="docker compose"
# alias k="kubectl"

# ─── Tasks ─────────────────────────────────────────
# Define tasks with `task <name> { ... }`.
# Add a description with @description above the task.
# Add parameters with [name] or [name="default"].
# Add dependencies with depends=[task1, task2].
# Add parallel dependencies with depends_parallel=[task1, task2].
# Add a confirmation prompt with @confirm above the task.

@description Say hello
task hello {
  echo "Hello from $PROJECT!"
}

@description Greet someone by name
task greet [name="world"] {
  echo "Hello, $name!"
}

@description Run the full build pipeline
task build depends=[clean] {
  echo "Building $PROJECT..."
  # your build commands here
}

@description Clean build artifacts
task clean {
  echo "Cleaning..."
  # rm -rf target/ dist/ build/
}

@confirm Are you sure you want to nuke everything?
@description Remove all build artifacts and caches
task nuke {
  echo "Nuking..."
  # rm -rf target/ dist/ node_modules/ .cache/
}

# ─── Tips ──────────────────────────────────────────
#
# Parameters:
#   task greet --name=Claude        # named arg
#   [name="world"]                  # optional (has default)
#   [name]                          # required (no default)
#
# Dependencies:
#   task build depends=[clean, lint] { ... }
#   Dependencies run in order before the task body.
#
# Parallel dependencies:
#   task ci depends_parallel=[lint, test] { ... }
#   These run concurrently — use for independent tasks.
#
# Confirmation prompts:
#   @confirm Are you sure?
#   task dangerous { ... }
#   Asks for confirmation before running. Skipped with --dry-run.
#
# Dotenv:
#   dotenv ".env"
#   Loads environment variables from a file before each task.
#
# Organizing with includes:
#   1. Create a tasks/ directory
#   2. Add focused Taskfiles: tasks/docker.Taskfile, tasks/test.Taskfile
#   3. Include them here: include "tasks/docker.Taskfile"
#   4. Run namespaced: task docker:up
#
# Dry run:
#   task build --dry-run            # shows the script without running it
#
# Shell completions:
#   task --completions bash >> ~/.bashrc
#   task --completions zsh >> ~/.zshrc
#   task --completions fish > ~/.config/fish/completions/task.fish
#
# Task body is plain bash — use any shell commands you like.
"##;

/// Check if the directory looks like an existing project (has recognizable config files).
fn has_project_files(dir: &Path) -> bool {
    const PROJECT_FILES: &[&str] = &[
        "package.json",
        "Cargo.toml",
        "go.mod",
        "pyproject.toml",
        "requirements.txt",
        "Gemfile",
        "Makefile",
        "Dockerfile",
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];
    PROJECT_FILES.iter().any(|f| dir.join(f).exists())
}

/// Ask the user whether to run discover or create a template.
/// Returns true if discover was chosen and should run.
fn offer_discover() -> bool {
    eprint!(
        "\n{} Project files detected. Run {} to generate tasks from them? [Y/n] ",
        "?".cyan().bold(),
        "discover".green()
    );
    io::stderr().flush().ok();

    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    let answer = answer.trim().to_lowercase();
    answer.is_empty() || answer == "y" || answer == "yes"
}

/// Create a Taskfile directly (for --init). Returns true on success.
pub fn create() -> bool {
    let cwd = match env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "{} Could not determine current directory: {}",
                "error:".red().bold(),
                e
            );
            return false;
        }
    };
    let target = cwd.join("Taskfile");

    if target.exists() {
        eprintln!(
            "{} Taskfile already exists in {}",
            "warning:".yellow().bold(),
            cwd.display()
        );
        return false;
    }

    if has_project_files(&cwd) && offer_discover() {
        crate::discover::run_discover(&cwd);
        return true;
    }

    if let Err(e) = fs::write(&target, TEMPLATE) {
        eprintln!("{} Could not create Taskfile: {}", "error:".red().bold(), e);
        return false;
    }

    eprintln!("{} Created {}", "✓".green().bold(), target.display());
    eprintln!("  Run {} to see available tasks.", "task".cyan());
    true
}

/// Prompt the user to create a Taskfile (interactive, when no Taskfile found).
pub fn prompt_and_create() -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    let target = cwd.join("Taskfile");

    if has_project_files(&cwd) {
        eprint!(
            "{} No Taskfile found. Run {} to generate tasks from this project? [Y/n] ",
            "?".cyan().bold(),
            "discover".green()
        );
        io::stderr().flush().ok();

        let mut answer = String::new();
        io::stdin().read_line(&mut answer).ok()?;
        let answer = answer.trim().to_lowercase();

        if answer.is_empty() || answer == "y" || answer == "yes" {
            crate::discover::run_discover(&cwd);
            let target = cwd.join("Taskfile");
            if target.exists() {
                return Some(target);
            }
            return None;
        }

        // User said no to discover — offer template instead
        eprint!(
            "{} Create a template Taskfile instead? [y/N] ",
            "?".cyan().bold()
        );
        io::stderr().flush().ok();

        let mut answer2 = String::new();
        io::stdin().read_line(&mut answer2).ok()?;
        let answer2 = answer2.trim().to_lowercase();

        if answer2 != "y" && answer2 != "yes" {
            return None;
        }
    } else {
        eprint!(
            "{} No Taskfile found. Create one in {}? [Y/n] ",
            "?".cyan().bold(),
            cwd.display()
        );
        io::stderr().flush().ok();

        let mut answer = String::new();
        io::stdin().read_line(&mut answer).ok()?;
        let answer = answer.trim().to_lowercase();

        if !answer.is_empty() && answer != "y" && answer != "yes" {
            return None;
        }
    }

    if let Err(e) = fs::write(&target, TEMPLATE) {
        eprintln!("{} Could not create Taskfile: {}", "error:".red().bold(), e);
        return None;
    }

    eprintln!("{} Created {}", "✓".green().bold(), target.display());
    eprintln!("  Run {} to see available tasks.", "task".cyan());

    Some(target)
}
