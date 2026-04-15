# Task

A modern task runner that reads `Taskfile` files. Think of it as Make's task running with the expressiveness of shell scripts, scoped per-project.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/youpkoopmansdev/taskfile/main/install/install.sh | sh
```

Or with Cargo:

```sh
cargo install --git https://github.com/youpkoopmansdev/taskfile.git
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

#### Root inheritance

Aliases, exports, and dotenv defined in the root Taskfile are **inherited by all included files**. This lets you define shared shortcuts in one place and use them everywhere:

```bash
# Taskfile (root)
alias dc="docker compose"
alias br="bun run"
export APP_NAME="myapp"

include "tasks/docker.Taskfile"
include "tasks/node.Taskfile"
```

```bash
# tasks/docker.Taskfile — dc alias inherited from root
task up {
  dc up -d
}
```

```bash
# tasks/node.Taskfile — dc AND br inherited from root
task dev {
  dc exec app br dev
}

task build {
  echo "Building $APP_NAME"
  br build
}
```

This mirrors how shell environments work — parent scope flows down. Aliases defined inside an included file only apply to tasks in that file and its own children (they don't leak to siblings).

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
Task 0.8.0

Usage:
  task <name> [-- args...]

Options:
  --list, -l       List all available tasks
  --init           Create a new Taskfile
  --discover       Discover tasks from project files
  --dry-run        Show the script without running it
  --file, -f       Use a specific Taskfile path
  --completions    Generate shell completions (bash, zsh, fish)
  --update         Update to the latest version
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

## Getting started

### New project

Run `task --init` (or just `task`) in your project directory:

```sh
cd my-project
task --init
# ✓ Created /path/to/my-project/Taskfile
```

This creates a Taskfile with documented examples covering exports, aliases, parameters, dependencies, parallel deps, dotenv, confirm prompts, and includes — everything you need to get started.

If you simply run `task` without a Taskfile, it will interactively ask if you'd like to create one.

**Smart detection:** If `--init` (or `task` with no Taskfile) detects existing project files (like `package.json`, `Cargo.toml`, `docker-compose.yml`, etc.), it will offer to run `--discover` instead of creating a template — so you get real tasks from day one:

```
? Project files detected. Run discover to generate tasks from them? [Y/n]
```

If you decline, it falls back to the template.

### Existing project

Run `task --discover` to scan your project and generate tasks from files that already exist:

```sh
cd my-project
task --discover
```

```
discover: Scanning /path/to/my-project...

  ✓ package.json (npm/yarn/pnpm) (6 tasks)
  ✓ docker-compose.yml (8 tasks)

Select tasks to add:
  [1] dev — Start Vue/Nuxt dev server (from package.json)
  [2] build — Build for production (from package.json)
  [3] test — Run tests with Vitest (from package.json)
  [4] lint — Run ESLint (from package.json)
  [5] up — Start all services (from docker-compose.yml)
  [6] down — Stop all services (from docker-compose.yml)
  ...

Selection (enter numbers to toggle, enter to confirm, q to cancel): 1-6
✓ Created /path/to/my-project/Taskfile with 6 tasks
```

It detects your tools, extracts existing scripts/targets, and writes proper Taskfile tasks — no manual copy-pasting needed.

**What it scans:**

| File | What it discovers |
|------|-------------------|
| `package.json` | npm/yarn/pnpm/bun scripts, Vue/React/Next/Nuxt framework tasks |
| `Cargo.toml` | build, test, clippy, release (workspace-aware) |
| `docker-compose.yml` / `compose.yaml` | up, down, logs, restart + per-service tasks |
| `Dockerfile` | docker build + run |
| `Makefile` | all targets with their recipe bodies |
| `go.mod` | build, test, vet, run |
| `pyproject.toml` / `requirements.txt` | Poetry, uv, pip, pytest |
| `Gemfile` | bundler, Rails, RSpec |

Run it again after adding new tools — it skips tasks that already exist in your Taskfile.

## Taskfile discovery

`task` walks up from the current directory until it finds a `Taskfile`. This means you can run `task` from any subdirectory in your project.

Use `--file` / `-f` to override discovery and point to a specific Taskfile:

```sh
task build -f ./other/Taskfile
```

## Editor support

IDE plugins provide syntax highlighting, error checking, completions, hover docs, go-to-definition, and document symbols.

### VS Code

1. Go to the [latest release](https://github.com/youpkoopmansdev/taskfile/releases)
2. Download the `taskfile-*.vsix` file
3. In VS Code: open the Command Palette (`Cmd+Shift+P`) → `Extensions: Install from VSIX...` → select the downloaded file

That's it — open any `Taskfile` or `*.Taskfile` and you'll get highlighting + full language support.

### JetBrains (RustRover, IntelliJ Ultimate, WebStorm, etc.)

1. Go to the [latest release](https://github.com/youpkoopmansdev/taskfile/releases)
2. Download the `taskfile-jetbrains-*.zip` file
3. In your IDE: `Settings` → `Plugins` → `⚙️` → `Install Plugin from Disk...` → select the downloaded file
4. Restart the IDE

> **Note:** The LSP integration requires a commercial JetBrains IDE. Community Edition only gets syntax highlighting.

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

The `taskfile-lsp` binary is included in the install script. If you installed with Cargo, install it separately:

```sh
cargo install --git https://github.com/youpkoopmansdev/taskfile.git --name taskfile-lsp
```

### Other editors

Any editor with LSP support can use `taskfile-lsp`. It communicates over stdio:

```sh
taskfile-lsp
```

## License

MIT
