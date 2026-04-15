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
    let mut entries: Vec<(&String, &String)> = param_values.iter().collect();
    entries.sort_by_key(|(k, _)| k.as_str());
    entries
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Alias, Export, Task};
    use crate::resolver::ResolvedTask;
    use std::path::PathBuf;

    fn make_resolved(body: &str) -> ResolvedTask {
        ResolvedTask {
            qualified_name: "test".into(),
            task: Task {
                name: "test".into(),
                description: None,
                params: vec![],
                dependencies: vec![],
                body: body.into(),
                line: 1,
            },
            aliases: vec![],
            exports: vec![],
            source_file: PathBuf::from("Taskfile"),
        }
    }

    #[test]
    fn script_includes_shell_options() {
        let resolved = make_resolved("echo hi");
        let script = build_script(&resolved, &HashMap::new());
        assert!(script.starts_with("set -euo pipefail"));
    }

    #[test]
    fn script_includes_exports() {
        let mut resolved = make_resolved("echo hi");
        resolved.exports = vec![Export {
            key: "FOO".into(),
            value: "bar".into(),
        }];
        let script = build_script(&resolved, &HashMap::new());
        assert!(script.contains("export FOO=\"bar\""));
    }

    #[test]
    fn script_converts_alias_to_function() {
        let mut resolved = make_resolved("echo hi");
        resolved.aliases = vec![Alias {
            name: "dc".into(),
            value: "docker compose".into(),
        }];
        let script = build_script(&resolved, &HashMap::new());
        assert!(script.contains("dc() { docker compose \"$@\"; }"));
    }

    #[test]
    fn script_params_sorted_deterministically() {
        let resolved = make_resolved("echo hi");
        let mut params = HashMap::new();
        params.insert("zebra".into(), "z".into());
        params.insert("alpha".into(), "a".into());
        params.insert("mid".into(), "m".into());
        let script = build_script(&resolved, &params);
        let alpha_pos = script.find("alpha=").unwrap();
        let mid_pos = script.find("mid=").unwrap();
        let zebra_pos = script.find("zebra=").unwrap();
        assert!(alpha_pos < mid_pos);
        assert!(mid_pos < zebra_pos);
    }

    #[test]
    fn shell_quote_escapes_special_chars() {
        assert_eq!(shell_quote("hello"), "\"hello\"");
        assert_eq!(shell_quote("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(shell_quote("$HOME"), "\"\\$HOME\"");
        assert_eq!(shell_quote("back\\slash"), "\"back\\\\slash\"");
        assert_eq!(shell_quote("`cmd`"), "\"\\`cmd\\`\"");
    }
}
