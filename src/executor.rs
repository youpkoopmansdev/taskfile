use std::collections::HashMap;
use std::process::ExitStatus;

use colored::Colorize;

use crate::resolver::ResolvedTask;
use crate::runner::TaskRunner;
use crate::script;

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("task '{name}' failed with exit code {code}")]
    TaskFailed { name: String, code: i32 },

    #[error("task '{name}' was terminated by signal")]
    TaskSignaled { name: String },

    #[error("failed to execute bash: {0}")]
    BashError(#[from] std::io::Error),

    #[error("missing required parameter '--{param}' for task '{task}'")]
    MissingParam { task: String, param: String },

    #[error("unknown task: '{name}'")]
    UnknownTask { name: String },

    #[error("dependency '{dep}' not found for task '{task}'")]
    DependencyNotFound { task: String, dep: String },
}

pub fn execute_task(
    name: &str,
    task_args: &[String],
    registry: &HashMap<String, ResolvedTask>,
    runner: &dyn TaskRunner,
) -> Result<ExitStatus, ExecError> {
    let resolved = registry.get(name).ok_or_else(|| ExecError::UnknownTask {
        name: name.to_string(),
    })?;

    let arg_map = parse_task_args(task_args);

    // Run dependencies first
    for dep in &resolved.task.dependencies {
        let dep_name = resolve_dep_name(name, dep);
        let dep_resolved =
            registry
                .get(&dep_name)
                .ok_or_else(|| ExecError::DependencyNotFound {
                    task: name.to_string(),
                    dep: dep_name.clone(),
                })?;

        eprintln!("{} {}", "→ dep:".dimmed(), dep_name.dimmed());
        let status = run_single_task(&dep_name, dep_resolved, &HashMap::new(), runner)?;
        if !status.success() {
            return Err(ExecError::TaskFailed {
                name: dep_name,
                code: status.code().unwrap_or(1),
            });
        }
    }

    let param_values = build_param_values(name, &resolved.task.params, &arg_map)?;
    run_single_task(name, resolved, &param_values, runner)
}

fn run_single_task(
    name: &str,
    resolved: &ResolvedTask,
    param_values: &HashMap<String, String>,
    runner: &dyn TaskRunner,
) -> Result<ExitStatus, ExecError> {
    let script = script::build_script(resolved, param_values);

    let status = runner.run_script(&script).map_err(ExecError::BashError)?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        if code == -1 {
            return Err(ExecError::TaskSignaled {
                name: name.to_string(),
            });
        }
        return Err(ExecError::TaskFailed {
            name: name.to_string(),
            code,
        });
    }

    Ok(status)
}

fn parse_task_args(args: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for arg in args {
        if let Some(kv) = arg.strip_prefix("--") {
            if let Some((key, value)) = kv.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            } else {
                map.insert(kv.to_string(), String::new());
            }
        }
    }
    map
}

fn build_param_values(
    task_name: &str,
    params: &[crate::parser::ast::Param],
    arg_map: &HashMap<String, String>,
) -> Result<HashMap<String, String>, ExecError> {
    let mut values = HashMap::new();

    for param in params {
        if let Some(val) = arg_map.get(&param.name) {
            values.insert(param.name.clone(), val.clone());
        } else if let Some(default) = &param.default {
            values.insert(param.name.clone(), default.clone());
        } else {
            return Err(ExecError::MissingParam {
                task: task_name.to_string(),
                param: param.name.clone(),
            });
        }
    }

    Ok(values)
}

fn resolve_dep_name(task_name: &str, dep: &str) -> String {
    if dep.contains(':') {
        return dep.to_string();
    }
    if let Some(ns) = task_name.rsplit_once(':') {
        format!("{}:{}", ns.0, dep)
    } else {
        dep.to_string()
    }
}
