# AGENTS.md — Taskfile

## Project Overview

**Task** is a modern, per-project task runner CLI written in Rust. It reads `Taskfile` files (a custom format) from the current directory or parent directories. Think of it as Make's task running + bashrc/zshrc expressiveness, scoped per-project.

This is a **monorepo** containing the CLI, LSP server, and IDE plugins.

- **Binaries:** `task` (CLI), `taskfile-lsp` (language server)
- **Current version:** 0.6.0
- **Rust edition:** 2024
- **Repository:** `github.com/youpkoopmansdev/taskfile`
- **Cargo workspace:** root package (`task`) + `lsp-server` member (`taskfile-lsp`)

## Architecture

```
Cargo.toml           — Workspace root + task CLI package
src/                 — Task CLI source
  main.rs            — Entry point, CLI dispatch (clap::Parser, completions, update, discovery, execution)
  cli.rs             — Clap derive arg definitions (Cli struct)
  parser/
    mod.rs           — Hand-written line-by-line parser: parse(input, filepath) -> Result<Ast>
    ast.rs           — AST node types (Task, Param, Alias, Export, Include, DotEnv)
    error.rs         — ParseError with file path + line number (syntax and IO variants)
  resolver.rs        — Processes includes, builds flat HashMap<String, ResolvedTask> with namespace prefixes
  executor.rs        — Resolves dependencies (sequential + parallel), handles @confirm, builds script, runs via subprocess
  runner.rs          — TaskRunner trait (Send + Sync) + BashRunner implementation
  script.rs          — Assembles bash preamble (set -euo pipefail, dotenv, exports, aliases-as-functions, params, body)
  discovery.rs       — Finds nearest Taskfile by walking up from cwd
  discover/          — `--discover` scans project files and interactively generates tasks
    mod.rs           — Orchestrator: run_discover() ties detectors → prompt → writer
    detector.rs      — Types: DiscoveredTask, Detector
    prompt.rs        — Interactive selection UI (numbered list, ranges, cancel)
    writer.rs        — Taskfile formatting + file I/O + existing task loading
    json.rs          — Lightweight JSON object extractor (no serde)
    detectors/
      mod.rs         — Detector registry (ALL constant) + sanitize_task_name helper
      node.rs        — package.json: scripts, framework detection, package manager detection
      rust.rs        — Cargo.toml: build, test, check, release, workspace detection
      docker.rs      — docker-compose/compose.yml: services, per-service tasks
      dockerfile.rs  — Dockerfile: build + run (skipped if compose present)
      makefile.rs    — Makefile: target extraction with recipe bodies
      go.rs          — go.mod: build, test, vet, cmd/ detection
      python.rs      — pyproject.toml/requirements.txt: poetry, uv, pip, pytest
      ruby.rs        — Gemfile: bundler, Rails, RSpec detection
  display.rs         — Artisan-style help output with task list grouped by namespace
  suggest.rs         — Levenshtein distance for "Did you mean?" suggestions on unknown tasks
  scaffold.rs      — `--init` Taskfile creation: detects project files → offers discover, else creates template
  updater.rs       — Self-update via GitHub releases (curl + tar) with daily background update check
tests/
  integration.rs   — Integration tests using tempfile directories
example/
  Taskfile           — Root example showcasing all features
  tasks/
    docker.Taskfile
    deploy.Taskfile
install/
  install.sh         — Curl-based install script (installs both task + taskfile-lsp)

lsp-server/          — Language Server Protocol server (workspace member)
  Cargo.toml         — Crate: taskfile-lsp
  src/
    main.rs          — Entry point, tower-lsp stdio transport
    backend.rs       — LanguageServer trait: diagnostics, completions, hover, go-to-def, symbols
    parser/
      mod.rs         — Error-recovering parser (never fails, collects diagnostics)
      ast.rs         — AST types with Span info for all nodes

editors/
  vscode/            — VS Code extension (TextMate grammar + LSP client)
    src/extension.ts
    syntaxes/taskfile.tmLanguage.json
    package.json
  jetbrains/         — JetBrains plugin (custom Kotlin lexer + LSP client)
    build.gradle.kts — Gradle plugin 2.14.0, Kotlin 2.3.20, intellijIdea("2026.1")
    src/main/kotlin/dev/youpkoopmans/taskfile/
      TaskfileLexer.kt
      TaskfileSyntaxHighlighter.kt
      TaskfileLspServerSupportProvider.kt
    src/main/resources/META-INF/plugin.xml
```

## Dependencies

### CLI (root Cargo.toml)
- `clap` v4 with `derive` feature — CLI argument parsing
- `clap_complete` v4 — Shell completion generation
- `thiserror` v2 — Error derive macros
- `colored` v3 — Terminal color output

