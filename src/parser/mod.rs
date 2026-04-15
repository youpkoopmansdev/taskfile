pub mod ast;
pub mod error;

use std::path::Path;

use ast::{Alias, Ast, DotEnv, Export, Include, Param, Task};
use error::ParseError;

pub fn parse(input: &str, filepath: &Path) -> Result<Ast, ParseError> {
    let mut tasks = Vec::new();
    let mut aliases = Vec::new();
    let mut exports = Vec::new();
    let mut includes = Vec::new();
    let mut dotenv = Vec::new();

    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;
    let mut pending_description: Option<String> = None;
    let mut pending_confirm: Option<String> = None;

    while i < lines.len() {
        let line_num = i + 1;
        let line = lines[i].trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }

        // Check for annotations that must precede a task
        let has_pending_annotation = pending_description.is_some() || pending_confirm.is_some();

        if line.starts_with("@description ") {
            pending_description = Some(
                line.strip_prefix("@description ")
                    .unwrap()
                    .trim()
                    .to_string(),
            );
            i += 1;
        } else if line.starts_with("@confirm") {
            let msg = line.strip_prefix("@confirm").unwrap().trim().to_string();
            pending_confirm = Some(if msg.is_empty() {
                "Are you sure?".to_string()
            } else {
                msg
            });
            i += 1;
        } else if line.starts_with("export ") {
            if has_pending_annotation {
                return Err(ParseError::syntax(
                    filepath,
                    line_num,
                    "@description/@confirm must be followed by a task definition",
                ));
            }
            exports.push(parse_export(line, filepath, line_num)?);
            i += 1;
        } else if line.starts_with("alias ") {
            if has_pending_annotation {
                return Err(ParseError::syntax(
                    filepath,
                    line_num,
                    "@description/@confirm must be followed by a task definition",
                ));
            }
            aliases.push(parse_alias(line, filepath, line_num)?);
            i += 1;
        } else if line.starts_with("include ") {
            if has_pending_annotation {
                return Err(ParseError::syntax(
                    filepath,
                    line_num,
                    "@description/@confirm must be followed by a task definition",
                ));
            }
            includes.push(parse_include(line, filepath, line_num)?);
            i += 1;
        } else if line.starts_with("dotenv ") {
            if has_pending_annotation {
                return Err(ParseError::syntax(
                    filepath,
                    line_num,
                    "@description/@confirm must be followed by a task definition",
                ));
            }
            dotenv.push(parse_dotenv(line, filepath, line_num)?);
            i += 1;
        } else if line.starts_with("task ") {
            let (mut task, next_i) = parse_task(&lines, i, filepath)?;
            if let Some(desc) = pending_description.take() {
                task.description = Some(desc);
            }
            if let Some(msg) = pending_confirm.take() {
                task.confirm = Some(msg);
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
        dotenv,
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

    // Parse the task header: name, optional [params], optional depends=[...], optional depends_parallel=[...], then {
    let mut cursor = rest;
    let description = None;
    let mut params = Vec::new();
    let mut dependencies = Vec::new();
    let mut parallel_dependencies = Vec::new();
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
        } else if cursor.starts_with("depends_parallel=[") {
            let (deps, rest) =
                parse_depends_prefixed(cursor, "depends_parallel=[", filepath, line_num)?;
            parallel_dependencies = deps;
            cursor = rest.trim_start();
        } else if cursor.starts_with("depends=[") {
            let (deps, rest) = parse_depends_prefixed(cursor, "depends=[", filepath, line_num)?;
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

    // Collect body lines until braces balance (string/comment-aware)
    let mut brace_depth: i32 = 1;
    let mut body_lines = Vec::new();

    while i < lines.len() {
        let l = lines[i];
        count_braces(l, &mut brace_depth);

        if brace_depth == 0 {
            // Don't include the closing brace line unless there's content before '}'
            let trimmed = l.trim();
            if trimmed != "}"
                && let Some(pos) = l.rfind('}')
            {
                let before = &l[..pos];
                if !before.trim().is_empty() {
                    body_lines.push(before);
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
            confirm: None,
            params,
            dependencies,
            parallel_dependencies,
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

    // Parse params handling quoted defaults with spaces
    let mut chars = inner.chars().peekable();
    while chars.peek().is_some() {
        // Skip whitespace between params
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }

        // Read param name (until '=' or whitespace or end)
        let mut name = String::new();
        while chars
            .peek()
            .is_some_and(|c| *c != '=' && !c.is_whitespace())
        {
            name.push(chars.next().unwrap());
        }

        if name.is_empty() {
            break;
        }

        if !is_valid_identifier(&name) {
            return Err(ParseError::syntax(
                filepath,
                line_num,
                format!(
                    "invalid parameter name '{}' — must be a valid identifier (letters, digits, underscores)",
                    name
                ),
            ));
        }

        if chars.peek() == Some(&'=') {
            chars.next(); // consume '='
            let default = if chars.peek() == Some(&'"') {
                // Quoted default — read until closing quote
                chars.next(); // consume opening "
                let mut val = String::new();
                let mut escaped = false;
                for ch in chars.by_ref() {
                    if escaped {
                        val.push(ch);
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == '"' {
                        break;
                    } else {
                        val.push(ch);
                    }
                }
                val
            } else {
                // Unquoted default — read until whitespace
                let mut val = String::new();
                while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                    val.push(chars.next().unwrap());
                }
                val
            };
            params.push(Param {
                name,
                default: Some(default),
            });
        } else {
            params.push(Param {
                name,
                default: None,
            });
        }
    }

    Ok((params, &input[end + 1..]))
}

fn parse_depends_prefixed<'a>(
    input: &'a str,
    prefix: &str,
    filepath: &Path,
    line_num: usize,
) -> Result<(Vec<String>, &'a str), ParseError> {
    let rest = input.strip_prefix(prefix).unwrap();
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

fn parse_dotenv(line: &str, filepath: &Path, line_num: usize) -> Result<DotEnv, ParseError> {
    let rest = line.strip_prefix("dotenv ").unwrap().trim();
    let path = unquote(rest);

    if path.is_empty() {
        return Err(ParseError::syntax(filepath, line_num, "empty dotenv path"));
    }

    Ok(DotEnv {
        path,
        line: line_num,
    })
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Count braces in a line, skipping braces inside single-quoted strings,
/// double-quoted strings, and comments.
fn count_braces(line: &str, depth: &mut i32) {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';

    for ch in line.chars() {
        // Comments outside of strings end brace counting for the rest of the line
        if !in_single && !in_double && ch == '#' {
            break;
        }

        if ch == '\'' && !in_double && prev != '\\' {
            in_single = !in_single;
        } else if ch == '"' && !in_single && prev != '\\' {
            in_double = !in_double;
        } else if !in_single && !in_double {
            if ch == '{' {
                *depth += 1;
            } else if ch == '}' {
                *depth -= 1;
            }
        }

        prev = ch;
    }
}

fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
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

    #[test]
    fn parse_braces_inside_strings() {
        let input = r#"task test {
  echo "use { and } in output"
  echo 'more {braces}'
  # comment with { brace
  echo "done"
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.tasks.len(), 1);
        assert!(ast.tasks[0].body.contains("use { and }"));
        assert!(ast.tasks[0].body.contains("done"));
    }

    #[test]
    fn parse_empty_task_body() {
        let input = "task noop {\n}\n";
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.tasks.len(), 1);
        assert_eq!(ast.tasks[0].name, "noop");
        assert!(ast.tasks[0].body.trim().is_empty());
    }

    #[test]
    fn description_before_non_task_is_error() {
        let input = "@description Some desc\nexport X=\"y\"\n";
        let result = parse(input, &test_path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("@description/@confirm must be followed by a task"));
    }

    #[test]
    fn param_default_with_spaces() {
        let input = r#"task greet [msg="hello world" name="foo bar"] {
  echo "$msg $name"
}"#;
        let ast = parse(input, &test_path()).unwrap();
        assert_eq!(ast.tasks[0].params.len(), 2);
        assert_eq!(
            ast.tasks[0].params[0].default.as_deref(),
            Some("hello world")
        );
        assert_eq!(ast.tasks[0].params[1].default.as_deref(), Some("foo bar"));
    }

    #[test]
    fn invalid_param_name_is_error() {
        let input = "task test [$invalid] {\n  echo hi\n}\n";
        let result = parse(input, &test_path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid parameter name"));
    }

    #[test]
    fn unquote_single_char() {
        // Ensure single-char strings don't panic
        assert_eq!(unquote("\""), "\"");
        assert_eq!(unquote("'"), "'");
        assert_eq!(unquote(""), "");
    }

    #[test]
    fn count_braces_string_aware() {
        let mut depth: i32 = 0;
        count_braces(r#"echo "{ hello }""#, &mut depth);
        assert_eq!(depth, 0);

        depth = 0;
        count_braces("real_brace {", &mut depth);
        assert_eq!(depth, 1);

        depth = 0;
        count_braces("} # closing { brace in comment", &mut depth);
        assert_eq!(depth, -1);
    }
}
