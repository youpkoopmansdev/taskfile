use std::collections::HashMap;

use crate::parser::ast::{Alias, Export};
use crate::resolver::ResolvedTask;

pub fn build_script(resolved: &ResolvedTask, param_values: &HashMap<String, String>) -> String {
    let sections: Vec<String> = vec![
        shell_options(),
        export_section(&resolved.exports),
        alias_section(&resolved.aliases),
        param_section(param_values),
        resolved.task.body.clone(),
    ];

    sections
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn shell_options() -> String {
    "set -euo pipefail".to_string()
}

fn export_section(exports: &[Export]) -> String {
    exports
        .iter()
        .map(|e| format!("export {}={}", e.key, shell_quote(&e.value)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn alias_section(aliases: &[Alias]) -> String {
    aliases
        .iter()
        .map(|a| format!("{}() {{ {} \"$@\"; }}", a.name, a.value))
        .collect::<Vec<_>>()
        .join("\n")
}

fn param_section(param_values: &HashMap<String, String>) -> String {
    param_values
        .iter()
        .map(|(name, value)| format!("{}={}", name, shell_quote(value)))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn shell_quote(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
            .replace('`', "\\`")
    )
}
