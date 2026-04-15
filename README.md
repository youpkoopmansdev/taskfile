# Task

A modern task runner that reads `Taskfile` files. Think of it as Make's task running with the expressiveness of shell scripts, scoped per-project.

## Install

Installs both the `task` CLI and `taskfile-lsp` language server:

```sh
curl -fsSL https://raw.githubusercontent.com/youpkoopmansdev/taskfile/main/install/install.sh | sh
```

Or with Cargo:

```sh
cargo install --git https://github.com/youpkoopmansdev/taskfile.git
cargo install --git https://github.com/youpkoopmansdev/taskfile.git --name taskfile-lsp
```

To update:

```sh
task --update
```

## Quick start

Create a `Taskfile` in your project root (or run `task --init`):

```bash
@description Start the development server
task dev {
  docker compose up -d
  echo "Ready at http://localhost:3000"
}

@description Run all tests
task test {
  cargo test
}
```

Then run:

```sh
task dev       # run a task
task           # list all available tasks
```

## Taskfile format

### Tasks

```bash
task build {
  cargo build --release
}
```

### Descriptions

Descriptions are defined with `@description` above the task:

```bash
@description Build the project for production
task build {
  cargo build --release
}
```

### Parameters

Parameters go in square brackets. Without a default they're required:

```bash
task deploy [env target="latest"] {
  echo "Deploying $target to $env"
}
```

```sh
task deploy -- --env=production              # target defaults to "latest"
task deploy -- --env=production --target=v2
```

### Dependencies

Run tasks in order before the task body:

```bash
task build depends=[clean, compile] {
  echo "Done"
}
```

### Parallel dependencies

Run independent tasks concurrently before the task body:

```bash
task ci depends_parallel=[lint, test] depends=[build] {
  echo "CI complete"
}
```

`depends_parallel` tasks run at the same time. You can combine both — parallel deps run first, then sequential deps, then the body.

### Exports and aliases

```bash
export PROJECT="myapp"
alias dc="docker compose"
```

Aliases are converted to shell functions automatically (so they work in non-interactive bash).

### Dotenv

Load environment variables from a `.env` file:

```bash
dotenv ".env"
dotenv ".env.local"
```

The file is sourced (with `set -a` for auto-export) at the start of every task in that scope. Missing files are silently skipped.

### Confirmation prompts

Add `@confirm` above a task to require confirmation before running:

```bash
@confirm Are you sure you want to deploy?
@description Deploy to production
task deploy:prod [version] {
  echo "Deploying $version..."
}
```

The user sees a `[y/N]` prompt. Default is no. Skipped in `--dry-run` mode.

### Includes

Split tasks into separate files:

```bash
include "tasks/docker.Taskfile"
include "tasks/deploy.Taskfile"
```

The filename stem becomes the namespace. A file `tasks/docker.Taskfile` containing `task up {}` registers as `docker:up`.

Exports, aliases, and dotenv from the parent file are inherited by included files.

## Project structure example

```
myproject/
  Taskfile
  .env
  tasks/
    docker.Taskfile
    deploy.Taskfile
```

**Taskfile:**
```bash
include "tasks/docker.Taskfile"
include "tasks/deploy.Taskfile"

dotenv ".env"

alias dc="docker compose -f docker/docker-compose.yml"
export SERVICE="app"

@description Build everything
task build depends=[clean] {
  echo "Building..."
}

@description Run lint and test in parallel, then build
task ci depends_parallel=[lint, test] depends=[build] {
  echo "CI pipeline complete"
}

@confirm Are you sure you want to reset?
@description Reset the project
task reset {
  rm -rf target/ dist/
}
```

**tasks/docker.Taskfile:**
```bash
@description Start containers
task up {
  dc up -d
}

@description Stop containers
task down {
  dc down
}
```

Running `task` shows:

```
Task 0.5.0

Usage:
  task <name> [-- args...]

Options:
  --list, -l       List all available tasks
  --init           Create a new Taskfile
  --dry-run        Show the script without running it
  --file, -f       Use a specific Taskfile path
  --completions    Generate shell completions (bash, zsh, fish)
  --help, -h       Show help
  --version, -v    Show version

Available tasks:
  build            Build everything
  ci               Run lint and test in parallel, then build
  reset            Reset the project

 docker:
  docker:down      Stop containers
  docker:up        Start containers
```

