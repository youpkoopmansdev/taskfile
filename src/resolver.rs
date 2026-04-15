use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::parser;
use crate::parser::ast::{Alias, Ast, Export, Task};
use crate::parser::error::ParseError;

#[derive(Debug, Clone)]
pub struct ResolvedTask {
    pub qualified_name: String,
    pub task: Task,
    pub aliases: Vec<Alias>,
    pub exports: Vec<Export>,
    pub source_file: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("{0}")]
    Parse(#[from] ParseError),

    #[error("circular include detected: {}", .chain.join(" -> "))]
    CircularInclude { chain: Vec<String> },

    #[error("include file not found: {path} (referenced from {from}:{line})")]
    IncludeNotFound {
        path: String,
        from: PathBuf,
        line: usize,
    },

    #[error(
        "duplicate task '{name}' (defined in {new_file}, previously defined in {existing_file})"
    )]
    DuplicateTask {
        name: String,
        existing_file: PathBuf,
        new_file: PathBuf,
    },
}

pub fn resolve(taskfile_path: &Path) -> Result<HashMap<String, ResolvedTask>, ResolveError> {
    let mut ctx = ResolveContext {
        registry: HashMap::new(),
        active_chain: HashSet::new(),
        processed: HashSet::new(),
        include_chain: Vec::new(),
    };

    resolve_file(taskfile_path, "", &[], &[], &mut ctx)?;

    Ok(ctx.registry)
}

struct ResolveContext {
    registry: HashMap<String, ResolvedTask>,
    active_chain: HashSet<PathBuf>,
    processed: HashSet<PathBuf>,
    include_chain: Vec<String>,
}

fn resolve_file(
    filepath: &Path,
    prefix: &str,
    parent_aliases: &[Alias],
    parent_exports: &[Export],
    ctx: &mut ResolveContext,
) -> Result<(), ResolveError> {
    let canonical = filepath
        .canonicalize()
        .map_err(|e| ParseError::io(filepath, e))?;

    // Circular include: same file appears in the current include chain
    if ctx.active_chain.contains(&canonical) {
        ctx.include_chain.push(filepath.display().to_string());
        return Err(ResolveError::CircularInclude {
            chain: ctx.include_chain.clone(),
        });
    }

    // Diamond include: already processed from a different branch — skip
    if ctx.processed.contains(&canonical) {
        return Ok(());
    }

    ctx.active_chain.insert(canonical.clone());
    ctx.include_chain.push(filepath.display().to_string());

    let content = std::fs::read_to_string(filepath).map_err(|e| ParseError::io(filepath, e))?;
    let ast = parser::parse(&content, filepath)?;

    let base_dir = filepath.parent().unwrap_or(Path::new("."));

    // Build combined aliases/exports: parent chain + this file's own
    let mut combined_aliases = parent_aliases.to_vec();
    combined_aliases.extend(ast.aliases.clone());
    let mut combined_exports = parent_exports.to_vec();
    combined_exports.extend(ast.exports.clone());

    // Register tasks from this file with combined scope
    register_tasks(
        &ast,
        prefix,
        filepath,
        &combined_aliases,
        &combined_exports,
        &mut ctx.registry,
    )?;

    // Process includes
    for include in &ast.includes {
        let include_path = base_dir.join(&include.path);
        if !include_path.exists() {
            return Err(ResolveError::IncludeNotFound {
                path: include.path.clone(),
                from: filepath.to_path_buf(),
                line: include.line,
            });
        }

        let namespace = include_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let child_prefix = if prefix.is_empty() {
            namespace.to_string()
        } else {
            format!("{}:{}", prefix, namespace)
        };

        resolve_file(
            &include_path,
            &child_prefix,
            &combined_aliases,
            &combined_exports,
            ctx,
        )?;
    }

    ctx.include_chain.pop();
    ctx.active_chain.remove(&canonical);
    ctx.processed.insert(canonical);

    Ok(())
}

