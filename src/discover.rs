use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use colored::Colorize;

/// A discovered task that can be added to a Taskfile.
struct DiscoveredTask {
    name: String,
    description: String,
    body: String,
    source: String,
}

/// A detector that scans for a specific project type / tool.
struct Detector {
    name: &'static str,
    detect: fn(&Path) -> Vec<DiscoveredTask>,
}

const DETECTORS: &[Detector] = &[
    Detector {
        name: "package.json (npm/yarn/pnpm)",
        detect: detect_package_json,
    },
    Detector {
        name: "Cargo.toml (Rust)",
        detect: detect_cargo,
    },
    Detector {
        name: "docker-compose.yml",
        detect: detect_docker_compose,
    },
    Detector {
        name: "Dockerfile",
        detect: detect_dockerfile,
    },
    Detector {
        name: "Makefile",
        detect: detect_makefile,
    },
    Detector {
        name: "go.mod (Go)",
        detect: detect_go,
    },
    Detector {
        name: "pyproject.toml / requirements.txt (Python)",
        detect: detect_python,
    },
    Detector {
        name: "Gemfile (Ruby)",
        detect: detect_ruby,
    },
];

pub fn run_discover(project_dir: &Path) {
    eprintln!(
        "{} Scanning {}...\n",
        "discover:".cyan().bold(),
        project_dir.display()
    );

    let mut all_tasks: Vec<DiscoveredTask> = Vec::new();

    for detector in DETECTORS {
        let tasks = (detector.detect)(project_dir);
        if !tasks.is_empty() {
            eprintln!(
                "  {} {} ({} tasks)",
                "✓".green(),
                detector.name,
                tasks.len()
            );
            all_tasks.extend(tasks);
        }
    }

    if all_tasks.is_empty() {
        eprintln!(
            "\n{} No project files detected. Nothing to discover.",
            "info:".dimmed()
        );
        return;
    }

    // Filter out tasks that already exist in the Taskfile
    let existing = load_existing_task_names(project_dir);
    let new_tasks: Vec<DiscoveredTask> = all_tasks
        .into_iter()
        .filter(|t| !existing.contains(&t.name))
        .collect();

    if new_tasks.is_empty() {
        eprintln!(
            "\n{} All discovered tasks already exist in your Taskfile.",
            "info:".dimmed()
        );
        return;
    }

    eprintln!("\n{}", "Select tasks to add:".yellow().bold());
    let mut selected = vec![true; new_tasks.len()];

    for (i, task) in new_tasks.iter().enumerate() {
        eprintln!(
            "  {} {} — {} {}",
            format!("[{}]", i + 1).dimmed(),
            task.name.green(),
            task.description,
            format!("(from {})", task.source).dimmed()
        );
    }

    eprint!(
        "\n{} (enter numbers to toggle, {} to confirm, {} to cancel): ",
        "Selection".cyan().bold(),
        "enter".green(),
        "q".red()
    );
    io::stderr().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return;
    }
    let input = input.trim();

    if input.eq_ignore_ascii_case("q") {
        eprintln!("{}", "Cancelled.".dimmed());
        return;
    }

    // Parse deselections — if user types numbers, toggle those off (everything starts selected)
    if !input.is_empty() {
        // If user provides specific numbers, select ONLY those
        selected = vec![false; new_tasks.len()];
        for part in input.split([',', ' ']) {
            let part = part.trim();
            if let Ok(n) = part.parse::<usize>() {
                if n >= 1 && n <= new_tasks.len() {
                    selected[n - 1] = true;
                }
            } else if let Some((start, end)) = part.split_once('-')
                && let (Ok(s), Ok(e)) =
                    (start.trim().parse::<usize>(), end.trim().parse::<usize>())
            {
                for n in s..=e {
                    if n >= 1 && n <= new_tasks.len() {
                        selected[n - 1] = true;
                    }
                }
            }
        }
    }

    let chosen: Vec<&DiscoveredTask> = new_tasks
        .iter()
        .enumerate()
        .filter(|(i, _)| selected[*i])
        .map(|(_, t)| t)
        .collect();

    if chosen.is_empty() {
        eprintln!("{}", "No tasks selected.".dimmed());
        return;
    }

    let taskfile_content = format_tasks(&chosen);
    let taskfile_path = project_dir.join("Taskfile");

    if taskfile_path.exists() {
        // Append to existing Taskfile
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&taskfile_path)
            .unwrap_or_else(|e| {
                eprintln!("{} Cannot open Taskfile: {e}", "error:".red().bold());
                std::process::exit(1);
            });
        file.write_all(taskfile_content.as_bytes())
            .unwrap_or_else(|e| {
                eprintln!("{} Cannot write to Taskfile: {e}", "error:".red().bold());
                std::process::exit(1);
            });
        eprintln!(
            "\n{} Appended {} tasks to {}",
            "✓".green().bold(),
            chosen.len(),
            taskfile_path.display()
        );
    } else {
        fs::write(&taskfile_path, taskfile_content).unwrap_or_else(|e| {
            eprintln!("{} Cannot create Taskfile: {e}", "error:".red().bold());
            std::process::exit(1);
        });
        eprintln!(
            "\n{} Created {} with {} tasks",
            "✓".green().bold(),
            taskfile_path.display(),
            chosen.len()
        );
    }

    for task in &chosen {
        eprintln!("  {} {}", "+".green(), task.name.green());
    }
}