## CLI

```
task <name> [-- args...]      Run a task
task <name> --dry-run         Show the bash script without running it
task --list, -l               List all available tasks
task --init                   Create a new Taskfile in the current directory
task --discover               Discover tasks from project files (interactive)
task --file, -f <path>        Use a specific Taskfile instead of discovery
task --completions <shell>    Generate shell completions (bash, zsh, fish, powershell, elvish)
task --update                 Update to the latest version
task --update=v0.1.0          Update to a specific version
task --help, -h               Show help
task --version, -v            Show version
task                          Show help + task list (or offer to create a Taskfile)
```

Task automatically checks for updates once per day and notifies you when a new version is available.

### Dry run

Preview the generated bash script without executing it:

```sh
task deploy:prod --dry-run -- --version=v2.0
```

This prints the full script (exports, aliases, dotenv, params, body) so you can inspect exactly what would run.

### Shell completions

Generate completions for your shell:

```sh
# Bash
task --completions bash >> ~/.bashrc

# Zsh
task --completions zsh >> ~/.zshrc

# Fish
task --completions fish > ~/.config/fish/completions/task.fish
```

## Getting started in a new project

Run `task --init` (or just `task`) in your project directory:

```sh
cd my-project
task --init
# ✓ Created /path/to/my-project/Taskfile
```

This creates a Taskfile with documented examples covering exports, aliases, parameters, dependencies, parallel deps, dotenv, confirm prompts, and includes — everything you need to get started.

If you simply run `task` without a Taskfile, it will interactively ask if you'd like to create one.

## Task discovery

Run `task --discover` to scan your project and interactively generate tasks from existing configuration files:

```sh
task --discover
# discover: Scanning /path/to/my-project...
#
#   ✓ package.json (Node.js) (8 tasks)
#   ✓ docker-compose.yml (3 tasks)
#
# Select tasks to add:
#   [1] dev — Start Vue dev server (from package.json)
#   [2] build — Build for production (from package.json)
#   ...
#
# Selection: 1-5
# ✓ Added 5 tasks to Taskfile
```

Supported project files:
- **package.json** — npm/yarn/pnpm/bun scripts, framework detection (Vue, React, Next, Nuxt)
- **Cargo.toml** — build, test, check, release (workspace-aware)
- **docker-compose.yml** / **compose.yaml** — up, down, logs + per-service tasks
- **Dockerfile** — build and run tasks
- **Makefile** — ports existing make targets
- **go.mod** — build, test, vet, lint
- **pyproject.toml** / **requirements.txt** — Poetry, uv, pip, pytest detection
- **Gemfile** — Rails, RSpec detection

## Taskfile discovery

`task` walks up from the current directory until it finds a `Taskfile`. This means you can run `task` from any subdirectory in your project.

Use `--file` / `-f` to override discovery and point to a specific Taskfile:

```sh
task build -f ./other/Taskfile
```

## Editor support

This repo includes a Language Server Protocol (LSP) server and IDE plugins for syntax highlighting, error checking, completions, hover docs, go-to-definition, and document symbols.

### VS Code

1. Download the `.vsix` file from the [latest release](https://github.com/youpkoopmansdev/taskfile/releases)
2. In VS Code: `Extensions` → `...` → `Install from VSIX...`

Or search for "Taskfile" in the VS Code Marketplace (once published).

### JetBrains (RustRover, IntelliJ Ultimate, WebStorm, etc.)

1. Download the `taskfile-jetbrains-*.zip` from the [latest release](https://github.com/youpkoopmansdev/taskfile/releases)
2. In your IDE: `Settings` → `Plugins` → `⚙️` → `Install Plugin from Disk...`

> **Note:** Requires a commercial JetBrains IDE — Community Edition does not support the LSP API.

### Neovim

Add to your LSP config:

```lua
vim.api.nvim_create_autocmd({ "BufRead", "BufNewFile" }, {
  pattern = { "Taskfile", "*.Taskfile" },
  callback = function()
    vim.bo.filetype = "taskfile"
    vim.lsp.start({ name = "taskfile-lsp", cmd = { "taskfile-lsp" } })
  end,
})
```

### Other editors

Any editor with LSP support can use `taskfile-lsp`. Start it with:

```sh
taskfile-lsp   # communicates over stdio
```

## License

MIT
