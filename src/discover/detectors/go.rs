use std::path::Path;

use crate::discover::detector::DiscoveredTask;

pub fn detect(dir: &Path) -> Vec<DiscoveredTask> {
    if !dir.join("go.mod").exists() {
        return Vec::new();
    }

    let mut tasks = vec![
        DiscoveredTask {
            name: "build".into(),
            description: "Build the Go project".into(),
            body: "go build ./...".into(),
            source: "go.mod".into(),
        },
        DiscoveredTask {
            name: "test".into(),
            description: "Run Go tests".into(),
            body: "go test ./...".into(),
            source: "go.mod".into(),
        },
        DiscoveredTask {
            name: "lint".into(),
            description: "Run Go vet".into(),
            body: "go vet ./...".into(),
            source: "go.mod".into(),
        },
    ];

    if dir.join("cmd").exists() {
        tasks.push(DiscoveredTask {
            name: "run".into(),
            description: "Run the application".into(),
            body: "go run ./cmd/...".into(),
            source: "go.mod (cmd/ detected)".into(),
        });
    }

    tasks
}
