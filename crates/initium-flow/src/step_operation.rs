use std::io;

use initium_config::StepConfig;

pub type PreparedStep = Box<dyn FnOnce() -> io::Result<()>>;

pub trait StepOperation {
    fn prepare(&self, step: &StepConfig) -> io::Result<PreparedStep>;
}
