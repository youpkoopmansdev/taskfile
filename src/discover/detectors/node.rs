use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::super::json::extract_json_object;
use super::sanitize_task_name;
use crate::discover::detector::DiscoveredTask;

pub fn detect(dir: &Path) -> Vec<DiscoveredTask> {
    let path = dir.join("package.json");
    if !path.exists() {
        return Vec::new();
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut tasks = Vec::new();
    let pm = detect_package_manager(dir);

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

    if tasks.is_empty() {
        tasks.push(DiscoveredTask {
            name: "install".into(),
            description: "Install dependencies".into(),
            body: format!("{pm} install"),
            source: "package.json".into(),
        });
    }

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
        && !scripts.contains_key("dev")
        && !scripts.contains_key("start")
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

pub fn detect_package_manager(dir: &Path) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn detect_vue_project() {
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

        let tasks = detect(tmp.path());
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"dev"));
        assert!(names.contains(&"build"));
        assert!(names.contains(&"preview"));
    }

    #[test]
    fn detects_pnpm() {
        let tmp = setup();
        fs::write(tmp.path().join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(detect_package_manager(tmp.path()), "pnpm");
    }

    #[test]
    fn detects_yarn() {
        let tmp = setup();
        fs::write(tmp.path().join("yarn.lock"), "").unwrap();
        assert_eq!(detect_package_manager(tmp.path()), "yarn");
    }

    #[test]
    fn detects_bun() {
        let tmp = setup();
        fs::write(tmp.path().join("bun.lockb"), "").unwrap();
        assert_eq!(detect_package_manager(tmp.path()), "bun");
    }
}
