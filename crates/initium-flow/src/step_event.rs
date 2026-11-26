use std::io;

#[derive(Debug)]
pub enum StepEvent<'a> {
    Started { id: &'a str },
    Succeeded { id: &'a str },
    Failed { id: &'a str, source: &'a io::Error },
}
