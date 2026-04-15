pub mod ast;

use ast::{Alias, Ast, Dependency, DotEnv, Export, Include, Param, Span, Task};
use ast::{Diagnostic, DiagnosticSeverity};

pub fn parse(input: &str) -> Ast {
    let mut tasks = Vec::new();
    let mut aliases = Vec::new();
    let mut exports = Vec::new();
    let mut includes = Vec::new();
    let mut dotenv = Vec::new();
    let mut diagnostics = Vec::new();

    let lines: Vec<&str> = input.lines().collect();
    let mut i: usize = 0;
    let mut pending_description: Option<(String, u32)> = None;
    let mut pending_confirm: Option<(String, u32)> = None;

    while i < lines.len() {
        let line_idx = i as u32; // 0-based line index for LSP
        let raw_line = lines[i];
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }

        let has_pending_annotation = pending_description.is_some() || pending_confirm.is_some();

        if line.starts_with("@description ") {
            let desc = line
                .strip_prefix("@description ")
                .unwrap()
                .trim()
                .to_string();
            pending_description = Some((desc, line_idx));
            i += 1;
        } else if line.starts_with("@confirm") {
            let msg = line.strip_prefix("@confirm").unwrap().trim().to_string();
            let msg = if msg.is_empty() {
                "Are you sure?".to_string()
            } else {
                msg
            };
            pending_confirm = Some((msg, line_idx));
            i += 1;
        } else if line.starts_with("export ") {
            if has_pending_annotation {
                let warn_line = pending_description
                    .as_ref()
                    .map(|(_, l)| *l)
                    .or(pending_confirm.as_ref().map(|(_, l)| *l))
                    .unwrap_or(line_idx);
                diagnostics.push(Diagnostic {
                    span: Span::new(warn_line, 0, warn_line, 0),
                    message: "@description/@confirm must be followed by a task definition"
                        .to_string(),
                    severity: DiagnosticSeverity::Warning,
                });
                pending_description = None;
                pending_confirm = None;
            }
            match parse_export(raw_line, line_idx) {
                Ok(exp) => exports.push(exp),
                Err(diag) => diagnostics.push(diag),
            }
            i += 1;
        } else if line.starts_with("alias ") {
            if has_pending_annotation {
                let warn_line = pending_description
                    .as_ref()
                    .map(|(_, l)| *l)
                    .or(pending_confirm.as_ref().map(|(_, l)| *l))
                    .unwrap_or(line_idx);
                diagnostics.push(Diagnostic {
                    span: Span::new(warn_line, 0, warn_line, 0),
                    message: "@description/@confirm must be followed by a task definition"
                        .to_string(),
                    severity: DiagnosticSeverity::Warning,
                });
                pending_description = None;
                pending_confirm = None;
            }
            match parse_alias(raw_line, line_idx) {
                Ok(a) => aliases.push(a),
                Err(diag) => diagnostics.push(diag),
            }
            i += 1;
        } else if line.starts_with("include ") {
            if has_pending_annotation {
                let warn_line = pending_description
                    .as_ref()
                    .map(|(_, l)| *l)
                    .or(pending_confirm.as_ref().map(|(_, l)| *l))
                    .unwrap_or(line_idx);
                diagnostics.push(Diagnostic {
                    span: Span::new(warn_line, 0, warn_line, 0),
                    message: "@description/@confirm must be followed by a task definition"
                        .to_string(),
                    severity: DiagnosticSeverity::Warning,
                });
                pending_description = None;
                pending_confirm = None;
            }
            match parse_include(raw_line, line_idx) {
                Ok(inc) => includes.push(inc),
                Err(diag) => diagnostics.push(diag),
            }
            i += 1;
        } else if line.starts_with("dotenv ") {
            if has_pending_annotation {
                let warn_line = pending_description
                    .as_ref()
                    .map(|(_, l)| *l)
                    .or(pending_confirm.as_ref().map(|(_, l)| *l))
                    .unwrap_or(line_idx);
                diagnostics.push(Diagnostic {
                    span: Span::new(warn_line, 0, warn_line, 0),
                    message: "@description/@confirm must be followed by a task definition"
                        .to_string(),
                    severity: DiagnosticSeverity::Warning,
                });
                pending_description = None;
                pending_confirm = None;
            }
            match parse_dotenv(raw_line, line_idx) {
                Ok(d) => dotenv.push(d),
                Err(diag) => diagnostics.push(diag),
            }
            i += 1;
        } else if line.starts_with("task ") {
            match parse_task(&lines, i) {
                Ok((mut task, next_i)) => {
                    if let Some((desc, _)) = pending_description.take() {
                        task.description = Some(desc);
                    }
                    if let Some((msg, _)) = pending_confirm.take() {
                        task.confirm = Some(msg);
                    }
                    tasks.push(task);
                    i = next_i;
                }
                Err(diag) => {
                    diagnostics.push(diag);
                    pending_description = None;
                    pending_confirm = None;
                    // Recovery: skip until we find a line starting with a top-level keyword or EOF
                    i += 1;
                    i = skip_to_next_toplevel(&lines, i);
                }
            }
        } else {
            diagnostics.push(Diagnostic {
                span: Span::new(line_idx, 0, line_idx, raw_line.len() as u32),
                message: format!("unexpected line: {}", line),
                severity: DiagnosticSeverity::Error,
            });
            i += 1;
        }
    }

    // Warn about trailing annotations
    if let Some((_, line)) = pending_description {
        diagnostics.push(Diagnostic {
            span: Span::new(line, 0, line, 0),
            message: "@description not followed by a task definition".to_string(),
            severity: DiagnosticSeverity::Warning,
        });
    }
    if let Some((_, line)) = pending_confirm {
        diagnostics.push(Diagnostic {
            span: Span::new(line, 0, line, 0),
            message: "@confirm not followed by a task definition".to_string(),
            severity: DiagnosticSeverity::Warning,
        });
    }

    Ast {
        tasks,
        aliases,
        exports,
        includes,
        dotenv,
        diagnostics,
    }
}

