use std::io;

use initium_config::StepConfig;

use crate::{PreparedStep, StepOperation};

use super::config::MountConfig;
use super::request::MountRequest;

pub struct MountStep;

impl StepOperation for MountStep {
    fn prepare(&self, step: &StepConfig) -> io::Result<PreparedStep> {
        let config: MountConfig = step
            .parameters
            .clone()
            .try_into()
            .map_err(|source| io::Error::new(io::ErrorKind::InvalidInput, source))?;
        let request = MountRequest::try_from(config)?;

        Ok(Box::new(move || request.execute()))
    }
}

#[cfg(test)]
#[path = "step.tests.rs"]
mod tests;
