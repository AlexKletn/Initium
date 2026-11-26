use std::error::Error;
use std::io;
use std::process::ExitCode;

use initium_config::{Config, Theme};
use initium_gui::{initialize, show_shutdown};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("initium-shutdown: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let (Some(path), Some(action), None) = (args.next(), args.next(), args.next()) else {
        return Err("usage: initium-shutdown <config.toml> <poweroff|reboot|--blank>".into());
    };

    let config = Config::parse(&std::fs::read_to_string(path)?)?;
    let shutdown = config.shutdown.ok_or("missing [shutdown] configuration")?;
    if action == "--blank" {
        if let Some(path) = shutdown.backlight_brightness {
            std::fs::write(path, b"0\n")?;
        }
    } else {
        let action = action
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid action encoding"))?
            .parse()?;
        initialize()?;
        show_shutdown(&shutdown, action, Theme::load_system()?)?;
    }
    Ok(())
}