fn skip_to_next_toplevel(lines: &[&str], from: usize) -> usize {
    let keywords = [
        "task ",
        "export ",
        "alias ",
        "include ",
        "dotenv ",
        "@description",
        "@confirm",
    ];
    let mut i = from;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if keywords.iter().any(|kw| trimmed.starts_with(kw)) {
            return i;
        }
        // Also stop if we hit a closing brace at depth 0 (stray brace)
        if trimmed == "}" {
            return i + 1;
        }
        i += 1;
    }
    i
}

fn parse_export(raw_line: &str, line_idx: u32) -> Result<Export, Diagnostic> {
    let leading = raw_line.len() - raw_line.trim_start().len();
    let line = raw_line.trim();
    let rest = line.strip_prefix("export ").unwrap().trim();
    let Some(eq_pos) = rest.find('=') else {
        return Err(Diagnostic {
            span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
            message: "expected '=' in export statement".to_string(),
            severity: DiagnosticSeverity::Error,
        });
    };

    let key = rest[..eq_pos].trim().to_string();
    let value = unquote(rest[eq_pos + 1..].trim());

    if key.is_empty() {
        return Err(Diagnostic {
            span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
            message: "empty export key".to_string(),
            severity: DiagnosticSeverity::Error,
        });
    }

    Ok(Export {
        key,
        value,
        span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
    })
}

fn parse_alias(raw_line: &str, line_idx: u32) -> Result<Alias, Diagnostic> {
    let leading = raw_line.len() - raw_line.trim_start().len();
    let line = raw_line.trim();
    let rest = line.strip_prefix("alias ").unwrap().trim();
    let Some(eq_pos) = rest.find('=') else {
        return Err(Diagnostic {
            span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
            message: "expected '=' in alias statement".to_string(),
            severity: DiagnosticSeverity::Error,
        });
    };

    let name = rest[..eq_pos].trim().to_string();
    let value = unquote(rest[eq_pos + 1..].trim());

    if name.is_empty() {
        return Err(Diagnostic {
            span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
            message: "empty alias name".to_string(),
            severity: DiagnosticSeverity::Error,
        });
    }

    Ok(Alias {
        name,
        value,
        span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
    })
}

