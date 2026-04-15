use std::path::Path;

/// A task discovered from project configuration files.
pub struct DiscoveredTask {
    pub name: String,
    pub description: String,
    pub body: String,
    pub source: String,
}

/// A detector that scans a project directory for a specific tool/framework.
pub struct Detector {
    pub name: &'static str,
    pub detect: fn(&Path) -> Vec<DiscoveredTask>,
}
