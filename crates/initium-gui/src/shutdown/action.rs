use std::str::FromStr;

#[derive(Clone, Copy)]
pub enum ShutdownAction {
    Poweroff,
    Reboot,
}

impl FromStr for ShutdownAction {
    type Err = std::io::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "poweroff" => Ok(Self::Poweroff),
            "reboot" => Ok(Self::Reboot),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "shutdown action must be poweroff or reboot",
            )),
        }
    }
}
