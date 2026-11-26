use std::error::Error;
use std::io;
use std::process::ExitCode;

use initium_config::Config;
use initium_flow::{CommandStep, Flow, MountStep, StepEvent};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("initium: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .filter(|_| args.next().is_none())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "usage: initium <config.toml>")
        })?;

    let config = Config::parse(&std::fs::read_to_string(path)?)?;
    let mut flow = Flow::default();
    flow.register("command", CommandStep);
    flow.register("mount", MountStep);
    flow.run(&config.steps, report)?;
    Ok(())
}

fn report(event: StepEvent<'_>) {
    match event {
        StepEvent::Started { id } => eprintln!("initium: {id}: started"),
        StepEvent::Succeeded { id } => eprintln!("initium: {id}: succeeded"),
        StepEvent::Failed { id, source } => eprintln!("initium: {id}: failed: {source}"),
    }
}