fn parse_include(raw_line: &str, line_idx: u32) -> Result<Include, Diagnostic> {
    let leading = raw_line.len() - raw_line.trim_start().len();
    let line = raw_line.trim();
    let rest = line.strip_prefix("include ").unwrap().trim();
    let path = unquote(rest);

    if path.is_empty() {
        return Err(Diagnostic {
            span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
            message: "empty include path".to_string(),
            severity: DiagnosticSeverity::Error,
        });
    }

    Ok(Include {
        path,
        span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
    })
}

fn parse_dotenv(raw_line: &str, line_idx: u32) -> Result<DotEnv, Diagnostic> {
    let leading = raw_line.len() - raw_line.trim_start().len();
    let line = raw_line.trim();
    let rest = line.strip_prefix("dotenv ").unwrap().trim();
    let path = unquote(rest);

    if path.is_empty() {
        return Err(Diagnostic {
            span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
            message: "empty dotenv path".to_string(),
            severity: DiagnosticSeverity::Error,
        });
    }

    Ok(DotEnv {
        path,
        span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
    })
}

fn parse_task(lines: &[&str], start: usize) -> Result<(Task, usize), Diagnostic> {
    let line_idx = start as u32;
    let raw_line = lines[start];
    let leading = raw_line.len() - raw_line.trim_start().len();
    let line = raw_line.trim();
    let rest = line.strip_prefix("task ").unwrap();

    let mut cursor = rest;
    let mut params = Vec::new();
    let mut dependencies = Vec::new();
    let mut parallel_dependencies = Vec::new();
    let mut found_open_brace = false;

    // The column where the task name starts (after "task ")
    let task_keyword_col = (leading + "task ".len()) as u32;

    // Parse task name
    let name_end = cursor
        .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .unwrap_or(cursor.len());
    let name = cursor[..name_end].to_string();

    if name.is_empty() {
        return Err(Diagnostic {
            span: Span::new(line_idx, task_keyword_col, line_idx, raw_line.len() as u32),
            message: "expected task name".to_string(),
            severity: DiagnosticSeverity::Error,
        });
    }

    let name_span = Span::new(
        line_idx,
        task_keyword_col,
        line_idx,
        task_keyword_col + name_end as u32,
    );

    cursor = cursor[name_end..].trim_start();

    // Parse optional parts in any order until we find '{'
    loop {
        if cursor.is_empty() {
            break;
        }

        if cursor.starts_with('{') {
            found_open_brace = true;
            break;
        }

        if cursor.starts_with('[') {
            match parse_params(cursor, line_idx, raw_line) {
                Ok((p, rest_str)) => {
                    params = p;
                    cursor = rest_str.trim_start();
                }
                Err(diag) => return Err(diag),
            }
        } else if cursor.starts_with("depends_parallel=[") {
            match parse_depends_prefixed(cursor, "depends_parallel=[", line_idx, raw_line) {
                Ok((deps, rest_str)) => {
                    parallel_dependencies = deps;
                    cursor = rest_str.trim_start();
                }
                Err(diag) => return Err(diag),
            }
        } else if cursor.starts_with("depends=[") {
            match parse_depends_prefixed(cursor, "depends=[", line_idx, raw_line) {
                Ok((deps, rest_str)) => {
                    dependencies = deps;
                    cursor = rest_str.trim_start();
                }
                Err(diag) => return Err(diag),
            }
        } else {
            return Err(Diagnostic {
                span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
                message: format!("unexpected token in task header: {}", cursor),
                severity: DiagnosticSeverity::Error,
            });
        }
    }

    // If we haven't found '{' yet, look for it on subsequent lines
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
            return Err(Diagnostic {
                span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
                message: "expected '{' to open task body".to_string(),
                severity: DiagnosticSeverity::Error,
            });
        }
    } else {
        i = start + 1;
    }

    if !found_open_brace {
        return Err(Diagnostic {
            span: Span::new(line_idx, leading as u32, line_idx, raw_line.len() as u32),
            message: format!("expected '{{' for task '{}'", name),
            severity: DiagnosticSeverity::Error,
        });
    }

    // Collect body lines until braces balance
    let mut brace_depth: i32 = 1;
    let mut body_lines = Vec::new();

    while i < lines.len() {
        let l = lines[i];
        count_braces(l, &mut brace_depth);

        if brace_depth == 0 {
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
        return Err(Diagnostic {
            span: Span::new(line_idx, leading as u32, (lines.len() - 1) as u32, 0),
            message: format!("unclosed '{{' for task '{}'", name),
            severity: DiagnosticSeverity::Error,
        });
    }

    let body = dedent_body(&body_lines);
    let end_line = (i - 1) as u32;
    let end_col = lines.get(i.wrapping_sub(1)).map_or(0, |l| l.len()) as u32;

    Ok((
        Task {
            name,
            name_span,
            description: None,
            confirm: None,
            params,
            dependencies,
            parallel_dependencies,
            body,
            span: Span::new(line_idx, leading as u32, end_line, end_col),
        },
        i,
    ))
}