fn format_tasks(tasks: &[&DiscoveredTask]) -> String {
    let mut output = String::new();
    output.push('\n');
    for task in tasks {
        output.push_str(&format!("@description {}\n", task.description));
        output.push_str(&format!("task {} {{\n", task.name));
        for line in task.body.lines() {
            output.push_str(&format!("  {}\n", line));
        }
        output.push_str("}\n\n");
    }
    output
}

fn load_existing_task_names(project_dir: &Path) -> Vec<String> {
    let taskfile = project_dir.join("Taskfile");
    if !taskfile.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(&taskfile) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    // Simple extraction — look for "task <name>" patterns
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("task ") && !trimmed.starts_with("task_") {
                let rest = trimmed.strip_prefix("task ")?;
                let name = rest.split([' ', '{', '[']).next()?;
                if !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ':')
                {
                    return Some(name.to_string());
                }
            }
            None
        })
        .collect()
}

// ─── Detectors ─────────────────────────────────────────────

fn detect_package_json(dir: &Path) -> Vec<DiscoveredTask> {
    let path = dir.join("package.json");
    if !path.exists() {
        return Vec::new();
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut tasks = Vec::new();

    // Detect the package manager
    let pm = detect_node_package_manager(dir);

    // Extract scripts from package.json using simple JSON parsing
    let scripts = extract_json_object(&content, "scripts");
    for (name, command) in &scripts {
        let task_name = sanitize_task_name(name);
        tasks.push(DiscoveredTask {
            name: task_name,
            description: format!("Run npm script: {name}"),
            body: format!("{pm} run {name}"),
            source: format!("package.json scripts.{name} → {command}"),
        });
    }

    // If no scripts found, suggest standard ones based on what we see
    if tasks.is_empty() {
        tasks.push(DiscoveredTask {
            name: "install".into(),
            description: "Install dependencies".into(),
            body: format!("{pm} install"),
            source: "package.json".into(),
        });
    }

    // Check for common frameworks
    let deps = extract_json_object(&content, "dependencies");
    let dev_deps = extract_json_object(&content, "devDependencies");
    let all_deps: HashMap<String, String> = deps.into_iter().chain(dev_deps).collect();

    if all_deps.contains_key("vue") || all_deps.contains_key("nuxt") {
        if !scripts.contains_key("dev") {
            tasks.push(DiscoveredTask {
                name: "dev".into(),
                description: "Start Vue/Nuxt dev server".into(),
                body: format!("{pm} run dev"),
                source: "package.json (Vue/Nuxt detected)".into(),
            });
        }
        if !scripts.contains_key("build") {
            tasks.push(DiscoveredTask {
                name: "build".into(),
                description: "Build for production".into(),
                body: format!("{pm} run build"),
                source: "package.json (Vue/Nuxt detected)".into(),
            });
        }
    }

    if (all_deps.contains_key("react") || all_deps.contains_key("next"))
        && !scripts.contains_key("dev") && !scripts.contains_key("start")
    {
        tasks.push(DiscoveredTask {
            name: "dev".into(),
            description: "Start React/Next dev server".into(),
            body: format!("{pm} run dev"),
            source: "package.json (React/Next detected)".into(),
        });
    }

    if all_deps.contains_key("vitest") && !scripts.contains_key("test") {
        tasks.push(DiscoveredTask {
            name: "test".into(),
            description: "Run tests with Vitest".into(),
            body: format!("{pm} run test"),
            source: "package.json (vitest detected)".into(),
        });
    }

    if all_deps.contains_key("jest") && !scripts.contains_key("test") {
        tasks.push(DiscoveredTask {
            name: "test".into(),
            description: "Run tests with Jest".into(),
            body: format!("{pm} run test"),
            source: "package.json (jest detected)".into(),
        });
    }

    if all_deps.contains_key("eslint") && !scripts.contains_key("lint") {
        tasks.push(DiscoveredTask {
            name: "lint".into(),
            description: "Run ESLint".into(),
            body: format!("{pm} run lint"),
            source: "package.json (eslint detected)".into(),
        });
    }

    tasks
}

fn detect_node_package_manager(dir: &Path) -> &'static str {
    if dir.join("bun.lockb").exists() || dir.join("bun.lock").exists() {
        "bun"
    } else if dir.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if dir.join("yarn.lock").exists() {
        "yarn"
    } else {
        "npm"
    }
}

