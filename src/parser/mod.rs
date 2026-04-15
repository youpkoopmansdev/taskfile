pub mod ast;
pub mod error;

use std::path::Path;

use ast::{Alias, Ast, Export, Include, Param, Task};
use error::ParseError;

pub fn parse(input: &str, filepath: &Path) -> Result<Ast, ParseError> {
    let mut tasks = Vec::new();
    let mut aliases = Vec::new();
    let mut exports = Vec::new();
    let mut includes = Vec::new();

    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;
    let mut pending_description: Option<String> = None;

    while i < lines.len() {
        let line_num = i + 1;
        let line = lines[i].trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }

        if line.starts_with("@description ") {
            pending_description = Some(
                line.strip_prefix("@description ")
                    .unwrap()
                    .trim()
                    .to_string(),
            );
            i += 1;
        } else if line.starts_with("export ") {
            exports.push(parse_export(line, filepath, line_num)?);
            i += 1;
        } else if line.starts_with("alias ") {
            aliases.push(parse_alias(line, filepath, line_num)?);
            i += 1;
        } else if line.starts_with("include ") {
            includes.push(parse_include(line, filepath, line_num)?);
            i += 1;
        } else if line.starts_with("task ") {
            let (mut task, next_i) = parse_task(&lines, i, filepath)?;
            if let Some(desc) = pending_description.take() {
                task.description = Some(desc);
            }
            tasks.push(task);
            i = next_i;
        } else {
            return Err(ParseError::syntax(
                filepath,
                line_num,
                format!("unexpected line: {}", line),
            ));
        }
    }

    Ok(Ast {
        tasks,
        aliases,
        exports,
        includes,
    })
}

fn parse_export(line: &str, filepath: &Path, line_num: usize) -> Result<Export, ParseError> {
    let rest = line.strip_prefix("export ").unwrap().trim();
    let Some(eq_pos) = rest.find('=') else {
        return Err(ParseError::syntax(
            filepath,
            line_num,
            "expected '=' in export statement",
        ));
    };

    let key = rest[..eq_pos].trim().to_string();
    let value = unquote(rest[eq_pos + 1..].trim());

    if key.is_empty() {
        return Err(ParseError::syntax(filepath, line_num, "empty export key"));
    }

    Ok(Export { key, value })
}

fn parse_alias(line: &str, filepath: &Path, line_num: usize) -> Result<Alias, ParseError> {
    let rest = line.strip_prefix("alias ").unwrap().trim();
    let Some(eq_pos) = rest.find('=') else {
        return Err(ParseError::syntax(
            filepath,
            line_num,
            "expected '=' in alias statement",
        ));
    };

    let name = rest[..eq_pos].trim().to_string();
    let value = unquote(rest[eq_pos + 1..].trim());

    if name.is_empty() {
        return Err(ParseError::syntax(filepath, line_num, "empty alias name"));
    }

    Ok(Alias { name, value })
}

fn parse_include(line: &str, filepath: &Path, line_num: usize) -> Result<Include, ParseError> {
    let rest = line.strip_prefix("include ").unwrap().trim();
    let path = unquote(rest);

    if path.is_empty() {
        return Err(ParseError::syntax(filepath, line_num, "empty include path"));
    }

    Ok(Include {
        path,
        line: line_num,
    })
}

