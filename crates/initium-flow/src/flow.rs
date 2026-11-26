use std::collections::{HashMap, HashSet};
use std::io;

use initium_config::StepConfig;

use crate::{FlowError, PreparedStep, StepEvent, StepOperation};

#[derive(Default)]
pub struct Flow {
    operations: HashMap<String, Box<dyn StepOperation>>,
}

impl Flow {
    pub fn register(
        &mut self,
        kind: impl Into<String>,
        operation: impl StepOperation + 'static,
    ) -> Option<Box<dyn StepOperation>> {
        self.operations.insert(kind.into(), Box::new(operation))
    }

    pub fn run(
        &self,
        steps: &[StepConfig],
        mut notify: impl FnMut(StepEvent<'_>),
    ) -> Result<(), FlowError> {
        let prepared = self.prepare(steps)?;
        for (step, operation) in steps.iter().zip(prepared) {
            let id = step.id.as_str();
            notify(StepEvent::Started { id });

            if let Err(source) = operation() {
                notify(StepEvent::Failed {
                    id,
                    source: &source,
                });
                return Err(FlowError::FailedStep {
                    id: id.to_owned(),
                    source,
                });
            }

            notify(StepEvent::Succeeded { id });
        }

        Ok(())
    }

    fn prepare(&self, steps: &[StepConfig]) -> Result<Vec<PreparedStep>, FlowError> {
        let mut ids = HashSet::new();

        steps
            .iter()
            .map(|step| {
                self.prepare_step(step, &mut ids)
                    .map_err(|source| FlowError::InvalidStep {
                        id: step.id.clone(),
                        source,
                    })
            })
            .collect()
    }

    fn prepare_step<'a>(
        &self,
        step: &'a StepConfig,
        ids: &mut HashSet<&'a str>,
    ) -> io::Result<PreparedStep> {
        if step.id.trim().is_empty() || !ids.insert(&step.id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "step IDs must be nonempty and unique",
            ));
        }

        let operation = self.operations.get(&step.kind).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown operation {:?}", step.kind),
            )
        })?;

        operation.prepare(step)
    }
}

#[cfg(test)]
#[path = "flow.tests.rs"]
mod tests;
