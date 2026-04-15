use std::path::Path;

use super::docker;
use crate::discover::detector::DiscoveredTask;

pub fn detect(dir: &Path) -> Vec<DiscoveredTask> {
    if !dir.join("Dockerfile").exists() {
        return Vec::new();
    }

    // Don't add docker build tasks if docker-compose is present
    if docker::find_compose_file(dir).is_some() {
        return Vec::new();
    }

    let project_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app")
        .to_lowercase();

    vec![
        DiscoveredTask {
            name: "build".into(),
            description: "Build Docker image".into(),
            body: format!("docker build -t {project_name} ."),
            source: "Dockerfile".into(),
        },
        DiscoveredTask {
            name: "run".into(),
            description: "Run Docker container".into(),
            body: format!("docker run --rm -it {project_name}"),
            source: "Dockerfile".into(),
        },
    ]
}
