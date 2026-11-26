use std::path::Path;
use std::time::{Duration, Instant};

const CHECK_INTERVAL: Duration = Duration::from_millis(16);

pub(super) struct Handoff<'a> {
    directory: Option<&'a Path>,
    check_after: Instant,
    locked: bool,
    switching_root: bool,
    display_manager_started: bool,
    greeter_presented: bool,
}

impl<'a> Handoff<'a> {
    pub fn new(directory: Option<&'a Path>) -> Self {
        Self {
            directory,
            check_after: Instant::now(),
            locked: false,
            switching_root: false,
            display_manager_started: false,
            greeter_presented: false,
        }
    }

    pub fn poll(&mut self, now: Instant) {
        if !self.greeter_presented
            && now >= self.check_after
            && let Some(directory) = self.directory
        {
            self.check_after = now + CHECK_INTERVAL;
            if let Ok(stage) = std::fs::read_to_string(directory.join("stage")) {
                let stage = stage.trim();
                self.switching_root |= switch_root_started(stage);
                self.display_manager_started |=
                    matches!(stage, "display-manager-started" | "greeter-presented");
                self.greeter_presented |= stage == "greeter-presented";
                self.locked |= self.switching_root
                    || (stage == "root-ready"
                        && matches!(directory.join("boot-pause").try_exists(), Ok(false)));
            }
        }
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub fn switching_root(&self) -> bool {
        self.switching_root
    }

    pub fn display_manager_started(&self) -> bool {
        self.display_manager_started
    }

    pub fn greeter_presented(&self) -> bool {
        self.greeter_presented
    }
}

fn switch_root_started(stage: &str) -> bool {
    matches!(
        stage,
        "switching-root" | "userspace" | "display-manager-started" | "greeter-presented"
    )
}

#[cfg(test)]
#[path = "handoff.tests.rs"]
mod tests;
