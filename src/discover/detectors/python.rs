use std::fs;
use std::path::Path;

use crate::discover::detector::DiscoveredTask;

pub fn detect(dir: &Path) -> Vec<DiscoveredTask> {
    let has_pyproject = dir.join("pyproject.toml").exists();
    let has_requirements = dir.join("requirements.txt").exists();
    let has_pipfile = dir.join("Pipfile").exists();

    if !has_pyproject && !has_requirements && !has_pipfile {
        return Vec::new();
    }

    let mut tasks = Vec::new();

    if has_pyproject {
        let content = fs::read_to_string(dir.join("pyproject.toml")).unwrap_or_default();

        if content.contains("[tool.poetry]") {
            tasks.push(DiscoveredTask {
                name: "install".into(),
                description: "Install dependencies with Poetry".into(),
                body: "poetry install".into(),
                source: "pyproject.toml (poetry)".into(),
            });
            tasks.push(DiscoveredTask {
                name: "test".into(),
                description: "Run tests".into(),
                body: "poetry run pytest".into(),
                source: "pyproject.toml (poetry)".into(),
            });
            tasks.push(DiscoveredTask {
                name: "lint".into(),
                description: "Run linter".into(),
                body: "poetry run ruff check .".into(),
                source: "pyproject.toml (poetry)".into(),
            });
        }

        if content.contains("[tool.uv]") {
            tasks.push(DiscoveredTask {
                name: "install".into(),
                description: "Install dependencies with uv".into(),
                body: "uv sync".into(),
                source: "pyproject.toml (uv)".into(),
            });
            tasks.push(DiscoveredTask {
                name: "test".into(),
                description: "Run tests".into(),
                body: "uv run pytest".into(),
                source: "pyproject.toml (uv)".into(),
            });
        }

        if tasks.is_empty() {
            tasks.push(DiscoveredTask {
                name: "install".into(),
                description: "Install the project".into(),
                body: "pip install -e .".into(),
                source: "pyproject.toml".into(),
            });
        }

        if (content.contains("[tool.pytest]") || dir.join("tests").exists())
            && !tasks.iter().any(|t| t.name == "test")
        {
            tasks.push(DiscoveredTask {
                name: "test".into(),
                description: "Run tests with pytest".into(),
                body: "pytest".into(),
                source: "pyproject.toml".into(),
            });
        }
    } else if has_requirements {
        tasks.push(DiscoveredTask {
            name: "install".into(),
            description: "Install Python dependencies".into(),
            body: "pip install -r requirements.txt".into(),
            source: "requirements.txt".into(),
        });
    }

    tasks
}
