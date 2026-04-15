# AGENTS.md — Taskfile CLI

## Project Overview

**Task** is a modern, per-project task runner CLI written in Rust. It reads `Taskfile` files (a custom format) from the current directory or parent directories. Think of it as Make's task running + bashrc/zshrc expressiveness, scoped per-project.

- **Binary name:** `task`
- **Crate name:** `task` (Cargo.toml `name = "task"`)
- **Current version:** 0.5.0
- **Rust edition:** 2024
- **Repository:** `github.com/youpkoopmansdev/taskfile`
- **Companion LSP project:** `github.com/youpkoopmansdev/taskfile-lsp`

## Architecture

```
src/
  main.rs          — Entry point, CLI dispatch (clap::Parser, completions, update, discovery, execution)
  cli.rs           — Clap derive arg definitions (Cli struct)
  parser/
    mod.rs         — Hand-written line-by-line parser: parse(input, filepath) -> Result<Ast>
    ast.rs         — AST node types (Task, Param, Alias, Export, Include, DotEnv)
    error.rs       — ParseError with file path + line number (syntax and IO variants)
  resolver.rs      — Processes includes, builds flat HashMap<String, ResolvedTask> with namespace prefixes
  executor.rs      — Resolves dependencies (sequential + parallel), handles @confirm, builds script, runs via subprocess
  runner.rs        — TaskRunner trait (Send + Sync) + BashRunner implementation
  script.rs        — Assembles bash preamble (set -euo pipefail, dotenv, exports, aliases-as-functions, params, body)
  discovery.rs     — Finds nearest Taskfile by walking up from cwd
  display.rs       — Artisan-style help output with task list grouped by namespace
  suggest.rs       — Levenshtein distance for "Did you mean?" suggestions on unknown tasks
  scaffold.rs      — `--init` Taskfile template generation with interactive prompt
  updater.rs       — Self-update via GitHub releases (curl + tar) with daily background update check
tests/
  integration.rs   — Integration tests using tempfile directories
example/
  Taskfile         — Root example showcasing all features
  tasks/
    docker.Taskfile
    deploy.Taskfile
install/
  install.sh       — Curl-based install script for CI/users
```

## Dependencies

**Runtime (Cargo.toml):**
- `clap` v4 with `derive` feature — CLI argument parsing
- `clap_complete` v4 — Shell completion generation
- `thiserror` v2 — Error derive macros
- `colored` v3 — Terminal color output

**Dev only:**
- `tempfile` v3 — Temporary directories for tests

**No other dependencies.** Keep it lean. No parser combinator libraries, no serde, no tokio.

## Taskfile Format Specification

The Taskfile is a structured file where task bodies are **opaque bash** (the parser does not parse inside `{ ... }`).

### Top-level constructs

```bash
# Comments
export KEY="value"
alias name="command"
include "path/to/file.Taskfile"
dotenv ".env"

@description Task description text
@confirm Are you sure you want to do this?
task name [param1 param2="default"] depends=[dep1, dep2] depends_parallel=[dep3, dep4] {
  # bash body — stored as raw string, not parsed
  echo "hello"
}
```

### Key parsing rules