fn parse_task(lines: &[&str], start: usize, filepath: &Path) -> Result<(Task, usize), ParseError> {
    let line_num = start + 1;
    let line = lines[start].trim();
    let rest = line.strip_prefix("task ").unwrap();

    // Parse the task header: name, optional description, optional [params], optional depends=[...], then {
    let mut cursor = rest;
    let description = None;
    let mut params = Vec::new();
    let mut dependencies = Vec::new();
    let mut found_open_brace = false;

    // Parse task name (alphanumeric, hyphens, underscores)
    let name_end = cursor
        .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .unwrap_or(cursor.len());
    let name = cursor[..name_end].to_string();

    if name.is_empty() {
        return Err(ParseError::syntax(filepath, line_num, "expected task name"));
    }
    cursor = cursor[name_end..].trim_start();

    // Now parse optional parts in any order until we find '{'
    loop {
        if cursor.is_empty() {
            break;
        }

        if cursor.starts_with('{') {
            found_open_brace = true;
            break;
        }

        if cursor.starts_with('[') {
            let (p, rest) = parse_params(cursor, filepath, line_num)?;
            params = p;
            cursor = rest.trim_start();
        } else if cursor.starts_with("depends=[") {
            let (deps, rest) = parse_depends(cursor, filepath, line_num)?;
            dependencies = deps;
            cursor = rest.trim_start();
        } else {
            return Err(ParseError::syntax(
                filepath,
                line_num,
                format!("unexpected token in task header: {}", cursor),
            ));
        }
    }

    // If we haven't found '{' yet, look for it on the next line(s)
    let mut i = start + 1;
    if !found_open_brace {
        while i < lines.len() {
            let l = lines[i].trim();
            if l.is_empty() || l.starts_with('#') {
                i += 1;
                continue;
            }
            if l.starts_with('{') {
                found_open_brace = true;
                i += 1;
                break;
            }
            return Err(ParseError::syntax(
                filepath,
                i + 1,
                "expected '{' to open task body",
            ));
        }
    } else {
        // Already consumed the '{' on the same line
    }

    if !found_open_brace {
        return Err(ParseError::syntax(
            filepath,
            line_num,
            format!("expected '{{' for task '{}'", name),
        ));
    }

    // Collect body lines until braces balance
    let mut brace_depth = 1;
    let mut body_lines = Vec::new();

    while i < lines.len() {
        let l = lines[i];
        for ch in l.chars() {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                brace_depth -= 1;
            }
        }

        if brace_depth == 0 {
            // Don't include the closing brace line unless there's content before '}'
            let trimmed = l.trim();
            if trimmed != "}" {
                // There might be content before the closing brace
                if let Some(pos) = l.rfind('}') {
                    let before = &l[..pos];
                    if !before.trim().is_empty() {
                        body_lines.push(before);
                    }
                }
            }
            i += 1;
            break;
        }

        body_lines.push(l);
        i += 1;
    }

    if brace_depth != 0 {
        return Err(ParseError::syntax(
            filepath,
            line_num,
            format!("unclosed '{{' for task '{}'", name),
        ));
    }

    // Dedent body: find minimum indentation and strip it
    let body = dedent_body(&body_lines);

    Ok((
        Task {
            name,
            description,
            params,
            dependencies,
            body,
            line: line_num,
        },
        i,
    ))
}

fn parse_params<'a>(
    input: &'a str,
    filepath: &Path,
    line_num: usize,
) -> Result<(Vec<Param>, &'a str), ParseError> {
    assert!(input.starts_with('['));
    let Some(end) = input.find(']') else {
        return Err(ParseError::syntax(
            filepath,
            line_num,
            "unterminated parameter list",
        ));
    };

    let inner = &input[1..end];
    let mut params = Vec::new();

    for token in inner.split_whitespace() {
        if let Some(eq_pos) = token.find('=') {
            let name = token[..eq_pos].to_string();
            let default = unquote(&token[eq_pos + 1..]);
            params.push(Param {
                name,
                default: Some(default),
            });
        } else {
            params.push(Param {
                name: token.to_string(),
                default: None,
            });
        }
    }

    Ok((params, &input[end + 1..]))
}

