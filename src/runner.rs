use std::process::{Command, ExitStatus};

pub trait TaskRunner: Send + Sync {
    fn run_script(&self, script: &str) -> Result<ExitStatus, std::io::Error>;
}

pub struct BashRunner;

impl TaskRunner for BashRunner {
    fn run_script(&self, script: &str) -> Result<ExitStatus, std::io::Error> {
        Command::new("bash").arg("-c").arg(script).status()
    }
}
