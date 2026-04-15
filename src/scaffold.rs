use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use colored::Colorize;

const TEMPLATE: &str = r##"# ─────────────────────────────────────────────────
# Taskfile — your project's task runner
# ─────────────────────────────────────────────────
#
# Run tasks:   task <name>
# List tasks:  task --list  (or just: task)
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
# Organizing with includes:
#   1. Create a tasks/ directory
#   2. Add focused Taskfiles: tasks/docker.Taskfile, tasks/test.Taskfile
#   3. Include them here: include "tasks/docker.Taskfile"
#   4. Run namespaced: task docker:up
#
# Task body is plain bash — use any shell commands you like.
"##;

pub fn prompt_and_create() -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    let target = cwd.join("Taskfile");

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

    if let Err(e) = fs::write(&target, TEMPLATE) {
        eprintln!("{} Could not create Taskfile: {}", "error:".red().bold(), e);
        return None;
    }

    eprintln!("{} Created {}", "✓".green().bold(), target.display());
    eprintln!("  Run {} to see available tasks.", "task".cyan());

    Some(target)
}