1. **Hand-written parser** — line-by-line, tracking line numbers. No parser combinators.
2. `@description` and `@confirm` are **annotations** that must appear on the line(s) immediately before a `task` definition. They set `pending_description`/`pending_confirm` state that gets consumed when the next `task` line is parsed. If a non-task line follows an annotation, that's a parse error.
3. **Brace depth tracking** for task bodies — the `{` can be on the same line as `task` or the next line. The parser uses `count_braces()` which is string/comment-aware (doesn't count braces inside strings or comments).
4. **Parameters:** `[name]` = required, `[name="default"]` = optional. Parameters become shell variables.
5. **Dependencies:** `depends=[a, b]` = sequential (run in order), `depends_parallel=[c, d]` = parallel (run via `std::thread::scope`).
6. **Includes:** The filename stem becomes the namespace prefix. `tasks/docker.Taskfile` → `docker:*`. Nested includes chain: `docker:compose:*`.

### Namespace and scope rules

- Tasks in root Taskfile have no prefix.
- Exports, aliases, and dotenv from parent files **cascade down** to included namespaces.
- Include paths are **relative to the file containing the include statement**.
- Circular includes are detected and return an error. Diamond includes (same file via different paths) are handled by skipping already-processed files.

## CLI Interface

```
task <name> [-- args...]      Run a task (args passed as --key=value after --)
task --list, -l                List all available tasks with descriptions
task --init                    Create a new Taskfile in current directory
task --dry-run                 Print generated bash script without executing
task --file, -f <path>         Use a specific Taskfile path
task --completions <shell>     Generate shell completions (bash, zsh, fish, powershell, elvish)
task --update[=version]        Self-update from GitHub releases
task --help, -h                Show help
task --version, -v             Show version
```

**Task parameters** are passed after `--`:
```bash
task deploy -- --env=production --target=v2.0
```

## Execution Pipeline

1. **Discovery:** Find Taskfile (walk up from cwd, or use `--file`)
2. **Parse:** Hand-written parser produces `Ast` per file
3. **Resolve:** Process includes recursively, build flat `HashMap<String, ResolvedTask>`
4. **Execute:**
   a. Resolve sequential dependencies (run each first, abort on failure)
   b. Run parallel dependencies via `std::thread::scope`
   c. Handle `@confirm` prompt (`[y/N]`, skipped in dry-run)
   d. Build bash script: `set -euo pipefail` → dotenv sourcing → exports → aliases (as functions) → param variables → task body
   e. Execute via `std::process::Command::new("bash").arg("-c").arg(script)`
   f. Inherit stdio (stream stdout/stderr directly to terminal)

### Key implementation details

- **Aliases become functions:** `alias dc="docker compose"` → `dc() { docker compose "$@"; }` because `bash -c` doesn't expand aliases.
- **Dotenv sourcing:** `if [ -f "path" ]; then set -a; source "path"; set +a; fi`
- **Shell quoting:** `script::shell_quote()` escapes `\`, `"`, `$`, and backticks.
- **Parallel deps** require `TaskRunner: Send + Sync` trait bounds (see `runner.rs` line 3).
- **Exit codes** propagate — if a task fails with code N, the CLI exits with code N.

## Testing

- **41 unit tests** across parser, resolver, executor, script, discovery, suggest modules
- **15 integration tests** in `tests/integration.rs` (use `tempfile` for isolated directories)
- Run: `cargo test`
- CI: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## CI/CD

- **CI** (`.github/workflows/ci.yml`): fmt + clippy + test on push/PR to main
- **Release** (`.github/workflows/release.yml`): Triggered by `v*` tags. Builds 5 targets:
  - linux-x86_64, linux-aarch64 (cross-compiled with `gcc-aarch64-linux-gnu`), macos-x86_64, macos-aarch64, windows-x86_64
  - Unix: `.tar.gz`, Windows: `.zip`
  - Creates GitHub release with auto-generated notes

## Self-Update Mechanism

- `updater.rs` checks GitHub API for latest release (daily background check via spawned thread)
- `--update` downloads platform-specific tarball, extracts, replaces current binary (falls back to `sudo cp`)
- Version comparison: semantic version parsing, ignores pre-release tags

## Common Patterns When Modifying

- **Adding a new Taskfile construct:** Update `parser/ast.rs` (add to `Ast` struct), `parser/mod.rs` (parse it), `resolver.rs` (propagate to `ResolvedTask`), `script.rs` (emit in bash script), and tests.
- **Adding a CLI flag:** Update `cli.rs` (add field to `Cli`), `main.rs` (handle it), `display.rs` (show in help output).
- **Adding a new annotation:** Follow the `@description`/`@confirm` pattern — `pending_*` state in the parser that gets consumed by the next `task` line.

## Gotchas

- The parser's `count_braces()` function is string-aware and comment-aware — it won't miscount braces inside quoted strings or `# comments`. This is critical for correctness.
- `ResolvedTask` carries the **combined** aliases/exports/dotenv from the entire include chain (parent + own), not just the file's own declarations.
- Dependencies are resolved from the **same namespace context** — if task `docker:deploy` has `depends=[build]`, it resolves to `docker:build`, not `build`.
- The `--` separator between CLI flags and task args is a standard Unix convention required by clap's `#[arg(last = true)]`.