### LSP Server (lsp-server/Cargo.toml)
- `tower-lsp` v0.20 — LSP framework
- `tokio` v1 (full) — Async runtime
- `serde` + `serde_json` v1 — JSON serialization

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
- **Root inheritance:** Aliases, exports, and dotenv defined in the root Taskfile are injected into ALL task executions across all included files. This enables shared shortcuts (e.g. `alias dc="docker compose"` in root, used in `tasks/node.Taskfile` as `dc exec app ...`).
- **Downward cascade:** Each parent's aliases/exports/dotenv combine with its children's. A child file gets everything from all ancestors above it.
- **No sibling leakage:** Aliases defined inside `tasks/a.Taskfile` are NOT available in `tasks/b.Taskfile` — only in `a`'s own tasks and its sub-includes.
- Include paths are **relative to the file containing the include statement**.
- Circular includes are detected and return an error. Diamond includes (same file via different paths) are handled by skipping already-processed files.

#### Common pattern: cross-domain tasks

When node tasks need to run through Docker, define shared aliases in root:

```bash
# Taskfile
alias dc="docker compose"
alias br="bun run"
include "tasks/docker.Taskfile"
include "tasks/node.Taskfile"
```

`tasks/node.Taskfile` can then use `dc exec app br dev` — both aliases inherited from root.

## CLI Interface

```
task <name> [-- args...]      Run a task (args passed as --key=value after --)
task --list, -l                List all available tasks with descriptions
task --init                    Create Taskfile (auto-detects project → offers discover)
task --discover                Discover tasks from project files (interactive)
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

- **52 CLI unit tests** across parser, resolver, executor, script, discovery, suggest, discover modules
- **15 integration tests** in `tests/integration.rs` (use `tempfile` for isolated directories)
- **7 LSP parser tests** in `lsp-server/src/parser/mod.rs`
- Run all: `cargo test --workspace`
- CI: `cargo fmt --check --all && cargo clippy --workspace -- -D warnings && cargo test --workspace`

## CI/CD

- **CI** (`.github/workflows/ci.yml`): Three parallel jobs:
  - Rust (fmt + clippy + test on ubuntu/macos/windows for full workspace)
  - VS Code Extension (npm ci + compile)
  - JetBrains Plugin (Java 21 + Gradle + buildPlugin)
- **Release** (`.github/workflows/release.yml`): Triggered by `v*` tags. Builds:
  - `task` + `taskfile-lsp` binaries for 5 targets (linux-x86_64, linux-aarch64, macos-x86_64, macos-aarch64, windows-x86_64)
  - VS Code extension (.vsix)
  - JetBrains plugin (.zip)
  - Each platform archive contains **both** `task` and `taskfile-lsp` binaries
  - Creates GitHub release with auto-generated notes

## Self-Update Mechanism

- `updater.rs` checks GitHub API for latest release (daily background check via spawned thread)
- `--update` downloads platform-specific tarball, extracts, replaces **both** `task` and `taskfile-lsp` binaries (falls back to `sudo cp`)
- Version comparison: semantic version parsing, ignores pre-release tags

## LSP Server

The LSP server (`lsp-server/`) is a **separate Rust crate** in the workspace with its own parser.

### Why two parsers?

| | CLI Parser (`src/parser/`) | LSP Parser (`lsp-server/src/parser/`) |
|---|---|---|
| Error handling | `Result<Ast, ParseError>` — fails on first error | Never fails — collects `Vec<Diagnostic>` in AST |
| Span info | Only `line: usize` on tasks/includes | Full `Span` (start_line, start_col, end_line, end_col) on all nodes |
| Purpose | Execution (must be correct or fail) | Editor support (must be resilient) |

**If you add a new Taskfile construct, you must update BOTH parsers.**

### LSP features

Diagnostics, completions (keywords + task names in depends), hover (task description/params/deps), go-to-definition (task names + include paths), document symbols.

### JetBrains plugin build config (hard-won knowledge)

| Component | Version | Why |
|-----------|---------|-----|
| IntelliJ Platform Gradle Plugin | **2.14.0** | Requires Gradle 9.0+ |
| Kotlin | **2.3.20** | IntelliJ 2026.1 ships Kotlin 2.3.0 metadata |
| Gradle | **9.0** | Required by plugin 2.14.0 |
| Java | **21** | Required for Gradle 9 + Kotlin 2.3 |
| Platform target | `intellijIdea("2026.1")` | Use `intellijIdea()` not `intellijIdeaCommunity()` |

**Critical:** `com.intellij.modules.lsp` only exists in commercial JetBrains IDEs (RustRover, IntelliJ Ultimate, etc.) — NOT Community Edition.

## Common Patterns When Modifying

- **Adding a new Taskfile construct:** Update `src/parser/ast.rs` + `src/parser/mod.rs` (CLI), `lsp-server/src/parser/ast.rs` + `lsp-server/src/parser/mod.rs` (LSP), `resolver.rs`, `script.rs`, `editors/vscode/syntaxes/taskfile.tmLanguage.json`, `editors/jetbrains/.../TaskfileLexer.kt`, and tests.
- **Adding a CLI flag:** Update `cli.rs` (add field to `Cli`), `main.rs` (handle it), `display.rs` (show in help output).
- **Adding a new detector:** Create a file in `src/discover/detectors/`, implement `pub fn detect(dir: &Path) -> Vec<DiscoveredTask>`, add it to `ALL` in `detectors/mod.rs`, and add the project file to `PROJECT_FILES` in `scaffold.rs`.
- **Adding a new annotation:** Follow the `@description`/`@confirm` pattern — `pending_*` state in the parser that gets consumed by the next `task` line.
- **Adding a new LSP feature:** Update `ServerCapabilities` in `backend.rs` `initialize()`, implement the trait method. Editor clients pick it up automatically.

## Gotchas

- **`--init` and `task` (no Taskfile) are smart:** `scaffold.rs` checks `PROJECT_FILES` to detect existing projects. If project files are found, it offers `--discover` first. If no project files exist (empty dir), it creates the template directly. Both `create()` (for `--init`) and `prompt_and_create()` (for bare `task`) follow this flow.

- The parser's `count_braces()` function is string-aware and comment-aware — it won't miscount braces inside quoted strings or `# comments`. This is critical for correctness.
- `ResolvedTask` carries the **combined** aliases/exports/dotenv from the entire include chain (parent + own), not just the file's own declarations.
- Dependencies are resolved from the **same namespace context** — if task `docker:deploy` has `depends=[build]`, it resolves to `docker:build`, not `build`.
- The `--` separator between CLI flags and task args is a standard Unix convention required by clap's `#[arg(last = true)]`.
- Never add `bundledPlugin("com.intellij.modules.platform")` to `build.gradle.kts` — it's a plugin.xml `<depends>` module, not a Gradle dependency.
- The `gradle.properties` must include `kotlin.stdlib.default.dependency = false` to avoid stdlib conflicts with IntelliJ's bundled Kotlin.

