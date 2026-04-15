use std::fs;
use std::path::{Path, PathBuf};

use super::sanitize_task_name;
use crate::discover::detector::DiscoveredTask;

pub fn detect(dir: &Path) -> Vec<DiscoveredTask> {
    let path = match find_compose_file(dir) {
        Some(p) => p,
        None => return Vec::new(),
    };

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let filename = path.file_name().unwrap_or_default().to_string_lossy();
    let services = extract_services(&content);

    let mut tasks = vec![
        DiscoveredTask {
            name: "up".into(),
            description: "Start all services".into(),
            body: "docker compose up -d".into(),
            source: filename.to_string(),
        },
        DiscoveredTask {
            name: "down".into(),
            description: "Stop all services".into(),
            body: "docker compose down".into(),
            source: filename.to_string(),
        },
        DiscoveredTask {
            name: "logs".into(),
            description: "Tail logs for all services".into(),
            body: "docker compose logs -f".into(),
            source: filename.to_string(),
        },
        DiscoveredTask {
            name: "restart".into(),
            description: "Restart all services".into(),
            body: "docker compose restart".into(),
            source: filename.to_string(),
        },
        DiscoveredTask {
            name: "ps".into(),
            description: "Show running containers".into(),
            body: "docker compose ps".into(),
            source: filename.to_string(),
        },
    ];

    for service in &services {
        let svc = sanitize_task_name(service);
        tasks.push(DiscoveredTask {
            name: format!("{svc}-logs"),
            description: format!("Tail logs for {service}"),
            body: format!("docker compose logs -f {service}"),
            source: format!("{filename} service: {service}"),
        });
        tasks.push(DiscoveredTask {
            name: format!("{svc}-restart"),
            description: format!("Restart {service}"),
            body: format!("docker compose restart {service}"),
            source: format!("{filename} service: {service}"),
        });
        tasks.push(DiscoveredTask {
            name: format!("{svc}-shell"),
            description: format!("Open shell in {service}"),
            body: format!("docker compose exec {service} sh"),
            source: format!("{filename} service: {service}"),
        });
    }

    tasks
}

pub fn find_compose_file(dir: &Path) -> Option<PathBuf> {
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

fn extract_services(content: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn extract_services_basic() {
        let yaml = r#"
services:
  web:
    image: nginx
  db:
    image: postgres
  redis:
    image: redis
"#;
        let services = extract_services(yaml);
        assert_eq!(services, vec!["web", "db", "redis"]);
    }

    #[test]
    fn detect_compose_services() {
        let tmp = tempfile::tempdir().unwrap();
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

        let tasks = detect(tmp.path());
        let names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"up"));
        assert!(names.contains(&"down"));
        assert!(names.contains(&"app-logs"));
        assert!(names.contains(&"db-shell"));
    }
}
