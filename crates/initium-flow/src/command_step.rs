use std::io;
use std::path::PathBuf;
use std::process::Command;

use initium_config::StepConfig;
use serde::Deserialize;

use crate::{PreparedStep, StepOperation};

pub struct CommandStep;

impl StepOperation for CommandStep {
    fn prepare(&self, step: &StepConfig) -> io::Result<PreparedStep> {
        let config: CommandConfig = step
            .parameters
            .clone()
            .try_into()
            .map_err(|source| io::Error::new(io::ErrorKind::InvalidInput, source))?;
        if config.program.as_os_str().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "command program must be nonempty",
            ));
        }

        Ok(Box::new(move || config.run()))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandConfig {
    program: PathBuf,
    #[serde(default)]
    args: Vec<String>,
}

impl CommandConfig {
    fn run(self) -> io::Result<()> {
        let status = Command::new(&self.program).args(&self.args).status()?;
        if status.success() {
            return Ok(());
        }

        Err(io::Error::other(format!(
            "{} exited with {status}",
            self.program.display()
        )))
    }
}
