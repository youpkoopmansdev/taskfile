use std::fs;
use std::path::Path;

use crate::discover::detector::DiscoveredTask;

pub fn detect(dir: &Path) -> Vec<DiscoveredTask> {
    if !dir.join("Gemfile").exists() {
        return Vec::new();
    }

    let content = fs::read_to_string(dir.join("Gemfile")).unwrap_or_default();
    let mut tasks = vec![DiscoveredTask {
        name: "install".into(),
        description: "Install Ruby dependencies".into(),
        body: "bundle install".into(),
        source: "Gemfile".into(),
    }];

    if content.contains("rails") || dir.join("config").join("routes.rb").exists() {
        tasks.push(DiscoveredTask {
            name: "server".into(),
            description: "Start Rails server".into(),
            body: "bundle exec rails server".into(),
            source: "Gemfile (Rails detected)".into(),
        });
        tasks.push(DiscoveredTask {
            name: "console".into(),
            description: "Open Rails console".into(),
            body: "bundle exec rails console".into(),
            source: "Gemfile (Rails detected)".into(),
        });
        tasks.push(DiscoveredTask {
            name: "db-migrate".into(),
            description: "Run database migrations".into(),
            body: "bundle exec rails db:migrate".into(),
            source: "Gemfile (Rails detected)".into(),
        });
        tasks.push(DiscoveredTask {
            name: "test".into(),
            description: "Run Rails tests".into(),
            body: "bundle exec rails test".into(),
            source: "Gemfile (Rails detected)".into(),
        });
    }

    if content.contains("rspec") {
        tasks.push(DiscoveredTask {
            name: "test".into(),
            description: "Run RSpec tests".into(),
            body: "bundle exec rspec".into(),
            source: "Gemfile (rspec detected)".into(),
        });
    }

    tasks
}