fn detect_cargo(dir: &Path) -> Vec<DiscoveredTask> {
    let path = dir.join("Cargo.toml");
    if !path.exists() {
        return Vec::new();
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut tasks = vec![
        DiscoveredTask {
            name: "build".into(),
            description: "Build the project".into(),
            body: "cargo build".into(),
            source: "Cargo.toml".into(),
        },
        DiscoveredTask {
            name: "test".into(),
            description: "Run tests".into(),
            body: "cargo test".into(),
            source: "Cargo.toml".into(),
        },
        DiscoveredTask {
            name: "check".into(),
            description: "Check for compilation errors".into(),
            body: "cargo clippy -- -D warnings".into(),
            source: "Cargo.toml".into(),
        },
        DiscoveredTask {
            name: "release".into(),
            description: "Build for release".into(),
            body: "cargo build --release".into(),
            source: "Cargo.toml".into(),
        },
    ];

    // Check for workspace
    if content.contains("[workspace]") {
        tasks.push(DiscoveredTask {
            name: "test-all".into(),
            description: "Run tests for all workspace members".into(),
            body: "cargo test --workspace".into(),
            source: "Cargo.toml (workspace)".into(),
        });
    }

    // Check for benchmarks
    if dir.join("benches").exists() {
        tasks.push(DiscoveredTask {
            name: "bench".into(),
            description: "Run benchmarks".into(),
            body: "cargo bench".into(),
            source: "Cargo.toml (benches/ detected)".into(),
        });
    }

    tasks
}

fn detect_docker_compose(dir: &Path) -> Vec<DiscoveredTask> {
    let compose_file = find_compose_file(dir);
    let path = match compose_file {
        Some(p) => p,
        None => return Vec::new(),
    };

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    let mut tasks = Vec::new();

    // Extract service names from docker-compose
    let services = extract_compose_services(&content);

    tasks.push(DiscoveredTask {
        name: "up".into(),
        description: "Start all services".into(),
        body: "docker compose up -d".into(),
        source: filename.to_string(),
    });

    tasks.push(DiscoveredTask {
        name: "down".into(),
        description: "Stop all services".into(),
        body: "docker compose down".into(),
        source: filename.to_string(),
    });

    tasks.push(DiscoveredTask {
        name: "logs".into(),
        description: "Tail logs for all services".into(),
        body: "docker compose logs -f".into(),
        source: filename.to_string(),
    });

    tasks.push(DiscoveredTask {
        name: "restart".into(),
        description: "Restart all services".into(),
        body: "docker compose restart".into(),
        source: filename.to_string(),
    });

    tasks.push(DiscoveredTask {
        name: "ps".into(),
        description: "Show running containers".into(),
        body: "docker compose ps".into(),
        source: filename.to_string(),
    });

    // Per-service tasks for notable services
    for service in &services {
        let svc = sanitize_task_name(service);
        tasks.push(DiscoveredTask {
            name: format!("{svc}:logs"),
            description: format!("Tail logs for {service}"),
            body: format!("docker compose logs -f {service}"),
            source: format!("{filename} service: {service}"),
        });
        tasks.push(DiscoveredTask {
            name: format!("{svc}:restart"),
            description: format!("Restart {service}"),
            body: format!("docker compose restart {service}"),
            source: format!("{filename} service: {service}"),
        });
        tasks.push(DiscoveredTask {
            name: format!("{svc}:shell"),
            description: format!("Open shell in {service}"),
            body: format!("docker compose exec {service} sh"),
            source: format!("{filename} service: {service}"),
        });
    }

    tasks
}

fn find_compose_file(dir: &Path) -> Option<PathBuf> {
    let candidates = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];
    for name in candidates {
        let p = dir.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn extract_compose_services(content: &str) -> Vec<String> {
    let mut services = Vec::new();
    let mut in_services = false;
    let mut services_indent: Option<usize> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "services:" {
            in_services = true;
            services_indent = None;
            continue;
        }

        if in_services {
            // Detect top-level keys that end services block
            if !line.starts_with(' ') && !line.starts_with('\t') && !trimmed.is_empty() {
                break;
            }

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let indent = line.len() - line.trim_start().len();

            if services_indent.is_none() {
                services_indent = Some(indent);
            }

            // Service names are at the first indentation level under services:
            if Some(indent) == services_indent && trimmed.ends_with(':') {
                let name = trimmed.trim_end_matches(':').trim();
                if !name.is_empty() {
                    services.push(name.to_string());
                }
            }
        }
    }

    services
}

fn detect_dockerfile(dir: &Path) -> Vec<DiscoveredTask> {
    if !dir.join("Dockerfile").exists() {
        return Vec::new();
    }

    // Don't add docker build tasks if docker-compose is present (it handles that)
    if find_compose_file(dir).is_some() {
        return Vec::new();
    }

    let project_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app")
        .to_lowercase();

    vec![
        DiscoveredTask {
            name: "docker:build".into(),
            description: "Build Docker image".into(),
            body: format!("docker build -t {project_name} ."),
            source: "Dockerfile".into(),
        },
        DiscoveredTask {
            name: "docker:run".into(),
            description: "Run Docker container".into(),
            body: format!("docker run --rm -it {project_name}"),
            source: "Dockerfile".into(),
        },
    ]
}

fn detect_makefile(dir: &Path) -> Vec<DiscoveredTask> {
    let path = if dir.join("Makefile").exists() {
        dir.join("Makefile")
    } else if dir.join("makefile").exists() {
        dir.join("makefile")
    } else {
        return Vec::new();
    };

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut tasks = Vec::new();

    for line in content.lines() {
        // Match Makefile targets: "target: [deps]" at the start of a line
        if let Some(colon_pos) = line.find(':') {
            let target = line[..colon_pos].trim();
            // Skip variables, includes, .PHONY, etc.
            if target.is_empty()
                || target.starts_with('.')
                || target.starts_with('#')
                || target.starts_with('\t')
                || target.starts_with(' ')
                || target.contains('=')
                || target.contains('$')
                || target.contains('%')
            {
                continue;
            }

            // Find the recipe lines (indented with tab) following this target
            let mut body_lines = Vec::new();
            let target_line_idx = content.lines().position(|l| std::ptr::eq(l, line));
            if let Some(idx) = target_line_idx {
                for recipe_line in content.lines().skip(idx + 1) {
                    if recipe_line.starts_with('\t') {
                        body_lines.push(recipe_line.trim_start_matches('\t'));
                    } else if recipe_line.trim().is_empty() {
                        continue;
                    } else {
                        break;
                    }
                }
            }

            let body = if body_lines.is_empty() {
                format!("make {target}")
            } else {
                body_lines.join("\n")
            };

            let name = sanitize_task_name(target);
            tasks.push(DiscoveredTask {
                name: format!("make:{name}"),
                description: format!("Makefile target: {target}"),
                body,
                source: "Makefile".into(),
            });
        }
    }

    tasks
}

fn detect_go(dir: &Path) -> Vec<DiscoveredTask> {
    if !dir.join("go.mod").exists() {
        return Vec::new();
    }

    let mut tasks = vec![
        DiscoveredTask {
            name: "build".into(),
            description: "Build the Go project".into(),
            body: "go build ./...".into(),
            source: "go.mod".into(),
        },
        DiscoveredTask {
            name: "test".into(),
            description: "Run Go tests".into(),
            body: "go test ./...".into(),
            source: "go.mod".into(),
        },
        DiscoveredTask {
            name: "lint".into(),
            description: "Run Go vet".into(),
            body: "go vet ./...".into(),
            source: "go.mod".into(),
        },
    ];

    if dir.join("cmd").exists() {
        tasks.push(DiscoveredTask {
            name: "run".into(),
            description: "Run the application".into(),
            body: "go run ./cmd/...".into(),
            source: "go.mod (cmd/ detected)".into(),
        });
    }

    tasks
}

fn detect_python(dir: &Path) -> Vec<DiscoveredTask> {
    let has_pyproject = dir.join("pyproject.toml").exists();
    let has_requirements = dir.join("requirements.txt").exists();
    let has_pipfile = dir.join("Pipfile").exists();

    if !has_pyproject && !has_requirements && !has_pipfile {
        return Vec::new();
    }

    let mut tasks = Vec::new();

    if has_pyproject {
        let content = fs::read_to_string(dir.join("pyproject.toml")).unwrap_or_default();

        // Detect poetry
        if content.contains("[tool.poetry]") {
            tasks.push(DiscoveredTask {
                name: "install".into(),
                description: "Install dependencies with Poetry".into(),
                body: "poetry install".into(),
                source: "pyproject.toml (poetry)".into(),
            });
            tasks.push(DiscoveredTask {
                name: "test".into(),
                description: "Run tests".into(),
                body: "poetry run pytest".into(),
                source: "pyproject.toml (poetry)".into(),
            });
            tasks.push(DiscoveredTask {
                name: "lint".into(),
                description: "Run linter".into(),
                body: "poetry run ruff check .".into(),
                source: "pyproject.toml (poetry)".into(),
            });
        }

        // Detect uv
        if content.contains("[tool.uv]") {
            tasks.push(DiscoveredTask {
                name: "install".into(),
                description: "Install dependencies with uv".into(),
                body: "uv sync".into(),
                source: "pyproject.toml (uv)".into(),
            });
            tasks.push(DiscoveredTask {
                name: "test".into(),
                description: "Run tests".into(),
                body: "uv run pytest".into(),
                source: "pyproject.toml (uv)".into(),
            });
        }

        // Generic pyproject
        if tasks.is_empty() {
            tasks.push(DiscoveredTask {
                name: "install".into(),
                description: "Install the project".into(),
                body: "pip install -e .".into(),
                source: "pyproject.toml".into(),
            });
        }

        // Detect pytest
        if (content.contains("[tool.pytest]") || dir.join("tests").exists())
            && !tasks.iter().any(|t| t.name == "test")
        {
            tasks.push(DiscoveredTask {
                name: "test".into(),
                description: "Run tests with pytest".into(),
                body: "pytest".into(),
                source: "pyproject.toml".into(),
            });
        }
    } else if has_requirements {
        tasks.push(DiscoveredTask {
            name: "install".into(),
            description: "Install Python dependencies".into(),
            body: "pip install -r requirements.txt".into(),
            source: "requirements.txt".into(),
        });
    }

    tasks
}

fn detect_ruby(dir: &Path) -> Vec<DiscoveredTask> {
    if !dir.join("Gemfile").exists() {
        return Vec::new();
    }

    let content = fs::read_to_string(dir.join("Gemfile")).unwrap_or_default();
    let mut tasks = Vec::new();

    tasks.push(DiscoveredTask {
        name: "install".into(),
        description: "Install Ruby dependencies".into(),
        body: "bundle install".into(),
        source: "Gemfile".into(),
    });

    if content.contains("rails") || dir.join("config").join("routes.rb").exists() {
        tasks.push(DiscoveredTask {
            name: "server".into(),
            description: "Start Rails server".into(),
            body: "bundle exec rails server".into(),
            source: "Gemfile (Rails detected)".into(),
        });
        tasks.push(DiscoveredTask {
            name: "console".into(),
            description: "Open Rails console".into(),
            body: "bundle exec rails console".into(),
            source: "Gemfile (Rails detected)".into(),
        });
        tasks.push(DiscoveredTask {
            name: "db:migrate".into(),
            description: "Run database migrations".into(),
            body: "bundle exec rails db:migrate".into(),
            source: "Gemfile (Rails detected)".into(),
        });
        tasks.push(DiscoveredTask {
            name: "test".into(),
            description: "Run Rails tests".into(),
            body: "bundle exec rails test".into(),
            source: "Gemfile (Rails detected)".into(),
        });
    }

    if content.contains("rspec") {
        tasks.push(DiscoveredTask {
            name: "test".into(),
            description: "Run RSpec tests".into(),
            body: "bundle exec rspec".into(),
            source: "Gemfile (rspec detected)".into(),
        });
    }

    tasks
}

// ─── Helpers ───────────────────────────────────────────────

fn sanitize_task_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c == ':' || c == '/' {
                ':'
            } else {
                '-'
            }
        })
        .collect()
}