fn parse_depends<'a>(
    input: &'a str,
    filepath: &Path,
    line_num: usize,
) -> Result<(Vec<String>, &'a str), ParseError> {
    let rest = input.strip_prefix("depends=[").unwrap();
    let Some(end) = rest.find(']') else {
        return Err(ParseError::syntax(
            filepath,
            line_num,
            "unterminated depends list",
        ));
    };

    let inner = &rest[..end];
    let deps: Vec<String> = inner
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok((deps, &rest[end + 1..]))
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn dedent_body(lines: &[&str]) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let min_indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);

    lines
        .iter()
        .map(|l| {
            if l.len() >= min_indent {
                &l[min_indent..]
            } else {
                l.trim()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_path() -> PathBuf {
        PathBuf::from("test.Taskfile")
    }

    #[test]
    fn parse_single_task() {
        let input = r#"task build {
  echo "building"
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.tasks.len(), 1);
        assert_eq!(ast.tasks[0].name, "build");
        assert!(ast.tasks[0].body.contains("echo"));
    }

    #[test]
    fn parse_multiple_tasks() {
        let input = r#"task build {
  echo "building"
}

task test {
  cargo test
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.tasks.len(), 2);
        assert_eq!(ast.tasks[0].name, "build");
        assert_eq!(ast.tasks[1].name, "test");
    }

    #[test]
    fn parse_task_with_description() {
        let input = r#"@description Build the project
task build {
  cargo build
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(
            ast.tasks[0].description.as_deref(),
            Some("Build the project")
        );
    }

    #[test]
    fn parse_task_with_params() {
        let input = r#"task greet [name="world"] {
  echo "Hello, $name!"
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.tasks[0].params.len(), 1);
        assert_eq!(ast.tasks[0].params[0].name, "name");
        assert_eq!(ast.tasks[0].params[0].default.as_deref(), Some("world"));
    }

    #[test]
    fn parse_task_with_required_param() {
        let input = r#"task deploy [env target="latest"] {
  echo "$env $target"
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.tasks[0].params.len(), 2);
        assert_eq!(ast.tasks[0].params[0].name, "env");
        assert!(ast.tasks[0].params[0].default.is_none());
        assert_eq!(ast.tasks[0].params[1].default.as_deref(), Some("latest"));
    }

    #[test]
    fn parse_task_with_dependencies() {
        let input = r#"task build depends=[clean, compile] {
  echo "Done"
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.tasks[0].dependencies, vec!["clean", "compile"]);
    }

    #[test]
    fn parse_exports() {
        let input = r#"export PROJECT_NAME="myapp"
export PORT=3000"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.exports.len(), 2);
        assert_eq!(ast.exports[0].key, "PROJECT_NAME");
        assert_eq!(ast.exports[0].value, "myapp");
        assert_eq!(ast.exports[1].key, "PORT");
        assert_eq!(ast.exports[1].value, "3000");
    }

    #[test]
    fn parse_aliases() {
        let input = r#"alias dc="docker compose"
alias k="kubectl""#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.aliases.len(), 2);
        assert_eq!(ast.aliases[0].name, "dc");
        assert_eq!(ast.aliases[0].value, "docker compose");
    }

    #[test]
    fn parse_includes() {
        let input = r#"include "tasks/docker.Taskfile"
include "tasks/deploy.Taskfile""#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.includes.len(), 2);
        assert_eq!(ast.includes[0].path, "tasks/docker.Taskfile");
    }

    #[test]
    fn parse_comments_and_empty_lines() {
        let input = r#"# This is a comment

# Another comment
task build {
  echo "hi"
}
"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.tasks.len(), 1);
    }

    #[test]
    fn parse_nested_braces() {
        let input = r#"task build {
  if [ -f "file" ]; then
    echo "found"
  fi
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert!(ast.tasks[0].body.contains("if [ -f"));
        assert!(ast.tasks[0].body.contains("fi"));
    }

    #[test]
    fn parse_brace_on_next_line() {
        let input = r#"task build
{
  echo "building"
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.tasks[0].name, "build");
        assert!(ast.tasks[0].body.contains("echo"));
    }

    #[test]
    fn error_missing_brace() {
        let input = "task build\n  echo hi\n";
        let result = parse(input, &test_path());
        assert!(result.is_err());
    }

    #[test]
    fn error_missing_task_name() {
        let input = "task {\n  echo hi\n}\n";
        let result = parse(input, &test_path());
        assert!(result.is_err());
    }

    #[test]
    fn error_unclosed_brace() {
        let input = "task build {\n  echo hi\n";
        let result = parse(input, &test_path());
        assert!(result.is_err());
    }

    #[test]
    fn parse_full_taskfile() {
        let input = r#"# Project config
export PROJECT="myapp"
export PORT=3000

alias dc="docker compose"

include "tasks/docker.Taskfile"

@description Set up the project
task setup {
  echo "Setting up $PROJECT..."
}

@description Build it
task build depends=[clean] [target="release"] {
  cargo build --$target
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.exports.len(), 2);
        assert_eq!(ast.aliases.len(), 1);
        assert_eq!(ast.includes.len(), 1);
        assert_eq!(ast.tasks.len(), 2);
        assert_eq!(
            ast.tasks[0].description.as_deref(),
            Some("Set up the project")
        );
        assert_eq!(ast.tasks[1].dependencies, vec!["clean"]);
        assert_eq!(ast.tasks[1].params[0].default.as_deref(), Some("release"));
    }
}
