pub mod docker;
pub mod dockerfile;
pub mod go;
pub mod makefile;
pub mod node;
pub mod python;
pub mod ruby;
pub mod rust;

use super::detector::Detector;

/// All registered detectors, checked in order.
pub const ALL: &[Detector] = &[
    Detector {
        name: "package.json (npm/yarn/pnpm)",
        detect: node::detect,
    },
    Detector {
        name: "Cargo.toml (Rust)",
        detect: rust::detect,
    },
    Detector {
        name: "docker-compose.yml",
        detect: docker::detect,
    },
    Detector {
        name: "Dockerfile",
        detect: dockerfile::detect,
    },
    Detector {
        name: "Makefile",
        detect: makefile::detect,
    },
    Detector {
        name: "go.mod (Go)",
        detect: go::detect,
    },
    Detector {
        name: "pyproject.toml / requirements.txt (Python)",
        detect: python::detect,
    },
    Detector {
        name: "Gemfile (Ruby)",
        detect: ruby::detect,
    },
];

pub fn sanitize_task_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Returns true if the name is a valid task name worth discovering.
pub fn is_valid_task_name(name: &str) -> bool {
    let trimmed = name.trim_matches('-');
    !trimmed.is_empty() && trimmed.chars().any(|c| c.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_names() {
        assert_eq!(sanitize_task_name("build:prod"), "build-prod");
        assert_eq!(sanitize_task_name("test.unit"), "test-unit");
        assert_eq!(sanitize_task_name("lint/fix"), "lint-fix");
        assert_eq!(sanitize_task_name("lint:oxlint"), "lint-oxlint");
    }

    #[test]
    fn valid_task_names() {
        assert!(is_valid_task_name("build"));
        assert!(is_valid_task_name("lint-oxlint"));
        assert!(!is_valid_task_name("--"));
        assert!(!is_valid_task_name("---"));
        assert!(!is_valid_task_name(""));
    }
}