/// Simple JSON object extractor — pulls key-value pairs from a top-level object field.
/// Not a full JSON parser but handles the common case of `"scripts": { "key": "value" }`.
fn extract_json_object(json: &str, field: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    let search = format!("\"{}\"", field);
    let field_start = match json.find(&search) {
        Some(pos) => pos + search.len(),
        None => return result,
    };

    // Find the opening brace
    let rest = &json[field_start..];
    let brace_start = match rest.find('{') {
        Some(pos) => field_start + pos,
        None => return result,
    };

    // Find matching closing brace
    let mut depth = 0;
    let mut brace_end = brace_start;
    for (i, c) in json[brace_start..].chars().enumerate() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    brace_end = brace_start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    let obj = &json[brace_start + 1..brace_end];

    // Extract "key": "value" pairs
    let mut chars = obj.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '"' {
            chars.next();
            let key: String = chars.by_ref().take_while(|&c| c != '"').collect();
            // Skip colon
            while let Some(&c) = chars.peek() {
                if c == ':' {
                    chars.next();
                    break;
                }
                chars.next();
            }
            // Skip whitespace
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }
            // Read value
            if let Some(&'"') = chars.peek() {
                chars.next();
                let value: String = chars.by_ref().take_while(|&c| c != '"').collect();
                result.insert(key, value);
            }
        } else {
            chars.next();
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn extract_json_scripts() {
        let json = r#"{
            "name": "myapp",
            "scripts": {
                "dev": "vite",
                "build": "vite build",
                "test": "vitest"
            }
        }"#;
        let scripts = extract_json_object(json, "scripts");
        assert_eq!(scripts.get("dev").unwrap(), "vite");
        assert_eq!(scripts.get("build").unwrap(), "vite build");
        assert_eq!(scripts.get("test").unwrap(), "vitest");
    }

    #[test]
    fn extract_compose_services_basic() {
        let yaml = r#"
services:
  web:
    image: nginx
  db:
    image: postgres
  redis:
    image: redis
"#;
        let services = extract_compose_services(yaml);
        assert_eq!(services, vec!["web", "db", "redis"]);
    }

    #[test]
    fn detect_package_json_vue() {
        let tmp = setup();
        fs::write(
            tmp.path().join("package.json"),
            r#"{
                "name": "my-vue-app",
                "scripts": {
                    "dev": "vite",
                    "build": "vite build",
                    "preview": "vite preview"
                },
                "dependencies": {
                    "vue": "^3.4.0"
                },
                "devDependencies": {
                    "vitest": "^1.0.0"
                }
            }"#,
        )
        .unwrap();

        let tasks = detect_package_json(tmp.path());
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"dev"));
        assert!(names.contains(&"build"));
        assert!(names.contains(&"preview"));
    }

    #[test]
    fn detect_docker_compose_services() {
        let tmp = setup();
        fs::write(
            tmp.path().join("docker-compose.yml"),
            r#"services:
  app:
    build: .
  db:
    image: postgres
"#,
        )
        .unwrap();

        let tasks = detect_docker_compose(tmp.path());
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"up"));
        assert!(names.contains(&"down"));
        assert!(names.contains(&"app:logs"));
        assert!(names.contains(&"db:shell"));
    }

    #[test]
    fn detect_cargo_workspace() {
        let tmp = setup();
        fs::write(
            tmp.path().join("Cargo.toml"),
            r#"[workspace]
members = ["crates/*"]

[package]
name = "myapp"
"#,
        )
        .unwrap();

        let tasks = detect_cargo(tmp.path());
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"build"));
        assert!(names.contains(&"test"));
        assert!(names.contains(&"test-all"));
    }

    #[test]
    fn detect_makefile_targets() {
        let tmp = setup();
        fs::write(
            tmp.path().join("Makefile"),
            "build:\n\tgo build ./...\n\ntest:\n\tgo test ./...\n\n.PHONY: build test\n",
        )
        .unwrap();

        let tasks = detect_makefile(tmp.path());
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"make:build"));
        assert!(names.contains(&"make:test"));
        // .PHONY should be skipped
        assert!(!names.iter().any(|n| n.contains("PHONY")));
    }

    #[test]
    fn sanitize_names() {
        assert_eq!(sanitize_task_name("build:prod"), "build:prod");
        assert_eq!(sanitize_task_name("test.unit"), "test-unit");
        assert_eq!(sanitize_task_name("lint/fix"), "lint:fix");
    }

    #[test]
    fn load_existing_tasks() {
        let tmp = setup();
        fs::write(
            tmp.path().join("Taskfile"),
            "task build {\n  cargo build\n}\n\ntask test {\n  cargo test\n}\n",
        )
        .unwrap();

        let names = load_existing_task_names(tmp.path());
        assert!(names.contains(&"build".to_string()));
        assert!(names.contains(&"test".to_string()));
    }

    #[test]
    fn detects_pnpm() {
        let tmp = setup();
        fs::write(tmp.path().join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(detect_node_package_manager(tmp.path()), "pnpm");
    }

    #[test]
    fn detects_yarn() {
        let tmp = setup();
        fs::write(tmp.path().join("yarn.lock"), "").unwrap();
        assert_eq!(detect_node_package_manager(tmp.path()), "yarn");
    }

    #[test]
    fn detects_bun() {
        let tmp = setup();
        fs::write(tmp.path().join("bun.lockb"), "").unwrap();
        assert_eq!(detect_node_package_manager(tmp.path()), "bun");
    }
}