fn register_tasks(
    ast: &Ast,
    prefix: &str,
    source_file: &Path,
    combined_aliases: &[Alias],
    combined_exports: &[Export],
    registry: &mut HashMap<String, ResolvedTask>,
) -> Result<(), ResolveError> {
    for task in &ast.tasks {
        let qualified_name = if prefix.is_empty() {
            task.name.clone()
        } else {
            format!("{}:{}", prefix, task.name)
        };

        if let Some(existing) = registry.get(&qualified_name) {
            return Err(ResolveError::DuplicateTask {
                name: qualified_name,
                existing_file: existing.source_file.clone(),
                new_file: source_file.to_path_buf(),
            });
        }

        registry.insert(
            qualified_name.clone(),
            ResolvedTask {
                qualified_name,
                task: task.clone(),
                aliases: combined_aliases.to_vec(),
                exports: combined_exports.to_vec(),
                source_file: source_file.to_path_buf(),
            },
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_single_file() {
        let tmp = tempfile::tempdir().unwrap();
        let tf = tmp.path().join("Taskfile");
        fs::write(
            &tf,
            r#"export FOO="bar"
alias ll="ls -la"

task build {
  echo "building"
}

task test {
  cargo test
}"#,
        )
        .unwrap();

        let registry = resolve(&tf).unwrap();
        assert_eq!(registry.len(), 2);
        assert!(registry.contains_key("build"));
        assert!(registry.contains_key("test"));
        assert_eq!(registry["build"].exports.len(), 1);
        assert_eq!(registry["build"].aliases.len(), 1);
    }

    #[test]
    fn resolve_with_namespace() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks_dir = tmp.path().join("tasks");
        fs::create_dir(&tasks_dir).unwrap();

        fs::write(
            tmp.path().join("Taskfile"),
            r#"include "tasks/docker.Taskfile"

task build {
  echo "building"
}"#,
        )
        .unwrap();

        fs::write(
            tasks_dir.join("docker.Taskfile"),
            r#"task up {
  echo "up"
}

task down {
  echo "down"
}"#,
        )
        .unwrap();

        let registry = resolve(&tmp.path().join("Taskfile")).unwrap();
        assert!(registry.contains_key("build"));
        assert!(registry.contains_key("docker:up"));
        assert!(registry.contains_key("docker:down"));
    }

    #[test]
    fn resolve_nested_includes() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks_dir = tmp.path().join("tasks");
        fs::create_dir(&tasks_dir).unwrap();

        fs::write(
            tmp.path().join("Taskfile"),
            r#"include "tasks/docker.Taskfile""#,
        )
        .unwrap();

        fs::write(
            tasks_dir.join("docker.Taskfile"),
            r#"include "compose.Taskfile"

task up {
  echo "up"
}"#,
        )
        .unwrap();

        fs::write(
            tasks_dir.join("compose.Taskfile"),
            r#"task ps {
  echo "ps"
}"#,
        )
        .unwrap();

        let registry = resolve(&tmp.path().join("Taskfile")).unwrap();
        assert!(registry.contains_key("docker:up"));
        assert!(registry.contains_key("docker:compose:ps"));
    }

    #[test]
    fn resolve_circular_include_detected() {
        let tmp = tempfile::tempdir().unwrap();

        fs::write(tmp.path().join("Taskfile"), r#"include "other.Taskfile""#).unwrap();

        fs::write(tmp.path().join("other.Taskfile"), r#"include "Taskfile""#).unwrap();

        let result = resolve(&tmp.path().join("Taskfile"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("circular include"));
    }

    #[test]
    fn resolve_missing_include() {
        let tmp = tempfile::tempdir().unwrap();

        fs::write(
            tmp.path().join("Taskfile"),
            r#"include "nonexistent.Taskfile""#,
        )
        .unwrap();

        let result = resolve(&tmp.path().join("Taskfile"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
    }

    #[test]
    fn resolve_duplicate_task_is_error() {
        let tmp = tempfile::tempdir().unwrap();

        fs::write(
            tmp.path().join("Taskfile"),
            r#"task build {
  echo "first"
}

task build {
  echo "second"
}"#,
        )
        .unwrap();

        let result = resolve(&tmp.path().join("Taskfile"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("duplicate task"));
    }

    #[test]
    fn resolve_diamond_include_no_duplicate() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks_dir = tmp.path().join("tasks");
        fs::create_dir(&tasks_dir).unwrap();

        // A includes B and C, both B and C include D
        fs::write(
            tmp.path().join("Taskfile"),
            r#"include "tasks/b.Taskfile"
include "tasks/c.Taskfile""#,
        )
        .unwrap();

        fs::write(
            tasks_dir.join("b.Taskfile"),
            r#"include "d.Taskfile"
task b_task {
  echo "b"
}"#,
        )
        .unwrap();

        fs::write(
            tasks_dir.join("c.Taskfile"),
            r#"include "d.Taskfile"
task c_task {
  echo "c"
}"#,
        )
        .unwrap();

        fs::write(
            tasks_dir.join("d.Taskfile"),
            r#"task shared {
  echo "shared"
}"#,
        )
        .unwrap();

        // Should succeed — D processed once via B, skipped via C
        let registry = resolve(&tmp.path().join("Taskfile")).unwrap();
        assert!(registry.contains_key("b:b_task"));
        assert!(registry.contains_key("c:c_task"));
        assert!(registry.contains_key("b:d:shared"));
    }
}
