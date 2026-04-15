# Taskfile Extension for VS Code

Syntax highlighting and language server support for [Taskfile](https://github.com/youpkoopmansdev/taskfile) files.

## Features

- **Syntax highlighting** — Keywords, task names, parameters, dependencies, annotations, strings, and embedded shell highlighting inside task bodies
- **Error checking** — Real-time diagnostics via the Taskfile language server
- **Completions** — Autocomplete for task names, parameters, and keywords
- **Hover information** — Documentation on hover for tasks and annotations
- **Go to definition** — Navigate to task definitions and included files
- **Document symbols** — Outline view of tasks, exports, and aliases

## Requirements

The language server binary `taskfile-lsp` must be available on your `PATH` or configured via the `taskfile.lspPath` setting.

### Install the language server

Build from source:

```sh
cd /path/to/taskfile
cargo build --release -p taskfile-lsp
# Copy target/release/taskfile-lsp to a directory on your PATH
```

## Configuration

| Setting           | Default         | Description                        |
| ----------------- | --------------- | ---------------------------------- |
| `taskfile.lspPath` | `taskfile-lsp` | Path to the `taskfile-lsp` binary |

## File associations

The extension activates for files named `Taskfile` or with a `.Taskfile` extension.

## Links

- [Taskfile project](https://github.com/youpkoopmansdev/taskfile)