fn parse_params<'a>(
    input: &'a str,
    line_idx: u32,
    raw_line: &str,
) -> Result<(Vec<Param>, &'a str), Diagnostic> {
    assert!(input.starts_with('['));
    let Some(end) = input.find(']') else {
        return Err(Diagnostic {
            span: Span::new(line_idx, 0, line_idx, raw_line.len() as u32),
            message: "unterminated parameter list".to_string(),
            severity: DiagnosticSeverity::Error,
        });
    };

    let inner = &input[1..end];
    let mut params = Vec::new();

    // Calculate the column offset where params start in the raw line
    let bracket_offset = raw_line.find(input).map(|p| p + 1).unwrap_or(0);

    let mut chars = inner.char_indices().peekable();
    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().is_some_and(|(_, c)| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }

        let param_start = chars.peek().map(|(idx, _)| *idx).unwrap_or(0);

        // Read param name
        let mut name = String::new();
        while chars
            .peek()
            .is_some_and(|(_, c)| *c != '=' && !c.is_whitespace())
        {
            name.push(chars.next().unwrap().1);
        }

        if name.is_empty() {
            break;
        }

        let param_end;
        let default;

        if chars.peek().map(|(_, c)| *c) == Some('=') {
            chars.next(); // consume '='
            if chars.peek().map(|(_, c)| *c) == Some('"') {
                chars.next(); // consume opening "
                let mut val = String::new();
                let mut escaped = false;
                for (_, ch) in chars.by_ref() {
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
                param_end = chars.peek().map(|(idx, _)| *idx).unwrap_or(inner.len());
                default = Some(val);
            } else {
                let mut val = String::new();
                while chars.peek().is_some_and(|(_, c)| !c.is_whitespace()) {
                    val.push(chars.next().unwrap().1);
                }
                param_end = chars.peek().map(|(idx, _)| *idx).unwrap_or(inner.len());
                default = Some(val);
            }
        } else {
            param_end = chars.peek().map(|(idx, _)| *idx).unwrap_or(inner.len());
            default = None;
        }

        let col_start = (bracket_offset + param_start) as u32;
        let col_end = (bracket_offset + param_end) as u32;

        params.push(Param {
            name,
            default,
            span: Span::new(line_idx, col_start, line_idx, col_end),
        });
    }

    Ok((params, &input[end + 1..]))
}

