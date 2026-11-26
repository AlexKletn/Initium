use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum FlowError {
    InvalidStep { id: String, source: io::Error },
    FailedStep { id: String, source: io::Error },
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStep { id, source } => write!(f, "invalid boot step {id:?}: {source}"),
            Self::FailedStep { id, source } => write!(f, "boot step {id:?} failed: {source}"),
        }
    }
}

impl Error for FlowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidStep { source, .. } | Self::FailedStep { source, .. } => Some(source),
        }
    }
}