## SOLID Principles

All code in this project must follow SOLID principles. These are not optional — they are enforced during review.

### Single Responsibility (SRP)

Each file/module/struct has **one reason to change**. Examples:

- `parser/` only parses — it returns an AST, never executes anything.
- `executor.rs` runs tasks — it doesn't parse or resolve.
- `discover/` is split into sub-modules: `detector.rs` (types), `prompt.rs` (UI), `writer.rs` (file I/O), `json.rs` (parsing), and one file per detector in `detectors/`. No file exceeds ~180 lines.

**Rule of thumb:** If a file exceeds **300 lines**, it likely has multiple responsibilities and should be split. Most files in this project are under 200 lines.

### Open/Closed (OCP)

Adding new behavior should not require modifying existing code:

- **New detectors:** Create a new file in `discover/detectors/`, implement a `pub fn detect(dir: &Path) -> Vec<DiscoveredTask>`, add it to the `ALL` registry in `detectors/mod.rs`. No other files change.
- **New Taskfile constructs:** Add to `ast.rs`, extend the parser match arm. Existing constructs are untouched.
- **New CLI flags:** Add a field to `Cli` struct, add dispatch in `main.rs`. Existing flags are untouched.

### Liskov Substitution (LSP)

- `TaskRunner` trait (`runner.rs`) — `BashRunner` is the only implementation today, but any `Send + Sync` runner can be substituted (e.g., for testing with a mock runner).
- `Detector` struct uses a function pointer `fn(&Path) -> Vec<DiscoveredTask>` — all detectors are interchangeable.

### Interface Segregation (ISP)

- Modules expose only what callers need. `discover/mod.rs` exports only `run_discover()` — internal types and functions are private.
- The parser exposes `parse(input, filepath) -> Result<Ast>` — callers don't interact with internal state machines.

### Dependency Inversion (DIP)

- The executor depends on the `TaskRunner` **trait**, not on `BashRunner` directly.
- Detectors depend on the `DiscoveredTask` **type**, not on the prompt or writer modules.
- The orchestrator (`discover/mod.rs`) composes independent pieces without them knowing about each other.

### Practical enforcement

When writing new code:
1. **One file = one responsibility.** If you're adding a new "kind of thing" (detector, formatter, IO handler), make it a new file.
2. **No God files.** If a file grows past 300 lines, refactor into sub-modules.
3. **Composition over coupling.** Modules should communicate through types, not reach into each other's internals.
4. **New features = new files.** Adding a detector? New file. Adding an output format? New file. Don't extend existing files with unrelated logic.

