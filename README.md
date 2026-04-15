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

## Quick start

Create a `Taskfile` in your project root:

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

```bash
task build depends=[clean, compile] {
  echo "Done"
}
```

Dependencies run in order before the task body.

### Exports and aliases

```bash
export PROJECT="myapp"
alias dc="docker compose"
```

Aliases are converted to shell functions automatically (so they work in non-interactive bash).

### Includes

Split tasks into separate files:

```bash
include "tasks/docker.Taskfile"
include "tasks/deploy.Taskfile"
```

The filename stem becomes the namespace. A file `tasks/docker.Taskfile` containing `task up {}` registers as `docker:up`.

Exports and aliases from the parent file are inherited by included files.

## Project structure example

```
myproject/
  Taskfile
  tasks/
    docker.Taskfile
    quality.Taskfile
```

**Taskfile:**
```bash
include "tasks/docker.Taskfile"
include "tasks/quality.Taskfile"

alias dc="docker compose -f docker/docker-compose.yml"
export SERVICE="app"
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
Task 0.1.0

Usage:
  task <name> [-- args...]

Options:
  --list, -l       List all available tasks
  --help, -h       Show help
  --version, -v    Show version

Available tasks:

 docker:
  docker:down    Stop containers
  docker:up      Start containers

 quality:
  quality:lint   Run all code quality checks
```

## CLI

```
task <name> [-- args...]      Run a task
task --list, -l               List all available tasks
task --help, -h               Show help
task --version, -v            Show version
task                          Show help + task list
```

## Taskfile discovery

`task` walks up from the current directory until it finds a `Taskfile`. This means you can run `task` from any subdirectory in your project.

## License

MIT