fn parse_depends_prefixed<'a>(
    input: &'a str,
    prefix: &str,
    line_idx: u32,
    raw_line: &str,
) -> Result<(Vec<Dependency>, &'a str), Diagnostic> {
    let rest = input.strip_prefix(prefix).unwrap();
    let Some(end) = rest.find(']') else {
        return Err(Diagnostic {
            span: Span::new(line_idx, 0, line_idx, raw_line.len() as u32),
            message: "unterminated depends list".to_string(),
            severity: DiagnosticSeverity::Error,
        });
    };

    let inner = &rest[..end];

    // Calculate column offset for dependency names
    let prefix_offset = raw_line.find(prefix).unwrap_or(0) + prefix.len();

    let deps: Vec<Dependency> = inner
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return None;
            }
            // Find position of this dep name within inner
            let name_start_in_inner =
                s.as_ptr() as usize - inner.as_ptr() as usize + (s.len() - s.trim_start().len());
            let col_start = (prefix_offset + name_start_in_inner) as u32;
            let col_end = col_start + trimmed.len() as u32;
            Some(Dependency {
                name: trimmed.to_string(),
                span: Span::new(line_idx, col_start, line_idx, col_end),
            })
        })
        .collect();

    Ok((deps, &rest[end + 1..]))
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

fn count_braces(line: &str, depth: &mut i32) {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';

    for ch in line.chars() {
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

    #[test]
    fn parse_single_task() {
        let input = "task build {\n  echo \"building\"\n}";
        let ast = parse(input);
        assert!(ast.diagnostics.is_empty());
        assert_eq!(ast.tasks.len(), 1);
        assert_eq!(ast.tasks[0].name, "build");
        assert!(ast.tasks[0].body.contains("echo"));
    }

    #[test]
    fn parse_task_with_depends() {
        let input = "task build depends=[clean, test] {\n  echo \"building\"\n}";
        let ast = parse(input);
        assert!(ast.diagnostics.is_empty());
        assert_eq!(ast.tasks[0].dependencies.len(), 2);
        assert_eq!(ast.tasks[0].dependencies[0].name, "clean");
        assert_eq!(ast.tasks[0].dependencies[1].name, "test");
    }

    #[test]
    fn parse_exports_and_aliases() {
        let input = "export FOO=\"bar\"\nalias ll=\"ls -la\"";
        let ast = parse(input);
        assert!(ast.diagnostics.is_empty());
        assert_eq!(ast.exports.len(), 1);
        assert_eq!(ast.exports[0].key, "FOO");
        assert_eq!(ast.aliases.len(), 1);
        assert_eq!(ast.aliases[0].name, "ll");
    }

    #[test]
    fn parse_include_and_dotenv() {
        let input = "include \"tasks/docker.Taskfile\"\ndotenv \".env\"";
        let ast = parse(input);
        assert!(ast.diagnostics.is_empty());
        assert_eq!(ast.includes.len(), 1);
        assert_eq!(ast.includes[0].path, "tasks/docker.Taskfile");
        assert_eq!(ast.dotenv.len(), 1);
        assert_eq!(ast.dotenv[0].path, ".env");
    }

    #[test]
    fn error_recovery_bad_line() {
        let input = "garbage line\ntask build {\n  echo hi\n}";
        let ast = parse(input);
        assert_eq!(ast.diagnostics.len(), 1);
        assert!(ast.diagnostics[0].message.contains("unexpected line"));
        assert_eq!(ast.tasks.len(), 1);
        assert_eq!(ast.tasks[0].name, "build");
    }

    #[test]
    fn error_recovery_annotation_not_followed_by_task() {
        let input = "@description Something\nexport FOO=\"bar\"";
        let ast = parse(input);
        assert!(!ast.diagnostics.is_empty());
        assert_eq!(ast.exports.len(), 1);
    }

    #[test]
    fn parse_full_example() {
        let input = r#"include "tasks/docker.Taskfile"

dotenv ".env"

export PROJECT="myapp"

alias ll="ls -la"

@description Build the project
task build depends=[clean] {
  echo "Building..."
}

task clean {
  rm -rf target/
}"#;
        let ast = parse(input);
        assert!(ast.diagnostics.is_empty(), "{:?}", ast.diagnostics);
        assert_eq!(ast.tasks.len(), 2);
        assert_eq!(ast.includes.len(), 1);
        assert_eq!(ast.dotenv.len(), 1);
        assert_eq!(ast.exports.len(), 1);
        assert_eq!(ast.aliases.len(), 1);
        assert_eq!(
            ast.tasks[0].description.as_deref(),
            Some("Build the project")
        );
    }
}
